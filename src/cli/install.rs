// the libraries...
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use anyhow::{Context, bail};

use dirs::home_dir;

// main entry point for the command
pub fn run(source: &str, name: Option<&str>, branch: Option<&str>) -> Result<(), anyhow::Error> {
    // to fix relative paths as absolute home paths
    let source_path = resolve_input_path(source);

    // if exist -> copy else -> clone
    let is_path = source_path.exists();

    let tape_name = match name {
        // when name given 
        Some(n) => n.trim().to_string(),
        // else make it the name of the local folder 
        None if is_path => source_path.file_name().and_then(|n| n.to_str()).unwrap_or("tape").to_string(),
        // else make it the name of the git repo
        // /.git removed -> user/repo .next() and unwrap for the name repo
        None => source.trim_end_matches('/').trim_end_matches(".git").rsplit('/').next().unwrap_or("tape").to_string(),
    };

    // home directory
    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    // join to ~/.tapes
    let tapes_dir = home.join(".tapes");

    // join the tape name we made to make the final directory name!
    let target_dir = tapes_dir.join(&tape_name);

    // if exists --> no overwrite 
    if target_dir.exists() {
        bail!("Tape {} already exists at {}", tape_name, target_dir.display());
    }

    // create *all* the dir cuz first time users wont have the .tapes directory
    fs::create_dir_all(&tapes_dir)?;

    // copy or clone based on whether exists or not
    if is_path {
        println!("Copying local tape '{tape_name}' from {}...", source_path.display());
        copy_dir(&source_path, &target_dir)?;
    } else {
        // print cloning
        println!("Copying tape '{tape_name}' from {source}");

        // prepare the git command
        let url = normalize_git_url(source);
        let mut cmd = Command::new("git");
        cmd.arg("clone");

        // add the branch tag
        if let Some(b) = branch {
            cmd.arg("--branch").arg(b);
        }

        // add the repo url and the path
        cmd.arg(&url).arg(&target_dir);

        let status = cmd.status().context("Failed to run git clone")?;

        if !status.success() {
            // _ bcz anything happens we have to clean the half cloned git directory
            let _ = fs:: remove_dir_all(&target_dir);
            bail!("git clone failed with status: {status}");
        }
    }

    // now making the manifest file..
    let manifest_path = target_dir.join("tape.toml");

    if !manifest_path.exists() {
        let example_toml = format!(
r#"[tape]
name = "{tape_name}"
desc = "Tape installed from {source}"

# [hooks]
# insert = [
#     # "hyprctl reload",
#     # "pkill -SIGUSR2 waybar",
# ]
# eject = [
#     # "hyprctl reload",
# ]
"#
        );

        fs::write(manifest_path, example_toml)?;
    }
    
    if let Some(data) = dirs::data_dir() {
        let tapes_data = data.join("tapes");
        // Ensure ~/.local/share/tapes exists
        let _ = fs::create_dir_all(&tapes_data);
        // Record format: simple name and source tracking
        let record = format!("name={tape_name}, source={source}\n");
        // Append entry to records.txt (create if not exists)
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(tapes_data.join("records.txt"))
            .and_then(|mut f| std::io::Write::write_all(&mut f, record.as_bytes()));
    }

    // Success confirmation message
    println!("Successfully installed tape '{tape_name}' to {}", target_dir.display());

    Ok(())
}   

fn resolve_input_path(source : &str) -> PathBuf {
    // if has ~ add the home_dir name..
    if let Some(stripped) = source.strip_prefix("~/"){
        if let Some(home)= home_dir() {
            return home.join(stripped);
        }
    }
    // else return
    return PathBuf::from(source);
}

fn normalize_git_url(source : &str) -> String {
    if !source.contains("://") && !source.starts_with("git@") && source.contains('/'){
        return format!("https://github.com/{}.git", source.trim_end_matches(".git"));
    } else{
        return source.to_string();
    }
}

//  copy the files from the tree src to dst..
fn copy_dir(source : &Path, destination : &Path) -> Result<(), anyhow::Error> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)?{
        let entry = entry?;
        let path = entry.path();
        // dont copy the .git
        if entry.file_name() == ".git" {
            continue;
        }
        let target = destination.join(entry.file_name());
        // recursive copy for all the subdirectories
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            fs::copy(&path, &target)?;
        }
    }

    return Ok(());
}