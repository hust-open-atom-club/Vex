use std::io;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph, Wrap},
};

use super::app::{App, AppMessage, BrowseSubMode, Focus, MessageKind};
#[cfg(test)]
use super::event::AppEvent;
use super::event::translate_key;
use super::scan::{self, ConfigEntry};
use super::theme;
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

pub(crate) fn draw(f: &mut Frame, app: &App) {
    if app.is_empty() {
        render_empty_screen(f);
        if app.show_help {
            render_help_overlay(f, app);
        }
        return;
    }

    let area = f.area();
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    render_top_bar(f, chunks[0]);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(chunks[1]);

    render_left(f, cols[0], app);
    render_right(f, cols[1], app);

    render_status_bar(f, chunks[2], app);

    if app.show_help {
        render_help_overlay(f, app);
    }
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

fn render_empty_screen(f: &mut Frame) {
    let area = f.area();
    let outer = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::border_style());
    let inner = outer.inner(area);
    f.render_widget(outer, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);

    render_top_bar(f, rows[0]);

    if rows[1].height >= 2 {
        let center = rows[1];
        let top_pad = center.height.saturating_sub(2) / 2;
        let center_rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(top_pad),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(center);

        let main = Paragraph::new("No configurations")
            .style(theme::dim_style())
            .alignment(Alignment::Center);
        let sub = Paragraph::new("Run `vex save` to create one")
            .style(theme::dim_style())
            .alignment(Alignment::Center);
        f.render_widget(main, center_rows[1]);
        f.render_widget(sub, center_rows[2]);
    }

    let bar = Paragraph::new(" q quit")
        .style(theme::dim_style())
        .alignment(Alignment::Left);
    f.render_widget(bar, rows[2]);
}

fn render_top_bar(f: &mut Frame, area: Rect) {
    let title = format!(" vex {} ", env!("CARGO_PKG_VERSION"));
    let badge_text = " browse ";
    let badge_width = badge_text.chars().count() as u16;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(badge_width)])
        .split(area);
    let left = Paragraph::new(title)
        .style(theme::dim_style())
        .alignment(Alignment::Left);
    let badge = Paragraph::new(badge_text).style(theme::badge_browse());
    f.render_widget(left, cols[0]);
    f.render_widget(badge, cols[1]);
}

fn render_left(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Left;
    let border_style = if focused {
        theme::accent_style()
    } else {
        theme::border_style()
    };
    let title_style = if focused {
        theme::accent_style()
    } else {
        theme::dim_style()
    };
    let title_text = match &app.browse_sub {
        BrowseSubMode::Idle => "Configurations".to_string(),
        BrowseSubMode::Filtering {
            accepted: false, ..
        } => "Configurations · filter".to_string(),
        BrowseSubMode::Filtering {
            accepted: true,
            query,
        } if !query.is_empty() => {
            format!("Configurations · /{}", query)
        }
        BrowseSubMode::Filtering { accepted: true, .. } => "Configurations".to_string(),
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(title_text, title_style));

    let inner = block.inner(area);
    f.render_widget(block, area);

    // If we're actively editing the filter, carve out a 2-row input area.
    let (input_area, list_area) = if matches!(
        &app.browse_sub,
        BrowseSubMode::Filtering {
            accepted: false,
            ..
        }
    ) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
            ])
            .split(inner);
        (Some((rows[0], rows[1])), rows[2])
    } else {
        (None, inner)
    };

    if let Some((row0, row1)) = input_area
        && let BrowseSubMode::Filtering { query, .. } = &app.browse_sub
    {
        let line = Line::from(vec![
            Span::styled("/ ", theme::accent_style()),
            Span::raw(query.clone()),
            Span::styled("▏", theme::accent_style()),
        ]);
        f.render_widget(Paragraph::new(line), row0);
        let sep: String = "─".repeat(row1.width as usize);
        f.render_widget(Paragraph::new(sep).style(theme::dim_style()), row1);
    }

    let visible = app.visible_indices();
    let items: Vec<ListItem> = visible
        .iter()
        .map(|&i| {
            let e = &app.entries[i];
            match e {
                ConfigEntry::Ok { name, config, .. } => {
                    let desc_text = match &config.desc {
                        Some(d) => truncate(d, 30),
                        None => "(no description)".to_string(),
                    };
                    ListItem::new(Line::from(vec![
                        Span::raw(name.clone()),
                        Span::raw("  "),
                        Span::styled(desc_text, theme::dim_style()),
                    ]))
                }
                ConfigEntry::Broken { name, .. } => {
                    let style = Style::default().fg(theme::BAD);
                    ListItem::new(Line::from(vec![
                        Span::styled(name.clone(), style),
                        Span::raw("  "),
                        Span::styled("<broken>", style),
                    ]))
                }
            }
        })
        .collect();

    let highlight = Style::default()
        .bg(ratatui::style::Color::Rgb(20, 60, 130))
        .add_modifier(Modifier::BOLD);

    // The ListState selection is the position within the visible slice.
    let selected_pos = visible.iter().position(|&i| i == app.selected);

    let list = List::new(items)
        .highlight_style(highlight)
        .highlight_symbol("▸ ");

    let mut state = ListState::default().with_selected(selected_pos);
    f.render_stateful_widget(list, list_area, &mut state);
}

fn render_right(f: &mut Frame, area: Rect, app: &App) {
    let focused = app.focus == Focus::Right;
    let border_style = if focused {
        theme::accent_style()
    } else {
        theme::border_style()
    };
    let title_text = app
        .current()
        .map(|e| e.name().to_string())
        .unwrap_or_default();
    let title_style = if focused {
        theme::accent_style()
    } else {
        theme::dim_style()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(border_style)
        .title(Span::styled(title_text, title_style));

    let lines: Vec<Line> = match app.current() {
        Some(ConfigEntry::Broken { path, error, .. }) => render_broken_lines(path, error),
        Some(ConfigEntry::Ok { config, path, .. }) => render_ok_lines(config, path),
        None => Vec::new(),
    };

    let content_lines = lines.len() as u16;
    let visible = area.height.saturating_sub(2);
    let max_scroll = content_lines.saturating_sub(visible);
    let scroll = app.right_scroll.min(max_scroll);

    let para = Paragraph::new(lines)
        .block(block)
        .wrap(Wrap { trim: false })
        .scroll((scroll, 0));
    f.render_widget(para, area);
}

fn render_broken_lines(path: &std::path::Path, error: &str) -> Vec<Line<'static>> {
    let bad = Style::default().fg(theme::BAD).add_modifier(Modifier::BOLD);
    vec![
        Line::from(Span::styled("⚠ Broken configuration", bad)),
        Line::from(""),
        Line::from(vec![
            Span::styled("File: ", theme::dim_style()),
            Span::raw(path.display().to_string()),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Error: ", theme::dim_style()),
            Span::raw(error.to_string()),
        ]),
    ]
}

fn render_ok_lines(
    config: &crate::config::QemuConfig,
    path: &std::path::Path,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    lines.push(Line::from(Span::styled("Description", theme::dim_style())));
    match &config.desc {
        Some(d) => lines.push(Line::from(vec![Span::raw("  "), Span::raw(d.clone())])),
        None => lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("(none)", theme::dim_style()),
        ])),
    }
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("QEMU Binary", theme::dim_style())));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::raw(config.qemu_bin.clone()),
    ]));
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("QEMU Version", theme::dim_style())));
    match &config.qemu_version {
        Some(v) => lines.push(Line::from(vec![Span::raw("  "), Span::raw(v.clone())])),
        None => lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("(unknown)", theme::dim_style()),
        ])),
    }
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("Args", theme::dim_style())));
    if config.args.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("(none)", theme::dim_style()),
        ]));
    } else {
        for (i, a) in config.args.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(format!("[{}] ", i), theme::dim_style()),
                Span::raw(a.clone()),
            ]));
        }
    }
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("Resources", theme::dim_style())));
    if config.resources.is_empty() {
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::styled("(none)", theme::dim_style()),
        ]));
    } else {
        let mut keys: Vec<&String> = config.resources.keys().collect();
        keys.sort();
        for k in keys {
            let r = &config.resources[k];
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::raw(k.clone()),
                Span::raw(" → "),
                Span::raw(r.path.clone()),
            ]));
            let sha_span = match &r.sha256 {
                Some(_) => Span::styled("sha256 ✓", Style::default().fg(theme::GOOD)),
                None => Span::styled("sha256 (none)", theme::dim_style()),
            };
            lines.push(Line::from(vec![
                Span::raw("     ["),
                Span::raw(format!("{:?}", r.kind)),
                Span::raw(", "),
                sha_span,
                Span::raw("]"),
            ]));
        }
    }
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled("File", theme::dim_style())));
    lines.push(Line::from(vec![
        Span::raw("  "),
        Span::raw(path.display().to_string()),
    ]));

    lines
}

fn render_status_bar(f: &mut Frame, area: Rect, app: &App) {
    if let Some(msg) = &app.last_message {
        let base = match msg.kind {
            MessageKind::Error => Style::default().fg(theme::BAD).add_modifier(Modifier::BOLD),
            MessageKind::Info => Style::default().fg(theme::WARN),
        };
        let line = Line::from(vec![
            Span::styled(format!(" {}  ", msg.text), base),
            Span::styled("(press any key to dismiss)", theme::dim_style()),
        ]);
        let p = Paragraph::new(line).alignment(Alignment::Left);
        f.render_widget(p, area);
        return;
    }

    let hint_text = match &app.browse_sub {
        BrowseSubMode::Idle => {
            " q quit  Tab focus  j/k nav  g/G top/bot  / filter  ? help  r refresh"
        }
        BrowseSubMode::Filtering {
            accepted: false, ..
        } => " Enter accept  Esc cancel  Backspace edit  ↑↓ nav",
        BrowseSubMode::Filtering { accepted: true, .. } => {
            " q quit  / edit filter  Esc clear  ? help  r refresh"
        }
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(20)])
        .split(area);

    let hint = Paragraph::new(hint_text)
        .style(theme::dim_style())
        .alignment(Alignment::Left);
    f.render_widget(hint, cols[0]);

    let broken_count = app
        .entries
        .iter()
        .filter(|e| matches!(e, ConfigEntry::Broken { .. }))
        .count();
    if broken_count > 0 {
        let text = format!("⚠ {} broken ", broken_count);
        let p = Paragraph::new(text)
            .style(Style::default().fg(theme::BAD))
            .alignment(Alignment::Right);
        f.render_widget(p, cols[1]);
    }
}

fn render_help_overlay(f: &mut Frame, _app: &App) {
    let full = f.area();
    let w = full.width.clamp(40, 64);
    let h = full.height.clamp(12, 24);
    let x = full.x + (full.width.saturating_sub(w)) / 2;
    let y = full.y + (full.height.saturating_sub(h)) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(theme::accent_style())
        .title(Span::styled(" Help · Browse mode ", theme::accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let accent = theme::accent_style();
    let dim = theme::dim_style();
    let lines: Vec<Line<'static>> = vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", accent)),
        Line::from("    j  ↓     move down"),
        Line::from("    k  ↑     move up"),
        Line::from("    g        go to first"),
        Line::from("    G        go to last"),
        Line::from("    Tab      switch focus L / R"),
        Line::from(""),
        Line::from(Span::styled("  Actions", accent)),
        Line::from("    Enter    launch selected QEMU"),
        Line::from("    /        enter filter mode"),
        Line::from("    r        reload configurations"),
        Line::from("    ?        toggle this help"),
        Line::from(""),
        Line::from(Span::styled("  Filter mode", accent)),
        Line::from("    Enter    accept filter"),
        Line::from("    Esc      cancel filter"),
        Line::from("    Backspace  edit query"),
        Line::from(""),
        Line::from(Span::styled("  Exit", accent)),
        Line::from("    q  Esc   quit Vex TUI"),
        Line::from("    Ctrl+C   force quit"),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close this help.", dim)),
    ];

    let p = Paragraph::new(lines).wrap(Wrap { trim: false });
    f.render_widget(p, inner);
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
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
