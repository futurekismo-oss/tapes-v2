use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::insert::Target;

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct AppState {
    pub active_tape: Option<String>,
    pub path: Option<PathBuf>,
    pub targets: Option<HashMap<String, Target>>,
}

impl AppState {
    pub fn default_path() -> Result<PathBuf, anyhow::Error> {
        let local_dir = dirs::data_dir().ok_or_else(|| anyhow::anyhow!("Could not get local dir"))?;
        let dir = local_dir.join("tapes");
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        Ok(dir.join("state.json"))
    }

    pub fn load_current() -> Result<Self, anyhow::Error> {
        Self::load(Self::default_path()?)
    }

    pub fn save_current(&self) -> Result<(), anyhow::Error> {
        self.save(Self::default_path()?)
    }

    pub fn load<P: AsRef<Path>>(file_path: P) -> Result<Self, anyhow::Error> {
        let path = file_path.as_ref();
        if !path.exists() {
            return Ok(AppState::default());
        }
        let content = fs::read_to_string(path)?;
        if content.trim().is_empty() {
            return Ok(AppState::default());
        }
        match serde_json::from_str(&content) {
            Ok(state) => Ok(state),
            Err(err) => {
                log::warn!(
                    "Corrupted state file at {}: {}. Falling back to default.",
                    path.display(),
                    err
                );
                if let Err(e) = fs::copy(path, path.with_extension("json.bak")) {
                    log::debug!("Could not backup corrupted state file: {e}");
                }
                Ok(AppState::default())
            }
        }
    }

    pub fn save<P: AsRef<Path>>(&self, file_path: P) -> Result<(), anyhow::Error> {
        let path = file_path.as_ref();

        let parent = path
            .parent()
            .ok_or_else(|| anyhow::anyhow!("File path has no parent directory"))?;

        let file_name = path
            .file_name()
            .ok_or_else(|| anyhow::anyhow!("Invalid file path"))?;

        let mut temp_file_name = file_name.to_os_string();
        temp_file_name.push(".tmp");
        let temp_path = parent.join(temp_file_name);

        let content = serde_json::to_string_pretty(self)?;

        fs::write(&temp_path, content)?;
        fs::rename(&temp_path, path)?;

        Ok(())
    }

    pub fn eject(&mut self) -> Result<(), anyhow::Error> {
        self.active_tape = None;
        self.path = None;
        self.targets = None;

        Ok(())
    }
}
