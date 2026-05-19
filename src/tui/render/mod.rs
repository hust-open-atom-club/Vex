//! Visual rendering for the TUI. Split into one submodule per pane / overlay.
//! All public entry is the [`draw`] dispatcher; submodule render fns are
//! `pub(super)` and not visible outside this module tree.

mod empty;
mod help;
mod left_pane;
mod right_pane;
mod status_bar;
mod top_bar;

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use super::app::App;
use super::theme;

pub(crate) fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

    top_bar::render(f, chunks[0], app);

    if app.is_empty() {
        empty::render_middle(f, chunks[1], app);
    } else if app.visible_indices().is_empty() {
        // Filter is active but no entry matches. Keep the left pane (so the
        // filter input and empty list stay visible) and replace the right
        // pane with a hint instead of showing details of an entry the user
        // can't actually see.
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(chunks[1]);
        left_pane::render(f, panes[0], app);
        render_no_match_message(f, panes[1]);
    } else {
        let panes = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(chunks[1]);
        left_pane::render(f, panes[0], app);
        // Mutable borrow: right_pane clamps app.right_scroll content-aware.
        right_pane::render(f, panes[1], app);
    }

    status_bar::render(f, chunks[2], app);

    if app.show_help {
        help::render(f, area, app);
    }
}

fn render_no_match_message(f: &mut Frame, area: Rect) {
    let lines = vec![
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled(
            "No matching configurations",
            theme::dim_style(),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Press Esc to clear the filter, or type to refine.",
            Style::default().fg(theme::DIM),
        )),
    ];
    let para = Paragraph::new(lines)
        .alignment(Alignment::Center)
        .wrap(Wrap { trim: false });
    f.render_widget(para, area);
}
