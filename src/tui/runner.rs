use std::io;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Frame, Terminal, backend::CrosstermBackend};

use super::app::{App, AppMessage, MessageKind};
#[cfg(test)]
use super::event::AppEvent;
use super::event::translate_key;
use super::render;
use super::scan::{self, ConfigEntry};
use crate::error::{VexError, VexResult};

/// Tracks partial terminal initialisation so we can clean up if an
/// early `?` aborts setup before main_loop takes over.
struct TerminalGuard {
    raw_mode_enabled: bool,
    alt_screen_entered: bool,
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Best-effort: ignore all errors so Drop never panics.
        if self.alt_screen_entered {
            let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        }
        if self.raw_mode_enabled {
            let _ = disable_raw_mode();
        }
    }
}

pub fn run(exit_after_init: bool) -> VexResult<()> {
    install_panic_hook();

    // Scan before raw mode so failures stay visible on the normal terminal.
    let entries = scan::scan_configs()?;
    let mut app = App::new(entries);

    let mut guard = TerminalGuard {
        raw_mode_enabled: false,
        alt_screen_entered: false,
    };

    enable_raw_mode().map_err(io_to_vex)?;
    guard.raw_mode_enabled = true;

    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen).map_err(io_to_vex)?;
    guard.alt_screen_entered = true;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(io_to_vex)?;

    let result = main_loop(&mut terminal, &mut app, exit_after_init);

    // Successful path: keep the original P4-2 3-line best-effort cleanup
    // (matches its sequencing exactly), then sync guard flags so the
    // upcoming Drop is a no-op.
    let _ = disable_raw_mode();
    let _ = execute!(terminal.backend_mut(), LeaveAlternateScreen);
    let _ = terminal.show_cursor();
    guard.raw_mode_enabled = false;
    guard.alt_screen_entered = false;
    drop(guard);

    result
}

fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    exit_after_init: bool,
) -> VexResult<()> {
    // Initial draw.
    terminal.draw(|f| draw(f, app)).map_err(io_to_vex)?;
    if exit_after_init {
        return Ok(());
    }

    loop {
        // Consume any pending launch from the previous tick BEFORE drawing.
        if let Some(idx) = app.take_pending_launch() {
            if let Some(ConfigEntry::Ok { config, .. }) = app.entries.get(idx) {
                let config = config.clone();
                match launch_selected_config(terminal, &config) {
                    Ok(None) => {}
                    Ok(Some(msg)) => app.last_message = Some(msg),
                    Err(e) => return Err(e),
                }
            }
            // Force a redraw on the next iteration — fall through.
            terminal.draw(|f| draw(f, app)).map_err(io_to_vex)?;
            continue;
        }

        let mut needs_redraw = false;
        if event::poll(std::time::Duration::from_millis(250)).map_err(io_to_vex)? {
            if let Event::Key(key) = event::read().map_err(io_to_vex)? {
                let ev = translate_key(key, app);
                if app.handle_event(ev) {
                    needs_redraw = true;
                }
                if app.should_quit {
                    return Ok(());
                }
            }
        } else {
            needs_redraw = true;
        }

        if needs_redraw {
            terminal.draw(|f| draw(f, app)).map_err(io_to_vex)?;
        }
    }
}

fn launch_selected_config(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    config: &crate::config::QemuConfig,
) -> VexResult<Option<AppMessage>> {
    use crate::commands::exec::prepare_command;
    use crossterm::cursor::{Hide, Show};

    // Step 1: Pre-flight — runs while still in alt-screen. Failure here
    // MUST NOT switch terminals.
    let mut prepared = match prepare_command(config, false) {
        Ok(p) => p,
        Err(e) => {
            return Ok(Some(AppMessage {
                text: format!("Cannot launch: {}", e),
                kind: MessageKind::Error,
            }));
        }
    };

    // Step 2: Leave alt-screen — LeaveAlternateScreen + Show, then disable_raw_mode.
    let mut stdout = io::stdout();
    execute!(stdout, LeaveAlternateScreen, Show).map_err(io_to_vex)?;
    disable_raw_mode().map_err(io_to_vex)?;

    // Step 3: Run QEMU (blocking on the real terminal).
    let status_result = prepared.command.status();

    // Step 4: Re-enter alt-screen — reverse order. Failures here are fatal.
    enable_raw_mode().map_err(io_to_vex)?;
    execute!(stdout, EnterAlternateScreen, Hide).map_err(io_to_vex)?;
    terminal.clear().map_err(io_to_vex)?;

    // Step 5: Interpret status_result AFTER re-entering.
    match status_result {
        Ok(s) if s.success() => Ok(None),
        Ok(s) => Ok(Some(AppMessage {
            text: format!(
                "QEMU exited with code {}",
                s.code()
                    .map(|c| c.to_string())
                    .unwrap_or_else(|| "?".to_string())
            ),
            kind: MessageKind::Info,
        })),
        Err(e) => Ok(Some(AppMessage {
            text: format!("Failed to launch QEMU: {}", e),
            kind: MessageKind::Error,
        })),
    }
}

/// Thin forwarder; all rendering logic lives in `crate::tui::render`.
/// Kept here so existing L2 snapshot tests in `src/tests/test_tui.rs`
/// can keep using `crate::tui::runner::draw`.
pub(crate) fn draw(f: &mut Frame, app: &App) {
    render::draw(f, app);
}

/// Headless state-machine driver — feeds a sequence of events into the App
/// without touching the terminal. Used by L3-style tests that exercise
/// multi-event flows without requiring a TTY.
///
/// Note: pending_launch is observed but NOT executed — tests that need to
/// verify launch behavior should check app.pending_launch directly after
/// the call returns. Here we proactively drain it so a Launch event does
/// not survive across iterations and confuse subsequent events.
#[cfg(test)]
pub(crate) fn run_state_machine(
    app: &mut App,
    events: impl IntoIterator<Item = AppEvent>,
) -> VexResult<()> {
    for ev in events {
        app.handle_event(ev);
        if app.should_quit {
            return Ok(());
        }
        app.pending_launch = None;
    }
    Ok(())
}

fn install_panic_hook() {
    let original = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(std::io::stdout(), LeaveAlternateScreen);
        original(info);
    }));
}

fn io_to_vex(e: std::io::Error) -> VexError {
    VexError::IoError {
        path: std::path::PathBuf::from("<terminal>"),
        operation: "TUI terminal I/O".to_string(),
        source: e,
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalGuard;

    #[test]
    fn terminal_guard_drop_is_best_effort() {
        // The guard's Drop must never panic, even when both flags are set
        // and the cleanup syscalls fail (likely the case in a no-TTY test
        // environment). We don't assert that the terminal was actually
        // restored — that requires a real TTY — only that Drop returns
        // normally.
        let guard = TerminalGuard {
            raw_mode_enabled: true,
            alt_screen_entered: true,
        };
        drop(guard);
    }
}
