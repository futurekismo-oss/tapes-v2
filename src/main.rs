mod cli;
pub mod hooks;
pub mod logger;
pub mod state;
pub mod toml;
pub mod util;

use clap::Parser;
use cli::structs::Cli;
use log::info;

use cli::{eject, insert, install, show, status, structs::Commands};
use state::AppState;

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    logger::init(cli.logging);
    info!("Tapes v2 startup");

    match cli.command {
        Commands::Show { name } => show::run(name)?,
        Commands::Insert {
            name,
            path,
            dry_run,
            no_hooks,
        } => insert::run(&mut load_app()?, name, path, dry_run, no_hooks, cli.quiet)?,
        Commands::Status => status::show(load_app()?, cli.verbose)?,
        Commands::Eject { dry_run, no_hooks } => {
            eject::run(&mut load_app()?, dry_run, no_hooks, cli.quiet)?
        }
        Commands::Install { source, name, branch, dir, no_hooks } => {
            install::run(&source, name, branch, dir, no_hooks)?;
        }
        Commands::Commit => {
            anyhow::bail!("The 'commit' command is not yet implemented.");
        }
    }

    Ok(())
}

fn load_app() -> Result<AppState, anyhow::Error> {
    AppState::load_current()
}
