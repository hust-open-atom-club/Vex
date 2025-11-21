use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::process::Command;

use crate::config::{QemuConfig, config_file};

#[derive(Args)]
#[clap(about = "Execute a saved QEMU configuration")]
pub struct ExecArgs {
    #[arg(help = "Configuration name to execute")]
    pub name: String,

    #[arg(
        short = 'd',
        long = "debug",
        help = "Start QEMU in debug mode (GDB server on port 1234)"
    )]
    pub debug: bool,

    #[arg(
        short = 'f',
        long = "full",
        help = "Show full QEMU command line arguments"
    )]
    pub full: bool,

    #[arg(
        trailing_var_arg = true,
        help = "Arguments to substitute $0, $1, $2, etc. in the configuration"
    )]
    pub params: Vec<String>,
}

/// TODO: Currently the debug port is fixed at 1234. It should be adaptive or configurable.
pub fn exec_command(name: String, params: Vec<String>, debug: bool, full: bool) -> Result<()> {
    let config_path = config_file(&name)?;
    if !config_path.exists() {
        anyhow::bail!(
            "Configuration '{}' does not exist. Create it first with 'vex save'",
            name
        );
    }

    let config_json = fs::read_to_string(&config_path).context("Failed to read config file")?;
    let config: QemuConfig =
        serde_json::from_str(&config_json).context("Failed to deserialize configuration")?;

    // Substitute parameters in arguments
    let mut exec_args = substitute_params(&config.args, &params)?;

    if debug {
        // Add debug parameters
        exec_args.push("-s".to_string());
        exec_args.push("-S".to_string());
    }

    // Print startup message
    print_startup_message(&name, &config, &exec_args, debug, full);

    let status = Command::new(&config.qemu_bin)
        .args(&exec_args)
        .status()
        .with_context(|| format!("Failed to execute QEMU: {}", config.qemu_bin))?;

    if !status.success() {
        anyhow::bail!(
            "QEMU execution failed with exit code: {}",
            status.code().unwrap_or(-1)
        );
    }

    Ok(())
}

/// Substitute $0, $1, $2, etc. in arguments with provided parameters
fn substitute_params(args: &[String], params: &[String]) -> Result<Vec<String>> {
    let mut result = Vec::new();

    for arg in args {
        let mut substituted = arg.clone();

        // Replace $0, $1, $2, etc. with corresponding parameters
        for (i, param) in params.iter().enumerate() {
            let placeholder = format!("${}", i);
            substituted = substituted.replace(&placeholder, param);
        }

        // Check if there are any unsubstituted placeholders
        if substituted.contains("$") {
            // Extract all placeholders like $0, $1, etc.
            let mut missing_params = Vec::new();
            for i in 0..100 {
                // Check up to $99
                let placeholder = format!("${}", i);
                if substituted.contains(&placeholder) && i >= params.len() {
                    missing_params.push(placeholder);
                }
            }

            if !missing_params.is_empty() {
                anyhow::bail!(
                    "Missing parameters for placeholders: {}. Provided {} parameter(s), but configuration requires more.",
                    missing_params.join(", "),
                    params.len()
                );
            }
        }

        result.push(substituted);
    }

    Ok(result)
}

/// Print a user-friendly startup message
fn print_startup_message(
    name: &str,
    config: &QemuConfig,
    args: &[String],
    debug: bool,
    full: bool,
) {
    // Build the header
    if let Some(desc) = &config.desc {
        print!("Starting configuration '{}' ({})", name, desc);
    } else {
        print!("Starting configuration '{}'", name);
    }

    // Show full command if -f flag is used
    if full {
        println!(": {} {:?}", config.qemu_bin, args);
    } else {
        println!();
    }

    // Show debug info if in debug mode
    if debug {
        println!("Debug mode enabled. GDB server: localhost:1234");
        println!("Connect with: gdb -ex 'target remote localhost:1234'");
    }
}
