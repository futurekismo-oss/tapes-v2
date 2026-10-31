use anyhow::{anyhow, bail, Result};
use log::error;
use std::{env, process::Command};

use crate::toml::TapeManifest;

pub fn run_hooks(hooks: Vec<String>, quiet: bool) -> Result<(), anyhow::Error> {
    if hooks.is_empty() {
        return Ok(());
    }

    let mut failed = false;

    for hook in hooks {
        let hook = hook.trim();

        if !quiet {
            println!(" - '{}'", hook);
        }

        if let Err(err) = run_command(hook) {
            error!("Hook failed: '{}': {}", hook, err);
            failed = true;
        }
    }

    if failed {
        bail!("One or more hooks failed");
    }

    Ok(())
}

pub fn run_command(command: &str) -> Result<(), anyhow::Error> {
    let shell = get_user_preferred_shell();

    let status = Command::new(shell).arg("-c").arg(command).status()?;

    if !status.success() {
        return Err(anyhow!("Command failed with exit status: {}", status));
    }

    Ok(())
}

fn get_user_preferred_shell() -> String {
    env::var("SHELL").unwrap_or("sh".to_string())
}

fn run_lifecycle_hooks(
    hooks: Option<Vec<String>>,
    no_hooks: bool,
    quiet: bool,
    stage: &str,
) -> Result<(), anyhow::Error> {
    if no_hooks {
        return Ok(());
    }

    let Some(hooks) = hooks else {
        return Ok(());
    };

    if hooks.is_empty() {
        return Ok(());
    }

    println!("Running hooks");

    if let Err(err) = run_hooks(hooks, quiet) {
        error!("{} hooks completed with warnings/errors: {}", stage, err);
    }

    Ok(())
}

pub fn run_eject_hooks(
    manifest: TapeManifest,
    no_hooks: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    let hooks = manifest.hooks.map(|h| h.eject);
    run_lifecycle_hooks(hooks, no_hooks, quiet, "Eject")
}

pub fn run_insert_hooks(
    manifest: TapeManifest,
    no_hooks: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    let hooks = manifest.hooks.map(|h| h.insert);
    run_lifecycle_hooks(hooks, no_hooks, quiet, "Insert")
}

pub fn run_install_hooks(
    manifest: TapeManifest,
    no_hooks: bool,
    // quiet: bool,
) -> Result<(), anyhow::Error> {
    let hooks = manifest.hooks.map(|h| h.eject);
    run_lifecycle_hooks(hooks, no_hooks, false, "Install")
}

