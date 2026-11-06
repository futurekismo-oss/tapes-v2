use crate::{
    AppState,
    cli::eject::eject_tapes,
    hooks::run_insert_hooks,
    toml::TapeManifest,
    util::{
        MARKER_FILE_NAME, atomic_swap, find_targets, get_staging_config_path, get_tapes_data,
        get_tapes_default_path, get_user_config_dir, load_manifest, save_state_data,
        yes_or_no_prompt,
    },
};
use anyhow::bail;
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
    quiet: bool,
) -> Result<(), anyhow::Error> {
    info!("starting insert command");

    let config_dir = get_user_config_dir()?;
    let new_config_dir = get_staging_config_path()?;

    // Fall back to default_tape from config if no name or path is provided
    let default_tape = crate::config::Config::get().default_tape.clone();
    let effective_name = name.or(default_tape);

    // check tape exists first before doing anything or asking to eject
    let (tape_path, manifest) = resolve_tape(effective_name.as_ref(), path)?;
    let tape_name = manifest.tape.name.clone();

    // another tape in? ask user to eject it
    if app.active_tape.is_some() && !dry_run {
        info!("Another tape is currently inserted: {:?}", app.active_tape);

        if !yes_or_no_prompt("Another tape is currently inserted.\nEject?") {
            bail!("Another tape is currently inserted, doing nothing, exiting");
        };

        eject_tapes(app, &config_dir, &new_config_dir, quiet, no_hooks)?;
        info!("Eject previously active tape successfully");

        app.save_current()?;
    }

    if config_dir.is_symlink() {
        bail!("{config_dir:?} is a symlink, prone to fail");
    }

    app.path = Some(tape_path);
    app.active_tape = Some(tape_name);

    info!(
        "App Path: {:?}\nApp Active tape: {:?}\n :: Not Saved yet",
        app.path, app.active_tape,
    );

    let targets: HashMap<String, Target> = resolve_targets(app, &new_config_dir)?;
    info!("targets: {:#?}", targets);

    if dry_run {
        println!("Created {new_config_dir:?}...");

        info!("Created {new_config_dir:?}...");

        for (name, target) in &targets {
            println!(
                "{name}: symlinking {:?} -> {:?}",
                target.target_path, target.symlink_dest
            );

            info!(
                "{name}: symlinking {:?} -> {:?}",
                target.target_path, target.symlink_dest
            );
        }

        return Ok(());
    }

    app.targets = Some(targets);

    if new_config_dir.exists() {
        error!("{new_config_dir:?} exists - something has gone wrong");
        bail!(
            ".config.tapes exists, did something go wrong? bug report here https://github.com/futurekismo-oss/tapes/issues"
        );
    }

    fs::create_dir(&new_config_dir)?;
    info!("Created {new_config_dir:?}");

    let marker_path = new_config_dir.join(MARKER_FILE_NAME);
    File::create(&marker_path)?;
    info!("Created: {marker_path:?}");

    if let Err(err) = symlink_targets(
        app.targets.as_ref().expect("app.targets should exist"),
        quiet,
    ) {
        let _ = fs::remove_dir_all(&new_config_dir);
        return Err(err);
    }

    if let Err(err) = atomic_swap(&config_dir, &new_config_dir) {
        let _ = fs::remove_dir_all(&new_config_dir);
        return Err(err.into());
    }
    info!("atomic swapped: {config_dir:?} -> {new_config_dir:?}");

    println!(
        "Inserted: {}",
        app.active_tape.as_deref().unwrap_or_default()
    );

    save_state_data(app)?;

    if let Some(ref path) = app.path {
        run_insert_hooks(&manifest, path, no_hooks, quiet)?;
    }

    Ok(())
}

pub fn resolve_tape(
    name: Option<&String>,
    path: Option<PathBuf>,
) -> anyhow::Result<(PathBuf, TapeManifest)> {
    if let Some(name) = name {
        let name_lower = name.to_lowercase();
        info!("fetching tapes: {name_lower}");

        let manifest = get_tapes_data(&name_lower)?;
        info!("manifest: {:#?}", manifest);

        return Ok((get_tapes_default_path()?.join(&name_lower), manifest));
    }

    if let Some(p) = path {
        info!("Given tape path: {:?}", p);
        return Ok((p.canonicalize()?, load_manifest(&p)?));
    }

    if Path::new("./tape.toml").exists() {
        println!("Found tape.toml in current dir. \nInserting...");

        info!("Found tape.toml in current dir.");

        let cwd = std::env::current_dir()?;
        return Ok((cwd.clone(), load_manifest(&cwd)?));
    }

    anyhow::bail!("No path provided and no tape.toml found in the current directory")
}

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
pub struct Target {
    pub(crate) target_path: PathBuf,
    pub(crate) symlink_dest: PathBuf,
    pub(crate) from_tapes: bool,
}

fn resolve_targets(
    app: &AppState,
    new_config_dir: &Path,
) -> Result<HashMap<String, Target>, anyhow::Error> {
    let config_dir = get_user_config_dir()?;
    let mut targets = HashMap::with_capacity(64);

    let mut add_targets = |dir: &Path, from_tapes: bool| -> Result<(), anyhow::Error> {
        for target_path in find_targets(dir)? {
            let (name, target) = map_target(target_path, new_config_dir, from_tapes);
            targets.insert(name, target);
        }
        Ok(())
    };

    if config_dir.exists() {
        add_targets(&config_dir, false)?;
    }

    let tapes_path = app.path.as_deref().expect("path just set");
    add_targets(tapes_path, true)?;

    Ok(targets)
}

fn map_target(target_path: PathBuf, new_config_dir: &Path, from_tapes: bool) -> (String, Target) {
    let file_name = target_path.file_name().unwrap().to_owned();
    let target_name = file_name.to_string_lossy().into_owned();
    let symlink_dest = new_config_dir.join(&file_name);

    let resolved_path = match from_tapes {
        true => target_path,
        false => symlink_dest.clone(),
    };

    let target = Target {
        target_path: resolved_path,
        symlink_dest,
        from_tapes,
    };

    (target_name, target)
}

pub fn symlink_targets(
    targets_map: &HashMap<String, Target>,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    for target in targets_map.values() {
        if let Err(e) = unix_fs::symlink(&target.target_path, &target.symlink_dest) {
            return Err(anyhow::anyhow!(
                "Failed to create symlink {:?} -> {:?}: {}",
                target.symlink_dest,
                target.target_path,
                e
            ));
        }

        if !quiet {
            println!(
                "symlinking {:?} -> {:?}",
                target.target_path, target.symlink_dest
            );
        }
    }

    Ok(())
}
