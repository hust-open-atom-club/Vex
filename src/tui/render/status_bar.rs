use ratatui::{
    Frame,
    layout::{Alignment, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::app::{App, BrowseSubMode, MessageKind};
use crate::tui::theme;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) {
    if let Some(msg) = &app.last_message {
        let base = match msg.kind {
            MessageKind::Error => Style::default().fg(theme::BAD).add_modifier(Modifier::BOLD),
            MessageKind::Info => Style::default().fg(theme::WARN),
        };
        let line = Line::from(vec![
            Span::styled(format!(" {}  ", msg.text), base),
            Span::styled("(press any key to dismiss)", theme::dim_style()),
        ]);
        f.render_widget(Paragraph::new(line).alignment(Alignment::Left), area);
        return;
    }

    if app.show_help {
        let line = bracket_hints(&[("any key", "dismiss help")]);
        f.render_widget(Paragraph::new(line).alignment(Alignment::Left), area);
        return;
    }

    let line = match &app.browse_sub {
        BrowseSubMode::Idle => bracket_hints(&[
            ("q", "quit"),
            ("Tab", "focus"),
            ("j/k", "nav"),
            ("Enter", "launch"),
            ("/", "filter"),
            ("?", "help"),
            ("r", "refresh"),
        ]),
        BrowseSubMode::Filtering {
            accepted: false, ..
        } => bracket_hints(&[
            ("Enter", "accept"),
            ("Esc", "cancel"),
            ("Backspace", "edit"),
            ("↑↓", "nav"),
        ]),
        BrowseSubMode::Filtering { accepted: true, .. } => bracket_hints(&[
            ("q", "quit"),
            ("/", "edit filter"),
            ("Esc", "clear"),
            ("?", "help"),
            ("r", "refresh"),
        ]),
    };
    f.render_widget(Paragraph::new(line).alignment(Alignment::Left), area);
}

fn bracket_hints(items: &[(&str, &str)]) -> Line<'static> {
    let dim = theme::dim_style();
    let fg = Style::default().fg(theme::FG);
    let mut spans: Vec<Span<'static>> = vec![Span::raw(" ")];
    for (i, (key, label)) in items.iter().enumerate() {
        if i > 0 {
            spans.push(Span::raw("   "));
        }
        spans.push(Span::styled("[", dim));
        spans.push(Span::styled((*key).to_string(), fg));
        spans.push(Span::styled("]", dim));
        spans.push(Span::raw(" "));
        spans.push(Span::styled((*label).to_string(), dim));
    }
    Line::from(spans)
}
