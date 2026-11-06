use anyhow::{Context, bail};
use log::{error, info};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{AppState, hooks, util::get_tapes_default_path};

pub struct InstallOptions {
    pub source: String,
    pub name: Option<String>,
    pub branch: Option<String>,
    pub dir: Option<String>,
    pub no_hooks: bool,
    pub yes: bool,
    pub force: bool,
    pub full_clone: bool,
    pub quiet: bool,
}

pub fn run(opts: InstallOptions) -> Result<(), anyhow::Error> {
    let source_trimmed = opts.source.trim();
    let source_path = expand_tilde(source_trimmed);
    let is_explicit = is_explicit_local_path(source_trimmed);
    let is_local_dir = is_local_source(&source_path, is_explicit)?;
    let tape_name = resolve_tape_name(opts.name.as_deref(), source_trimmed, &source_path, is_local_dir);

    info!("Resolved tape name: {tape_name}");

    let tapes_dir = get_tapes_default_path()?;
    let target_dir = tapes_dir.join(&tape_name);

    info!("tapes_dir: {:?}, target_dir: {:?}", tapes_dir, target_dir);

    prepare_target_dir(&target_dir, &tape_name, opts.force, opts.quiet)?;
    fs::create_dir_all(&tapes_dir)?;

    install_source(
        &opts,
        source_trimmed,
        &source_path,
        is_local_dir,
        &target_dir,
        &tapes_dir,
        &tape_name,
    )?;

    ensure_manifest(&target_dir, &tape_name, source_trimmed)?;
    record_installation(&tape_name, source_trimmed);

    let manifest = crate::util::load_manifest(&target_dir)?;
    if let Err(err) =
        hooks::run_install_hooks(&manifest, &target_dir, opts.no_hooks, opts.yes, opts.quiet)
    {
        error!("Install hook failed for tape '{tape_name}': {err}");
        eprintln!("\nError: Install hook failed: {err}");
        eprintln!(
            "Tape files remain at '{}', but install hooks did not complete successfully.",
            target_dir.display()
        );
        bail!("Install hooks failed for tape '{tape_name}'");
    }

    info!(
        "Successfully installed tape '{tape_name}' to {}",
        target_dir.display()
    );
    if !opts.quiet {
        println!(
            "Successfully installed tape '{tape_name}' to {}",
            target_dir.display()
        );
    }

    Ok(())
}

fn is_explicit_local_path(source: &str) -> bool {
    !source.contains("://")
        && !source.starts_with("git@")
        && (source.starts_with('.') || source.starts_with('/') || source.starts_with('~'))
}

fn is_local_source(path: &Path, is_explicit: bool) -> Result<bool, anyhow::Error> {
    if !is_explicit {
        return Ok(path.is_dir());
    }
    if !path.exists() {
        bail!("Local source path does not exist: {}", path.display());
    }
    if !path.is_dir() {
        bail!("Local source path is not a directory: {}", path.display());
    }
    Ok(true)
}

fn resolve_tape_name(
    name: Option<&str>,
    source: &str,
    source_path: &Path,
    is_local_dir: bool,
) -> String {
    let chosen = match name {
        Some(n) if !n.trim().is_empty() => n.trim().to_lowercase(),
        _ if is_local_dir => source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("tape")
            .to_lowercase(),
        _ => source
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or("tape")
            .to_lowercase(),
    };

    if chosen.is_empty() {
        "tape".to_string()
    } else {
        chosen
    }
}

fn prepare_target_dir(
    target_dir: &Path,
    tape_name: &str,
    force: bool,
    quiet: bool,
) -> Result<(), anyhow::Error> {
    if !target_dir.exists() {
        return Ok(());
    }

    if !force {
        bail!(
            "Tape '{tape_name}' already exists at {}.\nUse --force to overwrite.",
            target_dir.display()
        );
    }

    // Prevent destructive overwrite if this tape is currently active
    if let Ok(app) = AppState::load_current()
        && app.active_tape.as_deref() == Some(tape_name)
    {
        bail!(
            "Tape '{tape_name}' is currently inserted. Please eject it before reinstalling: 'tapes eject'"
        );
    }

    if !quiet {
        println!("Removing existing tape at {}...", target_dir.display());
    }
    fs::remove_dir_all(target_dir).with_context(|| {
        format!("Failed to remove existing tape at {}", target_dir.display())
    })
}

fn install_source(
    opts: &InstallOptions,
    source_trimmed: &str,
    source_path: &Path,
    is_local_dir: bool,
    target_dir: &Path,
    tapes_dir: &Path,
    tape_name: &str,
) -> Result<(), anyhow::Error> {
    if is_local_dir {
        if !opts.quiet {
            println!(
                "Copying local tape '{tape_name}' from {}...",
                source_path.display()
            );
        }
        let src_to_copy = resolve_subdir(source_path, opts.dir.as_deref())?;
        return copy_dir(&src_to_copy, target_dir);
    }

    if !opts.quiet {
        println!("Cloning tape '{tape_name}' from {source_trimmed}...");
    }

    let temp_clone_dir = opts.dir.as_ref().map(|_| {
        tapes_dir.join(format!(
            ".tmp-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_nanos()
        ))
    });

    let url = normalize_git_url(source_trimmed);
    let mut cmd = Command::new("git");
    cmd.arg("clone");

    if !opts.full_clone {
        cmd.arg("--depth").arg("1");
    }

    if let Some(ref b) = opts.branch {
        cmd.arg("--branch").arg(b);
    }

    let clone_dest = temp_clone_dir.as_deref().unwrap_or(target_dir);
    cmd.arg(&url).arg(clone_dest);

    info!("git command: {:?}", cmd);
    let status = cmd.status().context("Failed to execute git clone")?;

    if !status.success() {
        if let Some(ref tmp) = temp_clone_dir {
            let _ = fs::remove_dir_all(tmp);
        }
        let _ = fs::remove_dir_all(target_dir);
        bail!("git clone failed with status: {status}");
    }

    if let Some(ref tmp) = temp_clone_dir {
        let copy_res = resolve_subdir(tmp, opts.dir.as_deref())
            .and_then(|sub| copy_dir(&sub, target_dir));
        let _ = fs::remove_dir_all(tmp);
        copy_res?;
    }

    Ok(())
}

fn ensure_manifest(
    target_dir: &Path,
    tape_name: &str,
    source_trimmed: &str,
) -> Result<(), anyhow::Error> {
    let manifest_path = target_dir.join("tape.toml");
    if manifest_path.exists() {
        return Ok(());
    }

    let example_toml = format!(
        r#"[tape]
name = "{tape_name}"
desc = "Tape installed from {source_trimmed}"

# [hooks]
# insert = [
#     # "hyprctl reload",
#     # "pkill waybar",
#     # "qs -c {tape_name} -d" # -d runs in daemon mode
# ]
# eject = [
#     # "hyprctl reload",
#     # "qs kill", 
#     # "waybar &"
# ]
# install = [
#     # "./setup.sh"
# ]
"#
    );

    fs::write(&manifest_path, example_toml)?;
    Ok(())
}

fn expand_tilde(path_str: &str) -> PathBuf {
    if let Some(rest) = path_str.strip_prefix("~/")
        && let Some(home) = dirs::home_dir()
    {
        return home.join(rest);
    }
    if path_str == "~"
        && let Some(home) = dirs::home_dir()
    {
        return home;
    }
    PathBuf::from(path_str)
}

fn resolve_subdir(base: &Path, dir: Option<&str>) -> Result<PathBuf, anyhow::Error> {
    let Some(d) = dir else {
        return Ok(base.to_path_buf());
    };
    let sub = base.join(d.trim_start_matches('/'));
    if !sub.exists() {
        bail!("Directory '{d}' does not exist in {}", base.display());
    }
    Ok(sub)
}

fn normalize_git_url(source: &str) -> String {
    if source.contains("://") || source.starts_with("git@") {
        return source.to_string();
    }

    // Support shorthands like gitlab:user/repo, gl:user/repo, gh:user/repo
    if let Some(rest) = source
        .strip_prefix("gitlab:")
        .or_else(|| source.strip_prefix("gl:"))
    {
        return format!("https://gitlab.com/{}.git", rest.trim_end_matches(".git"));
    }
    if let Some(rest) = source
        .strip_prefix("github:")
        .or_else(|| source.strip_prefix("gh:"))
    {
        return format!("https://github.com/{}.git", rest.trim_end_matches(".git"));
    }
    if let Some(rest) = source
        .strip_prefix("codeberg:")
        .or_else(|| source.strip_prefix("cb:"))
    {
        return format!("https://codeberg.org/{}.git", rest.trim_end_matches(".git"));
    }

    for host in &["github.com", "gitlab.com", "codeberg.org"] {
        if let Some(rest) = source.strip_prefix(&format!("{host}/")) {
            return format!("https://{host}/{}.git", rest.trim_end_matches(".git"));
        }
    }

    if source.contains('/') {
        format!("https://github.com/{}.git", source.trim_end_matches(".git"))
    } else {
        source.to_string()
    }
}

fn copy_dir(source: &Path, destination: &Path) -> Result<(), anyhow::Error> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();
        let target = destination.join(entry.file_name());

        let file_type = entry.file_type()?;

        if file_type.is_symlink() {
            let link_target = fs::read_link(&path)?;
            if target.symlink_metadata().is_ok() {
                let _ = fs::remove_file(&target);
            }
            std::os::unix::fs::symlink(&link_target, &target)
                .with_context(|| format!("Failed to create symlink at {}", target.display()))?;
        } else if file_type.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            fs::copy(&path, &target).with_context(|| {
                format!("Failed to copy {} to {}", path.display(), target.display())
            })?;
        }
    }

    Ok(())
}

fn record_installation(tape_name: &str, source: &str) {
    if let Some(data) = dirs::data_dir() {
        let tapes_data = data.join("tapes");
        let _ = fs::create_dir_all(&tapes_data);
        let records_path = tapes_data.join("install_records.txt");

        let mut lines = Vec::new();
        if let Ok(content) = fs::read_to_string(&records_path) {
            for line in content.lines() {
                if !line.starts_with(&format!("name={tape_name},")) {
                    lines.push(line.to_string());
                }
            }
        }
        lines.push(format!("name={tape_name}, source={source}"));
        let mut updated = lines.join("\n");
        updated.push('\n');
        let _ = fs::write(&records_path, updated);
    }
}
