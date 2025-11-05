use anyhow::{Context, Result};
use clap::Args;
use std::fs;

use crate::config::{config_file, QemuConfig};
use crate::utils::io::{prompt_user, prompt_user_default_no};

#[derive(Args)]
#[clap(about = "Save QEMU configuration")]
pub struct SaveArgs {
    #[arg(help = "Configuration name for later reference")]
    pub name: String,

    #[arg(help = "Path to the QEMU executable (e.g., qemu-system-x86_64)")]
    pub qemu_bin: String,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true, help = "QEMU startup arguments")]
    pub qemu_args: Vec<String>,

    #[arg(short = 'd', long = "desc", help = "Optional description for the configuration")]
    pub desc: Option<String>,

    #[arg(short = 'f', long = "force", help = "Force save without confirmation if configuration exists")]
    pub force: bool,
}

pub fn save_command(
    force: bool,
    name: String,
    desc: Option<String>,
    qemu_bin: String,
    qemu_args: Vec<String>,
) -> Result<()> {
    let config_path = config_file(&name)?;
    
    // Check if debug parameters -s or -S are present
    let has_debug_args = qemu_args.iter().any(|arg| arg == "-s" || arg == "-S");
    
    let mut final_args = qemu_args.clone();
    
    if has_debug_args {
        println!("Debug parameters '-s' or '-S' detected in startup arguments");
        println!("These parameters are used to start GDB debugging server, but saving them to configuration may not be the best practice.");
        println!("Suggestion: Skip saving these parameters and use 'vex exec -d' to start remote debugging mode");
        println!("Skip saving debug parameters and use exec -d for remote debugging? [Y/n]");
        
        if prompt_user()? {
            // User chose to skip debug parameters
            final_args = qemu_args.iter()
                .filter(|&arg| arg != "-s" && arg != "-S")
                .cloned()
                .collect();
            println!("Debug parameters have been skipped, saved configuration will not include -s or -S parameters");
            println!("To start debugging mode, use: vex exec -d {}", name);
        } else {
            println!("Debug parameters will be included in the saved configuration");
        }
    }

    let config = QemuConfig {
        qemu_bin: qemu_bin.clone(),
        args: final_args,
        desc,
    };

    if config_path.exists() && !force {
        println!("Configuration '{}' already exists, overwrite? [y/N]", name);
        if !prompt_user_default_no()? {
            println!("Save cancelled");
            return Ok(());
        }
    }

    let config_json = serde_json::to_string_pretty(&config).context("Failed to serialize configuration")?;
    fs::write(&config_path, config_json).context("Failed to save config file")?;
    
    if let Some(desc) = &config.desc {
        println!("Configuration '{}' with description '{}' saved to {:?}", name, desc, config_path);
    } else {
        println!("Configuration '{}' saved to {:?}", name, config_path);
    }

    Ok(())
}
