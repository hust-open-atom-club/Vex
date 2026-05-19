use clap::Args;
use regex::Regex;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::config::{QemuConfig, ResourceRef, load_config};
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

#[derive(Debug)]
pub struct PreparedCommand {
    pub command: Command,
    pub final_args: Vec<String>,
}

pub fn prepare_command(config: &QemuConfig, debug: bool) -> VexResult<PreparedCommand> {
    let mut final_args = config.args.clone();

    final_args = substitute_params(&final_args, |k| std::env::var(k).ok());

    check_resource_files(&config.resources)?;
    final_args = substitute_resources(&final_args, &config.resources)?;

    if debug {
        final_args.push("-s".to_string());
        final_args.push("-S".to_string());
    }

    let mut command = Command::new(&config.qemu_bin);
    command.args(&final_args);

    Ok(PreparedCommand {
        command,
        final_args,
    })
}

pub fn exec_command(name: String, debug: bool, full: bool) -> VexResult<()> {
    let config = load_config(&name)?;

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

    let mut prepared = prepare_command(&config, debug)?;

    print_startup_message(&name, &config, &prepared.final_args, debug, full);

    let status = prepared
        .command
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
        if !config.resources.is_empty() {
            println!("  Resources:");
            for (key, r) in &config.resources {
                println!(
                    "    {} -> {} ({:?}{})",
                    key,
                    r.path,
                    r.kind,
                    r.sha256.as_ref().map(|_| ", sha256").unwrap_or("")
                );
            }
        }
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

pub(crate) fn substitute_resources(
    args: &[String],
    resources: &HashMap<String, ResourceRef>,
) -> VexResult<Vec<String>> {
    let re = Regex::new(r"\$\{res:([A-Za-z_][A-Za-z0-9_]*)\}").unwrap();
    let mut out = Vec::with_capacity(args.len());
    for (i, arg) in args.iter().enumerate() {
        for caps in re.captures_iter(arg) {
            let key = &caps[1];
            if !resources.contains_key(key) {
                return Err(VexError::UnknownResourceReference {
                    key: key.to_string(),
                    arg_index: i,
                });
            }
        }
        let replaced = re.replace_all(arg, |caps: &regex::Captures| {
            resources[&caps[1]].path.clone()
        });
        out.push(replaced.into_owned());
    }
    Ok(out)
}

pub(crate) fn check_resource_files(resources: &HashMap<String, ResourceRef>) -> VexResult<()> {
    for (key, r) in resources {
        if !Path::new(&r.path).exists() {
            return Err(VexError::ResourceFileNotFound {
                key: key.clone(),
                path: PathBuf::from(&r.path),
            });
        }
    }
    Ok(())
}
