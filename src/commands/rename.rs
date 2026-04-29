use clap::Args;
use std::fs;

use crate::config::{QemuConfig, config_file, validate_config_name};
use crate::error::{VexError, VexResult};
use crate::utils::io::prompt_user_default_no;

#[derive(Args, Debug)]
pub struct RenameArgs {
    pub old_name: String,
    pub new_name: String,
    #[arg(short = 'd', long = "desc")]
    pub desc: Option<String>,
    #[arg(short = 'f', long = "force")]
    pub force: bool,
}

pub fn rename_command(
    desc: Option<String>,
    force: bool,
    old_name: String,
    new_name: String,
) -> VexResult<()> {
    validate_config_name(&old_name)?;
    validate_config_name(&new_name)?;

    let old_config_path = config_file(&old_name)?;
    if !old_config_path.exists() {
        return Err(VexError::ConfigNotFound {
            name: old_name.clone(),
        });
    }

    let new_config_path = config_file(&new_name)?;
    if new_config_path.exists() && !force {
        println!(
            "Configuration '{}' already exists, overwrite? [y/N]",
            new_name
        );
        if !prompt_user_default_no()? {
            println!("Rename cancelled");
            return Ok(());
        }
    }

    let config_json = fs::read_to_string(&old_config_path).map_err(|e| VexError::IoError {
        path: old_config_path.clone(),
        operation: "read config file".to_string(),
        source: e,
    })?;
    let mut config: QemuConfig = serde_json::from_str(&config_json)
        .map_err(|e| VexError::ConfigParseFailed { source: e })?;

    if let Some(new_desc) = desc {
        config.desc = Some(new_desc);
    }

    let new_config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| VexError::ConfigSerializeFailed { source: e })?;
    fs::write(&new_config_path, new_config_json).map_err(|e| VexError::IoError {
        path: new_config_path.clone(),
        operation: "save new config file".to_string(),
        source: e,
    })?;

    fs::remove_file(&old_config_path).map_err(|e| VexError::IoError {
        path: old_config_path,
        operation: "delete old config file".to_string(),
        source: e,
    })?;

    if let Some(desc) = &config.desc {
        println!(
            "Configuration '{}' renamed to '{}' with description '{}'",
            old_name, new_name, desc
        );
    } else {
        println!("Configuration '{}' renamed to '{}'", old_name, new_name);
    }

    Ok(())
}
