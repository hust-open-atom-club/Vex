use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::process::Command;

use crate::config::{config_file, QemuConfig};

#[derive(Args)]
#[clap(about = "Execute a saved QEMU configuration")]
pub struct ExecArgs {
    #[arg(help = "Configuration name to execute")]
    pub name: String,

    #[arg(short = 'd', long = "debug", help = "Start QEMU in debug mode (GDB server on port 1234)")]
    pub debug: bool,

    #[arg(short = 'f', long = "full", help = "Show full QEMU command line arguments")]
    pub full: bool,
}

/// TODO: Currently the debug port is fixed at 1234. It should be adaptive or configurable.
pub fn exec_command(name: String, debug: bool, full: bool) -> Result<()> {
    let config_path = config_file(&name)?;
    if !config_path.exists() {
        anyhow::bail!("Configuration '{}' does not exist. Create it first with 'vex save'", name);
    }

    let config_json = fs::read_to_string(&config_path).context("Failed to read config file")?;
    let config: QemuConfig = serde_json::from_str(&config_json).context("Failed to deserialize configuration")?;

    let mut exec_args = config.args.clone();

    if debug {
        // Add debug parameters
        exec_args.push("-s".to_string());
        exec_args.push("-S".to_string());
    }

    // Print startup message
    print_startup_message(&name, &config, &exec_args, debug, full);

    println!();
    println!();

    let status = Command::new(&config.qemu_bin)
        .args(&exec_args)
        .status()
        .with_context(|| format!("Failed to execute QEMU: {}", config.qemu_bin))?;

    if !status.success() {
        anyhow::bail!("QEMU execution failed with exit code: {}", status.code().unwrap_or(-1));
    }

    Ok(())
}

/// Print a user-friendly startup message
fn print_startup_message(name: &str, config: &QemuConfig, args: &[String], debug: bool, full: bool) {
    // Build the header
    let header = if let Some(desc) = &config.desc {
        format!("Starting configuration '{}' ({})", name, desc)
    } else {
        format!("Starting configuration '{}'", name)
    };

    println!("{}", header);

    // Show full command if -f flag is used
    if full {
        println!("  QEMU: {}", config.qemu_bin);
        println!("  Args: {:?}", args);
    }

    // Show debug info if in debug mode
    if debug {
        println!("  Mode: DEBUG");
        println!("  GDB server: localhost:1234");
        println!("\n💡 You can connect with: gdb -ex 'target remote localhost:1234'");
    }
}
