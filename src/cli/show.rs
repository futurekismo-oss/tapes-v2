use std::{fs, path::Path};

use crate::{
    toml::TapeManifest,
    util::{find_targets, get_tapes_data, get_tapes_default_path},
};

pub fn run(name: Option<String>) -> Result<(), anyhow::Error> {
    let tapes_path = get_tapes_default_path()?;
    // let tapes_dir = tapes_path.expect("tapes path should exist").join(name);

    if let Some(name) = name {
        let name = name.to_lowercase();

        let tapes = get_tapes_data(&name)?;
        let tapes_dir = tapes_path.join(name);

        display_tape_info(tapes, &tapes_dir);
    } else {
        println!("All Tapes (in ~/.tapes)");

        for entry in fs::read_dir(tapes_path)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                if path.join("tape.toml").exists() {
                    if let Some(folder_name) = path.file_name().and_then(|n| n.to_str()) {
                        println!("- {}: {}", folder_name, path.display().to_string())
                    }
                }
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
