use std::fs;

use crate::{
    toml::TapeManifest,
    util::{get_tapes_data, get_tapes_default_path},
};

pub fn run(name: Option<String>) -> Result<(), anyhow::Error> {
    if let Some(name) = name {
        let name = name.to_lowercase();

        let tapes = get_tapes_data(&name)?;
        display_tape_info(tapes);
    } else {
        println!("All Tapes (in ~/.tapes)");

        let tapes_path = get_tapes_default_path()?;
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

fn display_tape_info(tape: TapeManifest) {
    println!("Name: {}", tape.tape.name);
    println!("Description: {}", tape.tape.desc);

    if let Some(hooks) = &tape.hooks {
        println!("Hooks");

        if !hooks.insert.is_empty() {
            println!("  Insert:");
            for hook in &hooks.insert {
                println!("\t- {}", hook);
            }
        }

        if !hooks.eject.is_empty() {
            println!("  Eject:");
            for hook in &hooks.eject {
                println!("\t- {}", hook);
            }
        }
    }
}
