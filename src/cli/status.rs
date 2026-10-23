use crate::state::AppState;

pub fn show(app: AppState) -> Result<(), anyhow::Error> {
    if app.active_tape.is_some() || app.path.is_some() {
        println!("Current tape: ");
        println!("Tape name: {}", app.active_tape.unwrap_or_default());

        if let Some(path) = app.path {
            println!("Tape path: {}", path.display());
        }
    } else {
        println!("No tape is currently inserted");
    }

    Ok(())
}
