use std::{fs, path::Path};

use crate::util::{MARKER_FILE_NAME, atomic_swap, find_targets, load_manifest};

fn write(path: &Path, content: &str) {
    fs::write(path, content).unwrap();
}

fn names(found: &[std::path::PathBuf]) -> Vec<String> {
    found
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect()
}

// ---------------------------------------------------------------- atomic_swap

#[test]
fn swap_exchanges_two_directories() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(&a.path().join("file"), "A");
    write(&b.path().join("file"), "B");

    atomic_swap(a.path(), b.path()).unwrap();

    assert_eq!(fs::read_to_string(a.path().join("file")).unwrap(), "B");
    assert_eq!(fs::read_to_string(b.path().join("file")).unwrap(), "A");
}

#[test]
fn swap_works_when_one_dir_is_empty() {
    let a = tempfile::tempdir().unwrap();
    let b = tempfile::tempdir().unwrap();
    write(&a.path().join("file"), "A");

    atomic_swap(a.path(), b.path()).unwrap();

    assert!(!a.path().join("file").exists());
    assert_eq!(fs::read_to_string(b.path().join("file")).unwrap(), "A");
}


// ---------------------------------------------------------------- find_targets

#[test]
fn find_targets_skips_git_and_manifest_files() {
    let dir = tempfile::tempdir().unwrap();
    for ignored in [
        "tape.toml",
        ".git",
        ".gitattributes",
        ".gitignore",
        ".github",
        ".editorconfig",
        "README.md",
        "LICENSE",
        MARKER_FILE_NAME,
    ] {
        write(&dir.path().join(ignored), "ignore me");
    }
    fs::create_dir_all(dir.path().join("hypr")).unwrap();
    write(&dir.path().join("hypr/hyprland.conf"), "conf");

    let found = find_targets(dir.path()).unwrap();

    assert_eq!(names(&found), vec!["hypr".to_string()]);
}

#[test]
fn find_targets_includes_plain_files() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("starship.toml"), "prompt");
    write(&dir.path().join(".zshrc"), "zsh");

    let found = find_targets(dir.path()).unwrap();

    assert_eq!(names(&found), vec![".zshrc".to_string(), "starship.toml".to_string()]);
}

#[test]
fn find_targets_returns_empty_for_missing_dir() {
    let missing = tempfile::tempdir().unwrap().path().join("does-not-exist");
    let found = find_targets(&missing).unwrap();
    assert!(found.is_empty());
}

#[test]
fn find_targets_returns_empty_for_empty_dir() {
    let dir = tempfile::tempdir().unwrap();
    let found = find_targets(dir.path()).unwrap();
    assert!(found.is_empty());
}

// ---------------------------------------------------------------- load_manifest

#[test]
fn load_manifest_parses_full_example() {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir.path().join("tape.toml"),
        r#"
[tape]
name = "cyberpunk"
desc = "A cool cyberpunk rice"
dependencies = ["hyprland", "quickshell"]

[hooks]
insert = ["hyprctl reload"]
eject = ["hyprctl reload"]
install = []
"#,
    );

    let manifest = load_manifest(dir.path()).unwrap();

    assert_eq!(manifest.tape.name, "cyberpunk");
    assert_eq!(manifest.tape.desc, "A cool cyberpunk rice");
    let hooks = manifest.hooks.expect("hooks should parse");
    assert_eq!(hooks.insert, vec!["hyprctl reload".to_string()]);
    assert_eq!(hooks.eject, vec!["hyprctl reload".to_string()]);
    assert!(hooks.install.is_empty());
}

#[test]
fn load_manifest_accepts_minimal_manifest_without_hooks() {
    let dir = tempfile::tempdir().unwrap();
    write(
        &dir.path().join("tape.toml"),
        "[tape]\nname = \"mini\"\ndesc = \"bare bones\"\n",
    );

    let manifest = load_manifest(dir.path()).unwrap();

    assert_eq!(manifest.tape.name, "mini");
    assert!(manifest.hooks.is_none());
    assert!(manifest.tape.dependencies.is_none());
}

#[test]
fn load_manifest_missing_file_is_error() {
    let dir = tempfile::tempdir().unwrap();
    let err = load_manifest(dir.path()).unwrap_err();
    assert!(err.to_string().contains("tape.toml"));
}

#[test]
fn load_manifest_malformed_toml_is_error_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    write(&dir.path().join("tape.toml"), "{{{{ definitely not toml");
    assert!(load_manifest(dir.path()).is_err());
}

#[test]
fn load_manifest_missing_required_field_is_error() {
    let dir = tempfile::tempdir().unwrap();
    // `name` is required; a manifest without it must fail cleanly
    write(&dir.path().join("tape.toml"), "[tape]\ndesc = \"no name\"\n");
    assert!(load_manifest(dir.path()).is_err());
}
