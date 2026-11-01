use std::io;
use clap::CommandFactory;
use clap_complete::{Shell, generate};
use log::info;


pub fn print_completions<Cli: CommandFactory>(shell: Shell) -> Result<(), anyhow::Error> {
   let mut cmd = Cli::command();

    generate(shell, &mut cmd, "tapes-v2", &mut io::stdout());

    Ok(())
}
