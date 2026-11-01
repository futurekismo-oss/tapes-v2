use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{Context, bail};
use dirs::{config_dir, data_dir, home_dir};
use log::info;
use nix::fcntl::{RenameFlags, renameat2};

use crate::{state::AppState, toml::TapeManifest};

// Loads and parses the tape.toml manifest from a tape directory.
pub fn load_manifest(tape_dir: &Path) -> Result<TapeManifest, anyhow::Error> {
    let manifest_path = tape_dir.join("tape.toml");
    info!("manifest path: {manifest_path:?}");

    if !manifest_path.exists() {
        bail!("a tape.toml does not exist in {}", tape_dir.display());
    }

    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest at {manifest_path:?}"))?;

    let manifest: TapeManifest = toml::from_str(&content)
        .with_context(|| format!("Failed to parse manifest at {manifest_path:?}"))?;

    Ok(manifest)
}

// Loads the manifest of a tape registered under the default tapes path.
pub fn get_tapes_data(tape_name: &str) -> Result<TapeManifest, anyhow::Error> {
    let tapes_path = get_tapes_default_path()?;
    info!("tapes path: {tapes_path:?}");

    if !tapes_path.is_dir() {
        bail!("{tapes_path:?} is not a valid directory");
    }

    let tape_dir = tapes_path.join(tape_name);
    info!("tape dir: {tape_dir:?}");

    load_manifest(&tape_dir)
}

pub const STAGING_DIR_NAME: &str = ".config.tapes";
pub const MARKER_FILE_NAME: &str = ".tapes-managed";

pub fn get_staging_config_path() -> Result<PathBuf, anyhow::Error> {
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    Ok(home.join(STAGING_DIR_NAME))
}

pub fn get_tapes_default_path() -> Result<PathBuf, anyhow::Error> {
    let tapes_dir = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let tapes_dir = tapes_dir.join(".tapes");

    // make ~/.tapes if first time running
    if !tapes_dir.exists() {
        fs::create_dir_all(&tapes_dir)?;
    }
    Ok(tapes_dir)
}

pub fn get_tapes_data_path() -> Result<PathBuf, anyhow::Error> {
    let local_dir = data_dir().ok_or_else(|| anyhow::anyhow!("Could not get local dir"))?;
    let tapes_data_path = local_dir.join("tapes");

    if !tapes_data_path.exists() {
        std::fs::create_dir_all(&tapes_data_path)?;
    }

    Ok(tapes_data_path)
}

pub fn save_state_data(data: &mut AppState) -> Result<(), anyhow::Error> {
    data.save_current()
}

pub fn find_targets(path: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
    let mut detected: Vec<PathBuf> = Vec::new();

    // skip repo metadata and tape manifests so only real app configs get symlinked into ~/.config
    const IGNORED: &[&str] = &[
        "tape.toml",
        ".git",
        ".gitattributes",
        "README.md",
        ".github",
        ".gitignore",
        ".editorconfig",
        "LICENSE",
        MARKER_FILE_NAME,
    ];

    // if dir doesn't exist yet (fresh install / no .config), nothing to find
    if !path.exists() {
        return Ok(detected);
    }

    for target in fs::read_dir(path)? {
        let target = target?;

        if let Some(name) = target.file_name().to_str()
            && !IGNORED.contains(&name)
        {
            detected.push(target.path());
        }
    }

    Ok(detected)
}

pub fn atomic_swap<P: AsRef<Path>, Q: AsRef<Path>>(path1: P, path2: Q) -> io::Result<()> {
    renameat2(
        nix::fcntl::AT_FDCWD, // Uses AT_FDCWD (relative to current working directory)
        path1.as_ref(),
        nix::fcntl::AT_FDCWD, // Uses AT_FDCWD
        path2.as_ref(),
        RenameFlags::RENAME_EXCHANGE,
    )
    .map_err(|e| io::Error::from_raw_os_error(e as i32))
}

pub fn validate_correct_tapes_dir_structure() -> Result<(), anyhow::Error> {
    let config_dir = config_dir().expect("Config dir should exist");
    let tapes_config_dir = get_staging_config_path()?;

    if !config_dir.exists() {
        bail!("{config_dir:?} doesn't exist, exiting");
    }
    info!("{:?} exists", config_dir);

    if !tapes_config_dir.exists() {
        bail!("{tapes_config_dir:?} doesn't exist, exiting");
    }
    info!("{:?} exists", tapes_config_dir);

    let marker_path = config_dir.join(MARKER_FILE_NAME);
    if !marker_path.exists() {
        bail!("{config_dir:?} is not managed by tapes")
    }
    info!("{:?} exists", marker_path);

    Ok(())
}

pub fn yes_or_no_prompt(message: &str) -> bool {
    use std::io::Write;

    print!("{} (y/n): ", message);
    std::io::stdout().flush().unwrap();

    let mut input = String::new();
    std::io::stdin().read_line(&mut input).unwrap();

    input.trim().to_lowercase() == "y"
}

pub fn get_all_tapes_in_dir(tapes_path: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
    let mut found_tapes: Vec<PathBuf> = Vec::new();

    if let Ok(entries) = fs::read_dir(&tapes_path) {
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() && path.join("tape.toml").exists() {
                found_tapes.push(path);
            }
        }
    }

    Ok(found_tapes)
}
