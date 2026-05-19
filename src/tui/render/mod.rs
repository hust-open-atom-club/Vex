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
    layout::{Constraint, Direction, Layout},
};

use super::app::App;

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
