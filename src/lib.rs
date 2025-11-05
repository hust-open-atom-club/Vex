pub mod commands;
pub mod config;
pub mod error;
pub mod utils;

use anyhow::Result;
use clap::Parser;

use commands::{Cli, Commands};
use commands::{exec_command, list_command, remove_command, rename_command, save_command};

/// Main application logic
pub fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Exec(args) => exec_command(args.name, args.debug, args.full),
        Commands::List(_) => list_command(),
        Commands::Rm(args) => remove_command(args.name),
        Commands::Rename(args) => rename_command(args.desc, args.force, args.old_name, args.new_name),
        Commands::Save(args) => save_command(args.force, args.name, args.desc, args.qemu_bin, args.qemu_args),
    }

}
