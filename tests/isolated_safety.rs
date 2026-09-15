//! Safety scenarios: traversal, clobber refusal, conflicts, dry-run.
//!
//! All sandboxed under `tempfile`. The point is proving an LLM-written
//! switcher cannot escape `$HOME` or delete user data — in CI, not on iron.

use std::fs;

use tapes::ops::{EjectOptions, InsertOptions};
use tapes::{StorePaths, TapeError, TapeName};

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

#[test]
fn rejects_dotdot_targets() {
    let (_tmp, paths) = sandbox();
    let dir = paths.library_dir.join("evil");
    fs::create_dir_all(&dir).expect("mkdir");
    fs::write(
        dir.join("tape.toml"),
        "schema_version = 2\n[tape]\nname = \"evil\"\nversion = \"0.1.0\"\ndesc = \"\"\n[targets]\n\"../evil\" = \"~/.config/evil\"\n",
    )
    .expect("write");
    let err = tapes::ops::insert(
        &paths,
        &TapeName::new("evil").expect("name"),
        &dir,
        InsertOptions::default(),
    )
    .expect_err("must reject ..");
    assert!(matches!(err, TapeError::Traversal(_)), "got {err:?}");
}

#[test]
fn refuses_to_delete_regular_files_without_backup() {
    // A regular file at dst must become a backup, never vanish.
    let (_tmp, paths) = sandbox();
    fs::create_dir_all(paths.config_home.join("foot")).expect("cfg");
    fs::write(paths.config_home.join("foot").join("foot.ini"), "orig").expect("write");
    let dir = paths.library_dir.join("r");
    fs::create_dir_all(dir.join(".config/foot")).expect("tape");
    fs::write(dir.join(".config/foot/foot.ini"), "rice").expect("write");

    tapes::ops::insert(
        &paths,
        &TapeName::new("r").expect("name"),
        &dir,
        InsertOptions::default(),
    )
    .expect("insert");

    let backups: Vec<_> = paths
        .backups_dir()
        .read_dir()
        .expect("ls")
        .filter_map(Result::ok)
        .collect();
    assert_eq!(backups.len(), 1, "exactly one timestamped backup");

    tapes::ops::eject(
        &paths,
        Some(&TapeName::new("r").expect("name")),
        EjectOptions::default(),
    )
    .expect("eject");
    assert_eq!(
        fs::read_to_string(paths.config_home.join("foot").join("foot.ini")).expect("restored"),
        "orig"
    );
}

#[test]
fn eject_never_deletes_user_retargeted_symlinks() {
    let (_tmp, paths) = sandbox();
    let dir = paths.library_dir.join("a");
    fs::create_dir_all(dir.join(".config/s")).expect("tape");
    fs::write(dir.join(".config/s/f"), "a").expect("write");
    tapes::ops::insert(
        &paths,
        &TapeName::new("a").expect("name"),
        &dir,
        InsertOptions::default(),
    )
    .expect("insert");

    // User repoints the link elsewhere.
    let dst = paths.config_home.join("s");
    fs::remove_file(&dst).expect("unlink");
    #[cfg(unix)]
    std::os::unix::fs::symlink("/tmp", &dst).expect("retarget");

    tapes::ops::eject(
        &paths,
        Some(&TapeName::new("a").expect("name")),
        EjectOptions::default(),
    )
    .expect("eject must not fail on retarget");
    assert_eq!(fs::read_link(&dst).expect("link kept"), std::path::PathBuf::from("/tmp"));
}

#[test]
fn dry_run_eject_changes_nothing() {
    let (_tmp, paths) = sandbox();
    let dir = paths.library_dir.join("a");
    fs::create_dir_all(dir.join(".config/s")).expect("tape");
    fs::write(dir.join(".config/s/f"), "a").expect("write");
    tapes::ops::insert(
        &paths,
        &TapeName::new("a").expect("name"),
        &dir,
        InsertOptions::default(),
    )
    .expect("insert");
    let report = tapes::ops::eject(
        &paths,
        Some(&TapeName::new("a").expect("name")),
        EjectOptions {
            dry_run: true,
            ..Default::default()
        },
    )
    .expect("dry eject");
    assert!(report.dry_run);
    assert!(paths.config_home.join("s").is_symlink(), "still linked");
    assert!(!tapes::ops::status(&paths).expect("status").active.is_empty());
}

#[test]
fn cli_smoke_in_sandbox_via_child_process() {
    // Child-process env only — parent env untouched, parallel-safe.
    let tmp = tempfile::tempdir().expect("tempdir");
    let home = tmp.path().join("home");
    let config = home.join(".config");
    let data = home.join(".local/share");
    fs::create_dir_all(&config).expect("config");
    fs::create_dir_all(data.join("tapes/demo/.config/hypr")).expect("tape");
    fs::write(data.join("tapes/demo/.config/hypr/hyprland.conf"), "x").expect("write");

    let mut cmd = assert_cmd::Command::cargo_bin("tape").expect("binary");
    cmd.env("HOME", &home)
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_DATA_HOME", &data)
        .args(["insert", "demo", "--no-hooks"])
        .assert()
        .success();

    assert!(config.join("hypr").is_symlink());

    let mut cmd = assert_cmd::Command::cargo_bin("tape").expect("binary");
    cmd.env("HOME", &home)
        .env("XDG_CONFIG_HOME", &config)
        .env("XDG_DATA_HOME", &data)
        .args(["eject", "--no-hooks"])
        .assert()
        .success();
    assert!(!config.join("hypr").is_symlink());
}
