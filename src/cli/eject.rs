use std::{fs, path::Path};

use anyhow::Context;
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

    validate_correct_tapes_dir_structure().context("No tapes is currently inserted")?;

    let tape_name = app.active_tape.clone().expect("should exist");

    if dry_run {
        println!("swaped {config_dir:?} -> {tapes_config_dir:?}");
        println!("removed {tapes_config_dir:?}");
        println!("cleared AppState");
        println!("ejected: {}", tape_name.to_string());

        return Ok(());
    }

    eject_tapes(app, &config_dir, &tapes_config_dir)?;
    println!("ejected: {}", tape_name.to_string());

    save_state_data(app)?;

    Ok(())
}

pub fn eject_tapes(
    app: &mut AppState,
    config_dir: &Path,
    tapes_config_dir: &Path,
) -> Result<(), anyhow::Error> {
    validate_correct_tapes_dir_structure()?; // Valiade twice for maximum safety

    app.eject()?;
    info!("cleared AppState");
    println!("removed {tapes_config_dir:?}");

    atomic_swap(&config_dir, &tapes_config_dir).context("atomic swap failed, your original config is untouched, you can delete .config and rename .config.tapes to .config to get your config back")?;
    info!("swaped {config_dir:?} -> {tapes_config_dir:?}");
    println!("swaped {config_dir:?} -> {tapes_config_dir:?}");

    fs::remove_dir_all(&tapes_config_dir).context("failed to remove .config.tapes, you can remove it manually")?;
    info!("removed {tapes_config_dir:?}");
    println!("removed {tapes_config_dir:?}");

    Ok(())
}
