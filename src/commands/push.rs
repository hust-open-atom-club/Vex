use clap::Args;
use std::fs;

use crate::config::{QemuConfig, config_file, validate_config, validate_config_name};
use crate::error::{VexError, VexResult};
use crate::remote::{PublishOutcome, RemoteSpec, publish_config};

#[derive(Args, Debug)]
pub struct PushArgs {
    pub remote_ref: String,
    pub local_name: String,
    #[arg(short = 'f', long = "force")]
    pub force: bool,
}

pub fn push_command(force: bool, remote_ref: String, local_name: String) -> VexResult<()> {
    let spec = RemoteSpec::parse(&remote_ref)?;
    validate_config_name(&local_name)?;
    let config_path = config_file(&local_name)?;
    if !config_path.exists() {
        return Err(VexError::ConfigNotFound {
            name: local_name.clone(),
        });
    }

    let config_json = fs::read_to_string(&config_path).map_err(|e| VexError::IoError {
        path: config_path.clone(),
        operation: "read config file".to_string(),
        source: e,
    })?;
    let config: QemuConfig = serde_json::from_str(&config_json)
        .map_err(|e| VexError::ConfigParseFailed { source: e })?;

    validate_config(&config)?;

    match publish_config(&spec, &config, force)? {
        PublishOutcome::Cancelled => {}
        PublishOutcome::NoChanges => {
            println!(
                "Remote configuration '{} / {}:{}' is already up to date",
                spec.id,
                spec.name,
                spec.resolved_tag()
            );
        }
        PublishOutcome::Pushed => {
            println!(
                "Pushed local configuration '{}' to '{} / {}:{}'",
                local_name,
                spec.id,
                spec.name,
                spec.resolved_tag()
            );
        }
    }

    Ok(())
}
