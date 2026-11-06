use anyhow::{Result, anyhow, bail};
use log::warn;
use std::{env, io::IsTerminal, path::Path, process::Command, sync::LazyLock};

use crate::{toml::TapeManifest, util::yes_or_no_prompt};

static USER_SHELL: LazyLock<String> =
    LazyLock::new(|| env::var("SHELL").unwrap_or_else(|_| "sh".to_string()));

pub fn run_hooks(
    hooks: &[String],
    working_dir: Option<&Path>,
    env_vars: &[(&str, String)],
    quiet: bool,
) -> Result<(), anyhow::Error> {
    if hooks.is_empty() {
        return Ok(());
    }

    let mut failed: Vec<(String, anyhow::Error)> = Vec::new();

    for hook in hooks {
        let hook = hook.trim();
        if hook.is_empty() {
            continue;
        }

        if !quiet {
            println!(" - '{hook}'");
        }

        if let Err(err) = run_command(hook, working_dir, env_vars) {
            warn!("Hook failed: '{hook}': {err}");
            if !quiet {
                eprintln!("   [!] Hook failed: '{hook}': {err}");
            }
            failed.push((hook.to_string(), err));
        }
    }

    if !failed.is_empty() {
        let count = failed.len();
        let summary = failed
            .into_iter()
            .map(|(h, err)| format!("'{h}' ({err})"))
            .collect::<Vec<_>>()
            .join(", ");
        bail!("{count} hook(s) failed: {summary}");
    }

    Ok(())
}

pub fn run_command(
    command: &str,
    working_dir: Option<&Path>,
    env_vars: &[(&str, String)],
) -> Result<(), anyhow::Error> {
    let mut cmd = Command::new(&*USER_SHELL);
    cmd.arg("-c").arg(command);

    if let Some(dir) = working_dir
        && dir.exists()
    {
        cmd.current_dir(dir);
    }

    for (key, val) in env_vars {
        cmd.env(key, val);
    }

    let status = cmd.status()?;

    if !status.success() {
        return Err(anyhow!("Command failed with exit status: {status}"));
    }

    Ok(())
}

fn run_lifecycle_hooks(
    hooks: Option<&[String]>,
    working_dir: Option<&Path>,
    env_vars: &[(&str, String)],
    no_hooks: bool,
    quiet: bool,
    stage: &str,
) -> Result<(), anyhow::Error> {
    let hooks = hooks.unwrap_or_default();
    if no_hooks || hooks.is_empty() {
        return Ok(());
    }

    if !quiet {
        println!("Running {stage} hooks");
    }

    if let Err(err) = run_hooks(hooks, working_dir, env_vars, quiet) {
        warn!("{stage} hooks completed with warnings/errors: {err}");
    }

    Ok(())
}

fn hook_env_vars(tape_name: &str, tape_dir: Option<&Path>) -> Vec<(&'static str, String)> {
    let mut envs = Vec::with_capacity(3);
    envs.push(("TAPE_NAME", tape_name.to_string()));
    if let Some(dir) = tape_dir {
        envs.push(("TAPE_DIR", dir.to_string_lossy().into_owned()));
    }
    if let Some(cfg) = dirs::config_dir() {
        envs.push(("CONFIG_DIR", cfg.to_string_lossy().into_owned()));
    }
    envs
}

pub fn run_eject_hooks(
    manifest: Option<&TapeManifest>,
    tape_name: &str,
    tape_path: Option<&Path>,
    no_hooks: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    let envs = hook_env_vars(tape_name, tape_path);

    let tape_hooks = manifest
        .and_then(|m| m.hooks.as_ref())
        .map(|h| h.eject.as_slice());
    run_lifecycle_hooks(tape_hooks, tape_path, &envs, no_hooks, quiet, "Eject")?;

    let global_hooks = &crate::config::Config::get().hooks.global.post_eject;
    run_lifecycle_hooks(
        Some(global_hooks),
        None,
        &envs,
        no_hooks,
        quiet,
        "global post-eject",
    )
}

pub fn run_insert_hooks(
    manifest: &TapeManifest,
    tape_path: &Path,
    no_hooks: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    let envs = hook_env_vars(&manifest.tape.name, Some(tape_path));

    let tape_hooks = manifest.hooks.as_ref().map(|h| h.insert.as_slice());
    run_lifecycle_hooks(
        tape_hooks,
        Some(tape_path),
        &envs,
        no_hooks,
        quiet,
        "Insert",
    )?;

    let global_hooks = &crate::config::Config::get().hooks.global.post_insert;
    run_lifecycle_hooks(
        Some(global_hooks),
        Some(tape_path),
        &envs,
        no_hooks,
        quiet,
        "global post-insert",
    )
}

pub fn run_install_hooks(
    manifest: &TapeManifest,
    target_dir: &Path,
    no_hooks: bool,
    yes: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    let hooks = manifest
        .hooks
        .as_ref()
        .map(|h| h.install.as_slice())
        .unwrap_or_default();

    if no_hooks || hooks.is_empty() {
        return Ok(());
    }

    if !yes && !confirm_install_hooks(&manifest.tape.name, hooks, quiet) {
        return Ok(());
    }

    if !quiet {
        println!("Running install hooks in {}...", target_dir.display());
    }

    let envs = hook_env_vars(&manifest.tape.name, Some(target_dir));
    run_hooks(hooks, Some(target_dir), &envs, quiet)
}

fn confirm_install_hooks(tape_name: &str, hooks: &[String], quiet: bool) -> bool {
    if !std::io::stdin().is_terminal() {
        if !quiet {
            println!("Non-interactive terminal detected, skipping install hooks.");
        }
        return false;
    }

    println!("\nTape '{tape_name}' defines the following install hook(s):");
    for hook in hooks {
        println!("  - '{hook}'");
    }

    let confirmed = yes_or_no_prompt("Do you want to run these install hooks?");
    if !confirmed && !quiet {
        println!("Skipping install hooks.");
    }

    confirmed
}
