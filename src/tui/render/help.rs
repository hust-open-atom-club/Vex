use ratatui::{
    Frame,
    layout::Rect,
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, Paragraph, Wrap},
};

use crate::tui::app::App;
use crate::tui::theme;

pub(super) fn render(f: &mut Frame, full: Rect, app: &App) {
    let w = full.width.min(64);
    let h = full.height.min(28);
    let x = full.x + full.width.saturating_sub(w) / 2;
    let y = full.y + full.height.saturating_sub(h) / 2;
    let area = Rect::new(x, y, w, h);

    f.render_widget(Clear, area);

    let title = if app.library.is_some() {
        " Help · Library mode "
    } else if app.edit.is_some() {
        " Help · Edit mode "
    } else {
        " Help · Browse mode "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::accent_style())
        .title(Span::styled(title, theme::accent_style()));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines = if app.library.is_some() {
        library_help_lines()
    } else if app.edit.is_some() {
        edit_help_lines()
    } else {
        browse_help_lines()
    };
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
}

fn library_help_lines() -> Vec<Line<'static>> {
    let accent = theme::accent_style();
    let dim = theme::dim_style();
    vec![
        Line::from(""),
        Line::from(Span::styled("  Library", accent)),
        Line::from("    Ctrl+L              exit library"),
        Line::from("    ↑ / ↓               navigate"),
        Line::from("    Space               toggle category collapse"),
        Line::from("    /                   filter"),
        Line::from("    n                   create new snippet"),
        Line::from("    e                   edit selected user snippet"),
        Line::from("    d                   delete selected user snippet"),
        Line::from("    Esc                 same as Ctrl+L"),
        Line::from(""),
        Line::from(Span::styled("  Snippet Editing", accent)),
        Line::from("    ↑ / ↓               field cycle"),
        Line::from("    ← / →               cursor (text) or change category"),
        Line::from("    Enter               edit selected arg token (in Args)"),
        Line::from("    a                   add empty arg (in Args)"),
        Line::from("    J / K               reorder args"),
        Line::from("    d                   delete selected arg"),
        Line::from("    Ctrl+S              save and close edit"),
        Line::from("    Esc                 cancel (confirms if unsaved)"),
        Line::from(""),
        Line::from(Span::styled("  Token edit", accent)),
        Line::from("    Enter / Esc         commit token, exit token edit"),
        Line::from("    ← / →               cursor"),
        Line::from("    Backspace           delete char"),
        Line::from(""),
        Line::from(Span::styled("  Exit", accent)),
        Line::from("    Ctrl+C              always quit"),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close this help.", dim)),
    ]
}

fn browse_help_lines() -> Vec<Line<'static>> {
    let accent = theme::accent_style();
    let dim = theme::dim_style();
    vec![
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
        Line::from("    e        edit selected configuration"),
        Line::from("    n        create new configuration"),
        Line::from("    Ctrl+L   open snippets library"),
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
    ]
}

fn edit_help_lines() -> Vec<Line<'static>> {
    let accent = theme::accent_style();
    let dim = theme::dim_style();
    vec![
        Line::from(""),
        Line::from(Span::styled("  Navigation", accent)),
        Line::from("    Tab / Shift+Tab   switch pane (Editor ↔ Snippets)"),
        Line::from("    ↑ / ↓             field cycle (Editor) / row nav (Snippets)"),
        Line::from("    ← / →             cursor move (text fields)"),
        Line::from("    Space             toggle category collapse (Snippets)"),
        Line::from("    /                 enter filter (Snippets)"),
        Line::from(""),
        Line::from(Span::styled("  Actions", accent)),
        Line::from("    →                 insert snippet into args (Snippets)"),
        Line::from("    a                 add empty arg + edit (Args field)"),
        Line::from("    Enter             edit selected arg token (Args field)"),
        Line::from("    J / K             reorder args down/up (Args field)"),
        Line::from("    d                 delete current arg (Args field)"),
        Line::from("    Ctrl+S            save and exit"),
        Line::from("    Esc               cancel (confirms if unsaved)"),
        Line::from(""),
        Line::from(Span::styled("  Token edit", accent)),
        Line::from("    Enter / Esc       commit token, exit token edit"),
        Line::from("    ← / →             cursor"),
        Line::from("    Backspace         delete char"),
        Line::from(""),
        Line::from(Span::styled("  Exit", accent)),
        Line::from("    Ctrl+C            always quit"),
        Line::from(""),
        Line::from(Span::styled("  Press any key to close this help.", dim)),
    ]
}
