use clap::Parser;

#[derive(Parser)]
#[clap(about = "Save QEMU configuration")]
pub struct SaveArgs {
    #[arg(help = "Configuration name for later reference")]
    pub name: String,

    #[arg(help = "Path to the QEMU executable (e.g., qemu-system-x86_64)")]
    pub qemu_bin: String,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true, help = "QEMU startup arguments")]
    pub qemu_args: Vec<String>,

    #[arg(short = 'd', long = "desc", help = "Optional description for the configuration")]
    pub desc: Option<String>,

    #[arg(short = 'f', long = "force", help = "Force save without confirmation if configuration exists")]
    pub force: bool,
}

#[derive(Parser)]
#[clap(about = "Rename a saved QEMU configuration")]
pub struct RenameArgs {
    #[arg(help = "Current configuration name")]
    pub old_name: String,

    #[arg(help = "New configuration name")]
    pub new_name: String,

    #[arg(short = 'd', long = "desc", help = "Update the configuration description")]
    pub desc: Option<String>,

    #[arg(short = 'f', long = "force", help = "Force rename without confirmation")]
    pub force: bool,
}

#[derive(Parser)]
#[clap(about = "Remove a saved QEMU configuration")]
pub struct RemoveArgs {
    #[arg(help = "Configuration name to remove")]
    pub name: String,
}

#[derive(Parser)]
#[clap(about = "List all saved QEMU configurations")]
pub struct ListArgs;

#[derive(Parser)]
#[clap(about = "Execute a saved QEMU configuration")]
pub struct ExecArgs {
    #[arg(help = "Configuration name to execute")]
    pub name: String,

    #[arg(short = 'd', long = "debug", help = "Start QEMU in debug mode (GDB server on port 1234)")]
    pub debug: bool,
}

#[derive(Parser)]
pub enum Vex {
    Save(SaveArgs),
    Rename(RenameArgs),
    Rm(RemoveArgs),
    List(ListArgs),
    Exec(ExecArgs),
}