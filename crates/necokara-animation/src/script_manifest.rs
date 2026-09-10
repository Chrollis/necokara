//! Animation script manifest: enumerate the animation scripts shipped in the
//! presets folder.
//!
//! The scripts directory (by convention `python/scripts/presets/`) contains a
//! `manifest.json` describing every available animation script:
//!
//! ```json
//! [
//!   {
//!     "name": { "en_US": "Fade In", "ja_JP": "フェードイン" },
//!     "file_name": "FadeIn.py"
//!   }
//! ]
//! ```
//!
//! - `file_name` is the script's identity: it is the plain file name inside
//!   the scripts directory, so it is unique by construction. Any stable key
//!   the application needs is derived from it in code, not hand-written in the
//!   manifest.
//! - `name` maps locale codes to display names. Locales are dynamic: any
//!   locale may be present and none is required, so a UI that finds no
//!   translation falls back to the script name.
//!
//! [`parse_script_manifest`] validates the JSON without touching the file
//! system; [`load_script_manifest`] reads `<scripts_dir>/manifest.json` and
//! keeps only entries whose script file actually exists (missing scripts are
//! silently skipped).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use necokara_error::CkError;

/// Error codes used by this module.
pub mod codes {
    use necokara_error::ck_code;

    /// `manifest.json` could not be read.
    pub const READ_FAILED: &str = ck_code!("necokara-animation", script_manifest, read_failed);
    /// The manifest is not valid JSON.
    pub const INVALID_JSON: &str = ck_code!("necokara-animation", script_manifest, invalid_json);
    /// The manifest's top level is not a JSON array.
    pub const NOT_AN_ARRAY: &str = ck_code!("necokara-animation", script_manifest, not_an_array);
    /// A manifest entry is not a JSON object.
    pub const ENTRY_NOT_OBJECT: &str =
        ck_code!("necokara-animation", script_manifest, entry_not_object);
    /// A required field is missing.
    pub const MISSING_FIELD: &str = ck_code!("necokara-animation", script_manifest, missing_field);
    /// A field has the wrong JSON type.
    pub const WRONG_TYPE: &str = ck_code!("necokara-animation", script_manifest, wrong_type);
    /// A localized name is an empty string.
    pub const EMPTY_NAME: &str = ck_code!("necokara-animation", script_manifest, empty_name);
    /// `file_name` is not a plain `.py` file name.
    pub const INVALID_FILE_NAME: &str =
        ck_code!("necokara-animation", script_manifest, invalid_file_name);
    /// `file_name` appears more than once.
    pub const DUPLICATE_FILE_NAME: &str =
        ck_code!("necokara-animation", script_manifest, duplicate_file_name);
}

/// Manifest file name expected inside the scripts directory.
pub const MANIFEST_FILE_NAME: &str = "manifest.json";

/// Locale used as a secondary fallback for [`AnimationScript::display_name`].
pub const DEFAULT_LOCALE: &str = "en_US";

/// Localized display names: locale code -> text.
pub type LocalizedNames = BTreeMap<String, String>;

/// Strip the `.py` extension from a script file name to get its stable script
/// name, e.g. `FadeIn.py` -> `FadeIn`.
///
/// This is the key stored in project files and in [`crate::LineAnimRun`].
pub fn script_name_of(file_name: &str) -> &str {
    file_name.strip_suffix(".py").unwrap_or(file_name)
}

/// One animation script declared by `manifest.json`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AnimationScript {
    /// Localized display names, e.g. `en_US -> "Fade In"`.
    pub name: LocalizedNames,
    /// Plain script file name inside the scripts directory, e.g. `FadeIn.py`.
    ///
    /// This doubles as the script's identity: file names are unique within one
    /// directory, so no separate manifest id is needed. Stable keys used in
    /// project files are derived from this in code.
    pub file_name: String,
}

impl AnimationScript {
    /// Build a manifest entry.
    pub fn new(name: LocalizedNames, file_name: impl Into<String>) -> Self {
        Self {
            name,
            file_name: file_name.into(),
        }
    }

    /// Display name for exactly `locale`, if present.
    pub fn name_for(&self, locale: &str) -> Option<&str> {
        self.name.get(locale).map(String::as_str)
    }

    /// The script's code-facing name: `file_name` without the `.py` extension,
    /// e.g. `FadeIn`.
    pub fn script_name(&self) -> &str {
        script_name_of(&self.file_name)
    }

    /// Best display name for `locale`: the exact locale, then
    /// [`DEFAULT_LOCALE`], then [`AnimationScript::script_name`].
    pub fn display_name(&self, locale: &str) -> &str {
        self.name_for(locale)
            .or_else(|| self.name_for(DEFAULT_LOCALE))
            .unwrap_or_else(|| self.script_name())
    }

    /// Path of this script inside `scripts_dir`.
    pub fn path_in(&self, scripts_dir: &Path) -> PathBuf {
        scripts_dir.join(&self.file_name)
    }
}

/// Read and validate `<scripts_dir>/manifest.json`.
///
/// Entries whose script file does not exist are silently skipped, so the
/// result only lists scripts that can actually be run.
pub fn load_script_manifest(scripts_dir: &Path) -> Result<Vec<AnimationScript>, CkError> {
    let manifest_path = scripts_dir.join(MANIFEST_FILE_NAME);
    let json = std::fs::read_to_string(&manifest_path).map_err(|e| {
        CkError::new(
            codes::READ_FAILED,
            format!("could not read animation manifest: {}", manifest_path.display()),
        )
        .with_source(e.to_string())
    })?;

    let scripts = parse_script_manifest(&json)?;
    Ok(scripts
        .into_iter()
        .filter(|script| script.path_in(scripts_dir).is_file())
        .collect())
}

/// Parse and validate a manifest JSON string without touching the file system.
pub fn parse_script_manifest(json: &str) -> Result<Vec<AnimationScript>, CkError> {
    let root: serde_json::Value = serde_json::from_str(json).map_err(|e| {
        CkError::new(codes::INVALID_JSON, "animation manifest is not valid JSON")
            .with_source(e.to_string())
    })?;

    let entries = root.as_array().ok_or_else(|| {
        CkError::new(
            codes::NOT_AN_ARRAY,
            "animation manifest must be a JSON array",
        )
    })?;

    let mut scripts: Vec<AnimationScript> = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        let object = entry.as_object().ok_or_else(|| {
            entry_error(
                codes::ENTRY_NOT_OBJECT,
                index,
                "manifest entry is not a JSON object",
            )
        })?;

        let name = parse_names(object.get("name"), index)?;
        let file_name = require_string(object, "file_name", index)?;
        validate_file_name(&file_name, index)?;

        if scripts
            .iter()
            .any(|script| script.file_name == file_name)
        {
            return Err(entry_error(
                codes::DUPLICATE_FILE_NAME,
                index,
                &format!("duplicate file_name '{file_name}'"),
            ));
        }

        scripts.push(AnimationScript::new(name, file_name));
    }

    Ok(scripts)
}

/// Read a required string field from a manifest entry.
fn require_string(
    object: &serde_json::Map<String, serde_json::Value>,
    key: &str,
    index: usize,
) -> Result<String, CkError> {
    match object.get(key) {
        None => Err(entry_error(
            codes::MISSING_FIELD,
            index,
            &format!("missing required field '{key}'"),
        )),
        Some(value) => value.as_str().map(str::to_string).ok_or_else(|| {
            entry_error(
                codes::WRONG_TYPE,
                index,
                &format!("field '{key}' must be a string"),
            )
        }),
    }
}

/// Parse the `name` map. Locales are dynamic: any key is accepted, none is
/// required, and an empty map is allowed (the UI then falls back to the script
/// name).
fn parse_names(
    value: Option<&serde_json::Value>,
    index: usize,
) -> Result<LocalizedNames, CkError> {
    let object = match value {
        None => {
            return Err(entry_error(
                codes::MISSING_FIELD,
                index,
                "missing required field 'name'",
            ))
        }
        Some(value) => value.as_object().ok_or_else(|| {
            entry_error(
                codes::WRONG_TYPE,
                index,
                "field 'name' must be an object of locale -> string",
            )
        })?,
    };

    let mut names = LocalizedNames::new();
    for (locale, raw) in object {
        let text = raw.as_str().ok_or_else(|| {
            entry_error(
                codes::WRONG_TYPE,
                index,
                &format!("name.{locale} must be a string"),
            )
        })?;
        if text.is_empty() {
            return Err(entry_error(
                codes::EMPTY_NAME,
                index,
                &format!("name.{locale} must not be empty"),
            ));
        }
        names.insert(locale.clone(), text.to_string());
    }
    Ok(names)
}

/// Validate a script file name: a plain `.py` name with no path separators.
fn validate_file_name(file_name: &str, index: usize) -> Result<(), CkError> {
    let plain = !file_name.is_empty()
        && file_name != "."
        && file_name != ".."
        && !file_name.contains('/')
        && !file_name.contains('\\')
        && !file_name.contains(':')
        && file_name.trim() == file_name;
    if plain && file_name.ends_with(".py") {
        Ok(())
    } else {
        Err(entry_error(
            codes::INVALID_FILE_NAME,
            index,
            &format!("file_name '{file_name}' must be a plain .py file name"),
        ))
    }
}

/// Build an error that points at one manifest entry.
fn entry_error(code: &'static str, index: usize, message: &str) -> CkError {
    CkError::new(code, format!("manifest[{index}]: {message}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_json() -> &'static str {
        r#"[
            {
                "name": { "en_US": "Fade In", "ja_JP": "フェードイン" },
                "file_name": "FadeIn.py"
            },
            {
                "name": { "zh_CN": "逐字淡出" },
                "file_name": "PerCharFade.py"
            }
        ]"#
    }

    #[test]
    fn parses_valid_manifest_in_order() {
        let scripts = parse_script_manifest(valid_json()).expect("valid manifest");
        assert_eq!(scripts.len(), 2);
        assert_eq!(scripts[0].file_name, "FadeIn.py");
        assert_eq!(scripts[0].script_name(), "FadeIn");
        assert_eq!(scripts[0].name_for("en_US"), Some("Fade In"));
        assert_eq!(scripts[0].name_for("ja_JP"), Some("フェードイン"));
        assert_eq!(scripts[1].file_name, "PerCharFade.py");
    }

    #[test]
    fn arbitrary_locales_and_missing_locales_are_allowed() {
        let json = r#"[
            {
                "name": { "ko_KR": "가", "pt_BR": "Fade" },
                "file_name": "X.py"
            },
            {
                "name": {},
                "file_name": "Y.py"
            }
        ]"#;
        let scripts = parse_script_manifest(json).expect("dynamic locales");
        assert_eq!(scripts[0].name_for("ko_KR"), Some("가"));
        assert!(scripts[1].name.is_empty());
    }

    #[test]
    fn display_name_falls_back_locale_then_default_then_script_name() {
        let script = AnimationScript::new(
            LocalizedNames::from([
                ("en_US".to_string(), "Fade In".to_string()),
                ("ja_JP".to_string(), "フェードイン".to_string()),
            ]),
            "FadeIn.py",
        );
        assert_eq!(script.display_name("ja_JP"), "フェードイン");
        // Falls back to en_US for unknown locales.
        assert_eq!(script.display_name("fr_FR"), "Fade In");

        let no_default = AnimationScript::new(
            LocalizedNames::from([("zh_CN".to_string(), "淡入".to_string())]),
            "OnlyZh.py",
        );
        // No translation at all -> the script name (file name without .py).
        assert_eq!(no_default.display_name("fr_FR"), "OnlyZh");
    }

    #[test]
    fn script_name_strips_extension() {
        assert_eq!(script_name_of("FadeIn.py"), "FadeIn");
        assert_eq!(script_name_of("FadeIn"), "FadeIn");
    }

    #[test]
    fn path_in_joins_scripts_dir() {
        let script = AnimationScript::new(LocalizedNames::new(), "FadeIn.py");
        assert_eq!(
            script.path_in(Path::new("python/scripts/presets")),
            PathBuf::from("python/scripts/presets").join("FadeIn.py")
        );
    }

    #[test]
    fn rejects_non_array_top_level() {
        let err = parse_script_manifest(r#"{"file_name":"X.py"}"#).unwrap_err();
        assert_eq!(err.code, codes::NOT_AN_ARRAY);
    }

    #[test]
    fn rejects_invalid_json() {
        let err = parse_script_manifest("{ not json").unwrap_err();
        assert_eq!(err.code, codes::INVALID_JSON);
    }

    #[test]
    fn rejects_entry_not_object() {
        let err = parse_script_manifest(r#"["nope"]"#).unwrap_err();
        assert_eq!(err.code, codes::ENTRY_NOT_OBJECT);
    }

    #[test]
    fn rejects_missing_field() {
        let err = parse_script_manifest(r#"[{"name":{}}]"#).unwrap_err();
        assert_eq!(err.code, codes::MISSING_FIELD);

        let err = parse_script_manifest(r#"[{"file_name":"X.py"}]"#).unwrap_err();
        assert_eq!(err.code, codes::MISSING_FIELD);
    }

    #[test]
    fn rejects_wrong_types() {
        let err = parse_script_manifest(r#"[{"name":{},"file_name":1}]"#).unwrap_err();
        assert_eq!(err.code, codes::WRONG_TYPE);

        let err = parse_script_manifest(r#"[{"name":[],"file_name":"X.py"}]"#).unwrap_err();
        assert_eq!(err.code, codes::WRONG_TYPE);

        let err =
            parse_script_manifest(r#"[{"name":{"en_US":1},"file_name":"X.py"}]"#).unwrap_err();
        assert_eq!(err.code, codes::WRONG_TYPE);
    }

    #[test]
    fn rejects_empty_name_value() {
        let err = parse_script_manifest(r#"[{"name":{"en_US":""},"file_name":"X.py"}]"#)
            .unwrap_err();
        assert_eq!(err.code, codes::EMPTY_NAME);
    }

    #[test]
    fn rejects_invalid_file_names() {
        for file_name in [
            "",
            "FadeIn",
            "FadeIn.txt",
            "sub/FadeIn.py",
            "sub\\FadeIn.py",
            "C:FadeIn.py",
            "../FadeIn.py",
        ] {
            let json = format!(
                r#"[{{"name":{{}},"file_name":"{}"}}]"#,
                file_name.replace('\\', "\\\\")
            );
            let err = parse_script_manifest(&json).unwrap_err();
            assert_eq!(
                err.code,
                codes::INVALID_FILE_NAME,
                "file_name {file_name:?} should be invalid"
            );
        }
    }

    #[test]
    fn rejects_duplicate_file_name() {
        let json = r#"[
            {"name":{},"file_name":"A.py"},
            {"name":{},"file_name":"A.py"}
        ]"#;
        let err = parse_script_manifest(json).unwrap_err();
        assert_eq!(err.code, codes::DUPLICATE_FILE_NAME);
    }

    #[test]
    fn load_reads_manifest_and_skips_missing_scripts() {
        let dir = temp_dir("load");
        std::fs::write(
            dir.join(MANIFEST_FILE_NAME),
            r#"[
                {"name":{"en_US":"Present"},"file_name":"Present.py"},
                {"name":{"en_US":"Missing"},"file_name":"Missing.py"}
            ]"#,
        )
        .expect("write manifest");
        std::fs::write(dir.join("Present.py"), "# stub\n").expect("write script");

        let scripts = load_script_manifest(&dir).expect("load manifest");
        assert_eq!(scripts.len(), 1);
        assert_eq!(scripts[0].file_name, "Present.py");

        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn load_reports_unreadable_manifest() {
        let dir = temp_dir("empty");
        let err = load_script_manifest(&dir).unwrap_err();
        assert_eq!(err.code, codes::READ_FAILED);
        std::fs::remove_dir_all(&dir).ok();
    }

    /// Create a unique temporary directory for a test.
    fn temp_dir(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let dir = std::env::temp_dir().join(format!(
            "necokara_anim_manifest_{tag}_{}_{}",
            std::process::id(),
            nanos
        ));
        std::fs::create_dir_all(&dir).expect("create temp dir");
        dir
    }
}
