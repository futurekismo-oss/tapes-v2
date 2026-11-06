use std::{fs, path::Path};
use tempfile::tempdir;

use crate::util::{MARKER_FILE_NAME, atomic_swap, find_targets};

fn write(path: &Path, content: &str) {
    fs::write(path, content).unwrap();
}

fn read(path: &Path) -> String {
    fs::read_to_string(path).unwrap()
}


fn names(found: &[std::path::PathBuf]) -> Vec<String> {
    found.iter().map(|p| p.file_name().unwrap().to_string_lossy().into_owned()).collect()
}


#[test]
fn atomic_swap_exchanges_two_directories() {
    let a = tempdir().unwrap(); 
    let b = tempdir().unwrap(); 

    let a_file = a.path().join("file");
    let b_file = b.path().join("file");

    write(&a_file, "A");
    write(&b_file, "B");

    atomic_swap(a.path(), b.path()).unwrap();

    assert_eq!(read(&a_file), "B");
    assert_eq!(read(&b_file), "A");
}


#[test]
fn find_targets_skips_git_and_manifest_files() {
    let dir = tempdir().unwrap();
    for ignored in [
        "tape.toml",
        ".git",
        ".gitattributes",
        "README.md",
        ".github",
        ".gitignore",
        ".editorconfig",
        "LICENSE",
        MARKER_FILE_NAME,
    ] {
        write(&dir.path().join(ignored), "ignore me");    
    }

    fs::create_dir_all(dir.path().join("hypr")).unwrap();
    write(&dir.path().join("hypr/hyprland.conf"), "hyprland config");

    let found =  find_targets(dir.path()).unwrap();

    assert_eq!(names(&found), vec!["hypr".to_string()]);
}
