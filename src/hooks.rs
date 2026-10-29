use std::{env, process::Command};
use anyhow::{anyhow, Result};

pub fn run_hooks(hooks: Vec<String>) -> Result<(), anyhow::Error> {
    for hook in hooks {
        let hook = hook.trim();
        
        println!("running '{}'", hook);       
        run_command(hook)?;
    }
    
    Ok(())
}


pub fn run_command(command: &str) -> Result<(), anyhow::Error> {
    let shell = get_user_preferred_shell();

    let status = Command::new(shell).arg("-c").arg(command).status()?;

    if !status.success() {
        return Err(anyhow!("Command failed with exit status: {}", status));
    }

    Ok(())
}

fn get_user_preferred_shell() -> String {
    env::var("SHELL").unwrap_or("sh".to_string())
}
