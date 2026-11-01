use clap_complete::engine::{CompletionCandidate, ValueCompleter};
use std::ffi::OsStr;

use crate::util::{get_all_tapes_in_dir, get_tapes_default_path};

#[derive(Clone)]
pub struct TapeCompleter;

impl ValueCompleter for TapeCompleter {
    fn complete(&self, current: &OsStr) -> Vec<CompletionCandidate> {
        let current_str = current.to_str().unwrap_or("");

        let Ok(tapes_path) = get_tapes_default_path() else {
            return Vec::new();
        };

        let Ok(tapes) = get_all_tapes_in_dir(&tapes_path) else {
            return Vec::new();
        };

        let mut candidates = Vec::new();

        for tape in tapes {
            let Some(name) = tape.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            if name.starts_with(current_str) {
                candidates.push(CompletionCandidate::new(name));
            }
        }

        candidates
    }
}
