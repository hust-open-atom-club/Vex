use clap::Args;
use std::fs;

use crate::config::{QemuConfig, config_file, validate_config, validate_config_name};
use crate::error::{VexError, VexResult};
use crate::utils::io::{prompt_user, prompt_user_default_no};
use crate::utils::qemu::get_qemu_version;

#[derive(Args, Debug)]
pub struct SaveArgs {
    pub name: String,
    pub qemu_bin: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub qemu_args: Vec<String>,
    #[arg(short = 'd', long = "desc")]
    pub desc: Option<String>,
    #[arg(short = 'f', long = "force")]
    pub force: bool,
}

pub fn save_command(
    force: bool,
    name: String,
    desc: Option<String>,
    qemu_bin: String,
    qemu_args: Vec<String>,
) -> VexResult<()> {
    validate_config_name(&name)?;
    let config_path = config_file(&name)?;

    let has_debug_args = qemu_args.iter().any(|arg| arg == "-s" || arg == "-S");

    let mut final_args = qemu_args.clone();

    if has_debug_args {
        println!("Debug parameters '-s' or '-S' detected in startup arguments");
        println!(
            "These parameters are used to start GDB debugging server, but saving them to configuration may not be the best practice."
        );
        println!(
            "Suggestion: Skip saving these parameters and use 'vex exec -d' to start remote debugging mode"
        );
        println!("Skip saving debug parameters and use exec -d for remote debugging? [Y/n]");

        if prompt_user()? {
            final_args = qemu_args
                .iter()
                .filter(|&arg| arg != "-s" && arg != "-S")
                .cloned()
                .collect();
            println!(
                "Debug parameters have been skipped, saved configuration will not include -s or -S parameters"
            );
            println!("To start debugging mode, use: vex exec -d {}", name);
        } else {
            println!("Debug parameters will be included in the saved configuration");
        }
    }

    let qemu_version = get_qemu_version(&qemu_bin);

    if let Some(v) = &qemu_version {
        println!("Detected QEMU version: {}", v);
    }
    let config = QemuConfig {
        qemu_bin: qemu_bin.clone(),
        args: final_args,
        desc,
        qemu_version,
    };

    validate_config(&config)?;

    if config_path.exists() && !force {
        println!("Configuration '{}' already exists, overwrite? [y/N]", name);
        if !prompt_user_default_no()? {
            println!("Save cancelled");
            return Ok(());
        }
    }

    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| VexError::ConfigSerializeFailed { source: e })?;
    fs::write(&config_path, config_json).map_err(|e| VexError::IoError {
        path: config_path.clone(),
        operation: "save config file".to_string(),
        source: e,
    })?;

    if let Some(desc) = &config.desc {
        println!(
            "Configuration '{}' with description '{}' saved to {:?}",
            name, desc, config_path
        );
    } else {
        println!("Configuration '{}' saved to {:?}", name, config_path);
    }

    Ok(())
}
