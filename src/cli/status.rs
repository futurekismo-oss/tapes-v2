use anyhow::bail;

use crate::{
    cli::show::display_tape_info,
    state::AppState,
    util::{
        MARKER_FILE_NAME, get_staging_config_path, get_user_config_dir, load_manifest,
        validate_correct_tapes_dir_structure,
    },
};

pub fn show(app: AppState, verbose: bool) -> Result<(), anyhow::Error> {
    if app.active_tape.is_none() && app.path.is_none() {
        let cfg = get_user_config_dir()?;
        let tapes_config_dir = get_staging_config_path()?;
        let marker = cfg.join(MARKER_FILE_NAME);

        if !tapes_config_dir.exists() && !marker.exists() {
            println!("No tape is currently inserted.");
            return Ok(());
        }

        println!("No active tape recorded, but tapes-managed artifacts were found:");
        if tapes_config_dir.exists() {
            println!(
                " - Found leftover staging directory at {}",
                tapes_config_dir.display()
            );
        }
        if marker.exists() {
            println!(" - Found marker file at {}", marker.display());
        }
        println!("You can run 'tapes eject' to recover your original configuration.");
        return Ok(());
    }

    if let Err(error) = validate_correct_tapes_dir_structure() {
        println!("A tape is marked active in state, but directory structure validation failed:\n");
        println!("Has something gone wrong?");
        println!("Bug Report Here: https://github.com/futurekismo-oss/tapes/issues");
        bail!(error);
    }

    let tape_name = app.active_tape.unwrap_or_default();
    let tape_path = app.path.unwrap_or_default();

    println!("Current tape:");
    if verbose && tape_path.exists() {
        let tape = load_manifest(&tape_path)?;
        display_tape_info(tape, &tape_path);
        return Ok(());
    }

    println!("Tape name: {}", tape_name);
    if !tape_path.as_os_str().is_empty() {
        println!("Tape path: {}", tape_path.display());
    }

    Ok(())
}
