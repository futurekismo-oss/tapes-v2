mod cli;

use clap::Parser;
use cli::structs::Cli;

fn main() {

    let cli = Cli::parse();

    match cli.commands {
        _ => todo!()
    }
   
}
