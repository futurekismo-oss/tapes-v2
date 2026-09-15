//! Tape identity + manifest parsing.
//!
//! Accepts v2 manifests and v1 files (no `schema_version`, deps under
//! `[tape]`, `insert`/`eject` hook names). Tapes without any `tape.toml`
//! (e.g. raw `ii-p3drovfx` clones) are valid — the directory name is used.

use std::collections::BTreeMap;
use std::fmt;
use std::path::{Component, Path, PathBuf};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::TapeError;
use crate::store::{expand_config_path, expand_home};

/// Validated tape identifier (`type-newtype-ids`, `api-parse-dont-validate`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct TapeName(String);

impl TapeName {
    /// Validate without allocating more than needed.
    pub fn new(raw: &str) -> Result<Self, TapeError> {
        validate_name(raw)?;
        Ok(Self(raw.to_owned()))
    }

    #[inline]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for TapeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl FromStr for TapeName {
    type Err = TapeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::new(s)
    }
}

impl AsRef<str> for TapeName {
    #[inline]
    fn as_ref(&self) -> &str {
        &self.0
    }
}

/// Shared name rules for tapes, records, and lock files.
pub(crate) fn validate_name(raw: &str) -> Result<(), TapeError> {
    if raw.is_empty() || raw.len() > 64 {
        return Err(TapeError::InvalidName(raw.to_owned()));
    }
    if raw == "." || raw == ".." || raw.starts_with('-') {
        return Err(TapeError::InvalidName(raw.to_owned()));
    }
    if raw
        .chars()
        .any(|c| matches!(c, '/' | '\\' | '\0') || c.is_control())
    {
        return Err(TapeError::InvalidName(raw.to_owned()));
    }
    if raw.contains("..") {
        return Err(TapeError::InvalidName(raw.to_owned()));
    }
    Ok(())
}

/// A single mapping: `src_rel` (relative to tape dir) → `dst` (absolute).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Target {
    /// Relative source inside the tape dir, e.g. `.config/hypr`.
    pub src_rel: PathBuf,
    /// Absolute destination, e.g. `/home/u/.config/hypr`.
    pub dst: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct TapeInfo {
    pub name: String,
    pub version: String,
    pub desc: String,
    #[serde(default)]
    pub provides: Vec<String>,
    #[serde(default)]
    pub requires: Vec<String>,
    /// v1 shape: `tape.dependencies = ["hyprland"]`. Merged into
    /// `dependencies.binaries` at runtime.
    #[serde(default)]
    pub dependencies: Option<Vec<String>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Dependencies {
    #[serde(default)]
    pub binaries: Vec<String>,
    /// Informational only in v2.0 — never auto-installed.
    #[serde(default)]
    pub packages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct Hooks {
    #[serde(default)]
    pub pre_insert: Vec<String>,
    #[serde(default, alias = "insert")]
    pub post_insert: Vec<String>,
    #[serde(default)]
    pub pre_eject: Vec<String>,
    #[serde(default, alias = "eject")]
    pub post_eject: Vec<String>,
}

impl Hooks {
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.pre_insert.is_empty()
            && self.post_insert.is_empty()
            && self.pre_eject.is_empty()
            && self.post_eject.is_empty()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct TapeManifest {
    #[serde(default = "default_schema")]
    pub schema_version: u32,
    pub tape: TapeInfo,
    #[serde(default)]
    pub dependencies: Dependencies,
    /// `"tape-rel-src" = "~/dst"`. If absent, auto-detected.
    #[serde(default)]
    pub targets: BTreeMap<String, String>,
    #[serde(default)]
    pub hooks: Hooks,
}

const fn default_schema() -> u32 {
    1
}

impl TapeManifest {
    /// Load `tape.toml` if present, else synthesize a minimal manifest from
    /// the directory name so manifest-less rices (end4/ii forks) just work.
    ///
    /// # Errors
    /// Returns [`TapeError::Manifest`] on invalid TOML and
    /// [`TapeError::InvalidName`] on bad fallback names.
    pub fn load_or_synth(tape_dir: &Path) -> Result<Self, TapeError> {
        let manifest_path = tape_dir.join("tape.toml");
        if manifest_path.is_file() {
            return Self::load_from_file(&manifest_path);
        }
        let fallback = tape_dir
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("tape");
        validate_name(fallback)?;
        Ok(Self {
            schema_version: crate::SCHEMA_VERSION,
            tape: TapeInfo {
                name: fallback.to_owned(),
                version: "0.0.0".to_owned(),
                desc: String::new(),
                provides: Vec::new(),
                requires: Vec::new(),
                dependencies: None,
            },
            dependencies: Dependencies::default(),
            targets: BTreeMap::new(),
            hooks: Hooks::default(),
        })
    }

    /// Strict load with validation.
    ///
    /// # Errors
    /// Returns [`TapeError::Manifest`] or [`TapeError::InvalidName`].
    pub fn load_from_file(path: &Path) -> Result<Self, TapeError> {
        let body = std::fs::read_to_string(path).map_err(|e| TapeError::Io {
            path: path.to_path_buf(),
            source: e,
        })?;
        let mut manifest: Self = toml::from_str(&body).map_err(|e| TapeError::Manifest {
            path: path.to_path_buf(),
            message: e.to_string(),
        })?;
        // Merge v1 `tape.dependencies` into the v2 location.
        if let Some(legacy) = manifest.tape.dependencies.take() {
            manifest.dependencies.binaries.extend(legacy);
        }
        manifest.dependencies.binaries.sort();
        manifest.dependencies.binaries.dedup();
        validate_name(&manifest.tape.name)?;
        for (src, _) in &manifest.targets {
            let rel = Path::new(src);
            if rel.is_absolute()
                || rel.components().any(|c| matches!(c, Component::ParentDir))
            {
                return Err(TapeError::Traversal(rel.to_path_buf()));
            }
        }
        Ok(manifest)
    }

    /// Effective binary deps (v1-merged, sorted, deduped).
    #[must_use]
    pub fn binaries(&self) -> &[String] {
        &self.dependencies.binaries
    }
}

/// Resolve concrete [`Target`]s for a tape.
///
/// Order: explicit `[targets]` map, else `.config/`, else `dots/.config/`,
/// else top-level entries (skipping metadata files).
///
/// # Errors
/// Returns [`TapeError::Traversal`] / [`TapeError::EscapesHome`] on bad maps.
pub fn resolve_targets(
    tape_dir: &Path,
    manifest: &TapeManifest,
    home: &Path,
    config_home: &Path,
) -> Result<Vec<Target>, TapeError> {
    if !manifest.targets.is_empty() {
        let mut out = Vec::with_capacity(manifest.targets.len());
        for (src, dst_tpl) in &manifest.targets {
            let rel = PathBuf::from(src);
            if rel.is_absolute()
                || rel.components().any(|c| matches!(c, Component::ParentDir))
            {
                return Err(TapeError::Traversal(rel));
            }
            if !tape_dir.join(&rel).exists() {
                continue;
            }
            let dst = expand_config_path(dst_tpl, home, config_home);
            ensure_under_home(&dst, home)?;
            out.push(Target {
                src_rel: rel,
                dst,
            });
        }
        out.sort_by(|a, b| a.dst.cmp(&b.dst));
        return Ok(out);
    }

    let base = if tape_dir.join(".config").is_dir() {
        tape_dir.join(".config")
    } else if tape_dir.join("dots").join(".config").is_dir() {
        tape_dir.join("dots").join(".config")
    } else {
        // Bare layout (e.g. a quickshell-only clone): entries map 1:1 into
        // `~/.config/<entry>`.
        let mut out = Vec::new();
        let entries = std::fs::read_dir(tape_dir).map_err(|e| TapeError::Io {
            path: tape_dir.to_path_buf(),
            source: e,
        })?;
        for entry in entries.filter_map(Result::ok) {
            let name = entry.file_name();
            let Some(name_str) = name.to_str() else { continue };
            if matches!(
                name_str,
                "tape.toml" | ".git" | ".gitmodules" | "README.md" | "LICENSE" | "sdata" | "setup" | "setup-ii-p3drovfx.sh" | "diagnose" | "dots-extra"
            ) {
                continue;
            }
            // Skip installer-ish extras at top level of foreign rices.
            if name_str.starts_with('.') && name_str != ".config" {
                continue;
            }
            let rel = PathBuf::from(name_str);
            let dst = config_home.join(&rel);
            ensure_under_home(&dst, home)?;
            out.push(Target { src_rel: rel, dst });
        }
        out.sort_by(|a, b| a.dst.cmp(&b.dst));
        return Ok(out);
    };

    // Prefixed layout: mirror `base/*` into `config_home/*`, preserving the
    // tape-relative prefix for the symlink source.
    let prefix = base
        .strip_prefix(tape_dir)
        .unwrap_or(Path::new(".config"))
        .to_path_buf();
    let mut out = Vec::new();
    let entries = std::fs::read_dir(&base).map_err(|e| TapeError::Io {
        path: base.clone(),
        source: e,
    })?;
    for entry in entries.filter_map(Result::ok) {
        let name = entry.file_name();
        let rel = prefix.join(name);
        let entry_name = rel
            .strip_prefix(&prefix)
            .unwrap_or(&rel)
            .to_path_buf();
        let dst = config_home.join(entry_name);
        ensure_under_home(&dst, home)?;
        out.push(Target { src_rel: rel, dst });
    }
    out.sort_by(|a, b| a.dst.cmp(&b.dst));
    Ok(out)
}

/// Expand `~`/`$HOME` templates for tests and manifest targets.
pub fn expand_target_template(raw: &str, home: &Path, config_home: &Path) -> PathBuf {
    if raw.starts_with("~/.config/") || raw == "~/.config" {
        expand_config_path(raw, home, config_home)
    } else {
        expand_home(raw, home)
    }
}

fn ensure_under_home(path: &Path, home: &Path) -> Result<(), TapeError> {
    if path.starts_with(home) {
        Ok(())
    } else {
        Err(TapeError::EscapesHome(path.to_path_buf()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_names_accepted() {
        for good in ["dms", "caelestia", "ii-p3drovfx", "rice_v1", "Foo.Bar"] {
            assert!(TapeName::new(good).is_ok(), "{good}");
        }
    }

    #[test]
    fn invalid_names_rejected() {
        for bad in ["", ".", "..", "-x", "a/b", "a\\b", "a\0b", "a..b/x"] {
            assert!(TapeName::new(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn v1_hooks_alias_to_post_hooks() {
        let manifest: TapeManifest = toml::from_str(
            r#"
[tape]
name = "x"
version = "0.1.0"
desc = "v1"
[hooks]
insert = ["echo hi"]
eject = ["echo bye"]
"#,
        )
        .expect("parse v1 hooks");
        assert_eq!(manifest.hooks.post_insert, vec!["echo hi"]);
        assert_eq!(manifest.hooks.post_eject, vec!["echo bye"]);
    }

    #[test]
    fn v1_tape_dependencies_merge() {
        let path = tempfile::tempdir().expect("tempdir");
        let manifest_path = path.path().join("tape.toml");
        std::fs::write(
            &manifest_path,
            "[tape]\nname = \"x\"\nversion = \"0.1.0\"\ndesc = \"v1\"\ndependencies = [\"hyprland\"]\n",
        )
        .expect("write");
        let manifest = TapeManifest::load_from_file(&manifest_path).expect("load");
        assert_eq!(manifest.binaries(), &["hyprland".to_owned()]);
    }

    #[test]
    fn traversal_in_targets_rejected() {
        let dir = tempfile::tempdir().expect("tempdir");
        let manifest = TapeManifest {
            schema_version: 2,
            tape: TapeInfo {
                name: "x".into(),
                version: "0.1.0".into(),
                desc: String::new(),
                provides: Vec::new(),
                requires: Vec::new(),
                dependencies: None,
            },
            dependencies: Dependencies::default(),
            targets: BTreeMap::from([("../evil".into(), "~/.config/evil".into())]),
            hooks: Hooks::default(),
        };
        let home = dir.path().join("home");
        let err = resolve_targets(dir.path(), &manifest, &home, &home.join(".config"))
            .expect_err("must reject ..");
        assert!(matches!(err, TapeError::Traversal(_)));
    }
}
