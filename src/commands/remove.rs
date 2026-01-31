use anyhow::{Context, Result};
use clap::Args;
use std::fs;

use crate::config::config_file;

#[derive(Args, Debug)]
pub struct RemoveArgs {
    /// Configuration name to remove.
    ///
    /// This action is irreversible. The configuration file will be permanently deleted.
    ///
    /// # Examples
    ///
    /// Delete a configuration:
    /// ```shell
    /// vex rm test-vm
    /// ```
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
