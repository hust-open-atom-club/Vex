pub mod completions;
pub mod exec;
pub mod list;
pub mod print;
pub mod remove;
pub mod rename;
pub mod save;

pub use completions::{CompletionsArgs, completions_command};
pub use exec::{ExecArgs, exec_command};
pub use list::{ListArgs, list_command};
pub use print::{PrintArgs, print_command};
pub use remove::{RemoveArgs, remove_command};
pub use rename::{RenameArgs, rename_command};
pub use save::{SaveArgs, save_command};

use clap::{Parser, Subcommand};

#[derive(Subcommand)]
pub enum Commands {
    Save(SaveArgs),
    Rename(RenameArgs),
    Rm(RemoveArgs),
    List(ListArgs),
    Print(PrintArgs),
    Exec(ExecArgs),
    Completions(CompletionsArgs),
}

#[derive(Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
