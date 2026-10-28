use crate::{
    toml::TapeManifest,
    util::{atomic_swap, get_tapes_data, get_tapes_default_path, load_manifest, save_state_data},
    AppState,
};
use anyhow::bail;
use dirs::{config_dir, home_dir};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use std::fs;
use std::os::unix::fs as unix_fs;

pub fn run(
    app: &mut AppState,
    name: Option<String>,
    path: Option<PathBuf>,
    dry_run: bool,
    no_hooks: bool,
) -> Result<(), anyhow::Error> {
    info!("starting insert command");
    info!("Current tapes: {:?}", app.active_tape);

    if app.active_tape.is_some() {
        app.eject()?;
    }

    let (tape_path, manifest) = resolve_tape(name.as_ref(), path)?;
    let tape_name = manifest.tape.name.clone();
    let config_dir = config_dir().expect("config dir should exist");

    if config_dir.is_symlink() {
        bail!("{config_dir:?} is a symlink, prone to fail");
    }

    app.path = Some(tape_path);
    app.active_tape = Some(tape_name);

    let home = home_dir().expect("Home dir should exist");
    let new_config_dir = home.join(".config.tapes");

    let targets = resolve_targets(app, &new_config_dir)?;
    app.targets = Some(targets);

    if dry_run {
        println!("Created {new_config_dir:?}...");
        return Ok(());
    }

    if new_config_dir.exists() {
        info!("{new_config_dir:?} exists - something has gone wrong");
        bail!(".config.tapes exists, did something go wrong? bug report here https://github.com/futurekismo-oss/tapes-v2/issues");
    }

    fs::create_dir(&new_config_dir)?;
    info!("Created {new_config_dir:?}");

    let marker_path = new_config_dir.join(".tapes-managed");
    File::create(&marker_path)?;
    info!("Created: {marker_path:?}");

    if let Err(err) = symlink_targets(app.targets.as_ref().expect("app.targets should exist")) {
        error!("symlink_targets failed: {err}");
        return Err(err);
    }

    if let Err(err) = atomic_swap(&config_dir, &new_config_dir) {
        error!("atomic swap failed: {err}");
        return Err(err.into());
    }
    info!("atomic swapped: {config_dir:?} -> {new_config_dir:?}");

    save_state_data(app)?;

    Ok(())
}

fn resolve_tape(
    name: Option<&String>,
    path: Option<PathBuf>,
) -> anyhow::Result<(PathBuf, TapeManifest)> {
    if let Some(name) = name {
        let manifest = get_tapes_data(name)?;
        return Ok((get_tapes_default_path()?.join(name), manifest));
    }

    if let Some(p) = path {
        return Ok((p.canonicalize()?, load_manifest(&p)?));
    }

    if Path::new("./tape.toml").exists() {
        println!("Found tape.toml in current dir. \nInserting...");
        let cwd = std::env::current_dir()?;
        return Ok((cwd.clone(), load_manifest(&cwd)?));
    }

    anyhow::bail!("No path provided and no tape.toml found in the current directory")
}

fn find_targets(path: &Path) -> Result<Vec<PathBuf>, anyhow::Error> {
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Target {
    target_path: PathBuf,
    symlink_dest: PathBuf,
    from_tapes: bool,
}

fn resolve_targets(
    app: &AppState,
    new_config_dir: &Path,
) -> Result<HashMap<String, Target>, anyhow::Error> {
    let config_dir = config_dir().expect("Config dir should exist");

    let configs = find_targets(&config_dir)?
        .into_iter()
        .map(|p| map_target(p, new_config_dir, false));

    let tapes_path = app.path.as_deref().expect("path just set");
    let tapes = find_targets(tapes_path)?
        .into_iter()
        .map(|p| map_target(p, new_config_dir, true));

    Ok(configs.chain(tapes).collect())
}

fn map_target(target_path: PathBuf, new_config_dir: &Path, from_tapes: bool) -> (String, Target) {
    let file_name = target_path.file_name().unwrap().to_owned();
    let target_name = file_name.to_string_lossy().into_owned();
    let symlink_dest = new_config_dir.join(&file_name);

    let resolved_path = if from_tapes {
        target_path
    } else {
        symlink_dest.clone()
    };

    let target = Target {
        target_path: resolved_path,
        symlink_dest,
        from_tapes,
    };

    (target_name, target)
}

fn symlink_targets(targets_map: &HashMap<String, Target>) -> Result<(), anyhow::Error> {
    let targets: Vec<&Target> = targets_map.values().collect();

    for target in &targets {
        if let Err(e) = unix_fs::symlink(&target.target_path, &target.symlink_dest) {
            error!(
                "Error: {e}, failed to symlink {:?} -> {:?}",
                target.target_path, target.symlink_dest
            );
            continue;
        }

        println!(
            "symlinking {:?} -> {:?}",
            target.target_path, target.symlink_dest
        );
    }

    Ok(())
}
