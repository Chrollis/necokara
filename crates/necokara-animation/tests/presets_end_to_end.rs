//! End-to-end test: run the shipped preset scripts through `run_script`.
//!
//! The test is skipped (not failed) when the interpreter cannot be spawned at
//! all, so a machine without Python still builds. Set `NECOKARA_TEST_PYTHON`
//! to point at a specific interpreter.

use std::path::{Path, PathBuf};

use necokara_animation::{
    run_script, LineAnimation, MainCharInput, ScriptLineInput, StateKeyframe,
};

fn presets_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("python")
        .join("scripts")
        .join("presets")
}

fn python() -> PathBuf {
    std::env::var_os("NECOKARA_TEST_PYTHON")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("python"))
}

/// Run a script, skipping the test when no interpreter is available.
fn run_or_skip(
    script: &str,
    fps: f64,
    lines: &[ScriptLineInput],
) -> Option<Vec<LineAnimation>> {
    let path = presets_dir().join(script);
    match run_script(&python(), &path, fps, lines) {
        Ok(animations) => Some(animations),
        Err(err) if err.code.ends_with("spawn_failed") => {
            eprintln!("skipping end-to-end test: could not spawn python ({err})");
            None
        }
        Err(err) => panic!("{script} failed: {err}"),
    }
}

fn line(script: &str, lead_in_ms: i64, lead_out_ms: i64) -> ScriptLineInput {
    ScriptLineInput {
        line_index: 7,
        script: script.to_string(),
        line_start_ms: 1000,
        line_end_ms: 2000,
        lead_in_ms,
        lead_out_ms,
        main: vec![
            MainCharInput {
                char_index: 0,
                start_ms: 1000,
                duration_ms: 300,
            },
            MainCharInput {
                char_index: 1,
                start_ms: 1300,
                duration_ms: 200,
            },
        ],
        ruby: vec![necokara_animation::RubyCharInput {
            ruby_index: 0,
            start_ms: 1000,
            duration_ms: 150,
        }],
    }
}

fn opacity_at(keyframes: &[StateKeyframe], time_ms: i64) -> Option<f64> {
    necokara_animation::sample_state(keyframes, time_ms).map(|state| state.opacity)
}

#[test]
fn fade_in_fades_from_window_start_to_line_start() {
    let Some(animations) = run_or_skip("FadeIn.py", 30.0, &[line("FadeIn", 200, 0)]) else {
        return;
    };
    let anim = &animations[0];
    assert_eq!(anim.line_window(), Some((800, 2000)));
    assert_eq!(anim.animation_in_ms, 200);
    assert_eq!(opacity_at(&anim.line, 800), Some(0.0));
    assert_eq!(opacity_at(&anim.line, 1000), Some(1.0));
    assert_eq!(opacity_at(&anim.line, 2000), Some(1.0));
    // Whole-line animation: no per-character tracks.
    assert!(anim.main.is_empty());
    assert!(anim.ruby.is_empty());
}

#[test]
fn fade_out_holds_then_fades_over_lead_out() {
    let Some(animations) = run_or_skip("FadeOut.py", 30.0, &[line("FadeOut", 0, 300)]) else {
        return;
    };
    let anim = &animations[0];
    assert_eq!(anim.line_window(), Some((1000, 2300)));
    assert_eq!(anim.animation_out_ms, 300);
    assert_eq!(opacity_at(&anim.line, 1000), Some(1.0));
    assert_eq!(opacity_at(&anim.line, 2000), Some(1.0));
    assert_eq!(opacity_at(&anim.line, 2300), Some(0.0));
}

#[test]
fn fade_in_out_fades_both_ends() {
    let Some(animations) = run_or_skip("FadeInOut.py", 30.0, &[line("FadeInOut", 200, 300)])
    else {
        return;
    };
    let anim = &animations[0];
    assert_eq!(anim.line_window(), Some((800, 2300)));
    assert_eq!(opacity_at(&anim.line, 800), Some(0.0));
    assert_eq!(opacity_at(&anim.line, 1000), Some(1.0));
    assert_eq!(opacity_at(&anim.line, 2000), Some(1.0));
    assert_eq!(opacity_at(&anim.line, 2300), Some(0.0));
}

#[test]
fn per_char_fade_animates_each_character() {
    let Some(animations) = run_or_skip("PerCharFade.py", 30.0, &[line("PerCharFade", 100, 200)])
    else {
        return;
    };
    let anim = &animations[0];
    assert_eq!(anim.line_window(), Some((900, 2200)));

    // Per-character fade-in, staggered by each character's own start.
    assert_eq!(anim.main.len(), 2);
    assert_eq!(opacity_at(&anim.main[0].keyframes, 900), Some(0.0));
    assert_eq!(opacity_at(&anim.main[0].keyframes, 1000), Some(1.0));
    assert_eq!(opacity_at(&anim.main[1].keyframes, 1300), Some(1.0));

    // Ruby fades with its own timing.
    assert_eq!(anim.ruby.len(), 1);
    assert_eq!(opacity_at(&anim.ruby[0].keyframes, 900), Some(0.0));
    assert_eq!(opacity_at(&anim.ruby[0].keyframes, 1000), Some(1.0));

    // The line holds, then fades out.
    assert_eq!(opacity_at(&anim.line, 2000), Some(1.0));
    assert_eq!(opacity_at(&anim.line, 2200), Some(0.0));

    // idle = animation_end - animation_out - (animation_start + animation_in)
    let idle = anim.line_window().unwrap().1 - anim.animation_out_ms
        - (anim.line_window().unwrap().0 + anim.animation_in_ms);
    assert_eq!(idle, 0, "per-character fade should leave no idle");
}

#[test]
fn every_shipped_preset_accepts_a_batch_of_lines() {
    let scripts = ["FadeIn.py", "FadeOut.py", "FadeInOut.py", "PerCharFade.py"];
    for script in scripts {
        let name = script.strip_suffix(".py").unwrap();
        let batch = [line(name, 120, 180), {
            let mut second = line(name, 120, 180);
            second.line_index = 8;
            second
        }];
        let Some(animations) = run_or_skip(script, 30.0, &batch) else {
            return;
        };
        assert_eq!(animations.len(), 2, "{script}");
        assert_eq!(animations[0].line_index, 7, "{script}");
        assert_eq!(animations[1].line_index, 8, "{script}");
        assert_eq!(animations[0].script, name, "{script}");
    }
}
