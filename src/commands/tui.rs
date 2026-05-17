use clap::Args;

#[derive(Args, Debug)]
pub struct TuiArgs {
    /// Internal flag for e2e tests: initialise the TUI then immediately exit.
    /// Hidden from --help.
    #[arg(long = "exit-after-init", hide = true)]
    pub exit_after_init: bool,
}

pub fn tui_command(exit_after_init: bool) -> crate::error::VexResult<()> {
    crate::tui::runner::run(exit_after_init)
}
