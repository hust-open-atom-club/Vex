use clap::Args;
use std::fs;

use crate::config::{config_file, validate_config};
use crate::error::{VexError, VexResult};
use crate::remote::{RemoteSpec, clone_remote_repo, load_published_config};
use crate::utils::io::prompt_user_default_no;

#[derive(Args, Debug)]
pub struct PullArgs {
    pub remote_ref: String,
    #[arg(short = 'f', long = "force")]
    pub force: bool,
}

pub fn pull_command(force: bool, remote_ref: String) -> VexResult<()> {
    let spec = RemoteSpec::parse(&remote_ref)?;
    let (_temp_dir, worktree) = clone_remote_repo()?;
    let published = load_published_config(&worktree, &spec)?;

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
