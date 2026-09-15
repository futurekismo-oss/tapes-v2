//! Lifecycle hooks. Explicit, opt-out, never silent.
//!
//! Hooks run via `sh -c` only when the caller allows. `--dry-run` and
//! `--no-hooks` never spawn. Failures are typed, never swallowed.

use crate::error::TapeError;

/// Run `commands` in order with `sh -c`.
///
/// # Errors
/// [`TapeError::HookFailed`] on non-zero exit or spawn failure.
pub fn run_hooks(commands: &[String], phase: &str) -> Result<(), TapeError> {
    for command in commands {
        if command.trim().is_empty() {
            continue;
        }
        tracing::info!(phase, command = %command, "running hook");
        let status = std::process::Command::new("sh")
            .arg("-c")
            .arg(command)
            .status()
            .map_err(|e| {
                TapeError::Io {
                    path: std::path::PathBuf::from(format!("hook:{phase}")),
                    source: e,
                }
            })?;
        if !status.success() {
            return Err(TapeError::HookFailed {
                command: command.clone(),
                code: status.code(),
            });
        }
    }
    Ok(())
}
