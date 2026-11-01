use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    toml::TapeManifest,
    util::{find_targets, get_all_tapes_in_dir, get_tapes_data, get_tapes_default_path},
};

pub fn run(name: Option<String>) -> Result<(), anyhow::Error> {
    let tapes_path = get_tapes_default_path()?;

    if let Some(name) = name {
        let name = name.to_lowercase();
        let tapes = get_tapes_data(&name)?;
        let tapes_dir = tapes_path.join(name);
        display_tape_info(tapes, &tapes_dir);
    } else {
        println!("All Tapes (in ~/.tapes)");

        let tapes = get_all_tapes_in_dir(&tapes_path)?;

        if tapes.is_empty() {
            println!("No tapes installed yet");

            return Ok(());
        }

        for tape in tapes {
            if let Some(name) = tape.file_name().and_then(|n| n.to_str()) {
                println!("- {name}: {}", tape.display());
            }
        }
    }

    Ok(())
}

pub fn display_tape_info(tape: TapeManifest, tapes_path: &Path) {
    println!("Name: {}", tape.tape.name);
    println!("Description: {}", tape.tape.desc);
    println!("Path: {}", tapes_path.to_string_lossy());

    if let Some(hooks) = &tape.hooks {
        println!("Hooks");

        if !hooks.install.is_empty() {
            println!("  Install:");
            for hook in &hooks.install {
                println!("   - {}", hook);
            }
        }

        if !hooks.insert.is_empty() {
            println!("  Insert:");
            for hook in &hooks.insert {
                println!("   - {}", hook);
            }
        }

        if !hooks.eject.is_empty() {
            println!("  Eject:");
            for hook in &hooks.eject {
                println!("   - {}", hook);
            }
        }
    }

    if let Ok(targets) = find_targets(tapes_path) {
        println!("Targets");

        for target in targets {
            let folder_name = target
                .file_name()
                .expect("targets should exist")
                .to_string_lossy();
            println!(" - {}", folder_name);
        }
    }
}
