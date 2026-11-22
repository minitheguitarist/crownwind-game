use std::path::Path;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_asset_pipeline"))
        .current_dir(Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."))
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn shipped_manifests_pass_both_entry_points() {
    for args in [
        &[][..],
        &["animation", "content/characters/prince_debug.json"],
        &["level", "content/levels/wind_steps.json"],
    ] {
        let result = run(args);
        assert!(
            result.status.success(),
            "{}",
            String::from_utf8_lossy(&result.stderr)
        );
    }
}

#[test]
fn invalid_input_reports_filename_and_issue() {
    let path = "content/levels/wind_steps.json";
    let result = run(&["animation", path]);
    let issue = String::from_utf8(result.stderr).unwrap();
    assert_eq!(result.status.code(), Some(1));
    assert!(
        issue.contains(path) && issue.contains("character_id"),
        "{issue}"
    );
    assert_eq!(
        run(&["level", "content/levels/missing.file"]).status.code(),
        Some(1)
    );
}

#[test]
fn usage_exit_codes() {
    assert!(run(&["--help"]).status.success());
    assert_eq!(run(&["animation"]).status.code(), Some(2));
}
