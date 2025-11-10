use anyhow::{Context, Result};
use clap::Args;
use std::fs;

use crate::config::{QemuConfig, config_file};

#[derive(Args)]
#[clap(about = "Print detailed information about a saved QEMU configuration")]
pub struct PrintArgs {
    #[arg(help = "Configuration name to print")]
    pub name: String,
}

pub fn print_command(name: String) -> Result<()> {
    let config_path = config_file(&name)?;
    if !config_path.exists() {
        anyhow::bail!(
            "Configuration '{}' does not exist. Use 'vex list' to see available configurations",
            name
        );
    }

    let config_json = fs::read_to_string(&config_path).context("Failed to read config file")?;
    let config: QemuConfig =
        serde_json::from_str(&config_json).context("Failed to deserialize configuration")?;

    // Print configuration details
    println!("Configuration: {}", name);
    println!("{}", "=".repeat(60));
    println!();

    // Print description if available
    if let Some(desc) = &config.desc {
        println!("Description:");
        println!("  {}", desc);
        println!();
    }

    // Print QEMU binary
    println!("QEMU Binary:");
    println!("  {}", config.qemu_bin);
    println!();

    // Print startup arguments
    println!("Startup Arguments:");
    if config.args.is_empty() {
        println!("  (no arguments)");
    } else {
        for (i, arg) in config.args.iter().enumerate() {
            println!("  [{}] {}", i, arg);
        }
    }
    println!();

    // Print full command line
    println!("Full Command:");
    let full_command = format!("{} {}", config.qemu_bin, config.args.join(" "));
    println!("  {}", full_command);
    println!();

    // Print configuration file location
    println!("Configuration File:");
    println!("  {:?}", config_path);

    Ok(())
}
