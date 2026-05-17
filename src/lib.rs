//! Vex — a minimalist QEMU command-line manager.
//!
//! Vex saves QEMU startup arguments as named configurations, lets you launch
//! them with a single `vex exec <name>`, and supports sharing configurations
//! either via Git remotes (`vex push` / `vex pull`) or via the read-only
//! Vex Hub HTTP protocol (`vex hub install`).
//!
//! # Modules
//!
//! - [`commands`] — Subcommand argument parsers and entry points.
//! - [`config`] — [`QemuConfig`](crate::config::QemuConfig) data model,
//!   on-disk storage, validation.
//! - [`remote`] — Git-backed registry: clone / fetch / push / fetch resources.
//! - [`hub`] — Vex Hub HTTP client (read-only).
//! - [`utils`] — Shared helpers (hashing, prompt I/O, QEMU version probe).
//! - [`error`] — Crate-wide error type [`VexError`](crate::error::VexError).
//!
//! # Minimal example
//!
//! ```no_run
//! // Programmatic use is not the primary surface; see the `vex` binary
//! // for the supported entry point.
//! vex::run().unwrap();
//! ```
//!
//! See `README.md` and `docs/HUB_PROTOCOL.md` for user-facing documentation.

pub mod commands;
pub mod config;
pub mod error;
pub mod hub;
pub mod remote;
pub mod tui;
pub mod utils;

#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::Parser;

use commands::{CacheCommands, Cli, Commands, HubCommands, ResourceCommands};
use commands::{
    cache_info_command, cache_list_command, cache_prune_command, cache_rm_command,
    completions_command, edit_command, exec_command, hub_info_command, hub_install_command,
    hub_list_command, hub_search_command, list_command, print_command, pull_command, push_command,
    remove_command, rename_command, resource_add_command, resource_list_command,
    resource_rm_command, save_command, tui_command,
};

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Exec(args) => exec_command(args.name, args.debug, args.full),
        Commands::List(_) => list_command(),
        Commands::Print(args) => print_command(args.name),
        Commands::Pull(args) => pull_command(
            args.force,
            args.remote_ref,
            args.fetch_resources,
            args.resource_dir,
        ),
        Commands::Push(args) => push_command(args.force, args.remote_ref, args.local_name),
        Commands::Rm(args) => remove_command(args.name),
        Commands::Edit(args) => edit_command(args.name),
        Commands::Rename(args) => {
            rename_command(args.desc, args.force, args.old_name, args.new_name)
        }
        Commands::Save(args) => save_command(
            args.force,
            args.name,
            args.desc,
            args.qemu_bin,
            args.qemu_args,
            args.images,
            args.firmwares,
            args.resources,
            args.no_checksum,
        ),
        Commands::Completions(args) => completions_command(args.shell),
        Commands::Resource(args) => match args.command {
            ResourceCommands::Add(a) => resource_add_command(a),
            ResourceCommands::Rm(a) => resource_rm_command(a),
            ResourceCommands::List(a) => resource_list_command(a),
        },
        Commands::Cache(args) => match args.command {
            CacheCommands::List(a) => cache_list_command(a),
            CacheCommands::Info(a) => cache_info_command(a),
            CacheCommands::Rm(a) => cache_rm_command(a),
            CacheCommands::Prune(a) => cache_prune_command(a),
        },
        Commands::Hub(args) => match args.command {
            HubCommands::Search(a) => hub_search_command(a),
            HubCommands::Info(a) => hub_info_command(a),
            HubCommands::Install(a) => hub_install_command(a),
            HubCommands::List(a) => hub_list_command(a),
        },
        Commands::Tui(args) => tui_command(args.exit_after_init),
    };
    result.map_err(|e| anyhow::anyhow!(e))
}
