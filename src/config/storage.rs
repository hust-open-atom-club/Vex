use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Get Vex config file storage directory (default ~/.vex/configs)
pub fn config_dir() -> Result<PathBuf> {
    let config_dir_env = std::env::var("VEX_CONFIG_DIR");
    if config_dir_env.is_ok() {
        Ok(PathBuf::from(config_dir_env.unwrap()))
    } else {
        let home = dirs::home_dir().context("Failed to get user home directory")?;
        let dir = home.join(".vex").join("configs");
        if !dir.exists() {
            fs::create_dir_all(&dir).context("Failed to create config directory")?;
        }
        Ok(dir)
    }
}

/// Get path to the config file for a given name
pub fn config_file(name: &str) -> Result<PathBuf> {
    let dir = config_dir()?;
    Ok(dir.join(format!("{}.json", name)))
}
