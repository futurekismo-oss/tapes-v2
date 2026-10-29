use std::{
    fs, io,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context};
use dirs::{config_dir, data_dir, home_dir};
use log::{error, info};
use nix::fcntl::{renameat2, RenameFlags};

use crate::{state::AppState, toml::TapeManifest};

// Loads and parses the tape.toml manifest from a tape directory.
pub fn load_manifest(tape_dir: &Path) -> Result<TapeManifest, anyhow::Error> {
    let manifest_path = tape_dir.join("tape.toml");
    info!("manifest path: {manifest_path:?}");

    if !manifest_path.exists() {
        error!("tape.toml not found in {}", tape_dir.display());
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

pub fn get_tapes_default_path() -> Result<PathBuf, anyhow::Error> {
    let tapes_dir = home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
    let tapes_dir = tapes_dir.join(".tapes");

    if tapes_dir.exists() {
        Ok(tapes_dir)
    } else {
        bail!("{} does not exist", tapes_dir.display().to_string())
    }
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
    let dir = get_tapes_data_path()?.join("state.json");
    data.save(dir)?;

    Ok(())
}

pub fn find_targets(path: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
    let mut detected: Vec<PathBuf> = Vec::new();

    for target in fs::read_dir(path)? {
        let target = target?;
        const IGNORED: &[&str] = &["tape.toml", ".git", "README.md", ".github"];

        if let Some(name) = target.file_name().to_str() {
            if !IGNORED.contains(&name) {
                detected.push(target.path());
            }
        }
    }

    Ok(detected)
}

pub fn atomic_swap<P: AsRef<Path>, Q: AsRef<Path>>(path1: P, path2: Q) -> io::Result<()> {
    renameat2(
        None, // Uses AT_FDCWD (relative to current working directory)
        path1.as_ref(),
        None, // Uses AT_FDCWD
        path2.as_ref(),
        RenameFlags::RENAME_EXCHANGE,
    )
    .map_err(|e| io::Error::from_raw_os_error(e as i32))
}

pub fn validate_correct_tapes_dir_structure() -> Result<(), anyhow::Error> {
    let home_dir = home_dir().expect("home dir should exist");
    let config_dir = config_dir().expect("Config dir should exist");
    let tapes_config_dir = home_dir.join(".config.tapes");

    if !config_dir.exists() {
        bail!("{config_dir:?} doesn't exist, exiting");
    }
    info!("{:?} exists", config_dir);

    if !tapes_config_dir.exists() {
        bail!("{tapes_config_dir:?} doesn't exist, exiting");
    }
    info!("{:?} exists", tapes_config_dir);

    let marker_path = config_dir.join(".tapes-managed");
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

pub fn resolve_home_dir_tilde(source: &str) -> PathBuf {
    if let Some(stripped) = source.strip_prefix("~/") {
        if let Some(home) = home_dir() {
            return home.join(stripped);
        }
    }
    return PathBuf::from(source);
}
