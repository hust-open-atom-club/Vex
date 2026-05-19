use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    text::Line,
    widgets::Paragraph,
};

use crate::tui::app::App;
use crate::tui::theme;

pub(super) fn render_middle(f: &mut Frame, area: Rect, _app: &App) {
    if area.height < 2 {
        return;
    }
    let top_pad = area.height.saturating_sub(2) / 2;
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(top_pad),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);

    let main = Paragraph::new(Line::from("No configurations"))
        .style(theme::dim_style())
        .alignment(Alignment::Center);
    let sub = Paragraph::new(Line::from("Run `vex save` to create one"))
        .style(theme::dim_style())
        .alignment(Alignment::Center);
    f.render_widget(main, rows[1]);
    f.render_widget(sub, rows[2]);
}
