use clap::{Args, Subcommand};
use std::fs;
use std::path::PathBuf;

use crate::config::{config_file, resource_cache_dir, validate_config, validate_config_name};
use crate::error::{VexError, VexResult};
use crate::hub::fetch::{fetch_index, fetch_published_config};
use crate::hub::types::HubEntryKind;
use crate::hub::{hub_base_url, parse_hub_spec, resolve_tag};
use crate::remote::fetch::fetch_to_cache;
use crate::utils::io::prompt_user_default_no;

#[derive(Args, Debug)]
pub struct HubArgs {
    #[command(subcommand)]
    pub command: HubCommands,
}

#[derive(Subcommand, Debug)]
pub enum HubCommands {
    /// Search hub entries by id/name/summary substring
    Search(HubSearchArgs),
    /// Show details of a hub entry
    Info(HubInfoArgs),
    /// Install a hub entry as a local configuration
    Install(HubInstallArgs),
    /// List all hub entries (optionally filtered by kind)
    List(HubListArgs),
}

#[derive(Args, Debug)]
pub struct HubSearchArgs {
    pub keyword: String,
}

#[derive(Args, Debug)]
pub struct HubInfoArgs {
    /// Format: `<id>/<name>[:<tag>]`
    pub spec: String,
}

#[derive(Args, Debug)]
pub struct HubInstallArgs {
    /// Format: `<id>/<name>[:<tag>]`
    pub spec: String,
    /// Override the local configuration name (default = `<name>`)
    #[arg(long = "as", value_name = "NAME")]
    pub local_name: Option<String>,
    /// Download resource files referenced by url after install
    #[arg(long = "fetch-resources")]
    pub fetch_resources: bool,
    /// Override the resource cache directory
    #[arg(long = "resource-dir", value_name = "DIR")]
    pub resource_dir: Option<PathBuf>,
    /// Overwrite existing local configuration without prompt
    #[arg(short = 'f', long = "force")]
    pub force: bool,
}

#[derive(Args, Debug)]
pub struct HubListArgs {
    /// Filter by entry kind
    #[arg(long = "kind", value_enum)]
    pub kind: Option<HubKindArg>,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
pub enum HubKindArg {
    Demo,
    Board,
    Firmware,
    Other,
}

impl HubKindArg {
    fn matches(self, k: &HubEntryKind) -> bool {
        matches!(
            (self, k),
            (HubKindArg::Demo, HubEntryKind::Demo)
                | (HubKindArg::Board, HubEntryKind::Board)
                | (HubKindArg::Firmware, HubEntryKind::Firmware)
                | (HubKindArg::Other, HubEntryKind::Other)
        )
    }
}

pub fn hub_search_command(args: HubSearchArgs) -> VexResult<()> {
    let base = hub_base_url();
    let index = fetch_index(&base)?;
    let needle = args.keyword.to_lowercase();
    let matches: Vec<_> = index
        .entries
        .iter()
        .filter(|e| {
            e.id.to_lowercase().contains(&needle)
                || e.name.to_lowercase().contains(&needle)
                || e.summary.to_lowercase().contains(&needle)
        })
        .collect();

    if matches.is_empty() {
        println!("No entries found matching '{}'", args.keyword);
        return Ok(());
    }

    println!(
        "Found {} entries matching '{}':\n",
        matches.len(),
        args.keyword
    );
    for e in matches {
        println!(
            "{}/{}  [{:?}]  (latest: {})",
            e.id, e.name, e.kind, e.latest_tag
        );
        println!("  {}", e.summary);
        println!("  tags: {}", e.tags.join(", "));
        println!("  updated: {}", e.updated_at);
        println!();
    }
    Ok(())
}

pub fn hub_info_command(args: HubInfoArgs) -> VexResult<()> {
    let (id, name, tag_opt) = parse_hub_spec(&args.spec)?;
    let base = hub_base_url();
    let tag = resolve_tag(&base, &id, &name, tag_opt.as_deref())?;
    let published = fetch_published_config(&base, &id, &name, &tag)?;
    let cfg = &published.config;

    println!("Hub entry: {}/{}:{}\n", id, name, tag);
    println!("Description: {}", cfg.desc.as_deref().unwrap_or("(none)"));
    println!("QEMU: {}", cfg.qemu_bin);
    println!("Args:");
    if cfg.args.is_empty() {
        println!("  (no arguments)");
    } else {
        for (i, a) in cfg.args.iter().enumerate() {
            println!("  [{}] {}", i, a);
        }
    }
    if !cfg.resources.is_empty() {
        println!("Resources:");
        let mut keys: Vec<&String> = cfg.resources.keys().collect();
        keys.sort();
        for key in keys {
            let r = &cfg.resources[key];
            println!(
                "  {}  [{:?}]  url: {}  sha256: {}",
                key,
                r.kind,
                r.url.as_deref().unwrap_or("(none)"),
                r.sha256.as_deref().unwrap_or("(none)")
            );
        }
    }
    Ok(())
}

pub fn hub_install_command(args: HubInstallArgs) -> VexResult<()> {
    let (id, name, tag_opt) = parse_hub_spec(&args.spec)?;
    let base = hub_base_url();
    let tag = resolve_tag(&base, &id, &name, tag_opt.as_deref())?;
    let mut published = fetch_published_config(&base, &id, &name, &tag)?;
    validate_config(&published.config)?;

    let local_name = args.local_name.unwrap_or_else(|| name.clone());
    validate_config_name(&local_name)?;
    let config_path = config_file(&local_name)?;
    if config_path.exists() && !args.force {
        println!(
            "Local configuration '{}' already exists, overwrite? [y/N]",
            local_name
        );
        if !prompt_user_default_no()? {
            println!("Install cancelled");
            return Ok(());
        }
    }

    let cache_dir = match args.resource_dir {
        Some(d) => d,
        None => resource_cache_dir()?,
    };

    let resource_keys: Vec<String> = published.config.resources.keys().cloned().collect();
    for key in resource_keys {
        let r = published.config.resources.get(&key).unwrap().clone();
        let url = match &r.url {
            Some(u) => u.clone(),
            None => {
                if args.fetch_resources {
                    eprintln!("warning: resource '{}' has no url, skipping fetch", key);
                }
                continue;
            }
        };
        if !args.fetch_resources {
            println!(
                "  resource '{}' available at: {}  (use --fetch-resources to download)",
                key, url
            );
            continue;
        }
        match fetch_to_cache(&url, r.sha256.as_deref(), &cache_dir) {
            Ok(local_path) => {
                if let Some(p) = local_path.to_str() {
                    if let Some(entry) = published.config.resources.get_mut(&key) {
                        entry.path = p.to_string();
                    }
                    println!("  fetched resource '{}' into {}", key, p);
                } else {
                    eprintln!(
                        "warning: cached resource '{}' has non-UTF8 path; original path retained",
                        key
                    );
                }
            }
            Err(e) => {
                eprintln!("warning: failed to fetch resource '{}': {}", key, e);
            }
        }
    }

    let json = serde_json::to_string_pretty(&published.config)
        .map_err(|e| VexError::ConfigSerializeFailed { source: e })?;
    fs::write(&config_path, json).map_err(|e| VexError::IoError {
        path: config_path.clone(),
        operation: "save installed configuration".into(),
        source: e,
    })?;

    println!("Installed '{}/{}:{}' as '{}'", id, name, tag, local_name);
    Ok(())
}

pub fn hub_list_command(args: HubListArgs) -> VexResult<()> {
    let base = hub_base_url();
    let index = fetch_index(&base)?;
    let entries: Vec<_> = index
        .entries
        .iter()
        .filter(|e| match &args.kind {
            Some(k) => k.matches(&e.kind),
            None => true,
        })
        .collect();

    for e in &entries {
        println!("{}/{}  [{:?}]  v{}", e.id, e.name, e.kind, e.latest_tag);
        println!("  {}", e.summary);
    }
    println!("Total: {} entries", entries.len());
    Ok(())
}
