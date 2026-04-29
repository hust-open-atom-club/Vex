use clap::Args;
use regex::Regex;
use std::collections::HashMap;
use std::fs;
use std::process::Command;

use crate::config::{QemuConfig, config_file, validate_config_name};
use crate::error::{VexError, VexResult};
use crate::utils::qemu::get_qemu_version;

#[derive(Args, Debug)]
pub struct ExecArgs {
    pub name: String,
    #[arg(short = 'd', long = "debug")]
    pub debug: bool,
    #[arg(short = 'f', long = "full")]
    pub full: bool,
}

pub fn exec_command(name: String, debug: bool, full: bool) -> VexResult<()> {
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
            _ => {}
        }
    }
    let mut exec_args = config.args.clone();

    exec_args = substitute_params(&exec_args, |k| std::env::var(k).ok());

    if debug {
        exec_args.push("-s".to_string());
        exec_args.push("-S".to_string());
    }

    print_startup_message(&name, &config, &exec_args, debug, full);

    let status = Command::new(&config.qemu_bin)
        .args(&exec_args)
        .status()
        .map_err(|e| VexError::QemuLaunchFailed {
            binary: config.qemu_bin.clone(),
            source: e,
        })?;

    if !status.success() {
        return Err(VexError::QemuExitError {
            binary: config.qemu_bin.clone(),
            exit_code: status.code(),
        });
    }

    Ok(())
}

fn print_startup_message(
    name: &str,
    config: &QemuConfig,
    args: &[String],
    debug: bool,
    full: bool,
) {
    let header = if let Some(desc) = &config.desc {
        format!("Starting configuration '{}' ({})", name, desc)
    } else {
        format!("Starting configuration '{}'", name)
    };

    println!("{}", header);

    if full {
        println!("  QEMU: {}", config.qemu_bin);
        println!("  Args: {:?}", args);
    }

    if debug {
        println!("  Mode: DEBUG");
        println!("  GDB server: localhost:1234");
        println!("\n💡 You can connect with: gdb -ex 'target remote localhost:1234'");
    }
}

pub(crate) fn substitute_params<F>(args: &[String], env_fn: F) -> Vec<String>
where
    F: Fn(&str) -> Option<String>,
{
    let re = Regex::new(r"\$\{([^}]+)\}").unwrap();
    args.iter()
        .map(|arg| {
            re.replace_all(arg, |caps: &regex::Captures| {
                env_fn(&caps[1]).unwrap_or_else(|| format!("${{{}}}", &caps[1]))
            })
            .to_string()
        })
        .collect()
}

#[allow(dead_code)]
pub(crate) fn substitute_params_with_map(
    args: &[String],
    env: &HashMap<String, String>,
) -> Vec<String> {
    substitute_params(args, |k| env.get(k).cloned())
}
