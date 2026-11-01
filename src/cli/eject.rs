use std::{fs, path::Path};

use anyhow::{Context, bail};
use dirs::config_dir;
use log::info;

use crate::{
    AppState,
    hooks::run_eject_hooks,
    util::{
        MARKER_FILE_NAME, atomic_swap, get_staging_config_path, load_manifest,
        validate_correct_tapes_dir_structure,
    },
};

pub fn run(
    app: &mut AppState,
    dry_run: bool,
    no_hooks: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    info!("eject command");

    let config_dir = config_dir().expect("Config dir should exist");
    let tapes_config_dir = get_staging_config_path()?;

    validate_correct_tapes_dir_structure().context("No tapes is currently inserted")?;

    let tape_name = app
        .active_tape
        .clone()
        .unwrap_or_else(|| "unknown".to_string());

    if dry_run {
        println!("swapped {config_dir:?} -> {tapes_config_dir:?}");
        println!("removed {tapes_config_dir:?}");
        println!("cleared AppState");
        println!("ejected: {tape_name}");

        return Ok(());
    }

    eject_tapes(app, &config_dir, &tapes_config_dir, quiet, no_hooks)?;
    app.save_current()?;

    Ok(())
}

pub fn eject_tapes(
    app: &mut AppState,
    config_dir: &Path,
    tapes_config_dir: &Path,
    quiet: bool,
    no_hooks: bool,
) -> Result<(), anyhow::Error> {
    // manifest should be retrieved before app data is cleared by app.eject()
    let manifest = if let Some(ref tapes_path) = app.path {
        load_manifest(tapes_path).ok()
    } else {
        None
    };

    app.eject()?;
    info!("cleared AppState");

    atomic_swap(config_dir, tapes_config_dir).context("atomic swap failed, your original config is untouched, you can delete .config and rename .config.tapes to .config to get your config back")?;
    info!("swapped {config_dir:?} -> {tapes_config_dir:?}");

    if !quiet {
        println!("swapped {config_dir:?} -> {tapes_config_dir:?}");
    }

    // Now it will only remove the dir after ensuring .tapes owns it
    if tapes_config_dir.join(MARKER_FILE_NAME).exists() {
        fs::remove_dir_all(tapes_config_dir)
            .context("failed to remove .config.tapes, you can remove it manually")?;
    } else {
        bail!(
            "{tapes_config_dir:?} was not removed, as tapes does not manage it ({MARKER_FILE_NAME} not found)"
        )
    }

    info!("removed {tapes_config_dir:?}");

    if !quiet {
        println!("removed {tapes_config_dir:?}");
    }

    if let Some(m) = manifest {
        run_eject_hooks(m, no_hooks, quiet)?;
    } else if !quiet {
        info!("Skipping eject hooks: tape manifest could not be loaded");
    }

    Ok(())
}
