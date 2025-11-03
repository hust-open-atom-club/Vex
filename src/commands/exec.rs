use anyhow::{Context, Result};
use std::fs;
use std::process::Command;

use crate::config::{config_file, QemuConfig};

/// TODO: Currently the debug port is fixed at 1234. It should be adaptive or configurable.
pub fn exec_command(name: String, debug: bool) -> Result<()> {
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
        if let Some(desc) = &config.desc {
            println!("Starting configuration '{}' ({}) in DEBUG mode: {} {:?}", name, desc, config.qemu_bin, exec_args);
        } else {
            println!("Starting configuration '{}' in DEBUG mode: {} {:?}", name, config.qemu_bin, exec_args);
        }
        println!("GDB debugging server started, you can connect to localhost:1234 using gdb");
    } else {
        if let Some(desc) = &config.desc {
            println!("Starting configuration '{}' ({}): {} {:?}", name, desc, config.qemu_bin, exec_args);
        } else {
            println!("Starting configuration '{}': {} {:?}", name, config.qemu_bin, exec_args);
        }
    }
    
    let status = Command::new(&config.qemu_bin)
        .args(&exec_args)
        .status()
        .with_context(|| format!("Failed to execute QEMU: {}", config.qemu_bin))?;

    if !status.success() {
        anyhow::bail!("QEMU execution failed with exit code: {}", status.code().unwrap_or(-1));
    }

    Ok(())
}
