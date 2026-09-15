//! Cassette Tapes v2 — atomic per-target dotfile switcher.
//!
//! Core rules (see `README.md`):
//! - Only declared targets are touched; unrelated `~/.config` entries stay.
//! - All filesystem mutation goes through [`ops`] with explicit [`store::StorePaths`].
//! - `lib` never prints; rendering lives in `output` / `main.rs`.
//! - Tests use `tempfile` dirs only — never real `$HOME`.

#![deny(clippy::correctness)]
#![warn(clippy::suspicious, clippy::style, clippy::complexity, clippy::perf)]
#![forbid(unsafe_code)]

pub mod deps;
pub mod error;
pub mod hooks;
pub mod ops;
pub mod output;
pub mod store;
pub mod tape;

pub use error::{LockError, TapeError};
pub use ops::{EjectOptions, EjectReport, InsertOptions, InsertReport};
pub use store::StorePaths;
pub use tape::{TapeManifest, TapeName};

/// Current on-disk schema. Bump only with migration code.
pub const SCHEMA_VERSION: u32 = 2;
