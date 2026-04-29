use std::fs;
use std::path::PathBuf;

use crate::config::QemuConfig;
use crate::config::validation::{parse_config_json, validate_config_name};
use crate::error::{VexError, VexResult};

pub fn config_dir() -> VexResult<PathBuf> {
    let dir = match std::env::var("VEX_CONFIG_DIR") {
        Ok(path) if !path.is_empty() => PathBuf::from(path),
        _ => {
            let home = dirs::home_dir().ok_or_else(|| VexError::IoError {
                path: PathBuf::from("~"),
                operation: "resolve home directory".to_string(),
                source: std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "failed to get user home directory",
                ),
            })?;
            home.join(".vex").join("configs")
        }
    };

    fs::create_dir_all(&dir).map_err(|e| VexError::IoError {
        path: dir.clone(),
        operation: "create config directory".to_string(),
        source: e,
    })?;
    Ok(dir)
}

pub fn config_file(name: &str) -> VexResult<PathBuf> {
    let dir = config_dir()?;
    Ok(dir.join(format!("{}.json", name)))
}

pub fn load_config(name: &str) -> VexResult<QemuConfig> {
    validate_config_name(name)?;
    let path = config_file(name)?;
    if !path.exists() {
        return Err(VexError::ConfigNotFound {
            name: name.to_string(),
        });
    }
    let content = fs::read_to_string(&path).map_err(|e| VexError::IoError {
        path: path.clone(),
        operation: "read config file".to_string(),
        source: e,
    })?;
    parse_config_json(&content)
}
