use clap::Args;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{
    QemuConfig, ResourceKind, ResourceRef, config_file, validate_config, validate_config_name,
};
use crate::error::{VexError, VexResult};
use crate::utils::hash::sha256_hex_of_file;
use crate::utils::io::{prompt_user, prompt_user_default_no};
use crate::utils::qemu::get_qemu_version;

#[derive(Args, Debug)]
pub struct SaveArgs {
    pub name: String,
    pub qemu_bin: String,
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub qemu_args: Vec<String>,
    #[arg(short = 'd', long = "desc")]
    pub desc: Option<String>,
    #[arg(short = 'f', long = "force")]
    pub force: bool,
    /// Bind an image resource as KEY=PATH (repeatable)
    #[arg(long = "image", value_parser = parse_resource_kv, value_name = "KEY=PATH")]
    pub images: Vec<(String, String)>,
    /// Bind a firmware resource as KEY=PATH (repeatable)
    #[arg(long = "firmware", value_parser = parse_resource_kv, value_name = "KEY=PATH")]
    pub firmwares: Vec<(String, String)>,
    /// Bind an "other" resource as KEY=PATH (repeatable)
    #[arg(long = "resource", value_parser = parse_resource_kv, value_name = "KEY=PATH")]
    pub resources: Vec<(String, String)>,
    /// Skip automatic SHA256 + size computation for resources
    #[arg(long = "no-checksum")]
    pub no_checksum: bool,
}

#[allow(clippy::too_many_arguments)]
pub fn save_command(
    force: bool,
    name: String,
    desc: Option<String>,
    qemu_bin: String,
    qemu_args: Vec<String>,
    images: Vec<(String, String)>,
    firmwares: Vec<(String, String)>,
    resources: Vec<(String, String)>,
    no_checksum: bool,
) -> VexResult<()> {
    validate_config_name(&name)?;
    let config_path = config_file(&name)?;

    let has_debug_args = qemu_args.iter().any(|arg| arg == "-s" || arg == "-S");

    let mut final_args = qemu_args.clone();

    if has_debug_args {
        println!("Debug parameters '-s' or '-S' detected in startup arguments");
        println!(
            "These parameters are used to start GDB debugging server, but saving them to configuration may not be the best practice."
        );
        println!(
            "Suggestion: Skip saving these parameters and use 'vex exec -d' to start remote debugging mode"
        );
        println!("Skip saving debug parameters and use exec -d for remote debugging? [Y/n]");

        if prompt_user()? {
            final_args = qemu_args
                .iter()
                .filter(|&arg| arg != "-s" && arg != "-S")
                .cloned()
                .collect();
            println!(
                "Debug parameters have been skipped, saved configuration will not include -s or -S parameters"
            );
            println!("To start debugging mode, use: vex exec -d {}", name);
        } else {
            println!("Debug parameters will be included in the saved configuration");
        }
    }

    let resources_map = build_resources(images, firmwares, resources, no_checksum)?;

    let qemu_version = get_qemu_version(&qemu_bin);

    if let Some(v) = &qemu_version {
        println!("Detected QEMU version: {}", v);
    }
    let config = QemuConfig {
        qemu_bin: qemu_bin.clone(),
        args: final_args,
        desc,
        qemu_version,
        resources: resources_map,
    };

    validate_config(&config)?;

    if config_path.exists() && !force {
        println!("Configuration '{}' already exists, overwrite? [y/N]", name);
        if !prompt_user_default_no()? {
            println!("Save cancelled");
            return Ok(());
        }
    }

    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| VexError::ConfigSerializeFailed { source: e })?;
    fs::write(&config_path, config_json).map_err(|e| VexError::IoError {
        path: config_path.clone(),
        operation: "save config file".to_string(),
        source: e,
    })?;

    if let Some(desc) = &config.desc {
        println!(
            "Configuration '{}' with description '{}' saved to {:?}",
            name, desc, config_path
        );
    } else {
        println!("Configuration '{}' saved to {:?}", name, config_path);
    }

    Ok(())
}

fn parse_resource_kv(s: &str) -> Result<(String, String), String> {
    let (k, v) = s
        .split_once('=')
        .ok_or_else(|| format!("expected KEY=PATH, got '{}'", s))?;
    if k.is_empty() {
        return Err("resource key cannot be empty".into());
    }
    if v.is_empty() {
        return Err("resource path cannot be empty".into());
    }
    Ok((k.to_string(), v.to_string()))
}

fn build_resources(
    images: Vec<(String, String)>,
    firmwares: Vec<(String, String)>,
    resources: Vec<(String, String)>,
    no_checksum: bool,
) -> VexResult<HashMap<String, ResourceRef>> {
    let triples: Vec<(String, String, ResourceKind)> = images
        .into_iter()
        .map(|(k, v)| (k, v, ResourceKind::Image))
        .chain(
            firmwares
                .into_iter()
                .map(|(k, v)| (k, v, ResourceKind::Firmware)),
        )
        .chain(
            resources
                .into_iter()
                .map(|(k, v)| (k, v, ResourceKind::Other)),
        )
        .collect();

    let mut seen: HashSet<String> = HashSet::new();
    for (k, _, _) in &triples {
        if !seen.insert(k.clone()) {
            return Err(VexError::ValidationError {
                field: Some(format!("resources[{}]", k)),
                reason: "resource key declared more than once".into(),
            });
        }
    }

    let mut map: HashMap<String, ResourceRef> = HashMap::new();
    for (key, path, kind) in triples {
        let (sha256, size) = if no_checksum {
            (None, None)
        } else {
            match fs::metadata(&path) {
                Ok(meta) => {
                    let s =
                        sha256_hex_of_file(Path::new(&path)).map_err(|e| VexError::IoError {
                            path: PathBuf::from(&path),
                            operation: "sha256 hash resource file".into(),
                            source: e,
                        })?;
                    (Some(s), Some(meta.len()))
                }
                Err(_) => {
                    eprintln!(
                        "warning: resource '{}' file not found at '{}', saved without checksum",
                        key, path
                    );
                    (None, None)
                }
            }
        };
        map.insert(
            key,
            ResourceRef {
                path,
                kind,
                sha256,
                size,
                url: None,
            },
        );
    }
    Ok(map)
}
