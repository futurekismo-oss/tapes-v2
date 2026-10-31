use crate::{
    cli::eject::eject_tapes,
    hooks::run_insert_hooks,
    toml::TapeManifest,
    util::{
        atomic_swap, find_targets, get_tapes_data, get_tapes_default_path, load_manifest,
        save_state_data, yes_or_no_prompt,
    },
    AppState,
};
use anyhow::bail;
use dirs::{config_dir, home_dir};
use log::{error, info};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use std::fs;
use std::os::unix::fs as unix_fs;

// Allow multiple CLI arguments passed directly to the runner without extra boilerplate
#[allow(clippy::too_many_arguments)]
pub fn run(
    // Mutable reference to the application state
    app: &mut AppState,
    // Optional library name of the tape to insert
    name: Option<String>,
    // Optional arbitrary filesystem path to the tape
    path: Option<PathBuf>,
    // Plan only; touch nothing on the filesystem
    dry_run: bool,
    // Skip pre/post hooks
    no_hooks: bool,
    // Reduce log output verbosity
    quiet: bool,
    // List of target names to selectively include from the tape
    include_targets: Option<Vec<String>>,
    // List of target names to selectively exclude from the tape
    exclude_targets: Option<Vec<String>>,
) -> Result<(), anyhow::Error> {
    info!("starting insert command");
    info!("Current tapes: {:?}", app.active_tape);

    let config_dir = config_dir().expect("config dir should exist");
    let home = home_dir().expect("Home dir should exist");

    if app.active_tape.is_some() && !dry_run {
        if !yes_or_no_prompt("Another tape is currently inserted.\nEject?") {
            bail!("Another tape is currently inserted, doing nothing, exiting");
        };

        let eject_staging = home.join(".config.tapes");
        eject_tapes(app, &config_dir, &eject_staging, quiet, no_hooks)?;
    }

    let new_config_dir = home.join(".config.tapes");
    let (tape_path, manifest) = resolve_tape(name.as_ref(), path)?;
    let tape_name = manifest.tape.name.clone();

    if config_dir.is_symlink() {
        bail!("{config_dir:?} is a symlink, prone to fail");
    }

    app.path = Some(tape_path);
    app.active_tape = Some(tape_name);

    // Parse include target list from comma-delimited arguments
    let parsed_include = include_targets.as_ref().map(|list| parse_target_list(list));
    // Parse exclude target list from comma-delimited arguments
    let parsed_exclude = exclude_targets.as_ref().map(|list| parse_target_list(list));

    // Resolve target mapping, applying inclusion and exclusion filters to tape targets
    let targets = resolve_targets(
        app,
        &new_config_dir,
        parsed_include.as_deref(),
        parsed_exclude.as_deref(),
    )?;
    app.targets = Some(targets);

    if dry_run {
        println!("Created {new_config_dir:?}...");

        for (name, target) in &targets {
            println!(
                "{name}: symlinking {:?} -> {:?}",
                target.target_path, target.symlink_dest
            );
        }

        return Ok(());
    }

    app.targets = Some(targets);


    if new_config_dir.exists() {
        info!("{new_config_dir:?} exists - something has gone wrong");
        bail!(".config.tapes exists, did something go wrong? bug report here https://github.com/futurekismo-oss/tapes-v2/issues");
    }

    fs::create_dir(&new_config_dir)?;
    info!("Created {new_config_dir:?}");

    let marker_path = new_config_dir.join(".tapes-managed");
    File::create(&marker_path)?;
    info!("Created: {marker_path:?}");

    if let Err(err) = symlink_targets(
        app.targets.as_ref().expect("app.targets should exist"),
        quiet,
    ) {
        error!("symlink_targets failed: {err}");
        return Err(err);
    }

    if let Err(err) = atomic_swap(&config_dir, &new_config_dir) {
        error!("atomic swap failed: {err}");
        return Err(err.into());
    }
    info!("atomic swapped: {config_dir:?} -> {new_config_dir:?}");

    println!(
        "Inserted: {}",
        app.active_tape.as_deref().unwrap().to_string()
    );

    save_state_data(app)?;

    run_insert_hooks(manifest, no_hooks, quiet)?;

    Ok(())
}

pub fn resolve_tape(
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

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Target {
    target_path: PathBuf,
    symlink_dest: PathBuf,
    from_tapes: bool,
}

// Resolves targets by preserving untouched user configs and applying filtered tape targets.
fn resolve_targets(
    // Application state holding the tape path
    app: &AppState,
    // Staging directory path (.config.tapes)
    new_config_dir: &Path,
    // Optional list of targets to include from the tape
    include_targets: Option<&[String]>,
    // Optional list of targets to exclude from the tape
    exclude_targets: Option<&[String]>,
) -> Result<HashMap<String, Target>, anyhow::Error> {
    // Locate the user's ~/.config directory
    let config_dir = config_dir().expect("Config dir should exist");

    // Discover existing user targets to preserve them from being lost
    let configs = find_targets(&config_dir)?
        // Turn into an iterator over paths
        .into_iter()
        // Map each preserved target with from_tapes set to false
        .map(|p| map_target(p, new_config_dir, false));

    // Retrieve the path of the tape being inserted
    let tapes_path = app.path.as_deref().expect("path just set");
    // Discover all candidate targets inside the tape
    let tape_targets = find_targets(tapes_path)?;

    // If an include list is provided, verify whether requested targets exist in tape
    if let Some(includes) = include_targets {
        // Iterate over each target name the user wanted to include
        for inc in includes {
            // Check if there is a matching target in the tape
            let exists = tape_targets
                .iter()
                .any(|p| p.file_name().and_then(|n| n.to_str()) == Some(inc));
            // Warn the user if their requested target was not found in the tape
            if !exists {
                eprintln!("Warning: target '{inc}' specified in --include-targets was not found in tape");
            }
        }
    }

    // Filter tape targets according to include and exclude rules
    let filtered_tapes = tape_targets.into_iter().filter(|p| {
        // Extract the target file or directory name
        let Some(name) = p.file_name().and_then(|n| n.to_str()) else {
            // Skip invalid non-UTF-8 entries
            return false;
        };
        // Check if this target passes include and exclude criteria
        should_include_target(name, include_targets, exclude_targets)
    });

    // Map accepted tape targets with from_tapes set to true
    let tapes = filtered_tapes.map(|p| map_target(p, new_config_dir, true));

    // Combine preserved configs and tape targets into a HashMap (tape targets overwrite preserved)
    Ok(configs.chain(tapes).collect())
}

// Determines if a tape target should be included based on include and exclude filters.
pub fn should_include_target(
    // Name of the target directory or file
    target_name: &str,
    // Optional list of targets allowed to be inserted
    include_targets: Option<&[String]>,
    // Optional list of targets forbidden from being inserted
    exclude_targets: Option<&[String]>,
) -> bool {
    // If an include list is specified, ensure this target is in the list
    if let Some(includes) = include_targets {
        // If not found in the include list, target is skipped
        if !includes.iter().any(|item| item == target_name) {
            // Target was not in the include list
            return false;
        }
    }

    // If an exclude list is specified, ensure this target is NOT in the list
    if let Some(excludes) = exclude_targets {
        // If found in the exclude list, target is skipped
        if excludes.iter().any(|item| item == target_name) {
            // Target was in the exclude list
            return false;
        }
    }

    // Passed all include and exclude rules
    true
}

// Parses raw CLI target arguments into clean target names.
// Supports comma separation and trims any surrounding whitespace.
pub fn parse_target_list(raw_targets: &[String]) -> Vec<String> {
    // Vector to store the resulting cleaned target names
    let mut targets = Vec::new();

    // Iterate through each raw argument string passed from CLI
    for item in raw_targets {
        // Split on comma delimiter only
        for part in item.split(',') {
            // Trim leading and trailing whitespace
            let trimmed = part.trim();
            // Discard empty entries
            if !trimmed.is_empty() {
                // Add the cleaned target name to our vector
                targets.push(trimmed.to_string());
            }
        }
    }

    // Return the cleaned list of target names
    targets
}

fn map_target(target_path: PathBuf, new_config_dir: &Path, from_tapes: bool) -> (String, Target) {
    let file_name = target_path.file_name().unwrap().to_owned();
    let target_name = file_name.to_string_lossy().into_owned();
    let symlink_dest = new_config_dir.join(&file_name);

    let resolved_path = if from_tapes {
        target_path
    } else {
        symlink_dest.clone()
    };

    let target = Target {
        target_path: resolved_path,
        symlink_dest,
        from_tapes,
    };

    (target_name, target)
}

fn symlink_targets(
    targets_map: &HashMap<String, Target>,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    let targets: Vec<&Target> = targets_map.values().collect();

    for target in &targets {
        if let Err(e) = unix_fs::symlink(&target.target_path, &target.symlink_dest) {
            error!(
                "Error: {e}, failed to symlink {:?} -> {:?}",
                target.target_path, target.symlink_dest
            );
            continue;
        }

        if !quiet {
            println!(
                "symlinking {:?} -> {:?}",
                target.target_path, target.symlink_dest
            );
        }
    }

    Ok(())
}

