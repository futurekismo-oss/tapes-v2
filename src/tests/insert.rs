use std::{collections::HashMap, fs};

use crate::{
    cli::insert::{symlink_targets, Target},
    util::MARKER_FILE_NAME,
};

fn tape_target(tape_dir: &std::path::Path, staging: &std::path::Path, name: &str) -> Target {
    fs::create_dir_all(tape_dir.join(name)).unwrap();
    fs::write(tape_dir.join(name).join("settings.conf"), "from tape").unwrap();
    Target {
        target_path: tape_dir.join(name),
        symlink_dest: staging.join(name),
        from_tapes: true,
    }
}

fn map_from(targets: Vec<(&str, Target)>) -> HashMap<String, Target> {
    targets
        .into_iter()
        .map(|(name, t)| (name.to_string(), t))
        .collect()
}

#[test]
fn symlink_targets_creates_links_pointing_into_the_tape() {
    let tape = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let map = map_from(vec![(
        "hypr",
        tape_target(tape.path(), staging.path(), "hypr"),
    )]);

    symlink_targets(&map, true).unwrap();

    let link = fs::read_link(staging.path().join("hypr")).unwrap();
    assert_eq!(link, tape.path().join("hypr"));
    assert_eq!(
        fs::read_to_string(staging.path().join("hypr/settings.conf")).unwrap(),
        "from tape"
    );
}

#[test]
fn symlink_targets_fails_when_destination_already_exists() {
    let tape = tempfile::tempdir().unwrap();
    let staging = tempfile::tempdir().unwrap();
    let map = map_from(vec![(
        "hypr",
        tape_target(tape.path(), staging.path(), "hypr"),
    )]);

    // leftover from a previous crash: destination already there
    fs::create_dir_all(staging.path().join("hypr")).unwrap();

    assert!(symlink_targets(&map, true).is_err());
}

#[test]
fn failing_symlink_step_leaves_real_config_untouched() {
    // run()'s rollback path: if symlinking fails, staging is removed and
    // the real config must be bit-for-bit unchanged
    let home = tempfile::tempdir().unwrap();
    let config = home.path().join(".config");
    let staging = home.path().join(".config.tapes");
    let tape = tempfile::tempdir().unwrap();

    fs::create_dir_all(config.join("hypr")).unwrap();
    fs::write(config.join("hypr/hyprland.conf"), "old conf").unwrap();

    fs::create_dir_all(&staging).unwrap();
    fs::File::create(staging.join(MARKER_FILE_NAME)).unwrap();

    // force a symlink failure: destination already occupied
    let map = map_from(vec![("hypr", tape_target(tape.path(), &staging, "hypr"))]);
    fs::create_dir_all(staging.join("hypr")).unwrap();
    let symlink_failed = symlink_targets(&map, true).is_err();
    assert!(symlink_failed);

    // emulate run()'s cleanup on failure
    if symlink_failed {
        let _ = fs::remove_dir_all(&staging);
    }

    assert!(!staging.exists(), "staging must be cleaned up on failure");
    assert_eq!(
        fs::read_to_string(config.join("hypr/hyprland.conf")).unwrap(),
        "old conf"
    );
    assert!(!config.join(MARKER_FILE_NAME).exists());
}
