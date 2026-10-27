use crate::{
    toml::TapeManifest,
    util::{atomic_swap, get_tapes_data, get_tapes_default_path, load_manifest, save_state_data},
    AppState,
};
use dirs::{config_dir, home_dir};
use log::info;
use serde::{Deserialize, Serialize};
use serde_json::map::Entry;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use std::fs;

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
        // DEBUG: replace
        app.eject()?;
    }

    let (tape_path, manifest) = resolve_tape(name.as_ref(), path)?;
    let tape_name = manifest.tape.name.clone();

    app.path = Some(tape_path);
    app.active_tape = Some(tape_name);

    // inserted methods will go here into to inserted the tape atomictly
    let targets = resolve_targets(&app)?;

    app.targets = Some(targets);

    save_state_data(app)?;

    println!("{:?}", app.active_tape);

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
}

fn resolve_targets(app: &AppState) -> Result<HashMap<String, Target>, anyhow::Error> {
    let config_dir = config_dir().expect("Config dir should exist");
    let home = home_dir().expect("Home dir should exist");

    let new_config_dir = home.join(".config-tapes");

    let configs = find_targets(&config_dir)?.into_iter().map(|target_path| {
        let file_name = target_path.file_name().unwrap().to_owned();
        let symlink_dest = new_config_dir.join(&file_name);
        let target_name = file_name.to_string_lossy().into_owned();
        let target = Target {
            target_path,
            symlink_dest,
        };
        (target_name, target)
    });

    let tapes = find_targets(&app.path.as_deref().expect("path just set"))?
        .into_iter()
        .map(|target_path| {
            let target_name = target_path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            let symlink_dest = new_config_dir.join(&target_name);
            let target = Target {
                target_path,
                symlink_dest,
            };
            (target_name, target)
        });

    Ok(configs.chain(tapes).collect())
}
