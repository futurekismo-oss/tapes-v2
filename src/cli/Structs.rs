use clap::{Parser, Subcommand};

/// Tapes V2 - A Multiple-dotfiles manager

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Cli {

    #[command(subcommand)]
    pub commands: Commands,
    
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Insert a tape
    Insert,

    /// Eject the currently inserted tape
    Eject,

    /// Print the currently inserted tape
    Current,
}
