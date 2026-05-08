use clap::Args;
use std::fs;
use std::path::PathBuf;

use crate::config::{config_file, resource_cache_dir, validate_config};
use crate::error::{VexError, VexResult};
use crate::remote::fetch::fetch_to_cache;
use crate::remote::{RemoteSpec, clone_remote_repo, load_published_config};
use crate::utils::io::prompt_user_default_no;

#[derive(Args, Debug)]
pub struct PullArgs {
    pub remote_ref: String,
    #[arg(short = 'f', long = "force")]
    pub force: bool,
    /// Download resource files referenced by url after pulling the config
    #[arg(long = "fetch-resources")]
    pub fetch_resources: bool,
    /// Override the default resource cache directory
    #[arg(long = "resource-dir", value_name = "DIR")]
    pub resource_dir: Option<PathBuf>,
}

pub fn pull_command(
    force: bool,
    remote_ref: String,
    fetch_resources: bool,
    resource_dir: Option<PathBuf>,
) -> VexResult<()> {
    let spec = RemoteSpec::parse(&remote_ref)?;
    let (_temp_dir, worktree) = clone_remote_repo()?;
    let mut published = load_published_config(&worktree, &spec)?;

    validate_config(&published.config)?;

    let config_path = config_file(&spec.name)?;
    if config_path.exists() && !force {
        println!(
            "Local configuration '{}' already exists, overwrite? [y/N]",
            spec.name
        );
        if !prompt_user_default_no()? {
            println!("Pull cancelled");
            return Ok(());
        }
    }

    let cache_dir = match resource_dir {
        Some(dir) => dir,
        None => resource_cache_dir()?,
    };

    let resource_keys: Vec<String> = published.config.resources.keys().cloned().collect();
    for key in resource_keys {
        let r = published.config.resources.get(&key).unwrap().clone();
        let url = match &r.url {
            Some(u) => u.clone(),
            None => {
                if fetch_resources {
                    eprintln!("warning: resource '{}' has no url, skipping fetch", key);
                }
                continue;
            }
        };
        if !fetch_resources {
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

    let config_json = serde_json::to_string_pretty(&published.config)
        .map_err(|e| VexError::ConfigSerializeFailed { source: e })?;
    fs::write(&config_path, config_json).map_err(|e| VexError::IoError {
        path: config_path.clone(),
        operation: "save pulled configuration".to_string(),
        source: e,
    })?;

    println!(
        "Pulled configuration '{} / {}:{}' into {:?}",
        spec.id, spec.name, published.tag, config_path
    );

    Ok(())
}
