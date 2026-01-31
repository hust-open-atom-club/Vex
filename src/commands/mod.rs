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

    /// Execute a saved configuration
    Exec(ExecArgs),

    /// Generate shell completion scripts
    Completions(CompletionsArgs),
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
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}
