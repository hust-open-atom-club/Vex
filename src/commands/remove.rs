use clap::Args;
use std::fs;

use crate::config::{config_file, sanitize_config_name};
use crate::error::{VexError, VexResult};

#[derive(Args, Debug)]
pub struct RemoveArgs {
    pub name: String,
}

pub fn remove_command(name: String) -> VexResult<()> {
    sanitize_config_name(&name)?;
    let config_path = config_file(&name)?;
    if !config_path.exists() {
        return Err(VexError::ConfigNotFound { name: name.clone() });
    }

    fs::remove_file(&config_path).map_err(|e| VexError::IoError {
        path: config_path,
        operation: "delete config file".to_string(),
        source: e,
    })?;
    println!("Configuration '{}' deleted", name);

    Ok(())
}
