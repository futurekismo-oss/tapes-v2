mod cli;
pub mod state;
pub mod toml;
pub mod util;

use clap::Parser;
use cli::structs::Cli;
use log::{info, LevelFilter};

use cli::{insert, show, structs::Commands};
use state::AppState;
use util::get_tapes_data_path;

fn main() -> Result<(), anyhow::Error> {
    let cli = Cli::parse();

    let state_path = get_tapes_data_path()?.join("state.json");
    let mut app = AppState::load(state_path)?;

    init_logging(cli.verbose);
    info!("Tapes v2 startup");

    match cli.command {
        Commands::Show { name } => show::run(name)?,
        Commands::Insert {
            name,
            path,
            dry_run,
            no_hooks,
        } => insert::run(&mut app, name, path, dry_run, no_hooks)?,
        _ => todo!(),
    }

    Ok(())
}

fn init_logging(verbose: bool) {
    let mut builder = env_logger::Builder::new();

    if let Ok(rust_log_env) = std::env::var("RUST_LOG") {
        builder.parse_filters(&rust_log_env);
    } else {
        let filter_level = if verbose {
            LevelFilter::Trace
        } else {
            LevelFilter::Warn
        };

        builder.filter_level(filter_level);
    };

    builder.init();
}
