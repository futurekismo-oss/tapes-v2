use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::cli::insert::Target;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppState {
    pub active_tape: Option<String>,
    pub path: Option<PathBuf>,
    pub targets: Option<HashMap<String, Target>>
}

impl AppState {
    pub fn load<P: AsRef<Path>>(file_path: P) -> Result<Self, anyhow::Error> {
        if !file_path.as_ref().exists() {
            return Ok(AppState {
                active_tape: None,
                path: None,
                targets: None,
            });
        }
        let content = fs::read_to_string(file_path)?;
        let state = serde_json::from_str(&content)?;
        Ok(state)
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
