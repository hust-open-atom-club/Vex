use anyhow::{Context, Result};
use clap::Args;
use std::fs;

use crate::config::{QemuConfig, config_dir};

#[derive(Args, Debug)]
pub struct ListArgs;

pub fn list_command() -> Result<()> {
    let dir = config_dir()?;
    if !dir.exists() {
        println!("No configurations saved yet.");
        return Ok(());
    }

    let entries = fs::read_dir(&dir).context("Failed to read config directory")?;
    let mut configs = Vec::new();

    for entry in entries {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json")
            && let Some(name) = path.file_stem().and_then(|s| s.to_str())
        {
            match fs::read_to_string(&path) {
                Ok(content) => {
                    if let Ok(config) = serde_json::from_str::<QemuConfig>(&content) {
                        configs.push((name.to_string(), config));
                    }
                }
                Err(_) => {
                    // Skip invalid config files
                    continue;
                }
            }
        }
    }

    if configs.is_empty() {
        println!("No configurations found.");
    } else {
        println!("Saved configurations:");
        for (name, config) in configs {
            if let Some(desc) = config.desc {
                println!("  {} - {}", name, desc);
            } else {
                println!("  {} - (no description)", name);
            }
            println!("    QEMU: {}", config.qemu_bin);
            println!("    Args: {:?}", config.args);
            println!();
        }
    }

    Ok(())
}
