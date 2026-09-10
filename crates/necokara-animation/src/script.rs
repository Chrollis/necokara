//! Script-calling layer: group lines by script and invoke the Python preset.
//!
//! The application layer extracts per-line data (character indices, times,
//! singing window) and hands it here as [`LineInputs`]. [`group_by_script`]
//! resolves each line's script name and window extension from
//! [`LineAnimAlloc`] and buckets the lines so every script runs once with a
//! batch of lines. [`run_script`] then invokes the script through
//! `necokara-spawn` (stdin JSON, stdout envelope) and converts the response.

use std::collections::BTreeMap;
use std::path::Path;

use necokara_error::CkError;
use necokara_spawn::ProcessInput;

use crate::alloc::LineAnimAlloc;
use crate::json;
use crate::keyframe::LineAnimation;
use crate::script_manifest::AnimationScript;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// A batch passed to [`run_script`] mixed several scripts.
    pub const MIXED_SCRIPTS: &str = ck_code!("necokara-animation", script, mixed_scripts);
    /// The script output does not cover exactly the requested lines.
    pub const OUTPUT_MISMATCH: &str = ck_code!("necokara-animation", script, output_mismatch);
    /// A script name has no entry in the preset manifest.
    pub const UNKNOWN_SCRIPT: &str = ck_code!("necokara-animation", script, unknown_script);
}

/// One main character's timing, in full main-stream indices.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MainCharInput {
    /// Full main character-stream index.
    pub char_index: usize,
    /// Start time in ms.
    pub start_ms: i64,
    /// Explicit duration in ms; `0` means unset.
    pub duration_ms: u32,
}

/// One ruby character's timing, in full ruby-stream indices.
///
/// Ruby is independent of its owning main character for animation purposes:
/// it carries its own `start_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RubyCharInput {
    /// Full ruby character-stream index.
    pub ruby_index: usize,
    /// Start time in ms.
    pub start_ms: i64,
    /// Explicit duration in ms; `0` means unset.
    pub duration_ms: u32,
}

/// One line's raw inputs, before the script and window extension are resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineInputs {
    /// Line index (see `necokara_lyrics::Lyrics::line_ranges`).
    pub line_index: usize,
    /// Line singing start in ms.
    pub line_start_ms: i64,
    /// Line singing end in ms.
    pub line_end_ms: i64,
    /// Timed main characters of the line.
    pub main: Vec<MainCharInput>,
    /// Timed ruby characters of the line.
    pub ruby: Vec<RubyCharInput>,
}

/// One line ready for a script call: script name plus resolved window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptLineInput {
    /// Line index.
    pub line_index: usize,
    /// Script name (file name without `.py`), e.g. `FadeIn`.
    pub script: String,
    /// Line singing start in ms.
    pub line_start_ms: i64,
    /// Line singing end in ms.
    pub line_end_ms: i64,
    /// Animation-window extension before the singing start, from the alloc run.
    pub lead_in_ms: i64,
    /// Animation-window extension after the singing end, from the alloc run.
    pub lead_out_ms: i64,
    /// Timed main characters of the line.
    pub main: Vec<MainCharInput>,
    /// Timed ruby characters of the line.
    pub ruby: Vec<RubyCharInput>,
}

/// Bucket lines by the script assigned in `alloc`.
///
/// Lines without an allocation have no animation and are skipped. The result
/// is keyed by script name and sorted (a `BTreeMap`), so call order is stable;
/// line order inside each bucket follows the input order.
pub fn group_by_script(
    alloc: &LineAnimAlloc,
    lines: impl IntoIterator<Item = LineInputs>,
) -> BTreeMap<String, Vec<ScriptLineInput>> {
    let mut groups: BTreeMap<String, Vec<ScriptLineInput>> = BTreeMap::new();

    for line in lines {
        let Some(run) = alloc.run_at(line.line_index) else {
            continue;
        };
        let script = run.script.clone();
        let lead_in_ms = run.lead_in_ms;
        let lead_out_ms = run.lead_out_ms;

        groups.entry(script.clone()).or_default().push(ScriptLineInput {
            line_index: line.line_index,
            script,
            line_start_ms: line.line_start_ms,
            line_end_ms: line.line_end_ms,
            lead_in_ms,
            lead_out_ms,
            main: line.main,
            ruby: line.ruby,
        });
    }

    groups
}

/// Invoke one preset script with a batch of lines and return its animations.
///
/// All lines must use the same script. An empty batch returns an empty result
/// without spawning anything.
pub fn run_script(
    python: &Path,
    script_path: &Path,
    fps: f64,
    lines: &[ScriptLineInput],
) -> Result<Vec<LineAnimation>, CkError> {
    if lines.is_empty() {
        return Ok(Vec::new());
    }

    let mut batch_script: Option<&str> = None;
    for line in lines {
        match batch_script {
            None => batch_script = Some(line.script.as_str()),
            Some(script) if script == line.script => {}
            Some(script) => {
                return Err(CkError::new(
                    codes::MIXED_SCRIPTS,
                    format!(
                        "run_script got script '{}' but line {} uses '{}'",
                        script, line.line_index, line.script
                    ),
                ))
            }
        }
    }

    let request = json::request_value(fps, lines)?;
    let payload = necokara_spawn::run_python_json(
        python,
        script_path,
        &[],
        ProcessInput::Json(&request),
    )?;
    let animations = json::line_animations_from_value(payload)?;

    check_output(lines, &animations)?;
    Ok(animations)
}

/// Run every script group, resolving script paths through the manifest.
///
/// Scripts are looked up by [`AnimationScript::script_name`]; a script without
/// a manifest entry is reported as [`codes::UNKNOWN_SCRIPT`].
pub fn run_groups(
    python: &Path,
    scripts_dir: &Path,
    manifest: &[AnimationScript],
    fps: f64,
    groups: BTreeMap<String, Vec<ScriptLineInput>>,
) -> Result<BTreeMap<String, Vec<LineAnimation>>, CkError> {
    let mut out = BTreeMap::new();

    for (script, lines) in groups {
        let entry = manifest
            .iter()
            .find(|entry| entry.script_name() == script)
            .ok_or_else(|| {
                CkError::new(
                    codes::UNKNOWN_SCRIPT,
                    format!("script '{script}' is not listed in the preset manifest"),
                )
            })?;
        let animations = run_script(python, &entry.path_in(scripts_dir), fps, &lines)?;
        out.insert(script, animations);
    }

    Ok(out)
}

/// Ensure the response covers exactly the requested lines, with matching
/// script names and no duplicates.
fn check_output(
    requested: &[ScriptLineInput],
    animations: &[LineAnimation],
) -> Result<(), CkError> {
    if animations.len() != requested.len() {
        return Err(CkError::new(
            codes::OUTPUT_MISMATCH,
            format!(
                "script returned {} line animation(s) for {} requested line(s)",
                animations.len(),
                requested.len()
            ),
        ));
    }

    for line in requested {
        let matches = animations
            .iter()
            .filter(|animation| animation.line_index == line.line_index)
            .count();
        if matches != 1 {
            return Err(CkError::new(
                codes::OUTPUT_MISMATCH,
                format!(
                    "line {} appeared {matches} time(s) in the script output",
                    line.line_index
                ),
            ));
        }

        let animation = animations
            .iter()
            .find(|animation| animation.line_index == line.line_index)
            .expect("count checked above");
        if animation.script != line.script {
            return Err(CkError::new(
                codes::OUTPUT_MISMATCH,
                format!(
                    "line {} returned script '{}' but '{}' was requested",
                    line.line_index, animation.script, line.script
                ),
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alloc::LineAnimRun;

    fn line(line_index: usize) -> LineInputs {
        LineInputs {
            line_index,
            line_start_ms: 1000 * line_index as i64,
            line_end_ms: 1000 * line_index as i64 + 800,
            main: vec![MainCharInput {
                char_index: line_index * 4,
                start_ms: 1000 * line_index as i64,
                duration_ms: 200,
            }],
            ruby: Vec::new(),
        }
    }

    #[test]
    fn groups_lines_by_script_and_resolves_the_window() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(0, 2, "FadeIn", 200, 100);
        alloc.set(2, 3, "PerCharFade", 50, 400);

        let groups = group_by_script(&alloc, (0..3).map(line));

        assert_eq!(groups.len(), 2);
        let fade_in = &groups["FadeIn"];
        assert_eq!(fade_in.len(), 2);
        assert_eq!(fade_in[0].line_index, 0);
        assert_eq!(fade_in[0].lead_in_ms, 200);
        assert_eq!(fade_in[0].lead_out_ms, 100);
        assert_eq!(fade_in[0].script, "FadeIn");
        assert_eq!(fade_in[0].main.len(), 1);

        let per_char = &groups["PerCharFade"];
        assert_eq!(per_char.len(), 1);
        assert_eq!(per_char[0].line_index, 2);
        assert_eq!(per_char[0].lead_in_ms, 50);
    }

    #[test]
    fn lines_without_allocation_are_skipped() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(1, 2, "FadeOut", 0, 0);

        let groups = group_by_script(&alloc, (0..3).map(line));

        assert_eq!(groups.len(), 1);
        assert_eq!(groups["FadeOut"].len(), 1);
        assert_eq!(groups["FadeOut"][0].line_index, 1);
    }

    #[test]
    fn group_keys_are_sorted_and_batches_preserve_line_order() {
        let mut alloc = LineAnimAlloc::new();
        alloc.set(0, 1, "Zeta", 0, 0);
        alloc.set(1, 3, "Alpha", 0, 0);

        let groups = group_by_script(&alloc, (0..3).map(line));

        let keys: Vec<&String> = groups.keys().collect();
        assert_eq!(keys, vec!["Alpha", "Zeta"]);
        assert_eq!(
            groups["Alpha"]
                .iter()
                .map(|line| line.line_index)
                .collect::<Vec<_>>(),
            vec![1, 2]
        );
    }

    #[test]
    fn custom_run_constructors_keep_window_fields() {
        let run = LineAnimRun::new(4, 6, "MyScript", 10, 20);
        assert_eq!(run.script, "MyScript");
        assert!(run.contains(5));
    }

    #[test]
    fn empty_batch_returns_empty_without_spawning() {
        let result = run_script(
            Path::new("definitely-missing-python"),
            Path::new("FadeIn.py"),
            30.0,
            &[],
        )
        .expect("empty batch must not spawn");
        assert!(result.is_empty());
    }

    #[test]
    fn mixed_scripts_are_rejected_before_spawning() {
        let a = ScriptLineInput {
            line_index: 0,
            script: "FadeIn".to_string(),
            line_start_ms: 0,
            line_end_ms: 100,
            lead_in_ms: 0,
            lead_out_ms: 0,
            main: Vec::new(),
            ruby: Vec::new(),
        };
        let mut b = a.clone();
        b.line_index = 1;
        b.script = "FadeOut".to_string();

        let err = run_script(
            Path::new("definitely-missing-python"),
            Path::new("FadeIn.py"),
            30.0,
            &[a, b],
        )
        .unwrap_err();
        assert_eq!(err.code, codes::MIXED_SCRIPTS);
    }

    #[test]
    fn check_output_rejects_missing_and_duplicate_lines() {
        let requested = vec![ScriptLineInput {
            line_index: 0,
            script: "FadeIn".to_string(),
            line_start_ms: 0,
            line_end_ms: 100,
            lead_in_ms: 0,
            lead_out_ms: 0,
            main: Vec::new(),
            ruby: Vec::new(),
        }];

        let err = check_output(&requested, &[]).unwrap_err();
        assert_eq!(err.code, codes::OUTPUT_MISMATCH);
    }

    #[test]
    fn unknown_script_in_manifest_is_reported() {
        let mut groups = BTreeMap::new();
        groups.insert("NotInManifest".to_string(), Vec::new());

        let err = run_groups(
            Path::new("definitely-missing-python"),
            Path::new("python/scripts/presets"),
            &[],
            30.0,
            groups,
        )
        .unwrap_err();
        assert_eq!(err.code, codes::UNKNOWN_SCRIPT);
    }
}
