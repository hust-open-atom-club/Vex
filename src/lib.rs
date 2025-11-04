pub mod cli;
pub mod commands;
pub mod config;
pub mod error;
pub mod utils;

use anyhow::Result;
use clap::Parser;

use cli::Vex;
use commands::{exec_command, list_command, remove_command, rename_command, save_command};

/// Main application logic
pub fn run() -> Result<()> {
    let vex = Vex::parse();
    match vex {
        Vex::Exec(args) => exec_command(args.name, args.debug),
        Vex::List(_) => list_command(),
        Vex::Rm(args) => remove_command(args.name),
        Vex::Rename(args) => rename_command(args.desc, args.force, args.old_name, args.new_name),
        Vex::Save(args) => save_command(args.force, args.name, args.desc, args.qemu_bin, args.qemu_args),
    }

}
