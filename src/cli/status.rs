use anyhow::bail;

use crate::{cli::show, state::AppState, util::validate_correct_tapes_dir_structure};

pub fn show(app: AppState, verbose: bool) -> Result<(), anyhow::Error> {
    if let Err(error) = validate_correct_tapes_dir_structure() {
        println!("Either no tapes is currently inserted or something has gone wrong");
        println!("Bug Report Here: https://github.com/futurekismo-oss/tapes-v2/issues");

        bail!(error);
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
