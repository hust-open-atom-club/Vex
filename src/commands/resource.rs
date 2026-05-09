use clap::{Args, Subcommand};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{
    ResourceKind, ResourceRef, config_file, load_config, validate_config, validate_resource_key,
};
use crate::error::{VexError, VexResult};
use crate::utils::hash::sha256_hex_of_file;
use crate::utils::io::prompt_user_default_no;

#[derive(Args, Debug)]
pub struct ResourceArgs {
    #[command(subcommand)]
    pub command: ResourceCommands,
}

#[derive(Subcommand, Debug)]
pub enum ResourceCommands {
    /// Add a resource binding to an existing configuration
    Add(ResourceAddArgs),
    /// Remove a resource binding from a configuration
    Rm(ResourceRmArgs),
    /// List resources of a configuration
    List(ResourceListArgs),
}

#[derive(Args, Debug)]
pub struct ResourceAddArgs {
    /// Configuration name
    pub config_name: String,
    /// Resource key (placeholder name used in args as ${res:KEY})
    pub key: String,
    /// File path to the resource
    pub path: String,
    /// Resource kind
    #[arg(long = "kind", value_enum, default_value_t = ResourceKindArg::Other)]
    pub kind: ResourceKindArg,
    /// Skip automatic SHA256 + size computation
    #[arg(long = "no-checksum")]
    pub no_checksum: bool,
    /// Allow adding even if the file does not exist locally
    #[arg(long = "allow-missing")]
    pub allow_missing: bool,
    /// Overwrite if KEY already exists in resources
    #[arg(short = 'f', long = "force")]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct ResourceRmArgs {
    pub config_name: String,
    pub key: String,
}

#[derive(Args, Debug)]
pub struct ResourceListArgs {
    pub config_name: String,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum ResourceKindArg {
    Image,
    Firmware,
    Other,
}

impl From<ResourceKindArg> for ResourceKind {
    fn from(v: ResourceKindArg) -> Self {
        match v {
            ResourceKindArg::Image => ResourceKind::Image,
            ResourceKindArg::Firmware => ResourceKind::Firmware,
            ResourceKindArg::Other => ResourceKind::Other,
        }
    }
}

pub fn resource_add_command(args: ResourceAddArgs) -> VexResult<()> {
    validate_resource_key(&args.key)?;

    let mut config = load_config(&args.config_name)?;
    let config_path = config_file(&args.config_name)?;

    if config.resources.contains_key(&args.key) && !args.force {
        println!(
            "Resource key '{}' already exists in '{}', overwrite? [y/N]",
            args.key, args.config_name
        );
        if !prompt_user_default_no()? {
            println!("Add cancelled");
            return Ok(());
        }
    }

    let path_obj = Path::new(&args.path);
    let exists = path_obj.exists();

    if !exists && !args.allow_missing {
        return Err(VexError::ResourceFileNotFound {
            key: args.key,
            path: PathBuf::from(&args.path),
        });
    }

    let (sha256, size) = if exists && !args.no_checksum {
        let s = sha256_hex_of_file(path_obj).map_err(|e| VexError::IoError {
            path: PathBuf::from(&args.path),
            operation: "sha256 hash resource file".into(),
            source: e,
        })?;
        let meta = fs::metadata(path_obj).map_err(|e| VexError::IoError {
            path: PathBuf::from(&args.path),
            operation: "read resource file metadata".into(),
            source: e,
        })?;
        (Some(s), Some(meta.len()))
    } else {
        if !exists {
            eprintln!(
                "warning: resource '{}' file not found at '{}', recorded without checksum",
                args.key, args.path
            );
        }
        (None, None)
    };

    config.resources.insert(
        args.key.clone(),
        ResourceRef {
            path: args.path,
            kind: args.kind.into(),
            sha256,
            size,
            url: None,
        },
    );

    validate_config(&config)?;

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| VexError::ConfigSerializeFailed { source: e })?;
    fs::write(&config_path, json).map_err(|e| VexError::IoError {
        path: config_path,
        operation: "save config file".into(),
        source: e,
    })?;

    println!("Resource '{}' added to '{}'", args.key, args.config_name);
    Ok(())
}

pub fn resource_rm_command(args: ResourceRmArgs) -> VexResult<()> {
    let mut config = load_config(&args.config_name)?;
    let config_path = config_file(&args.config_name)?;

    if config.resources.remove(&args.key).is_none() {
        return Err(VexError::ValidationError {
            field: Some(format!("resources[{}]", args.key)),
            reason: "resource key not found".into(),
        });
    }

    let json = serde_json::to_string_pretty(&config)
        .map_err(|e| VexError::ConfigSerializeFailed { source: e })?;
    fs::write(&config_path, json).map_err(|e| VexError::IoError {
        path: config_path,
        operation: "save config file".into(),
        source: e,
    })?;

    println!(
        "Resource '{}' removed from '{}'",
        args.key, args.config_name
    );
    Ok(())
}

pub fn resource_list_command(args: ResourceListArgs) -> VexResult<()> {
    let config = load_config(&args.config_name)?;

    if config.resources.is_empty() {
        println!("No resources bound to '{}'", args.config_name);
        return Ok(());
    }

    println!("Resources of '{}':", args.config_name);
    let mut keys: Vec<&String> = config.resources.keys().collect();
    keys.sort();
    for key in keys {
        let r = &config.resources[key];
        println!("  {}  [{:?}]", key, r.kind);
        println!("    path:   {}", r.path);
        println!("    sha256: {}", r.sha256.as_deref().unwrap_or("(none)"));
        println!(
            "    size:   {}",
            r.size
                .map(|s| format!("{} bytes", s))
                .unwrap_or_else(|| "(unknown)".into())
        );
    }
    Ok(())
}
