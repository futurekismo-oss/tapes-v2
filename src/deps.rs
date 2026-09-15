//! Dependency checks. No shell, no auto-install in v2.0.
//!
//! `binaries` are looked up via `PATH` splitting (no `sh -c "command -v"`).
//! `packages` are informational — v2 reports, never runs `paru/yay/pkexec`
//! on its own. Lifecycle stays in user-defined hooks.

use std::path::Path;

use crate::error::TapeError;

/// True when `binary` resolves to an executable file on `PATH`.
///
/// Never empty, never contains `/`, and never starts with `-` (flag injection).
pub fn binary_available(binary: &str) -> bool {
    if binary.is_empty() || binary.contains('/') || binary.contains('\\') || binary.starts_with('-') {
        return false;
    }
    let Some(paths) = std::env::var_os("PATH") else {
        return false;
    };
    for dir in std::env::split_paths(&paths) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(binary);
        if is_executable_file(&candidate) {
            return true;
        }
    }
    false
}

#[cfg(unix)]
fn is_executable_file(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    let Ok(md) = std::fs::metadata(path) else {
        return false;
    };
    md.is_file() && md.permissions().mode() & 0o111 != 0
}

#[cfg(not(unix))]
fn is_executable_file(path: &Path) -> bool {
    path.is_file()
}

/// Validate all `binaries`; return the missing subset (sorted, deduped).
#[must_use]
pub fn missing_binaries(binaries: &[String]) -> Vec<String> {
    let mut missing = Vec::new();
    for binary in binaries {
        if !binary_available(binary) && !missing.iter().any(|m: &String| m == binary) {
            missing.push(binary.clone());
        }
    }
    missing.sort();
    missing
}

/// Enforce presence of every binary.
///
/// # Errors
/// [`TapeError::MissingBinary`] listing all missing entries.
pub fn require_binaries(binaries: &[String]) -> Result<(), TapeError> {
    let missing = missing_binaries(binaries);
    if missing.is_empty() {
        Ok(())
    } else {
        Err(TapeError::MissingBinary(missing.join(", ")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_injection_shapes() {
        assert!(!binary_available(""));
        assert!(!binary_available("-rf"));
        assert!(!binary_available("a/b"));
        assert!(!binary_available("a\\b"));
    }

    #[test]
    fn missing_list_is_sorted_deduped() {
        let missing = missing_binaries(&[
            "definitely-not-a-real-binary-xyz-1".into(),
            "definitely-not-a-real-binary-xyz-1".into(),
            "definitely-not-a-real-binary-xyz-0".into(),
        ]);
        assert_eq!(missing.len(), 2);
        assert!(missing[0] < missing[1]);
    }
}
