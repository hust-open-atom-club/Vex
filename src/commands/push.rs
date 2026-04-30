use clap::Args;

use crate::config::{load_config, validate_config};
use crate::error::VexResult;
use crate::remote::{PublishOutcome, RemoteSpec, publish_config};

#[derive(Args, Debug)]
pub struct PushArgs {
    pub remote_ref: String,
    pub local_name: String,
    #[arg(short = 'f', long = "force")]
    pub force: bool,
}

pub fn push_command(force: bool, remote_ref: String, local_name: String) -> VexResult<()> {
    let spec = RemoteSpec::parse(&remote_ref)?;
    let config = load_config(&local_name)?;
    validate_config(&config)?;

    match publish_config(&spec, &config, force)? {
        PublishOutcome::Cancelled => {}
        PublishOutcome::NoChanges => {
            println!(
                "Remote configuration '{} / {}:{}' is already up to date",
                spec.id,
                spec.name,
                spec.resolved_tag()
            );
        }
        PublishOutcome::Pushed => {
            println!(
                "Pushed local configuration '{}' to '{} / {}:{}'",
                local_name,
                spec.id,
                spec.name,
                spec.resolved_tag()
            );
        }
    }

    Ok(())
}
