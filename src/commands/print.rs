use clap::Args;
use std::fs;

use crate::config::{QemuConfig, config_file, validate_config_name};
use crate::error::{VexError, VexResult};

#[derive(Args, Debug)]
pub struct PrintArgs {
    pub name: String,
}

pub fn print_command(name: String) -> VexResult<()> {
    validate_config_name(&name)?;
    let config_path = config_file(&name)?;
    if !config_path.exists() {
        return Err(VexError::ConfigNotFound { name: name.clone() });
    }

    let config_json = fs::read_to_string(&config_path).map_err(|e| VexError::IoError {
        path: config_path.clone(),
        operation: "read config file".to_string(),
        source: e,
    })?;
    let config: QemuConfig = serde_json::from_str(&config_json)
        .map_err(|e| VexError::ConfigParseFailed { source: e })?;

    println!("Configuration: {}", name);
    println!("{}", "=".repeat(60));
    println!();

    if let Some(desc) = &config.desc {
        println!("Description:");
        println!("  {}", desc);
        println!();
    }

    println!("QEMU Binary:");
    println!("  {}", config.qemu_bin);
    println!();

    println!("Startup Arguments:");
    if config.args.is_empty() {
        println!("  (no arguments)");
    } else {
        for (i, arg) in config.args.iter().enumerate() {
            println!("  [{}] {}", i, arg);
        }
    }
    println!();

    println!("Full Command:");
    let full_command = format!("{} {}", config.qemu_bin, config.args.join(" "));
    println!("  {}", full_command);
    println!();

    println!("Configuration File:");
    println!("  {:?}", config_path);

    Ok(())
}
