pub mod commands;
pub mod config;
pub mod error;
pub mod remote;
pub mod utils;

#[cfg(test)]
mod tests;

use anyhow::Result;
use clap::Parser;

use commands::{Cli, Commands};
use commands::{
    completions_command, edit_command, exec_command, list_command, print_command, pull_command,
    push_command, remove_command, rename_command, save_command,
};

pub fn run() -> Result<()> {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Exec(args) => exec_command(args.name, args.debug, args.full),
        Commands::List(_) => list_command(),
        Commands::Print(args) => print_command(args.name),
        Commands::Pull(args) => pull_command(args.force, args.remote_ref),
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
        ),
        Commands::Completions(args) => completions_command(args.shell),
    };
    result.map_err(|e| anyhow::anyhow!(e))
}
