use std::{fs, path::Path};

use dirs::{config_dir, home_dir};
use log::info;

use crate::{
    util::{atomic_swap, save_state_data, validate_correct_tapes_dir_structure},
    AppState,
};

pub fn run(app: &mut AppState, dry_run: bool, no_hooks: bool) -> Result<(), anyhow::Error> {
    info!("eject command");

    let home_dir = home_dir().expect("home dir should exist");
    let config_dir = config_dir().expect("Config dir should exist");
    let tapes_config_dir = home_dir.join(".config.tapes");

    validate_correct_tapes_dir_structure()?;

    if dry_run {
        println!("swaped {config_dir:?} -> {tapes_config_dir:?}");
        println!("removed {tapes_config_dir:?}");
        println!("cleared AppState");

        return Ok(());
    }

    eject_tapes(app, &config_dir, &tapes_config_dir)?;
    save_state_data(app)?;

    Ok(())
}

// TODO: refactor to not propagate errrors up but give descriptive next steps incase of failure
pub fn eject_tapes(
    app: &mut AppState,
    config_dir: &Path,
    tapes_config_dir: &Path,
) -> Result<(), anyhow::Error> {
    validate_correct_tapes_dir_structure()?; // Valiade twice for maximum safety

    atomic_swap(&config_dir, &tapes_config_dir)?;
    info!("swaped {config_dir:?} -> {tapes_config_dir:?}");

    fs::remove_dir_all(&tapes_config_dir)?;
    info!("removed {tapes_config_dir:?}");

    app.eject()?;
    info!("cleared AppState");

    Ok(())
}
