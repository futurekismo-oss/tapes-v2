use anyhow::{Result, anyhow, bail};
use log::warn;
use std::{env, path::Path, process::Command, sync::LazyLock};

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
    if no_hooks {
        return Ok(());
    }

    let Some(hooks) = hooks else {
        return Ok(());
    };

    if hooks.is_empty() {
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
    manifest: &TapeManifest,
    tape_path: Option<&Path>,
    no_hooks: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    let hooks = manifest.hooks.as_ref().map(|h| h.eject.as_slice());
    let envs = hook_env_vars(&manifest.tape.name, tape_path);
    run_lifecycle_hooks(hooks, tape_path, &envs, no_hooks, quiet, "Eject")
}

pub fn run_insert_hooks(
    manifest: &TapeManifest,
    tape_path: &Path,
    no_hooks: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    let hooks = manifest.hooks.as_ref().map(|h| h.insert.as_slice());
    let envs = hook_env_vars(&manifest.tape.name, Some(tape_path));
    run_lifecycle_hooks(hooks, Some(tape_path), &envs, no_hooks, quiet, "Insert")
}

pub fn run_install_hooks(
    manifest: &TapeManifest,
    target_dir: &Path,
    no_hooks: bool,
    yes: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    if no_hooks {
        return Ok(());
    }

    let Some(ref hooks) = manifest.hooks else {
        return Ok(());
    };

    if hooks.install.is_empty() {
        return Ok(());
    }

    if !yes {
        use std::io::IsTerminal;
        if !std::io::stdin().is_terminal() {
            if !quiet {
                println!("Non-interactive terminal detected, skipping install hooks.");
            }
            return Ok(());
        }

        println!(
            "\nTape '{}' defines the following install hook(s):",
            manifest.tape.name
        );
        for hook in &hooks.install {
            println!("  - '{hook}'");
        }
        if !yes_or_no_prompt("Do you want to run these install hooks?") {
            if !quiet {
                println!("Skipping install hooks.");
            }
            return Ok(());
        }
    }

    if !quiet {
        println!("Running install hooks in {}...", target_dir.display());
    }

    let envs = hook_env_vars(&manifest.tape.name, Some(target_dir));
    run_hooks(&hooks.install, Some(target_dir), &envs, quiet)
}
