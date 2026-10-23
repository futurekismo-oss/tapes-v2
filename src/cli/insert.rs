use crate::{
    toml::TapeManifest,
    util::{get_tapes_data, get_tapes_default_path, load_manifest, save_state_data},
    AppState,
};
use log::info;
use std::path::{Path, PathBuf};

pub fn run(
    app: &mut AppState,
    name: Option<String>,
    path: Option<PathBuf>,
    dry_run: bool,
    no_hooks: bool,
) -> Result<(), anyhow::Error> {
    info!("starting insert command");
    info!("Current tapes: {:?}", app.active_tape);

    if app.active_tape.is_some() {
        // Helper function right now, please replace // DEBUG
        app.eject()?;
    }

    let (tape_path, manifest) = resolve_tape(name.as_ref(), path)?;
    let tape_name = manifest.tape.name.clone();

    app.path = Some(tape_path);
    app.active_tape = Some(tape_name);

    println!("{:#?}", manifest);

    // inserted methods will go here into to inserted the tape atomictly

    // save state stuff
    save_state_data(app)?;

    println!("{:?}", app);

    Ok(())
}

fn resolve_tape(
    name: Option<&String>,
    path: Option<PathBuf>,
) -> anyhow::Result<(PathBuf, TapeManifest)> {
    if let Some(name) = name {
        let manifest = get_tapes_data(name)?;
        return Ok((get_tapes_default_path()?.join(name), manifest));
    }

    if let Some(p) = path {
        return Ok((p.canonicalize()?, load_manifest(&p)?));
    }

    if Path::new("./tape.toml").exists() {
        println!("Found tape.toml in current dir. \nInserting...");
        let cwd = std::env::current_dir()?;
        return Ok((cwd.clone(), load_manifest(&cwd)?));
    }

    anyhow::bail!("No path provided and no tape.toml found in the current directory")
}
