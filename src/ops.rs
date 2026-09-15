//! Insert / eject / status orchestration. Pure logic, no printing.
//!
//! Every function takes explicit [`StorePaths`] so integration tests run in
//! `tempfile` sandboxes without touching real `$HOME`.

use std::fs;
use std::path::{Path, PathBuf};

use crate::deps;
use crate::error::{LockError, TapeError};
use crate::hooks;
use crate::store::{
    ActiveState, FileLock, StorePaths, TargetRecord, atomic_write, create_symlink_atomic,
    remove_symlink_if_ours, slug_for_dst,
};
use crate::tape::{self, TapeManifest, TapeName, Target};

/// Options for [`insert`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct InsertOptions {
    pub reinsert: bool,
    pub dry_run: bool,
    pub no_hooks: bool,
}

/// One target inside an [`InsertReport`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlannedTarget {
    pub src: PathBuf,
    pub dst: PathBuf,
    pub action: TargetAction,
}

/// What would happen / happened for a target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetAction {
    Linked,
    Unchanged,
    BackedUpAndLinked,
    ReplacedSymlink,
}

impl std::fmt::Display for TargetAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Linked => write!(f, "linked"),
            Self::Unchanged => write!(f, "unchanged"),
            Self::BackedUpAndLinked => write!(f, "backed-up-and-linked"),
            Self::ReplacedSymlink => write!(f, "replaced-symlink"),
        }
    }
}

/// Result of [`insert`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsertReport {
    pub tape: String,
    pub dry_run: bool,
    pub targets: Vec<PlannedTarget>,
}

/// Options for [`eject`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct EjectOptions {
    pub dry_run: bool,
    pub no_hooks: bool,
}

/// Result of [`eject`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EjectedTarget {
    pub dst: PathBuf,
    pub restored_backup: bool,
}

/// Result of [`eject`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EjectReport {
    pub tape: String,
    pub dry_run: bool,
    pub targets: Vec<EjectedTarget>,
}

/// Insert `active_label` from `source_dir`.
///
/// `source_dir` is the tape content root (library entry or `--path` dir).
/// `active_label` is the name recorded in `active.json`.
///
/// # Errors
/// Validation, dep, conflict, lock, and I/O failures.
pub fn insert(
    paths: &StorePaths,
    active_label: &TapeName,
    source_dir: &Path,
    opts: InsertOptions,
) -> Result<InsertReport, TapeError> {
    paths.ensure_dirs()?;
    let _lock = acquire(paths)?;

    if !source_dir.is_dir() {
        return Err(TapeError::NotFound(
            active_label.to_string(),
            source_dir.to_path_buf(),
        ));
    }
    let canonical_source = fs::canonicalize(source_dir).map_err(|e| TapeError::Io {
        path: source_dir.to_path_buf(),
        source: e,
    })?;

    let manifest = TapeManifest::load_or_synth(&canonical_source)?;
    if !manifest.dependencies.binaries.is_empty() {
        deps::require_binaries(manifest.binaries())?;
    }
    if !manifest.dependencies.packages.is_empty() {
        tracing::info!(
            packages = ?manifest.dependencies.packages,
            "tape declares packages; v2 reports only, never auto-installs"
        );
    }

    let targets = tape::resolve_targets(&canonical_source, &manifest, &paths.home, &paths.config_home)?;
    if targets.is_empty() {
        return Err(TapeError::State(format!(
            "tape {active_label} declares no targets (empty .config and no [targets])"
        )));
    }

    let mut state = ActiveState::load(paths)?;
    let already_active = state.active.iter().any(|a| a == active_label.as_str());

    if already_active && !opts.reinsert && !opts.dry_run {
        // Idempotent fast path: every dst already ours → unchanged, no hooks.
        let mut all_ours = true;
        for target in &targets {
            let src_abs = canonical_source.join(&target.src_rel);
            match fs::read_link(&target.dst) {
                Ok(current) if current == src_abs => {}
                _ => {
                    all_ours = false;
                    break;
                }
            }
        }
        if all_ours {
            return Ok(InsertReport {
                tape: active_label.to_string(),
                dry_run: false,
                targets: targets
                    .iter()
                    .map(|t| PlannedTarget {
                        src: canonical_source.join(&t.src_rel),
                        dst: t.dst.clone(),
                        action: TargetAction::Unchanged,
                    })
                    .collect(),
            });
        }
        return Err(TapeError::State(format!(
            "tape {active_label} is already active (use --reinsert to replace)"
        )));
    }

    // Conflict scan before touching anything.
    let owned = load_all_records(paths)?;
    for target in &targets {
        if let Some(owner) = owner_of(&owned, &target.dst, active_label.as_str()) {
            if !opts.reinsert {
                return Err(TapeError::Conflict {
                    dst: target.dst.clone(),
                    owner,
                    requested: active_label.to_string(),
                });
            }
        }
    }

    if !opts.no_hooks && !opts.dry_run && !manifest.hooks.pre_insert.is_empty() {
        hooks::run_hooks(&manifest.hooks.pre_insert, "pre_insert")?;
    }

    let timestamp = timestamp_now();
    let mut planned = Vec::with_capacity(targets.len());

    for target in &targets {
        let src_abs = canonical_source.join(&target.src_rel);
        if !src_abs.exists() {
            tracing::warn!(src = %src_abs.display(), "skipping missing source");
            continue;
        }
        let action = apply_one_target(paths, active_label.as_str(), &src_abs, &target.dst, &timestamp, opts)?;
        planned.push(PlannedTarget {
            src: src_abs,
            dst: target.dst.clone(),
            action,
        });
    }

    if !opts.dry_run {
        if !opts.no_hooks && !manifest.hooks.post_insert.is_empty() {
            // Records already written; hook failure does not roll back links
            // (documented). Surface the error so callers see it.
            hooks::run_hooks(&manifest.hooks.post_insert, "post_insert")?;
        }
        if !state.active.iter().any(|a| a == active_label.as_str()) {
            state.active.push(active_label.to_string());
            state.save(paths)?;
        }
    }

    Ok(InsertReport {
        tape: active_label.to_string(),
        dry_run: opts.dry_run,
        targets: planned,
    })
}

fn apply_one_target(
    paths: &StorePaths,
    tape: &str,
    src_abs: &Path,
    dst: &Path,
    timestamp: &str,
    opts: InsertOptions,
) -> Result<TargetAction, TapeError> {
    // Fast path: already ours.
    if let Ok(current) = fs::read_link(dst) {
        if current == src_abs {
            return Ok(TargetAction::Unchanged);
        }
        // Symlink elsewhere: safe to replace (symlinks hold no content).
        // Ownership was checked by the caller for active tapes; stale links
        // from crashed runs are replaced too.
        if opts.dry_run {
            return Ok(TargetAction::ReplacedSymlink);
        }
        fs::remove_file(dst).map_err(|e| TapeError::Io {
            path: dst.to_path_buf(),
            source: e,
        })?;
        create_symlink_atomic(src_abs, dst)?;
        if !opts.dry_run {
            persist_record(paths, tape, src_abs, dst, None)?;
        }
        return Ok(TargetAction::ReplacedSymlink);
    }

    if dst.exists() {
        // Real file/dir — never delete; timestamped backup + link.
        if opts.dry_run {
            return Ok(TargetAction::BackedUpAndLinked);
        }
        let backup = unique_backup_path(paths, tape, dst, timestamp)?;
        if let Some(parent) = backup.parent() {
            fs::create_dir_all(parent).map_err(|e| TapeError::Io {
                path: parent.to_path_buf(),
                source: e,
            })?;
        }
        tracing::info!(from = %dst.display(), to = %backup.display(), "backing up");
        fs::rename(dst, &backup).map_err(|e| TapeError::Io {
            path: dst.to_path_buf(),
            source: e,
        })?;
        create_symlink_atomic(src_abs, dst)?;
        persist_record(paths, tape, src_abs, dst, Some(backup))?;
        return Ok(TargetAction::BackedUpAndLinked);
    }

    // Missing → create.
    if opts.dry_run {
        return Ok(TargetAction::Linked);
    }
    create_symlink_atomic(src_abs, dst)?;
    persist_record(paths, tape, src_abs, dst, None)?;
    Ok(TargetAction::Linked)
}

fn persist_record(
    paths: &StorePaths,
    tape: &str,
    src_abs: &Path,
    dst: &Path,
    backup: Option<PathBuf>,
) -> Result<(), TapeError> {
    let slug = slug_for_dst(dst, &paths.config_home);
    let record = TargetRecord {
        schema_version: crate::SCHEMA_VERSION,
        tape: tape.to_owned(),
        symlink_path: dst.to_path_buf(),
        symlink_target: src_abs.to_path_buf(),
        backup_path: backup,
        installed_at: TargetRecord::now(),
    };
    record.save(&paths.record_path(tape, &slug))
}

/// Eject one tape (or the single active tape when `name` is `None`).
///
/// Only symlinks pointing at recorded targets are removed. User-retargeted
/// links are skipped, never deleted.
///
/// # Errors
/// Empty state, unknown tape, lock, and I/O failures.
pub fn eject(
    paths: &StorePaths,
    name: Option<&TapeName>,
    opts: EjectOptions,
) -> Result<EjectReport, TapeError> {
    paths.ensure_dirs()?;
    let _lock = acquire(paths)?;

    let mut state = ActiveState::load(paths)?;
    if state.active.is_empty() {
        return Err(TapeError::State("no active tape, nothing to eject".into()));
    }
    let label: String = match name {
        Some(n) => {
            if !state.active.iter().any(|a| a == n.as_str()) {
                return Err(TapeError::NotFound(n.to_string(), paths.library_dir.clone()));
            }
            n.to_string()
        }
        None => {
            if state.active.len() != 1 {
                return Err(TapeError::State(format!(
                    "multiple tapes active ({}), specify which to eject",
                    state.active.join(", ")
                )));
            }
            state.active[0].clone()
        }
    };

    let tape_dir = paths.tape_dir(&label);
    let manifest = TapeManifest::load_or_synth(&tape_dir).unwrap_or_else(|_| TapeManifest {
        schema_version: crate::SCHEMA_VERSION,
        tape: crate::tape::TapeInfo {
            name: label.clone(),
            version: String::new(),
            desc: String::new(),
            provides: Vec::new(),
            requires: Vec::new(),
            dependencies: None,
        },
        dependencies: crate::tape::Dependencies::default(),
        targets: std::collections::BTreeMap::new(),
        hooks: crate::tape::Hooks::default(),
    });

    if !opts.no_hooks && !opts.dry_run && !manifest.hooks.pre_eject.is_empty() {
        hooks::run_hooks(&manifest.hooks.pre_eject, "pre_eject")?;
    }

    let records = records_for_tape(paths, &label)?;
    let mut out = Vec::with_capacity(records.len());
    for (record_path, record) in &records {
        let _ = record_path;
        if opts.dry_run {
            out.push(EjectedTarget {
                dst: record.symlink_path.clone(),
                restored_backup: record.backup_path.is_some(),
            });
            continue;
        }
        let removed = remove_symlink_if_ours(&record.symlink_path, &record.symlink_target)?;
        let mut restored = false;
        if let Some(backup) = &record.backup_path {
            if removed && !record.symlink_path.exists() && backup.exists() {
                if let Some(parent) = record.symlink_path.parent() {
                    fs::create_dir_all(parent).map_err(|e| TapeError::Io {
                        path: parent.to_path_buf(),
                        source: e,
                    })?;
                }
                tracing::info!(from = %backup.display(), to = %record.symlink_path.display(), "restoring backup");
                fs::rename(backup, &record.symlink_path).map_err(|e| TapeError::Io {
                    path: record.symlink_path.clone(),
                    source: e,
                })?;
                restored = true;
            }
        }
        out.push(EjectedTarget {
            dst: record.symlink_path.clone(),
            restored_backup: restored,
        });
    }

    if !opts.dry_run {
        for (record_path, _) in &records {
            match fs::remove_file(record_path) {
                Ok(()) => {}
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
                Err(e) => {
                    return Err(TapeError::Io {
                        path: record_path.clone(),
                        source: e,
                    })
                }
            }
        }
        state.active.retain(|a| a != &label);
        state.save(paths)?;

        if !opts.no_hooks && !manifest.hooks.post_eject.is_empty() {
            hooks::run_hooks(&manifest.hooks.post_eject, "post_eject")?;
        }
    }

    out.sort_by(|a, b| a.dst.cmp(&b.dst));
    Ok(EjectReport {
        tape: label,
        dry_run: opts.dry_run,
        targets: out,
    })
}

/// Current active stack with record counts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Status {
    pub active: Vec<String>,
    pub records: Vec<TargetRecord>,
}

/// # Errors
/// State I/O failures.
pub fn status(paths: &StorePaths) -> Result<Status, TapeError> {
    let state = ActiveState::load(paths)?;
    let mut records = Vec::new();
    for tape in &state.active {
        for (_, record) in records_for_tape(paths, tape)? {
            records.push(record);
        }
    }
    records.sort_by(|a, b| a.symlink_path.cmp(&b.symlink_path));
    Ok(Status {
        active: state.active,
        records,
    })
}

/// Tapes present in the library (directories only).
///
/// # Errors
/// I/O failures.
pub fn list_tapes(paths: &StorePaths) -> Result<Vec<(String, TapeManifest)>, TapeError> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(&paths.library_dir) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => {
            return Err(TapeError::Io {
                path: paths.library_dir.clone(),
                source: e,
            })
        }
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if name == "records" || name == "backups" {
            continue;
        }
        // Skip invalid names instead of failing the whole listing.
        if crate::tape::TapeName::new(name).is_err() {
            continue;
        }
        let manifest = TapeManifest::load_or_synth(&path)?;
        out.push((name.to_owned(), manifest));
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(out)
}

/// Capture `entries` (relative to `config_home`) into a new library tape.
///
/// Follows top-level symlinks to snapshot real content. Writes a minimal
/// `tape.toml` when none exists.
///
/// # Errors
/// Missing entries, invalid names, I/O failures.
pub fn snapshot(
    paths: &StorePaths,
    name: &TapeName,
    entries: &[String],
) -> Result<PathBuf, TapeError> {
    paths.ensure_dirs()?;
    if entries.is_empty() {
        return Err(TapeError::State(
            "snapshot needs --entry REL (repeatable); refusing to copy all of ~/.config blindly".into(),
        ));
    }
    let dest = paths.tape_dir(name.as_str());
    if dest.exists() {
        return Err(TapeError::State(format!("tape {} already exists", name)));
    }
    for entry in entries {
        let rel = Path::new(entry);
        if rel.is_absolute() || rel.components().any(|c| matches!(c, std::path::Component::ParentDir)) {
            return Err(TapeError::Traversal(rel.to_path_buf()));
        }
        let src = paths.config_home.join(rel);
        if !src.exists() && fs::symlink_metadata(&src).is_err() {
            return Err(TapeError::NotFound(entry.clone(), src));
        }
        let dst = dest.join(".config").join(rel);
        copy_tree(&src, &dst)?;
    }
    let manifest_path = dest.join("tape.toml");
    if !manifest_path.exists() {
        let body = format!(
            "schema_version = {}\n\n[tape]\nname = \"{}\"\nversion = \"0.1.0\"\ndesc = \"snapshot\"\n",
            crate::SCHEMA_VERSION,
            name
        );
        atomic_write(&manifest_path, body.as_bytes())?;
    }
    Ok(dest)
}

fn copy_tree(src: &Path, dst: &Path) -> Result<(), TapeError> {
    let md = fs::symlink_metadata(src).map_err(|e| TapeError::Io {
        path: src.to_path_buf(),
        source: e,
    })?;
    if md.file_type().is_symlink() {
        let target = fs::read_link(src).map_err(|e| TapeError::Io {
            path: src.to_path_buf(),
            source: e,
        })?;
        let resolved = if target.is_absolute() {
            target
        } else {
            src.parent().unwrap_or(Path::new("/")).join(target)
        };
        return copy_tree(&resolved, dst);
    }
    if md.is_dir() {
        fs::create_dir_all(dst).map_err(|e| TapeError::Io {
            path: dst.to_path_buf(),
            source: e,
        })?;
        let entries = fs::read_dir(src).map_err(|e| TapeError::Io {
            path: src.to_path_buf(),
            source: e,
        })?;
        for entry in entries.filter_map(Result::ok) {
            copy_tree(&entry.path(), &dst.join(entry.file_name()))?;
        }
        return Ok(());
    }
    if let Some(parent) = dst.parent() {
        fs::create_dir_all(parent).map_err(|e| TapeError::Io {
            path: parent.to_path_buf(),
            source: e,
        })?;
    }
    fs::copy(src, dst).map_err(|e| TapeError::Io {
        path: dst.to_path_buf(),
        source: e,
    })?;
    Ok(())
}

/// One preflight check row for `doctor`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Check {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// Environment preflight. Only `home`/`library` are hard failures; everything
/// else is advisory (`ok=false`) so `doctor` itself is safe in CI sandboxes.
#[must_use]
pub fn doctor(paths: &StorePaths) -> Vec<Check> {
    let mut checks = Vec::with_capacity(8);
    checks.push(Check {
        name: "home".into(),
        ok: paths.home.is_absolute() && paths.home.as_os_str() != "/",
        detail: paths.home.display().to_string(),
    });
    checks.push(Check {
        name: "config_home".into(),
        ok: paths.config_home.starts_with(&paths.home),
        detail: paths.config_home.display().to_string(),
    });
    checks.push(Check {
        name: "library_writable".into(),
        ok: ensure_writable(&paths.library_dir),
        detail: paths.library_dir.display().to_string(),
    });
    checks.push(Check {
        name: "git".into(),
        ok: deps::binary_available("git"),
        detail: "required for future --git support".into(),
    });
    checks.push(Check {
        name: "sh".into(),
        ok: deps::binary_available("sh"),
        detail: "required for hooks".into(),
    });
    for var in ["WAYLAND_DISPLAY", "HYPRLAND_INSTANCE_SIGNATURE", "XDG_RUNTIME_DIR"] {
        let present = std::env::var(var).map(|v| !v.is_empty()).unwrap_or(false);
        checks.push(Check {
            name: format!("env:{var}"),
            ok: present,
            detail: if present { "set".into() } else { "unset (warn-only outside a compositor)".into() },
        });
    }
    checks
}

fn ensure_writable(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".write-probe");
    match fs::write(&probe, b"ok") {
        Ok(()) => {
            let _ = fs::remove_file(&probe);
            true
        }
        Err(_) => false,
    }
}

fn acquire(paths: &StorePaths) -> Result<FileLock, TapeError> {
    FileLock::try_acquire(&paths.lock_file()).map_err(|e| match e {
        LockError::AlreadyHeld => TapeError::State("another tape process holds the lock".into()),
        LockError::Io { path, source } => TapeError::Io { path, source },
    })
}

fn load_all_records(paths: &StorePaths) -> Result<Vec<TargetRecord>, TapeError> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(paths.records_dir()) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => {
            return Err(TapeError::Io {
                path: paths.records_dir(),
                source: e,
            })
        }
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        if let Ok(record) = TargetRecord::load(&path) {
            out.push(record);
        }
    }
    Ok(out)
}

fn owner_of(records: &[TargetRecord], dst: &Path, requesting: &str) -> Option<String> {
    records
        .iter()
        .find(|r| r.symlink_path == dst && r.tape != requesting)
        .map(|r| r.tape.clone())
}

fn records_for_tape(paths: &StorePaths, tape: &str) -> Result<Vec<(PathBuf, TargetRecord)>, TapeError> {
    let mut out = Vec::new();
    let entries = match fs::read_dir(paths.records_dir()) {
        Ok(e) => e,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(out),
        Err(e) => {
            return Err(TapeError::Io {
                path: paths.records_dir(),
                source: e,
            })
        }
    };
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        let prefix = format!("{tape}__");
        if !name.starts_with(prefix.as_str()) || !name.ends_with(".json") {
            continue;
        }
        if let Ok(record) = TargetRecord::load(&path) {
            out.push((path, record));
        }
    }
    out.sort_by(|a, b| a.1.symlink_path.cmp(&b.1.symlink_path));
    Ok(out)
}

fn unique_backup_path(
    paths: &StorePaths,
    tape: &str,
    dst: &Path,
    timestamp: &str,
) -> Result<PathBuf, TapeError> {
    let slug = slug_for_dst(dst, &paths.config_home);
    let base = paths
        .backups_dir()
        .join(format!("{timestamp}_{tape}__{slug}"));
    if !base.exists() && fs::symlink_metadata(&base).is_err() {
        return Ok(base);
    }
    for n in 2_u32..1000 {
        let candidate = PathBuf::from(format!("{}-{n}", base.display()));
        if candidate.exists() || fs::symlink_metadata(&candidate).is_ok() {
            continue;
        }
        return Ok(candidate);
    }
    Err(TapeError::State("could not allocate a unique backup path".into()))
}

fn timestamp_now() -> String {
    let now = time::OffsetDateTime::now_utc();
    format!(
        "{:04}{:02}{:02}-{:02}{:02}{:02}",
        now.year(),
        u8::from(now.month()),
        now.day(),
        now.hour(),
        now.minute(),
        now.second()
    )
}

/// Human-facing target summary (used by `output`, kept here for reuse).
#[must_use]
pub fn describe_targets(targets: &[Target]) -> Vec<String> {
    targets
        .iter()
        .map(|t| format!("{} -> {}", t.src_rel.display(), t.dst.display()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::StorePaths;

    fn sandbox() -> (tempfile::TempDir, StorePaths) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let home = tmp.path().join("home");
        let config = home.join(".config");
        let library = tmp.path().join("lib").join("tapes");
        std::fs::create_dir_all(&config).expect("config");
        let paths = StorePaths::at_roots(home, config, library);
        paths.ensure_dirs().expect("ensure");
        (tmp, paths)
    }

    fn make_tape(library: &Path, name: &str, files: &[(&str, &str)]) -> PathBuf {
        let dir = library.join(name);
        for (rel, body) in files {
            let path = dir.join(rel);
            std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
            std::fs::write(&path, body).expect("write");
        }
        dir
    }

    #[test]
    fn insert_then_eject_roundtrip_restores_files() {
        let (_t, paths) = sandbox();
        // Arrange: existing user file + a tape with one entry.
        std::fs::create_dir_all(paths.config_home.join("hypr")).expect("cfg");
        std::fs::write(paths.config_home.join("hypr").join("mine.conf"), "mine").expect("write");
        let source = make_tape(
            &paths.library_dir,
            "dms",
            &[(".config/hypr/hyprland.conf", "rice")],
        );

        // Act: insert.
        let report = insert(
            &paths,
            &TapeName::new("dms").expect("name"),
            &source,
            InsertOptions::default(),
        )
        .expect("insert");
        assert_eq!(report.targets.len(), 1);

        // Assert: dst is a symlink into the tape; backup exists.
        let dst = paths.config_home.join("hypr");
        assert!(fs::symlink_metadata(&dst).expect("md").file_type().is_symlink());
        assert!(paths.backups_dir().read_dir().expect("ls").next().is_some());

        // Act: eject restores.
        let eject_report = eject(
            &paths,
            Some(&TapeName::new("dms").expect("name")),
            EjectOptions::default(),
        )
        .expect("eject");
        assert_eq!(eject_report.targets.len(), 1);
        assert!(eject_report.targets[0].restored_backup);
        assert_eq!(
            fs::read_to_string(paths.config_home.join("hypr").join("mine.conf")).expect("restored"),
            "mine"
        );
    }

    #[test]
    fn dry_run_touches_nothing() {
        let (_t, paths) = sandbox();
        let source = make_tape(&paths.library_dir, "x", &[(".config/a/f", "v")]);
        let report = insert(
            &paths,
            &TapeName::new("x").expect("name"),
            &source,
            InsertOptions {
                dry_run: true,
                ..Default::default()
            },
        )
        .expect("dry run");
        assert!(report.dry_run);
        assert!(!paths.config_home.join("a").exists());
        assert!(ActiveState::load(&paths).expect("state").active.is_empty());
    }

    #[test]
    fn conflict_without_reinsert_is_an_error() {
        let (_t, paths) = sandbox();
        let a = make_tape(&paths.library_dir, "a", &[(".config/s/f", "a")]);
        let b = make_tape(&paths.library_dir, "b", &[(".config/s/f", "b")]);
        insert(
            &paths,
            &TapeName::new("a").expect("name"),
            &a,
            InsertOptions::default(),
        )
        .expect("insert a");
        let err = insert(
            &paths,
            &TapeName::new("b").expect("name"),
            &b,
            InsertOptions::default(),
        )
        .expect_err("must conflict");
        assert!(matches!(err, TapeError::Conflict { .. }), "got {err:?}");
    }
}
