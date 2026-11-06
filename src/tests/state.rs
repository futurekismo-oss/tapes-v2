use std::{collections::HashMap, fs};

use crate::{cli::insert::Target, state::AppState};

fn sample_target(root: &std::path::Path) -> Target {
    Target {
        target_path: root.join("hypr"),
        symlink_dest: root.join(".config.tapes/hypr"),
        from_tapes: true,
    }
}

#[test]
fn state_save_then_load_round_trips() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.toml");

    let mut targets = HashMap::new();
    targets.insert("hypr".to_string(), sample_target(dir.path()));

    let state = AppState {
        active_tape: Some("cyberpunk".to_string()),
        path: Some(dir.path().join(".tapes/cyberpunk")),
        targets: Some(targets),
    };
    state.save(&path).unwrap();

    let loaded = AppState::load(&path).unwrap();

    assert_eq!(loaded, state);
}

#[test]
fn state_save_then_load_round_trips_empty_state() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.toml");

    let state = AppState::default();
    state.save(&path).unwrap();

    let loaded = AppState::load(&path).unwrap();

    assert_eq!(loaded, state);
}

#[test]
fn state_load_missing_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.toml");

    assert_eq!(AppState::load(&path).unwrap(), AppState::default());
}

#[test]
fn state_load_empty_file_returns_default() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.toml");
    fs::write(&path, "   \n\t\n").unwrap();

    assert_eq!(AppState::load(&path).unwrap(), AppState::default());
}

#[test]
fn state_load_corrupted_file_returns_default_and_backs_up() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.toml");
    fs::write(&path, "{{{{ corrupted").unwrap();

    let state = AppState::load(&path).unwrap();

    // corrupted input must never be a hard failure: fall back to default
    assert_eq!(state, AppState::default());
    // and the broken file should be preserved for inspection
    assert!(path.with_extension("toml.bak").exists());
    assert_eq!(
        fs::read_to_string(path.with_extension("toml.bak")).unwrap(),
        "{{{{ corrupted"
    );
}

#[test]
fn state_save_is_atomic_and_leaves_no_tmp_leftover() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("state.toml");

    // an existing state file gets replaced, not truncated in place
    fs::write(&path, "active_tape = \"old\"\n").unwrap();
    AppState::default().save(&path).unwrap();

    assert!(path.exists());
    assert!(!dir.path().join("state.toml.tmp").exists());
    assert_eq!(AppState::load(&path).unwrap(), AppState::default());
}

#[test]
fn state_eject_clears_all_fields() {
    let dir = tempfile::tempdir().unwrap();
    let mut targets = HashMap::new();
    targets.insert("hypr".to_string(), sample_target(dir.path()));

    let mut state = AppState {
        active_tape: Some("cyberpunk".to_string()),
        path: Some(dir.path().join(".tapes/cyberpunk")),
        targets: Some(targets),
    };

    state.eject().unwrap();

    assert_eq!(state, AppState::default());
}
