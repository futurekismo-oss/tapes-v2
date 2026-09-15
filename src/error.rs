//! Typed errors for the library surface.
//!
//! Binary crates should map these into `anyhow` with context at the CLI edge
//! (`err-thiserror-lib` + `err-anyhow-app`).

use std::path::PathBuf;

use thiserror::Error;

/// All recoverable failures. Messages are lowercase, no trailing punctuation
/// (`err-lowercase-msg`). No `unwrap` on these paths (`err-no-unwrap-prod`).
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum TapeError {
    #[error("invalid tape name {0:?}")]
    InvalidName(String),

    #[error("tape {0:?} not found at {1}")]
    NotFound(String, PathBuf),

    #[error("target conflict at {dst}: owned by active tape {owner:?}, requested by {requested:?} (use --reinsert)")]
    Conflict {
        dst: PathBuf,
        owner: String,
        requested: String,
    },

    #[error("refusing to replace {0} as {1}")]
    Refused(PathBuf, &'static str),

    #[error("missing dependency binary {0:?}")]
    MissingBinary(String),

    #[error("path escapes home: {0}")]
    EscapesHome(PathBuf),

    #[error("path traversal rejected: {0}")]
    Traversal(PathBuf),

    #[error("manifest error at {path}: {message}")]
    Manifest { path: PathBuf, message: String },

    #[error("state error: {0}")]
    State(String),

    #[error("hook failed: {command:?} exited {code:?}")]
    HookFailed { command: String, code: Option<i32> },

    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Lock contention vs real I/O failure, distinguished at the type level.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum LockError {
    #[error("another tape process holds the lock")]
    AlreadyHeld,
    #[error("lock io error at {path}: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}
