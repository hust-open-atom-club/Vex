use anyhow::{Context, Result};
use std::fs;
use std::path::PathBuf;

/// Get Vex config file storage directory (default ~/.vex/configs)
pub fn config_dir() -> Result<PathBuf> {
    let dir = match std::env::var("VEX_CONFIG_DIR") {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => {
            let home = dirs::home_dir().context("Failed to get user home directory")?;
            home.join(".vex").join("configs")
        }
    };

    fs::create_dir_all(&dir).context("Failed to create config directory")?;
    Ok(dir)
}

/// Get path to the config file for a given name
pub fn config_file(name: &str) -> Result<PathBuf> {
    let dir = config_dir()?;
    Ok(dir.join(format!("{}.json", name)))
}
