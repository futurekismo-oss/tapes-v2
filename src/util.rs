use std::{panic::Location, path::PathBuf};

use anyhow::{bail, Context};
use dirs::{config_local_dir, data_dir, home_dir};
use log::info;

use crate::toml::TapeManifest;

pub fn get_tapes_data(tape_name: &str) -> Result<TapeManifest, anyhow::Error> {
    // let tapes_path = get_tapes_default_path()?;
    let tapes_path = get_tapes_default_path()?;

    info!("tapes path: {tapes_path:?}");

    if !tapes_path.is_dir() {
        bail!("{tapes_path:?} is not a valid directory");
    }

    let tape_dir = tapes_path.join(tape_name);
    info!("tape dir: {tape_dir:?}");

    let manifest_path = tape_dir.join("tape.toml");
    info!("manifest path: {tape_dir:?}");

    let content = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("Failed to read manifest at {manifest_path:?}"))?;

    let manifest: TapeManifest = toml::from_str(&content)
        .with_context(|| format!("Failed to parse manifest at {manifest_path:?}"))?;

    Ok(manifest)
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

    if tapes_data_path.exists() {
        Ok(tapes_data_path)
    } else {
        bail!("{} does not exist", tapes_data_path.display().to_string())
    }
}
