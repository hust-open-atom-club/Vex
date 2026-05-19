use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::tui::app::{App, BrowseSubMode, Focus};
use crate::tui::scan::ConfigEntry;
use crate::tui::theme;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) {
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
        } if !query.is_empty() => format!("Configurations · /{}", query),
        BrowseSubMode::Filtering { accepted: true, .. } => "Configurations".to_string(),
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Span::styled(title_text, title_style));
    let inner = block.inner(area);
    f.render_widget(block, area);

    // Top/bottom padding row.
    let padded = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(inner);
    let body = padded[1];

    // Carve out the filter input + separator when actively editing.
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
            .split(body);
        (Some((rows[0], rows[1])), rows[2])
    } else {
        (None, body)
    };

    if let Some((row0, row1)) = input_area
        && let BrowseSubMode::Filtering { query, .. } = &app.browse_sub
    {
        let line = Line::from(vec![
            Span::styled(" / ", theme::accent_style()),
            Span::raw(query.clone()),
            Span::styled("█", theme::accent_style()),
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

    let selected_pos = visible.iter().position(|&i| i == app.selected);
    let list = List::new(items)
        .highlight_style(highlight)
        .highlight_symbol("▸ ");
    let mut state = ListState::default().with_selected(selected_pos);
    f.render_stateful_widget(list, list_area, &mut state);
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
