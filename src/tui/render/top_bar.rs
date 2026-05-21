use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph},
};

use crate::tui::app::{App, BrowseSubMode, is_builtin_snippet_name};
use crate::tui::scan::ConfigEntry;
use crate::tui::theme;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_style());
    let inner = block.inner(area);
    f.render_widget(block, area);

    let (badge_text, badge_style) = if let Some(lib) = &app.library {
        if lib.edit.is_some() {
            (" EDIT ", theme::badge_edit())
        } else {
            (" LIBRARY ", theme::badge_library())
        }
    } else if app.edit.is_some() {
        (" EDIT ", theme::badge_edit())
    } else {
        (" BROWSE ", theme::badge_browse())
    };
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

    let badge = Paragraph::new(badge_text).style(badge_style);
    f.render_widget(badge, cols[1]);
}

/// Build the inline "stats" run of spans shown after the brand.
fn stats_spans(app: &App) -> Vec<Span<'static>> {
    // Library mode replaces the entries stats with snippets stats.
    if let Some(lib) = &app.library {
        let bold = Style::default().add_modifier(Modifier::BOLD);
        let dim = theme::dim_style();
        let lib_color = Style::default()
            .fg(theme::BADGE_LIBRARY_BG)
            .add_modifier(Modifier::BOLD);

        // Editing sub-mode → focused title.
        if let Some(sedit) = &lib.edit {
            let title = match &sedit.mode {
                crate::tui::app::SnippetEditMode::Create => "Editing new snippet".to_string(),
                crate::tui::app::SnippetEditMode::Update { original_name } => {
                    format!("Editing snippet '{}'", original_name)
                }
            };
            return vec![Span::styled(title, dim)];
        }

        let total = lib.snippets.snippets.len();
        let builtin = lib
            .snippets
            .snippets
            .iter()
            .filter(|s| is_builtin_snippet_name(&s.name))
            .count();
        let user = total - builtin;
        return vec![
            Span::styled(total.to_string(), bold),
            Span::raw(" "),
            Span::styled("snippets", dim),
            Span::styled("  ·  ", dim),
            Span::styled(builtin.to_string(), bold),
            Span::raw(" "),
            Span::styled("builtin", dim),
            Span::styled("  ·  ", dim),
            Span::styled(user.to_string(), lib_color),
            Span::raw(" "),
            Span::styled("user", dim),
        ];
    }
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
