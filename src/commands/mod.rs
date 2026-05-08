pub mod cache;
pub mod completions;
pub mod edit;
pub mod exec;
pub mod hub;
pub mod list;
pub mod print;
pub mod pull;
pub mod push;
pub mod remove;
pub mod rename;
pub mod resource;
pub mod save;

pub use cache::{
    CacheArgs, CacheCommands, cache_info_command, cache_list_command, cache_prune_command,
    cache_rm_command,
};
pub use completions::{CompletionsArgs, completions_command};
pub use edit::{EditArgs, edit_command};
pub use exec::{ExecArgs, exec_command};
pub use hub::{
    HubArgs, HubCommands, hub_info_command, hub_install_command, hub_list_command,
    hub_search_command,
};
pub use list::{ListArgs, list_command};
pub use print::{PrintArgs, print_command};
pub use pull::{PullArgs, pull_command};
pub use push::{PushArgs, push_command};
pub use remove::{RemoveArgs, remove_command};
pub use rename::{RenameArgs, rename_command};
pub use resource::{
    ResourceArgs, ResourceCommands, resource_add_command, resource_list_command,
    resource_rm_command,
};
pub use save::{SaveArgs, save_command};

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Commands {
    /// Save a new QEMU configuration
    Save(SaveArgs),

    /// Rename a saved configuration
    Rename(RenameArgs),

    /// Remove a saved configuration
    Rm(RemoveArgs),

    /// List all saved configurations
    List(ListArgs),

    /// Print details of a configuration
    Print(PrintArgs),

    /// Pull a shared configuration from the remote registry
    Pull(PullArgs),

    /// Push a local configuration to the remote registry
    Push(PushArgs),

    /// Execute a saved configuration
    Exec(ExecArgs),

    /// Generate shell completion scripts
    Completions(CompletionsArgs),

    /// Edit a saved configuration interactively
    Edit(EditArgs),

    /// Manage resources bound to a configuration
    Resource(ResourceArgs),

    /// Manage the resource cache
    Cache(CacheArgs),

    /// Browse and install entries from the Vex Hub
    Hub(HubArgs),
}

#[derive(Parser)]
#[command(name = "vex")]
#[command(author = "Vex Team")]
#[command(version)]
#[command(about = "A minimalist QEMU command-line manager", long_about = None)]
#[command(help_template = "\
{before-help}{name} {version}
{author-with-newline}{about-with-newline}
{usage-heading}
    {usage}

{all-args}{after-help}
")]
#[command(after_help = "ENVIRONMENT VARIABLES:
    VEX_CONFIG_DIR              Local config storage directory
    VEX_REMOTE_URL              Git remote registry URL (required for push/pull)
    VEX_REMOTE_BRANCH           Branch used for remote distribution (default: main)
    VEX_REMOTE_GIT_NAME         Git author name for vex push commits
    VEX_REMOTE_GIT_EMAIL        Git author email for vex push commits
    VEX_RESOURCE_CACHE_DIR      Resource cache directory
    VEX_HUB_URL                 Vex Hub base URL (default: https://hub.vex.example/, placeholder)

See README.md for details.
")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
