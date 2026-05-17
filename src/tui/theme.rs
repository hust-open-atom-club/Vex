use ratatui::style::{Color, Modifier, Style};

// Core palette
#[allow(dead_code)]
pub const BG: Color = Color::Rgb(13, 17, 23);
#[allow(dead_code)]
pub const PANEL: Color = Color::Rgb(22, 27, 34);
pub const BORDER: Color = Color::Rgb(48, 54, 61);
#[allow(dead_code)]
pub const FG: Color = Color::Rgb(201, 209, 217);
pub const DIM: Color = Color::Rgb(139, 148, 158);
pub const ACCENT: Color = Color::Rgb(88, 166, 255);

// Status colors
#[allow(dead_code)]
pub const GOOD: Color = Color::Rgb(63, 185, 80);
#[allow(dead_code)]
pub const WARN: Color = Color::Rgb(210, 153, 34);
#[allow(dead_code)]
pub const BAD: Color = Color::Rgb(248, 81, 73);

// Mode badge backgrounds
pub const BADGE_BROWSE_BG: Color = Color::Rgb(31, 111, 235);
#[allow(dead_code)]
pub const BADGE_EDIT_BG: Color = Color::Rgb(187, 128, 9);
#[allow(dead_code)]
pub const BADGE_LIBRARY_BG: Color = Color::Rgb(137, 87, 229);
pub const BADGE_FG: Color = Color::Rgb(255, 255, 255);

pub fn dim_style() -> Style {
    Style::default().fg(DIM)
}

#[allow(dead_code)]
pub fn accent_style() -> Style {
    Style::default().fg(ACCENT)
}

pub fn border_style() -> Style {
    Style::default().fg(BORDER)
}

pub fn badge_browse() -> Style {
    Style::default()
        .bg(BADGE_BROWSE_BG)
        .fg(BADGE_FG)
        .add_modifier(Modifier::BOLD)
}

#[allow(dead_code)]
pub fn badge_edit() -> Style {
    Style::default()
        .bg(BADGE_EDIT_BG)
        .fg(BADGE_FG)
        .add_modifier(Modifier::BOLD)
}

#[allow(dead_code)]
pub fn badge_library() -> Style {
    Style::default()
        .bg(BADGE_LIBRARY_BG)
        .fg(BADGE_FG)
        .add_modifier(Modifier::BOLD)
}
