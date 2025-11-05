use anyhow::{Context, Result};
use clap::Parser;
use std::fs;

use crate::config::config_file;

#[derive(Args)]
#[clap(about = "Remove a saved QEMU configuration")]
pub struct RemoveArgs {
    #[arg(help = "Configuration name to remove")]
    pub name: String,
}

pub fn remove_command(name: String) -> Result<()> {
    let config_path = config_file(&name)?;
    if !config_path.exists() {
        anyhow::bail!("Configuration '{}' does not exist, cannot delete", name);
    }

    fs::remove_file(&config_path).context("Failed to delete config file")?;
    println!("Configuration '{}' deleted", name);

    Ok(())
}
