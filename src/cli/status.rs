use anyhow::bail;
use dirs::{config_dir, home_dir};

use crate::{cli::show, state::AppState};

pub fn show(app: AppState, verbose: bool) -> Result<(), anyhow::Error> {
    let home_dir = home_dir().expect("home dir should exist");
    let config_dir = config_dir().expect("Config dir should exist");
    let tapes_config_dir = home_dir.join(".config.tapes");

    if !config_dir.exists() {
        bail!("{config_dir:?} doesn't exist, exiting");
    }

    if !tapes_config_dir.exists() {
        bail!("{tapes_config_dir:?} doesn't exist, exiting");
    }

    if !config_dir.join(".tape-managed").exists() {
        bail!("{config_dir:?} is not managed by tapes")
    }

    if app.active_tape.is_some() || app.path.is_some() {
        let tape_name = app.active_tape.unwrap_or_default();

        println!("Current tape: ");
        println!("Tape name: {}", &tape_name);

        if let Some(path) = app.path {
            println!("Tape path: {}", path.display());
        }

        if verbose {
            show::run(Some(tape_name))?;
        }
    } else {
        println!("No tape is currently inserted");
    }

    Ok(())
}
