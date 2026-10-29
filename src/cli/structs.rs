use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "tape", version, about = "Atomic per-target dotfile switcher (v2)")]
pub struct Cli {
    /// Enable logging output
    #[arg(short, long, global = true)]
    pub logging: bool,

    /// Increase the verbosity of some commands
    #[arg(short, long, global = true)]
    pub verbose: bool,
  
    #[command(subcommand)]
    pub command: Commands
}


#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Insert a tape from the library or an arbitrary path.
    Insert {
        /// Library name (e.g. `dms`). Defaults to manifest name with --path.
        name: Option<String>,
        /// Arbitrary tape directory (e.g. a cloned ii-end4 fork).
        #[arg(long)]
        path: Option<PathBuf>,
        /// Plan only; touch nothing.
        #[arg(long)]
        dry_run: bool,
        /// Skip pre/post hooks.
        #[arg(long)]
        no_hooks: bool,
    },

    /// Eject the active tape (or the named one).
    Eject {
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

    // TODO: Plans for install and commit command
    // Install,
    Install {
        source: String,

        #[arg(short, long)]
        name: Option<String>,

        #[arg(short, long)]
        branch: Option<String>,
    },
    Commit,
       
}
