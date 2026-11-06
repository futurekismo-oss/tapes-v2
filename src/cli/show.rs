use std::path::Path;

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
        return Ok(());
    }

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

    Ok(())
}

pub fn display_tape_info(tape: TapeManifest, tapes_path: &Path) {
    println!("Name: {}", tape.tape.name);

    // Tapes description should not be required
    if let Some(description) = &tape.tape.desc {
        println!("Description: {}", description);
    }

    println!("Path: {}", tapes_path.to_string_lossy());

    if let Some(hooks) = &tape.hooks {
        println!("Hooks");
        print_hooks_section("Install", &hooks.install);
        print_hooks_section("Insert", &hooks.insert);
        print_hooks_section("Eject", &hooks.eject);
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

fn print_hooks_section(label: &str, hooks: &[String]) {
    if !hooks.is_empty() {
        println!("  {label}:");
        for hook in hooks {
            println!("   - {hook}");
        }
    }
}
