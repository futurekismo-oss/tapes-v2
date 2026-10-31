use anyhow::{bail, Context};
use dirs::home_dir;
use log::info;
use std::{
    fs, path::{Path, PathBuf}, process::Command, time::{SystemTime, UNIX_EPOCH},
};


pub fn run(
    source: &str,
    name: Option<String>,
    branch: Option<String>,
    dir: Option<String>,
) -> Result<(), anyhow::Error> {
    let source_path = PathBuf::from(source);

    let is_path = source_path.exists();

    let tape_name = if let Some(n) = name {
        n.trim().to_string()
    } else if is_path {
        source_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("tape")
            .to_string()
    // Get it from the repo if a path, dir or name is not provided
    } else {
        source
            .trim_end_matches('/')
            .trim_end_matches(".git")
            .rsplit('/')
            .next()
            .unwrap_or("tape")
            .to_string()
    };

    let tape_name = tape_name.to_lowercase(); // Make the dir name lowercase, no matter what

    let home = home_dir().ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let tapes_dir = home.join(".tapes");
    let target_dir = tapes_dir.join(&tape_name);

    if target_dir.exists() {
        bail!(
            "Tape {} already exists at {}",
            tape_name,
            target_dir.display()
        );
    }

    // create *all* the dir cuz first time users wont have the .tapes directory
    fs::create_dir_all(&tapes_dir)?;

    // Temporary dir for cloning
    let temp_clone_dir = tapes_dir.join(format!(
        ".tmp-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));

    // copy or clone based on whether exists or not
    if is_path {
        println!("Copying local tape '{tape_name}' from {source_path:?}...",);

        // If a specific dir provided locally, resolve it
        let src_to_copy = if let Some(ref d) = dir {
            let sub = source_path.join(d);
            if !sub.exists() {
                bail!("Directory '{}' does not exist in source path", d);
            }
            sub
        } else {
            source_path
        };
        copy_dir(&src_to_copy, &target_dir)?;
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

        let clone_destination = if dir.is_some() {
            &temp_clone_dir
        } else {
            &target_dir
        };

        // add the repo url and the path
        cmd.arg(&url).arg(&clone_destination);

        info!("git command: {:?}", cmd);

        let status = cmd.status().context("Failed to run git clone")?;

        if !status.success() {
            // _ bcz anything happens we have to clean the half cloned git directory
            let _ = fs::remove_dir_all(&temp_clone_dir);
            let _ = fs::remove_dir_all(&target_dir);
            bail!("git clone failed with status: {status}");
        }

        if let Some(ref d) = dir {
            let d = d.trim_start_matches('/');
            let sub = temp_clone_dir.join(d);

            info!("Requested subdir: {sub:?}");

            if !sub.exists() {
                let _ = fs::remove_dir_all(&temp_clone_dir);
                bail!("Directory '{}' does not exist in the cloned repository", d);
            }
            copy_dir(&sub, &target_dir)?;
            // Remove the full temporary repository clone, keeping only the extracted subdirectory
            let _ = fs::remove_dir_all(&temp_clone_dir);
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
#     # "pkill waybar",
#     # "qs -c {tape_name}"
# ]
# eject = [
#     # "hyprctl reload",
#     # "qs kill",
#     # "waybar &"
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
            .open(tapes_data.join("install_records.txt"))
            .and_then(|mut f| std::io::Write::write_all(&mut f, record.as_bytes()));
    }

    // Success confirmation message
    println!(
        "Successfully installed tape '{tape_name}' to {}",
        target_dir.display()
    );

    Ok(())
}

fn normalize_git_url(source: &str) -> String {
    if !source.contains("://") && !source.starts_with("git@") && source.contains('/') {
        return format!("https://github.com/{}.git", source.trim_end_matches(".git"));
    } else {
        return source.to_string();
    }
}

//  copy the files from the tree src to dst..
fn copy_dir(source: &Path, destination: &Path) -> Result<(), anyhow::Error> {
    fs::create_dir_all(destination)?;

    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let path = entry.path();

        // ignore specificed dir and files
        if !path.exists() {
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
