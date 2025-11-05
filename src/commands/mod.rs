pub mod exec;
pub mod list;
pub mod remove;
pub mod rename;
pub mod save;

pub use exec::{exec_command, ExecArgs};
pub use list::{list_command, ListArgs};
pub use remove::{remove_command, RemoveArgs};
pub use rename::{rename_command, RenameArgs};
pub use save::{save_command, SaveArgs};

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Commands {
    Save(SaveArgs),
    Rename(RenameArgs),
    Rm(RemoveArgs),
    List(ListArgs),
    Exec(ExecArgs),
}

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}