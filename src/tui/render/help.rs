use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::tui::app::App;
use crate::tui::theme;

pub(super) fn render(f: &mut Frame, full: Rect, _app: &App) {
    // Only cap the upper bound. Skipping the lower bound prevents
    // producing a Rect that exceeds the frame on tiny terminals — ratatui
    // would panic inside `render_widget` when the rect overflows.
    let w = full.width.min(64);
    let h = full.height.min(24);
    let x = full.x + full.width.saturating_sub(w) / 2;
    let y = full.y + full.height.saturating_sub(h) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
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
