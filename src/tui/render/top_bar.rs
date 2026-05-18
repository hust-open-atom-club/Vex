use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::tui::app::{App, BrowseSubMode};
use crate::tui::scan::ConfigEntry;
use crate::tui::theme;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_style());
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Compose left half (brand + stats) and right half (mode badge).
    let badge_text = " BROWSE ";
    let badge_width = badge_text.chars().count() as u16;
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(badge_width)])
        .split(inner);

    let mut spans: Vec<Span<'static>> = vec![
        Span::raw("  "),
        Span::styled(
            "VEX",
            Style::default()
                .fg(theme::ACCENT)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(env!("CARGO_PKG_VERSION"), theme::dim_style()),
        Span::styled("      ·      ", theme::dim_style()),
    ];
    spans.extend(stats_spans(app));

    let para = Paragraph::new(Line::from(spans)).alignment(Alignment::Left);
    f.render_widget(para, cols[0]);

    let badge = Paragraph::new(badge_text).style(theme::badge_browse());
    f.render_widget(badge, cols[1]);
}

/// Build the inline "stats" run of spans shown after the brand.
fn stats_spans(app: &App) -> Vec<Span<'static>> {
    let total = app.entries.len();
    let ok = app
        .entries
        .iter()
        .filter(|e| matches!(e, ConfigEntry::Ok { .. }))
        .count();
    let broken = total - ok;

    let bold = Style::default().add_modifier(Modifier::BOLD);
    let dim = theme::dim_style();
    let bad = Style::default().fg(theme::BAD).add_modifier(Modifier::BOLD);

    // Filter-with-query: show matched/total counter instead of breakdown.
    if let BrowseSubMode::Filtering { query, .. } = &app.browse_sub
        && !query.is_empty()
    {
        let matched = app.visible_indices().len();
        return vec![
            Span::styled(matched.to_string(), bold),
            Span::styled("/", dim),
            Span::styled(total.to_string(), dim),
            Span::raw(" "),
            Span::styled("configurations matched", dim),
        ];
    }

    let mut spans = vec![
        Span::styled(total.to_string(), bold),
        Span::raw(" "),
        Span::styled("configurations", dim),
        Span::styled("  ·  ", dim),
        Span::styled(ok.to_string(), bold),
        Span::raw(" "),
        Span::styled("ok", dim),
        Span::styled("  ·  ", dim),
    ];
    if broken == 0 {
        spans.push(Span::styled(broken.to_string(), dim));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("broken", dim));
    } else {
        spans.push(Span::styled("⚠ ", bad));
        spans.push(Span::styled(broken.to_string(), bad));
        spans.push(Span::raw(" "));
        spans.push(Span::styled("broken", bad));
    }
    spans
}
