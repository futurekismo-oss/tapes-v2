use anyhow::{Result, anyhow, bail};
use log::warn;
use std::{env, process::Command, sync::LazyLock};

use crate::toml::TapeManifest;

static USER_SHELL: LazyLock<String> =
    LazyLock::new(|| env::var("SHELL").unwrap_or_else(|_| "sh".to_string()));

pub fn run_hooks(hooks: Vec<String>, quiet: bool) -> Result<(), anyhow::Error> {
    if hooks.is_empty() {
        return Ok(());
    }

    for hook in hooks {
        let hook = hook.trim();
        if hook.is_empty() {
            continue;
        }

        if !quiet {
            println!(" - '{hook}'");
        }

        if let Err(err) = run_command(hook) {
            bail!("Hook failed: '{hook}': {err}");
        }
    }

    Ok(())
}

pub fn run_command(command: &str) -> Result<(), anyhow::Error> {
    let status = Command::new(&*USER_SHELL).arg("-c").arg(command).status()?;

    if !status.success() {
        return Err(anyhow!("Command failed with exit status: {status}"));
    }

    Ok(())
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
        warn!("{} hooks completed with warnings/errors: {}", stage, err);
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
    let hooks = manifest.hooks.map(|h| h.install);
    run_lifecycle_hooks(hooks, no_hooks, false, "Install")
}
