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

    /// Reduce the verbosity of commands
    #[arg(short, long, global = true)]
    pub quiet: bool,
  
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

        /// Only include specific targets from the tape (comma-separated, e.g. hyprland,quickshell).
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        // Stores target names that should be selectively symlinked from the tape
        include_targets: Option<Vec<String>>,

        /// Exclude specific targets from the tape (comma-separated, e.g. quickshell).
        #[arg(long, value_delimiter = ',', num_args = 1..)]
        // Stores target names that should be excluded from being symlinked from the tape
        exclude_targets: Option<Vec<String>>,
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
        /// Name of tape
        name: Option<String>,
    },

    /// Install a tape from a local path or Git repository into ~/.tapes.
    Install {
        /// Local directory path or Git repository URL / shorthand (user/repo).
        source: String,

        /// Custom name for the tape (defaults to folder or repository name).
        #[arg(short, long)]
        name: Option<String>,

        /// Specific branch, tag, or commit to clone (for Git repositories).
        #[arg(short, long)]
        branch: Option<String>,

        /// Specific dir from the Git repository
        #[arg(short, long)]
        dir: Option<String>,
       
    },

    Commit,
       
}

#[cfg(test)]
mod tests {
    // Import definitions from parent module
    use super::*;

    #[test]
    fn test_insert_command_args_include() {
        // Parse CLI arguments with --include-targets using commas
        let cli = Cli::try_parse_from([
            "tape",
            "insert",
            "caelestia",
            "--include-targets",
            "hyprland,quickshell",
        ])
        .expect("CLI arguments should parse successfully");

        // Verify the command is Insert and check parsed include targets
        if let Commands::Insert {
            name,
            include_targets,
            exclude_targets,
            ..
        } = cli.command
        {
            // Verify tape name
            assert_eq!(name, Some("caelestia".to_string()));
            // Verify include targets parsed into list
            assert_eq!(
                include_targets,
                Some(vec!["hyprland".to_string(), "quickshell".to_string()])
            );
            // Verify exclude targets is None
            assert_eq!(exclude_targets, None);
        } else {
            // Fail if not Insert command
            panic!("Expected Commands::Insert");
        }
    }

    #[test]
    fn test_insert_command_args_exclude() {
        // Parse CLI arguments with --exclude-targets
        let cli = Cli::try_parse_from([
            "tape",
            "insert",
            "caelestia",
            "--exclude-targets",
            "quickshell",
        ])
        .expect("CLI arguments should parse successfully");

        // Verify the command is Insert and check parsed exclude targets
        if let Commands::Insert {
            name,
            include_targets,
            exclude_targets,
            ..
        } = cli.command
        {
            // Verify tape name
            assert_eq!(name, Some("caelestia".to_string()));
            // Verify include targets is None
            assert_eq!(include_targets, None);
            // Verify exclude targets parsed
            assert_eq!(exclude_targets, Some(vec!["quickshell".to_string()]));
        } else {
            // Fail if not Insert command
            panic!("Expected Commands::Insert");
        }
    }

    #[test]
    fn test_insert_command_args_multiple_flags() {
        // Parse CLI arguments with multiple separate --include-targets flags
        let cli = Cli::try_parse_from([
            "tape",
            "insert",
            "caelestia",
            "--include-targets",
            "hyprland",
            "--include-targets",
            "quickshell",
        ])
        .expect("CLI arguments should parse successfully");

        // Verify the command is Insert and check accumulated include targets
        if let Commands::Insert {
            include_targets, ..
        } = cli.command
        {
            // Verify multiple flags were accumulated into a list
            assert_eq!(
                include_targets,
                Some(vec!["hyprland".to_string(), "quickshell".to_string()])
            );
        } else {
            // Fail if not Insert command
            panic!("Expected Commands::Insert");
        }
    }
}
