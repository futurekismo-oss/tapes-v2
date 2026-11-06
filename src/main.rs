mod cli;
pub mod config;
pub mod hooks;
pub mod logger;
pub mod state;
pub mod toml;
pub mod util;

#[cfg(test)]
mod tests;

use clap::{CommandFactory, Parser};
use clap_complete::CompleteEnv;
use cli::structs::Cli;
use log::info;

use cli::{config as cli_config, eject, insert, install, show, status, structs::Commands};
use state::AppState;

use crate::{cli::completions, util::get_tapes_data_path};

fn main() -> Result<(), anyhow::Error> {
    // Tab Completion for tapes dynamically
    // It must be at the top here so regular command parsing doesnt interefare
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();

    let tapes_data_path = get_tapes_data_path()?;
    let logging_path = tapes_data_path.join("tapes.log");

    logger::init(cli.logging, Some(&logging_path));

    info!("Tapes startup");

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
        Commands::Install {
            source,
            name,
            branch,
            dir,
            no_hooks,
            yes,
            force,
            full_clone,
        } => {
            install::run(install::InstallOptions {
                source,
                name,
                branch,
                dir,
                no_hooks,
                yes,
                force,
                full_clone,
                quiet: cli.quiet,
            })?;
        }
        Commands::Config { action } => {
            cli_config::run(action)?;
        }
    }

    Ok(())
}

fn load_app() -> Result<AppState, anyhow::Error> {
    AppState::load_current()
}
