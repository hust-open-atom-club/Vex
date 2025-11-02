use anyhow::{Context, Result};
use std::fs;

use crate::config::{config_file, QemuConfig};
use crate::utils::io::prompt_user_default_no;

pub fn rename_command(
    desc: Option<String>,
    force: bool,
    old_name: String,
    new_name: String,
) -> Result<()> {
    let old_config_path = config_file(&old_name)?;
    if !old_config_path.exists() {
        anyhow::bail!("Configuration '{}' does not exist, cannot rename", old_name);
    }

    let new_config_path = config_file(&new_name)?;
    if new_config_path.exists() && !force {
        println!("Configuration '{}' already exists, overwrite? [y/N]", new_name);
        if !prompt_user_default_no()? {
            println!("Rename cancelled");
            return Ok(());
        }
    }

    // Read the old configuration
    let config_json = fs::read_to_string(&old_config_path).context("Failed to read config file")?;
    let mut config: QemuConfig = serde_json::from_str(&config_json).context("Failed to deserialize configuration")?;

    // Update description if provided
    if let Some(new_desc) = desc {
        config.desc = Some(new_desc);
    }

    // Save to new location
    let new_config_json = serde_json::to_string_pretty(&config).context("Failed to serialize configuration")?;
    fs::write(&new_config_path, new_config_json).context("Failed to save new config file")?;

    // Remove old configuration
    fs::remove_file(&old_config_path).context("Failed to delete old config file")?;

    if let Some(desc) = &config.desc {
        println!("Configuration '{}' renamed to '{}' with description '{}'", old_name, new_name, desc);
    } else {
        println!("Configuration '{}' renamed to '{}'", old_name, new_name);
    }

    Ok(())
}
