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
        Vex::Save {
            force,
            name,
            desc,
            qemu_bin,
            qemu_args,
        } => save_command(force, name, desc, qemu_bin, qemu_args),

        Vex::Exec { name, debug } => exec_command(name, debug),

        Vex::Rm { name } => remove_command(name),

        Vex::List => list_command(),

        Vex::Rename {
            desc,
            force,
            old_name,
            new_name,
        } => rename_command(desc, force, old_name, new_name),
    }
}
