use anyhow::bail;

use crate::{
    cli::show::{self, display_tape_info},
    state::AppState,
    util::{load_manifest, validate_correct_tapes_dir_structure},
};

pub fn show(app: AppState, verbose: bool) -> Result<(), anyhow::Error> {
    if let Err(error) = validate_correct_tapes_dir_structure() {
        println!("No Tapes is currently inserted.\n");

        println!("Has something has gone wrong?");
        println!("Bug Report Here: https://github.com/futurekismo-oss/tapes-v2/issues");

        bail!(error);
    }

    if app.active_tape.is_some() || app.path.is_some() {
        let tape_name = app.active_tape.unwrap_or_default();
        let tape_path = app.path.clone().unwrap_or_default();

        println!("Current tape: ");
        if verbose {
            let tape = load_manifest(&tape_path)?;
            display_tape_info(tape, &tape_path);

            return Ok(());
        }
        println!("Tape name: {}", &tape_name);

        if let Some(tape_path) = app.path {
            println!("Tape path: {}", tape_path.display());
        }
    } 

    Ok(())
}
