use clap::Parser;

#[derive(Parser)]
#[clap(name = "save", about = "Save QEMU configuration")]
pub struct SaveArgs {
    #[arg(short = 'f', long = "force")]
    pub force: bool,

    pub name: String,

    #[arg(short = 'd', long = "desc")]
    pub desc: Option<String>,

    pub qemu_bin: String,

    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    pub qemu_args: Vec<String>,
}

#[derive(Parser)]
pub struct RenameArgs {
    #[arg(short = 'd', long = "desc")]
    pub desc: Option<String>,

    #[arg(short = 'f', long = "force")]
    pub force: bool,

    pub old_name: String,
    pub new_name: String,
}

#[derive(Parser)]
pub struct RemoveArgs {
    pub name: String,
}

#[derive(Parser)]
pub struct ListArgs;

#[derive(Parser)]
pub struct ExecArgs {
    pub name: String,

    #[arg(short = 'd', long = "debug")]
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