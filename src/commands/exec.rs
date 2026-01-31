use anyhow::{Context, Result};
use clap::Args;
use regex::Regex;
use std::fs;
use std::process::Command;

use crate::config::{QemuConfig, config_file};
use crate::utils::qemu::get_qemu_version;

#[derive(Args, Debug)]
pub struct ExecArgs {
    /// Configuration name to execute.
    ///
    /// # Examples
    ///
    /// Run a VM normally:
    /// ```shell
    /// vex exec my-vm
    /// ```
    ///
    /// Run in Debug mode (waits for GDB connection):
    /// ```shell
    /// vex exec -d my-vm
    /// ```
    pub name: String,

    /// Start QEMU in debug mode.
    ///
    /// This appends `-s -S` to the QEMU arguments:
    /// - `-s`: Shorthand for -gdb tcp::1234
    /// - `-S`: Freeze CPU at startup
    ///
    /// Useful for attaching a debugger (GDB) before the OS boots.
    #[arg(short = 'd', long = "debug")]
    pub debug: bool,

    /// Show full QEMU command line arguments before starting.
    #[arg(short = 'f', long = "full")]
    pub full: bool,
}

/// TODO: Currently the debug port is fixed at 1234. It should be adaptive or configurable.
pub fn exec_command(name: String, debug: bool, full: bool) -> Result<()> {
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

    if let Some(saved_ver) = &config.qemu_version {
        let current_ver = get_qemu_version(&config.qemu_bin);
        match current_ver {
            Some(curr) if curr != *saved_ver => {
                println!("WARNING: Version mismatch!");
                println!("   Configuration saved with QEMU {}", saved_ver);
                println!("   Current system has QEMU {}", curr);
                println!("   Some features might not work as expected.\n");
            }
            None => {
                println!("WARNING: Could not detect current QEMU version.\n");
            }
            _ => {} // Versions match, all good
        }
    }
    let mut exec_args = config.args.clone();

    // Substitute parameters in args
    exec_args = substitute_params(&exec_args);

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

/// Print a user-friendly startup message
fn print_startup_message(
    name: &str,
    config: &QemuConfig,
    args: &[String],
    debug: bool,
    full: bool,
) {
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

/// Substitute parameters in arguments using regex
pub(crate) fn substitute_params(args: &[String]) -> Vec<String> {
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
    args.iter()
        .map(|arg| {
            re.replace_all(arg, |caps: &regex::Captures| {
                std::env::var(&caps[1]).unwrap_or_else(|_| format!("${{{}}}", &caps[1]))
            })
            .to_string()
        })
        .collect()
}
