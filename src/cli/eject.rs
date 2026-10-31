use std::{fs, path::Path};

use anyhow::{bail, Context};
use dirs::{config_dir, home_dir};
use log::{error, info};

use crate::{
    AppState, hooks::{run_eject_hooks, run_hooks}, toml::TapeManifest, util::{atomic_swap, load_manifest, save_state_data, validate_correct_tapes_dir_structure},
};

pub fn run(
    app: &mut AppState,
    dry_run: bool,
    no_hooks: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
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

    eject_tapes(app, &config_dir, &tapes_config_dir, quiet, no_hooks)?;
    save_state_data(app)?;

    Ok(())
}

pub fn eject_tapes(
    app: &mut AppState,
    config_dir: &Path,
    tapes_config_dir: &Path,
    quiet: bool,
    no_hooks: bool,
) -> Result<(), anyhow::Error> {
    validate_correct_tapes_dir_structure()?; // Valiade twice for maximum safety

    // manifest should be cleared before app data is removed by app.eject
    // so this should stay about app.eject()
    let tapes_path = app.path.clone().expect("app.path should exist");
    let manifest = load_manifest(&tapes_path)?;

    app.eject()?;
    info!("cleared AppState");

    if !quiet {
        println!("removed {tapes_config_dir:?}");
    }

    atomic_swap(&config_dir, &tapes_config_dir).context("atomic swap failed, your original config is untouched, you can delete .config and rename .config.tapes to .config to get your config back")?;
    info!("swaped {config_dir:?} -> {tapes_config_dir:?}");

    if !quiet {
        println!("swaped {config_dir:?} -> {tapes_config_dir:?}");
    }

    // Eject can remove data unfornately?
    // but i can write a safety check

    // Now it will only remove the dir, after its ensure .tapes owns it
    if tapes_config_dir.join(".tapes-managed").exists() {
        fs::remove_dir_all(&tapes_config_dir)
            .context("failed to remove .config.tapes, you can remove it manually")?;
    } else {
        bail!("{tapes_config_dir:?} was not removed, as tapes doesnt not manage it (.tapes-managed not found)")
    }

    info!("removed {tapes_config_dir:?}");

    if !quiet {
        println!("removed {tapes_config_dir:?}");
    }

    run_eject_hooks(manifest, no_hooks, quiet)?;

    Ok(())
}


