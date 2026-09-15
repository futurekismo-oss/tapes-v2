//! Isolated insert/eject scenarios. Never touches real `$HOME`.
//!
//! Strategy: build a fake `{home, config_home, library}` under `tempfile`,
//! call `tapes::ops` directly. No env mutation, no network, no compositor.

use std::fs;

use tapes::ops::{EjectOptions, InsertOptions};
use tapes::{StorePaths, TapeName};

fn sandbox() -> (tempfile::TempDir, StorePaths) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("home");
    let config = home.join(".config");
    let library = tmp.path().join("data").join("tapes");
    fs::create_dir_all(&config).expect("config");
    let paths = StorePaths::at_roots(home, config, library);
    paths.ensure_dirs().expect("ensure");
    (tmp, paths)
}

fn write_tape(library: &std::path::Path, name: &str, rel: &str, body: &str) -> std::path::PathBuf {
    let dir = library.join(name);
    let path = dir.join(rel);
    fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    fs::write(&path, body).expect("write");
    dir
}

#[test]
fn manifest_less_rice_works_like_ii_end4_clone() {
    // Arrange: foreign rice with dots/.config layout and no tape.toml.
    let (_tmp, paths) = sandbox();
    let fork = write_tape(
        &paths.library_dir,
        "ii-fork",
        "dots/.config/quickshell/ii/shell.qml",
        "shell",
    );
    fs::create_dir_all(fork.join("dots/.config/hypr")).expect("hypr");
    fs::write(fork.join("dots/.config/hypr/hyprland.conf"), "hypr").expect("write");

    // Act.
    let report = tapes::ops::insert(
        &paths,
        &TapeName::new("ii-fork").expect("name"),
        &fork,
        InsertOptions::default(),
    )
    .expect("insert manifest-less");

    // Assert: both prefixed targets linked, nothing else in config touched.
    assert_eq!(report.targets.len(), 2);
    assert!(paths.config_home.join("quickshell/ii").is_symlink());
    assert!(paths.config_home.join("hypr").is_symlink());
}

#[test]
fn insert_is_idempotent_without_reinsert() {
    let (_tmp, paths) = sandbox();
    let source = write_tape(&paths.library_dir, "a", ".config/s/f", "v");
    let name = TapeName::new("a").expect("name");
    let first = tapes::ops::insert(&paths, &name, &source, InsertOptions::default()).expect("first");
    assert_eq!(first.targets.len(), 1);
    let second = tapes::ops::insert(&paths, &name, &source, InsertOptions::default()).expect("second");
    assert!(second
        .targets
        .iter()
        .all(|t| t.action == tapes::ops::TargetAction::Unchanged));
}

#[test]
fn user_files_outside_tape_are_preserved() {
    let (_tmp, paths) = sandbox();
    fs::create_dir_all(paths.config_home.join("chromium")).expect("chromium");
    fs::write(paths.config_home.join("chromium").join("keys"), "k").expect("write");
    let source = write_tape(&paths.library_dir, "dms", ".config/hypr/f", "rice");

    tapes::ops::insert(
        &paths,
        &TapeName::new("dms").expect("name"),
        &source,
        InsertOptions::default(),
    )
    .expect("insert");

    // Chromium untouched; hypr now a symlink.
    assert_eq!(
        fs::read_to_string(paths.config_home.join("chromium").join("keys")).expect("keys"),
        "k"
    );
    assert!(paths.config_home.join("hypr").is_symlink());

    tapes::ops::eject(
        &paths,
        Some(&TapeName::new("dms").expect("name")),
        EjectOptions::default(),
    )
    .expect("eject");
    assert_eq!(
        fs::read_to_string(paths.config_home.join("chromium").join("keys")).expect("keys2"),
        "k"
    );
}

#[test]
fn snapshot_captures_entries_and_reinserts() {
    let (_tmp, paths) = sandbox();
    fs::create_dir_all(paths.config_home.join("kitty")).expect("kitty");
    fs::write(paths.config_home.join("kitty").join("kitty.conf"), "font 12").expect("write");

    let dest = tapes::ops::snapshot(
        &paths,
        &TapeName::new("mine").expect("name"),
        &["kitty".to_owned()],
    )
    .expect("snapshot");
    assert!(dest.join(".config/kitty/kitty.conf").is_file());

    // Snapshot entry is insertable.
    let report = tapes::ops::insert(
        &paths,
        &TapeName::new("mine").expect("name"),
        &dest,
        InsertOptions::default(),
    )
    .expect("insert snapshot");
    assert_eq!(report.targets.len(), 1);
}
