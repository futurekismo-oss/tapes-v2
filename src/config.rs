use std::{collections::HashMap, fs, path::PathBuf, sync::OnceLock};

use anyhow::{Context, Result};
use dirs::data_local_dir;
use serde::{Deserialize, Serialize};

static CONFIG: OnceLock<Config> = OnceLock::new();

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    pub default_tape: Option<String>,
    #[serde(default)]
    pub install: InstallConfig,
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default)]
    pub targets: TargetsConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct InstallConfig {
    #[serde(default = "default_true")]
    pub shallow_clone: bool,
    #[serde(default = "default_true")]
    pub backup_on_force: bool,
    #[serde(default = "default_providers")]
    pub providers: HashMap<String, String>,
}

impl Default for InstallConfig {
    fn default() -> Self {
        Self {
            shallow_clone: true,
            backup_on_force: true,
            providers: default_providers(),
        }
    }
}

fn default_providers() -> HashMap<String, String> {
    let mut map = HashMap::new();
    map.insert("gh".to_string(), "https://github.com/".to_string());
    map.insert("github".to_string(), "https://github.com/".to_string());
    map.insert("gl".to_string(), "https://gitlab.com/".to_string());
    map.insert("gitlab".to_string(), "https://gitlab.com/".to_string());
    map.insert("cb".to_string(), "https://codeberg.org/".to_string());
    map.insert("codeberg".to_string(), "https://codeberg.org/".to_string());
    map
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct HooksConfig {
    #[serde(default)]
    pub global: GlobalHooksConfig,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GlobalHooksConfig {
    #[serde(default)]
    pub post_insert: Vec<String>,
    #[serde(default)]
    pub post_eject: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct TargetsConfig {
    #[serde(default)]
    pub ignore: Vec<String>,
}

fn default_true() -> bool {
    true
}

impl Config {
    /// Retrieve globally loaded configuration, or defaults if not initialized.
    pub fn get() -> &'static Config {
        CONFIG.get_or_init(|| match Self::load() {
            Ok(config) => config,
            Err(err) => {
                log::warn!("{err:#}");
                Self::default()
            }
        })
    }

    /// Set global configuration explicitly (e.g. for testing or initialization).
    pub fn set(config: Config) {
        let _ = CONFIG.set(config);
    }

    /// Config file location: ~/.local/share/tapes/config.toml
    pub fn default_config_path() -> Option<PathBuf> {
        data_local_dir().map(|h| h.join("tapes").join("config.toml"))
    }

    /// Find existing config file path at ~/.local/share/tapes/config.toml.
    pub fn find_config_file() -> Option<PathBuf> {
        let path = Self::default_config_path()?;
        path.exists().then_some(path)
    }

    /// Load configuration from file, or return default if no file exists.
    pub fn load() -> Result<Self> {
        let Some(path) = Self::find_config_file() else {
            return Ok(Self::default());
        };

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file at {}", path.display()))?;

        if content.trim().is_empty() {
            return Ok(Self::default());
        }

        toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file at {}", path.display()))
    }

    /// Returns a commented default configuration template.
    pub fn default_template() -> &'static str {
        r#"# ==============================================================================
# Tapes Configuration (~/.local/share/tapes/config.toml)
# ==============================================================================

# Default tape to insert when running `tapes insert` without arguments
# default_tape = "my-default"

[install]
# Clone with --depth 1 by default (set to false for full clone)
shallow_clone = true

# Create backup of existing tape directory on --force reinstall
backup_on_force = true

# Git provider shorthands (use as <provider>:<user>/<repo>)
[install.providers]
gh = "https://github.com/"
github = "https://github.com/"
gl = "https://gitlab.com/"
gitlab = "https://gitlab.com/"
cb = "https://codeberg.org/"
codeberg = "https://codeberg.org/"

# Global hooks executed across all tapes
[hooks.global]
# Commands executed after ANY tape is inserted
# post_insert = [
#     "notify-send 'Tapes' 'Switched tape to $TAPE_NAME'",
# ]

# Commands executed after ANY tape is ejected
# post_eject = [
#     "notify-send 'Tapes' 'Restored default config'",
# ]

[targets]
# Files or folders in a tape that should never be symlinked into ~/.config
ignore = [
    "wallpapers",
    "assets",
    "scripts",
    ".DS_Store",
]
"#
    }
}
