//! Integration test against the real preset manifest shipped in the repo.

use std::path::Path;

use necokara_animation::{load_script_manifest, script_name_of};

/// Every preset in the manifest must be runnable. The manifest is the
/// authoritative list of animation scripts, so this test only asserts that the
/// shipped scripts are enumerable and named as expected.
#[test]
fn repo_preset_manifest_loads() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("python")
        .join("scripts")
        .join("presets");

    let scripts = load_script_manifest(&dir).expect("repo presets manifest must load");
    assert!(
        !scripts.is_empty(),
        "preset manifest should list at least one script"
    );

    for script in &scripts {
        // The loader only returns scripts whose file exists.
        assert!(script.path_in(&dir).is_file(), "{} missing", script.file_name);
        assert!(!script.display_name("en_US").is_empty());
        assert!(!script.script_name().is_empty());
        // The script name is derived from the file name, not hand-written.
        assert_eq!(script.script_name(), script_name_of(&script.file_name));
    }

    let names: Vec<&str> = scripts.iter().map(|s| s.script_name()).collect();
    assert!(names.contains(&"FadeIn"), "names: {names:?}");
    assert!(names.contains(&"FadeOut"), "names: {names:?}");
    assert!(names.contains(&"FadeInOut"), "names: {names:?}");
    assert!(names.contains(&"PerCharFade"), "names: {names:?}");
}
