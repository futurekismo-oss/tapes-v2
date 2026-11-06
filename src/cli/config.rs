use std::fs;

use anyhow::{Context, Result, bail};
use clap::Subcommand;

use crate::config::Config;

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    /// Show the path of the active config file
    Path,
    /// Display the current effective configuration in TOML format
    Show,
    /// Create a sample configuration file at ~/.local/share/tapes/config.toml
    Init {
        /// Overwrite existing configuration file
        #[arg(short, long)]
        force: bool,
    },
}

pub fn run(action: Option<ConfigAction>) -> Result<(), anyhow::Error> {
    match action.unwrap_or(ConfigAction::Path) {
        ConfigAction::Path => {
            if let Some(path) = Config::find_config_file() {
                println!("{}", path.display());
                return Ok(());
            }

            let Some(default_path) = Config::default_config_path() else {
                println!("No config file found.");
                return Ok(());
            };

            println!(
                "No config file found (default location: {})",
                default_path.display()
            );
        }
        ConfigAction::Show => {
            let config = Config::load()?;
            let toml_str = toml::to_string_pretty(&config)
                .context("Failed to serialize configuration to TOML")?;
            println!("{toml_str}");
        }
        ConfigAction::Init { force } => {
            let path = Config::default_config_path()
                .ok_or_else(|| anyhow::anyhow!("Could not determine configuration directory"))?;

            if path.exists() && !force {
                bail!(
                    "Config file already exists at {}.\nUse -f / --force to overwrite.",
                    path.display()
                );
            }

            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).with_context(|| {
                    format!("Failed to create directory at {}", parent.display())
                })?;
            }

            fs::write(&path, Config::default_template()).with_context(|| {
                format!("Failed to write configuration file at {}", path.display())
            })?;

            println!("Created configuration file at {}", path.display());
        }
    }

    Ok(())
}
