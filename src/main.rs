mod cli;

use clap::Parser;
use cli::structs::Cli;

// Logging setup
extern crate pretty_env_logger;
#[macro_use]
extern crate log;

fn main() {
    pretty_env_logger::init();

    info!("Program started");

    let cli = Cli::parse();

    match cli.commands {
        _ => todo!(),
    }
}
