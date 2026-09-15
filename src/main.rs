//! Thin CLI edge. All logic lives in `tapes::ops`.
//!
//! Env is read here (`StorePaths::from_env`), then explicit paths are passed
//! down so the library stays sandbox-testable.

use std::path::{Path, PathBuf};

use anyhow::Context;
use clap::{Parser, Subcommand};
use tracing_subscriber::EnvFilter;

use tapes::{StorePaths, TapeName};

#[derive(Parser, Debug)]
#[command(name = "tape", about = "Atomic per-target dotfile switcher (v2)")]
struct Cli {
    /// Machine-readable output where supported.
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Insert a tape from the library or an arbitrary path.
    Insert {
        /// Library name (e.g. `dms`). Defaults to manifest name with --path.
        name: Option<String>,
        /// Arbitrary tape directory (e.g. a cloned ii-end4 fork).
        #[arg(long)]
        path: Option<PathBuf>,
        /// Eject-then-insert when already active or on conflict.
        #[arg(long)]
        reinsert: bool,
        /// Plan only; touch nothing.
        #[arg(long)]
        dry_run: bool,
        /// Skip pre/post hooks.
        #[arg(long)]
        no_hooks: bool,
    },
    /// Eject the active tape (or the named one).
    Eject {
        name: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        no_hooks: bool,
    },
    /// Show active stack + records.
    Status,
    /// List library tapes or detail one.
    Show {
        name: Option<String>,
    },
    /// Capture config entries into a new library tape.
    Snapshot {
        name: String,
        /// Relative entries under ~/.config (repeatable).
        #[arg(long = "entry")]
        entries: Vec<String>,
    },
    /// Validate a tape directory without mutating.
    Validate {
        path: PathBuf,
    },
    /// Environment preflight (warn-only outside a compositor).
    Doctor,
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(std::io::stderr)
        .init();

    let cli = Cli::parse();
    let json = cli.json;
    let paths = StorePaths::from_env().context("resolving store paths from env")?;

    match cli.command {
        Commands::Insert {
            name,
            path,
            reinsert,
            dry_run,
            no_hooks,
        } => {
            let (label, source) = resolve_insert_source(&paths, name.as_deref(), path.as_deref())?;
            let report = tapes::ops::insert(
                &paths,
                &label,
                &source,
                tapes::ops::InsertOptions {
                    reinsert,
                    dry_run,
                    no_hooks,
                },
            )?;
            tapes::output::print_insert(&report, json);
        }
        Commands::Eject { name, dry_run, no_hooks } => {
            let parsed = name
                .as_deref()
                .map(TapeName::new)
                .transpose()
                .map_err(|e| anyhow::anyhow!("{e}"))?;
            let report = tapes::ops::eject(
                &paths,
                parsed.as_ref(),
                tapes::ops::EjectOptions { dry_run, no_hooks },
            )?;
            tapes::output::print_eject(&report, json);
        }
        Commands::Status => {
            let status = tapes::ops::status(&paths)?;
            tapes::output::print_status(&status, json);
        }
        Commands::Show { name } => {
            if let Some(n) = name {
                let label = TapeName::new(&n).map_err(|e| anyhow::anyhow!("{e}"))?;
                let dir = paths.tape_dir(label.as_str());
                let manifest = tapes::TapeManifest::load_or_synth(&dir)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!({
                            "name": manifest.tape.name,
                            "version": manifest.tape.version,
                            "desc": manifest.tape.desc,
                            "binaries": manifest.binaries(),
                            "packages": manifest.dependencies.packages,
                        })
                    );
                } else {
                    println!("{} v{}", manifest.tape.name, manifest.tape.version);
                    if !manifest.tape.desc.is_empty() {
                        println!("  {}", manifest.tape.desc);
                    }
                    println!("  dir: {}", dir.display());
                    let targets = tapes::tape::resolve_targets(&dir, &manifest, &paths.home, &paths.config_home)
                        .map_err(|e| anyhow::anyhow!("{e}"))?;
                    for line in tapes::ops::describe_targets(&targets) {
                        println!("  {line}");
                    }
                }
            } else {
                let tapes = tapes::ops::list_tapes(&paths)?;
                if json {
                    println!(
                        "{}",
                        serde_json::json!(tapes.iter().map(|(n, m)| serde_json::json!({
                            "name": n, "version": m.tape.version, "desc": m.tape.desc,
                        })).collect::<Vec<_>>())
                    );
                } else if tapes.is_empty() {
                    println!("no tapes in {}", paths.library_dir.display());
                } else {
                    for (tape_name, manifest) in tapes {
                        println!("- {tape_name} v{}: {}", manifest.tape.version, manifest.tape.desc);
                    }
                }
            }
        }
        Commands::Snapshot { name, entries } => {
            let label = TapeName::new(&name).map_err(|e| anyhow::anyhow!("{e}"))?;
            let dest = tapes::ops::snapshot(&paths, &label, &entries)?;
            if json {
                println!("{}", serde_json::json!({ "tape": name, "dir": dest.display().to_string() }));
            } else {
                println!("snapshotted {name} -> {}", dest.display());
            }
        }
        Commands::Validate { path } => {
            let manifest =
                tapes::TapeManifest::load_or_synth(&path).map_err(|e| anyhow::anyhow!("{e}"))?;
            let targets =
                tapes::tape::resolve_targets(&path, &manifest, &paths.home, &paths.config_home)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
            let missing = tapes::deps::missing_binaries(manifest.binaries());
            if json {
                println!(
                    "{}",
                    serde_json::json!({
                        "name": manifest.tape.name,
                        "targets": tapes::ops::describe_targets(&targets),
                        "missing_binaries": missing,
                    })
                );
            } else {
                println!("valid: {} ({} targets)", manifest.tape.name, targets.len());
                for line in tapes::ops::describe_targets(&targets) {
                    println!("  {line}");
                }
                if !missing.is_empty() {
                    println!("missing binaries: {}", missing.join(", "));
                }
            }
        }
        Commands::Doctor => {
            let checks = tapes::ops::doctor(&paths);
            if !tapes::output::print_doctor(&checks) {
                anyhow::bail!("doctor found hard failures");
            }
        }
    }
    Ok(())
}

fn resolve_insert_source(
    paths: &StorePaths,
    name: Option<&str>,
    path: Option<&Path>,
) -> anyhow::Result<(TapeName, PathBuf)> {
    match (name, path) {
        (Some(n), Some(p)) => Ok((TapeName::new(n).map_err(|e| anyhow::anyhow!("{e}"))?, p.to_path_buf())),
        (Some(n), None) => {
            let label = TapeName::new(n).map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok((label.clone(), paths.tape_dir(label.as_str())))
        }
        (None, Some(p)) => {
            let manifest =
                tapes::TapeManifest::load_or_synth(p).map_err(|e| anyhow::anyhow!("{e}"))?;
            Ok((TapeName::new(&manifest.tape.name).map_err(|e| anyhow::anyhow!("{e}"))?, p.to_path_buf()))
        }
        (None, None) => anyhow::bail!("insert needs a tape name or --path DIR"),
    }
}
