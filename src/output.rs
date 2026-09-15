//! Rendering only. No filesystem access, no mutation.
//!
//! Keeps v1's mistake (I/O + `yansi` + logic in one function) from returning:
//! [`crate::ops`] returns data, this module formats it.

use crate::ops::{Check, EjectReport, InsertReport, Status};

/// Print an [`InsertReport`] as human lines or JSON.
pub fn print_insert(report: &InsertReport, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "tape": report.tape,
                "dry_run": report.dry_run,
                "targets": report.targets.iter().map(|t| serde_json::json!({
                    "src": t.src.display().to_string(),
                    "dst": t.dst.display().to_string(),
                    "action": t.action.to_string(),
                })).collect::<Vec<_>>(),
            })
        );
        return;
    }
    if report.dry_run {
        println!("dry-run: would insert {}", report.tape);
    } else {
        println!("inserted {}", report.tape);
    }
    for target in &report.targets {
        println!("  {} {} -> {}", target.action, target.src.display(), target.dst.display());
    }
}

/// Print an [`EjectReport`].
pub fn print_eject(report: &EjectReport, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "tape": report.tape,
                "dry_run": report.dry_run,
                "targets": report.targets.iter().map(|t| serde_json::json!({
                    "dst": t.dst.display().to_string(),
                    "restored_backup": t.restored_backup,
                })).collect::<Vec<_>>(),
            })
        );
        return;
    }
    if report.dry_run {
        println!("dry-run: would eject {}", report.tape);
    } else {
        println!("ejected {}", report.tape);
    }
    for target in &report.targets {
        println!(
            "  {} (backup restored: {})",
            target.dst.display(),
            target.restored_backup
        );
    }
}

/// Print [`Status`].
pub fn print_status(status: &Status, json: bool) {
    if json {
        println!(
            "{}",
            serde_json::json!({
                "active": status.active,
                "records": status.records.iter().map(|r| serde_json::json!({
                    "tape": r.tape,
                    "symlink_path": r.symlink_path.display().to_string(),
                    "symlink_target": r.symlink_target.display().to_string(),
                    "backup_path": r.backup_path.as_ref().map(|p| p.display().to_string()),
                })).collect::<Vec<_>>(),
            })
        );
        return;
    }
    if status.active.is_empty() {
        println!("no tapes currently inserted");
    } else {
        println!("active: {}", status.active.join(", "));
    }
    for record in &status.records {
        println!(
            "  {} -> {}{}",
            record.symlink_path.display(),
            record.symlink_target.display(),
            record
                .backup_path
                .as_ref()
                .map(|b| format!(" (backup {})", b.display()))
                .unwrap_or_default()
        );
    }
}

/// Print [`Check`] rows; returns `true` when all hard checks pass.
/// Advisory `env:*` rows may be false in CI sandboxes without failing.
pub fn print_doctor(checks: &[Check]) -> bool {
    let mut hard_ok = true;
    for check in checks {
        let mark = if check.ok { "ok  " } else { "warn" };
        println!("{mark} {:22} {}", check.name, check.detail);
        if !check.ok && !check.name.starts_with("env:") && check.name != "git" {
            hard_ok = false;
        }
    }
    hard_ok
}
