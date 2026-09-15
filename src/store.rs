//! Filesystem layout, atomic state, advisory locking, atomic symlinks.
//!
//! Everything here is explicit-path (no env access) so tests can pass
//! `tempfile` dirs. Env resolution lives only in
//! [`StorePaths::from_env`] at the CLI edge.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use fs4::fs_std::FileExt;
use serde::{Deserialize, Serialize};

use crate::error::{LockError, TapeError};

/// Resolved roots. `home` is the real `$HOME` (for `~/` expansion), not an
/// XDG dir. `config_home` is `$XDG_CONFIG_HOME` or `$HOME/.config`.
/// `library_dir` is `$XDG_DATA_HOME/tapes`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StorePaths {
    pub home: PathBuf,
    pub config_home: PathBuf,
    pub library_dir: PathBuf,
}

impl StorePaths {
    /// Test / isolated entry point — no env access.
    #[must_use]
    pub fn at_roots(home: PathBuf, config_home: PathBuf, library_dir: PathBuf) -> Self {
        Self {
            home,
            config_home,
            library_dir,
        }
    }

    /// Production entry point. Reads `HOME`, `XDG_CONFIG_HOME`,
    /// `XDG_DATA_HOME`. Fails closed on missing/root `HOME`.
    ///
    /// # Errors
    /// Returns [`TapeError::State`] when env is unusable.
    pub fn from_env() -> Result<Self, TapeError> {
        let home_raw = std::env::var("HOME").map_err(|_| TapeError::State("HOME must be set".into()))?;
        if home_raw.is_empty() || home_raw == "/" {
            return Err(TapeError::State("HOME must be set to a non-root dir".into()));
        }
        let home = PathBuf::from(home_raw);
        if !home.is_absolute() {
            return Err(TapeError::State("HOME must be absolute".into()));
        }
        let config_home = match std::env::var("XDG_CONFIG_HOME") {
            Ok(s) if !s.is_empty() => {
                let p = PathBuf::from(&s);
                if !p.is_absolute() {
                    return Err(TapeError::State("XDG_CONFIG_HOME must be absolute".into()));
                }
                p
            }
            _ => home.join(".config"),
        };
        let data_home = match std::env::var("XDG_DATA_HOME") {
            Ok(s) if !s.is_empty() => PathBuf::from(s),
            _ => home.join(".local").join("share"),
        };
        Ok(Self {
            home,
            config_home,
            library_dir: data_home.join("tapes"),
        })
    }

    #[must_use]
    pub fn active_file(&self) -> PathBuf {
        self.library_dir.join("active.json")
    }

    #[must_use]
    pub fn records_dir(&self) -> PathBuf {
        self.library_dir.join("records")
    }

    #[must_use]
    pub fn backups_dir(&self) -> PathBuf {
        self.library_dir.join("backups")
    }

    #[must_use]
    pub fn lock_file(&self) -> PathBuf {
        self.library_dir.join(".lock")
    }

    /// `ensure library + records + backups + config_home exist`.
    ///
    /// # Errors
    /// I/O failures with path context.
    pub fn ensure_dirs(&self) -> Result<(), TapeError> {
        for dir in [&self.library_dir, &self.config_home] {
            fs::create_dir_all(dir).map_err(|e| TapeError::Io {
                path: dir.clone(),
                source: e,
            })?;
        }
        for dir in [self.records_dir(), self.backups_dir()] {
            fs::create_dir_all(&dir).map_err(|e| TapeError::Io {
                path: dir.clone(),
                source: e,
            })?;
        }
        Ok(())
    }

    #[must_use]
    pub fn tape_dir(&self, name: &str) -> PathBuf {
        self.library_dir.join(name)
    }

    #[must_use]
    pub fn record_path(&self, tape: &str, slug: &str) -> PathBuf {
        self.records_dir().join(format!("{tape}__{slug}.json"))
    }
}

/// Expand `~/x`, `~`, `$HOME/x`, `$HOME`, else verbatim.
#[must_use]
pub fn expand_home(raw: &str, home: &Path) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/") {
        home.join(rest)
    } else if raw == "~" {
        home.to_path_buf()
    } else if let Some(rest) = raw.strip_prefix("$HOME/") {
        home.join(rest)
    } else if raw == "$HOME" {
        home.to_path_buf()
    } else {
        PathBuf::from(raw)
    }
}

/// Expand `~/.config/...` against `config_home`, else [`expand_home`].
#[must_use]
pub fn expand_config_path(raw: &str, home: &Path, config_home: &Path) -> PathBuf {
    if let Some(rest) = raw.strip_prefix("~/.config/") {
        config_home.join(rest)
    } else if raw == "~/.config" {
        config_home.to_path_buf()
    } else {
        expand_home(raw, home)
    }
}

/// Stable slug for a dst path inside records/backups.
#[must_use]
pub fn slug_for_dst(dst: &Path, config_home: &Path) -> String {
    let rel = dst.strip_prefix(config_home).unwrap_or(dst);
    let s = rel
        .to_string_lossy()
        .replace('/', "__")
        .replace('\\', "__");
    let mut slug: String = s
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@' | '+') {
                c
            } else {
                '-'
            }
        })
        .collect();
    if slug.is_empty() {
        slug.push_str("root");
    }
    slug.truncate(96);
    slug
}

/// Active stack: which tapes are inserted, in insertion order.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct ActiveState {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    #[serde(default)]
    pub active: Vec<String>,
}

const fn default_schema() -> u32 {
    crate::SCHEMA_VERSION
}

impl ActiveState {
    /// Load or default-empty (missing file = nothing active).
    ///
    /// # Errors
    /// Corrupt JSON or wrong schema.
    pub fn load(paths: &StorePaths) -> Result<Self, TapeError> {
        let path = paths.active_file();
        let body = match fs::read_to_string(&path) {
            Ok(b) => b,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => {
                return Err(TapeError::Io {
                    path: path.clone(),
                    source: e,
                })
            }
        };
        let state: Self =
            serde_json::from_str(&body).map_err(|e| TapeError::State(format!("parsing active.json: {e}")))?;
        if state.schema_version != crate::SCHEMA_VERSION {
            return Err(TapeError::State(format!(
                "active.json schema {} != supported {}",
                state.schema_version,
                crate::SCHEMA_VERSION
            )));
        }
        Ok(state)
    }

    /// Atomic save.
    ///
    /// # Errors
    /// I/O failures.
    pub fn save(&self, paths: &StorePaths) -> Result<(), TapeError> {
        let body = serde_json::to_string_pretty(self)
            .map_err(|e| TapeError::State(format!("serializing active.json: {e}")))?;
        atomic_write(paths.active_file().as_path(), body.as_bytes())
    }
}

/// One managed symlink + where the previous content went.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TargetRecord {
    pub schema_version: u32,
    pub tape: String,
    pub symlink_path: PathBuf,
    pub symlink_target: PathBuf,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub backup_path: Option<PathBuf>,
    pub installed_at: String,
}

impl TargetRecord {
    #[must_use]
    pub fn now() -> String {
        time::OffsetDateTime::now_utc()
            .format(&time::format_description::well_known::Rfc3339)
            .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_owned())
    }

    /// # Errors
    /// I/O or schema mismatch.
    pub fn load(path: &Path) -> Result<Self, TapeError> {
        let body = fs::read_to_string(path).map_err(|e| TapeError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        let record: Self =
            serde_json::from_str(&body).map_err(|e| TapeError::State(format!("parsing record: {e}")))?;
        if record.schema_version != crate::SCHEMA_VERSION {
            return Err(TapeError::State(format!(
                "record schema {} != {}",
                record.schema_version,
                crate::SCHEMA_VERSION
            )));
        }
        Ok(record)
    }

    /// # Errors
    /// I/O failures.
    pub fn save(&self, path: &Path) -> Result<(), TapeError> {
        let body = serde_json::to_string_pretty(self)
            .map_err(|e| TapeError::State(format!("serializing record: {e}")))?;
        atomic_write(path, body.as_bytes())
    }
}

/// Write `body` to `path` via `tmp → fsync → rename`. Parent fsync is
/// best-effort (warn via `tracing`, never fail the op over it).
///
/// # Errors
/// I/O failures with path context.
pub fn atomic_write(path: &Path, body: &[u8]) -> Result<(), TapeError> {
    let parent = path.parent().ok_or_else(|| TapeError::State(format!("no parent for {}", path.display())))?;
    fs::create_dir_all(parent).map_err(|e| TapeError::Io {
        path: parent.to_path_buf(),
        source: e,
    })?;
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);

    let write_then_rename = || -> Result<(), TapeError> {
        let mut file = fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&tmp)
            .map_err(|e| TapeError::Io {
                path: tmp.clone(),
                source: e,
            })?;
        file.write_all(body).map_err(|e| TapeError::Io {
            path: tmp.clone(),
            source: e,
        })?;
        file.sync_all().map_err(|e| TapeError::Io {
            path: tmp.clone(),
            source: e,
        })?;
        drop(file);
        fs::rename(&tmp, path).map_err(|e| TapeError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        Ok(())
    };

    if let Err(e) = write_then_rename() {
        let _ = fs::remove_file(&tmp);
        return Err(e);
    }
    if let Err(e) = fs::File::open(parent).and_then(|d| d.sync_all()) {
        tracing::warn!(parent = %parent.display(), error = %e, "parent fsync failed; content durable, rename may not survive power loss");
    }
    Ok(())
}

/// Process-held advisory lock. File persists as rendezvous; content meaningless.
#[derive(Debug)]
pub struct FileLock {
    _file: fs::File,
}

impl FileLock {
    /// Non-blocking acquire. Parent must exist (`ensure_dirs` first).
    ///
    /// # Errors
    /// [`LockError::AlreadyHeld`] on contention, [`LockError::Io`] otherwise.
    pub fn try_acquire(path: &Path) -> Result<Self, LockError> {
        let file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(path)
            .map_err(|e| LockError::Io {
                path: path.to_path_buf(),
                source: e,
            })?;
        match file.try_lock_exclusive() {
            Ok(true) => Ok(Self { _file: file }),
            Ok(false) => Err(LockError::AlreadyHeld),
            Err(e) => Err(LockError::Io {
                path: path.to_path_buf(),
                source: e,
            }),
        }
    }
}

/// Create `dst → src` atomically (`dst.rctmp → rename`).
///
/// - Parent dirs created.
/// - Real dir → [`TapeError::Refused`]; real file → [`TapeError::Refused`].
/// - Existing symlink to same target → no-op `Ok(false)`.
/// - Stale/other symlink → atomically replaced, `Ok(true)`.
///
/// Returns `true` when a write happened.
///
/// # Errors
/// I/O failures and [`TapeError::Refused`].
pub fn create_symlink_atomic(src: &Path, dst: &Path) -> Result<bool, TapeError> {
    let parent = dst
        .parent()
        .ok_or_else(|| TapeError::State(format!("no parent for {}", dst.display())))?;
    fs::create_dir_all(parent).map_err(|e| TapeError::Io {
        path: parent.to_path_buf(),
        source: e,
    })?;

    match fs::symlink_metadata(dst) {
        Ok(md) if md.file_type().is_symlink() => {
            let current = fs::read_link(dst).map_err(|e| TapeError::Io {
                path: dst.to_path_buf(),
                source: e,
            })?;
            if current == src {
                return Ok(false);
            }
            // Fall through to atomic replace below.
        }
        Ok(md) if md.file_type().is_dir() => {
            return Err(TapeError::Refused(dst.to_path_buf(), "a directory"));
        }
        Ok(_) => {
            return Err(TapeError::Refused(dst.to_path_buf(), "a regular file"));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(TapeError::Io {
                path: dst.to_path_buf(),
                source: e,
            })
        }
    }

    let file_name = dst.file_name().ok_or_else(|| TapeError::State(format!("no file name for {}", dst.display())))?;
    let mut tmp_name = file_name.to_owned();
    tmp_name.push(".rctmp");
    let tmp = parent.join(tmp_name);
    match fs::remove_file(&tmp) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => {
            return Err(TapeError::Io {
                path: tmp.clone(),
                source: e,
            })
        }
    }
    #[cfg(unix)]
    std::os::unix::fs::symlink(src, &tmp).map_err(|e| TapeError::Io {
        path: tmp.clone(),
        source: e,
    })?;
    #[cfg(not(unix))]
    std::os::windows::fs::symlink_dir(src, &tmp).map_err(|e| TapeError::Io {
        path: tmp.clone(),
        source: e,
    })?;
    fs::rename(&tmp, dst).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        TapeError::Io {
            path: dst.to_path_buf(),
            source: e,
        }
    })?;
    Ok(true)
}

/// Remove `dst` only if it is a symlink pointing at `expected`.
/// Missing dst → `Ok(false)`. Symlink elsewhere → [`TapeError::Refused`]
/// is NOT raised here; caller decides (we return `Ok(false)` + warn) so eject
/// never deletes user-retargeted links.
///
/// # Errors
/// I/O failures.
pub fn remove_symlink_if_ours(dst: &Path, expected: &Path) -> Result<bool, TapeError> {
    let md = match fs::symlink_metadata(dst) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(e) => {
            return Err(TapeError::Io {
                path: dst.to_path_buf(),
                source: e,
            })
        }
    };
    if !md.file_type().is_symlink() {
        return Ok(false);
    }
    let current = fs::read_link(dst).map_err(|e| TapeError::Io {
        path: dst.to_path_buf(),
        source: e,
    })?;
    if current != expected {
        tracing::warn!(dst = %dst.display(), current = ?current, expected = ?expected, "skipping user-retargeted symlink");
        return Ok(false);
    }
    fs::remove_file(dst).map_err(|e| TapeError::Io {
        path: dst.to_path_buf(),
        source: e,
    })?;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roots() -> (tempfile::TempDir, StorePaths) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        let config = home.join(".config");
        let library = tmp.path().join("tapes");
        let paths = StorePaths::at_roots(home, config, library);
        paths.ensure_dirs().expect("ensure");
        (tmp, paths)
    }

    #[test]
    fn expand_home_cases() {
        let home = Path::new("/h");
        assert_eq!(expand_home("~/x", home), PathBuf::from("/h/x"));
        assert_eq!(expand_home("~", home), PathBuf::from("/h"));
        assert_eq!(expand_home("$HOME/y", home), PathBuf::from("/h/y"));
        assert_eq!(expand_home("/etc/x", home), PathBuf::from("/etc/x"));
    }

    #[test]
    fn active_state_roundtrip_missing_is_empty() {
        let (_t, paths) = roots();
        let loaded = ActiveState::load(&paths).expect("load missing");
        assert!(loaded.active.is_empty());
        let state = ActiveState {
            schema_version: crate::SCHEMA_VERSION,
            active: vec!["dms".into()],
        };
        state.save(&paths).expect("save");
        let back = ActiveState::load(&paths).expect("reload");
        assert_eq!(back, state);
    }

    #[test]
    fn symlink_atomic_is_idempotent_and_refuses_files() {
        let (_t, paths) = roots();
        let src = paths.home.join("src");
        fs::create_dir_all(&src).expect("src");
        let dst = paths.config_home.join("hypr");
        assert!(create_symlink_atomic(&src, &dst).expect("create"));
        assert!(!create_symlink_atomic(&src, &dst).expect("idempotent"));
        assert_eq!(fs::read_link(&dst).expect("read"), src);

        let file_dst = paths.config_home.join("plain");
        fs::create_dir_all(paths.config_home.clone()).expect("cfg");
        fs::write(&file_dst, "mine").expect("write");
        let err = create_symlink_atomic(&src, &file_dst).expect_err("must refuse file");
        assert!(matches!(err, TapeError::Refused(_, _)));
        assert_eq!(fs::read_to_string(&file_dst).expect("kept"), "mine");
    }
}
