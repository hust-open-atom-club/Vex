use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Wrap},
};

use crate::config::QemuConfig;
use crate::tui::app::App;
use crate::tui::scan::ConfigEntry;
use crate::tui::theme;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(entry) = app.current() else {
        return;
    };

    // Vertical layout: top pad / title / spacer / cards (Min) / footer / bottom pad.
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // top padding
            Constraint::Length(1), // title
            Constraint::Length(1), // spacer
            Constraint::Min(3),    // cards area
            Constraint::Length(1), // footer (file path)
            Constraint::Length(1), // bottom padding
        ])
        .split(area);

    render_title(f, rows[1], entry);
    render_cards_area(f, rows[3], entry, app.right_scroll);
    render_footer(f, rows[4], entry);
}

fn render_title(f: &mut Frame, area: Rect, entry: &ConfigEntry) {
    let line = match entry {
        ConfigEntry::Ok { name, config, .. } => {
            let mut spans = vec![Span::styled(
                format!("  {}", name),
                Style::default().add_modifier(Modifier::BOLD),
            )];
            if let Some(desc) = &config.desc {
                spans.push(Span::styled("  ·  ", theme::dim_style()));
                spans.push(Span::styled(desc.clone(), theme::dim_style()));
            }
            Line::from(spans)
        }
        ConfigEntry::Broken { name, .. } => {
            let bad = Style::default().fg(theme::BAD).add_modifier(Modifier::BOLD);
            Line::from(vec![
                Span::styled(format!("  {}", name), bad),
                Span::styled("  ·  ", Style::default().fg(theme::BAD)),
                Span::styled("⚠ Broken configuration", bad),
            ])
        }
    };
    f.render_widget(Paragraph::new(line), area);
}

fn render_footer(f: &mut Frame, area: Rect, entry: &ConfigEntry) {
    let path = match entry {
        ConfigEntry::Ok { path, .. } => path,
        ConfigEntry::Broken { path, .. } => path,
    };
    let line = Line::from(vec![
        Span::raw("  "),
        Span::styled(path.display().to_string(), theme::dim_style()),
    ]);
    f.render_widget(Paragraph::new(line).alignment(Alignment::Left), area);
}

/// Logical "card" — knows its own height and how to render itself inside
/// a given `Rect`. Used for scroll-aware placement in [`render_cards_area`].
enum Card<'a> {
    Binary(&'a QemuConfig),
    Args(&'a QemuConfig),
    Resources(&'a QemuConfig),
    Error(&'a str),
    Hint(&'a str),
}

impl<'a> Card<'a> {
    fn height(&self) -> u16 {
        match self {
            Card::Binary(_) => 3,
            Card::Args(c) => 2 + c.args.len().max(1) as u16,
            Card::Resources(c) => {
                if c.resources.is_empty() {
                    3
                } else {
                    2 + (c.resources.len() as u16) * 2
                }
            }
            Card::Error(_) => 3,
            Card::Hint(_) => 4,
        }
    }

    fn render(&self, f: &mut Frame, area: Rect) {
        match self {
            Card::Binary(c) => render_binary_card(f, area, c),
            Card::Args(c) => render_args_card(f, area, c),
            Card::Resources(c) => render_resources_card(f, area, c),
            Card::Error(e) => render_error_card(f, area, e),
            Card::Hint(e) => render_hint_card(f, area, e),
        }
    }
}

fn build_cards(entry: &ConfigEntry) -> Vec<Card<'_>> {
    match entry {
        ConfigEntry::Ok { config, .. } => vec![
            Card::Binary(config),
            Card::Args(config),
            Card::Resources(config),
        ],
        ConfigEntry::Broken { error, .. } => vec![Card::Error(error), Card::Hint(error)],
    }
}

/// Place cards into `area` honouring `scroll`. Strategy A: line-precise
/// clamp on `scroll`, but a card whose top edge would be clipped is
/// skipped entirely (no headless cards). The runtime clamps `scroll` to
/// the content-aware maximum locally; the value in `app.right_scroll` is
/// never written back.
fn render_cards_area(f: &mut Frame, area: Rect, entry: &ConfigEntry, scroll: u16) {
    let cards = build_cards(entry);
    if cards.is_empty() {
        return;
    }
    let gap: u16 = 1;
    let heights_sum: u16 = cards.iter().map(|c| c.height()).sum();
    let total_lines: u16 = heights_sum + gap * (cards.len() as u16 - 1);
    let max_scroll = total_lines.saturating_sub(area.height);
    let effective = scroll.min(max_scroll);

    let mut virtual_y: u16 = 0;
    for c in &cards {
        let card_top = virtual_y;
        let card_h = c.height();
        let card_bottom = virtual_y + card_h;
        virtual_y = card_bottom + gap;

        // Entirely above the viewport — skip.
        if card_bottom <= effective {
            continue;
        }
        // Top edge would be clipped → skip the whole card (Strategy A).
        if card_top < effective {
            continue;
        }
        let screen_y = card_top - effective;
        if screen_y >= area.height {
            break;
        }
        let h = card_h.min(area.height - screen_y);
        let rect = Rect {
            x: area.x,
            y: area.y + screen_y,
            width: area.width,
            height: h,
        };
        c.render(f, rect);
    }
}

fn card_block(title: &str) -> Block<'_> {
    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(theme::border_style())
        .title(Span::styled(title.to_string(), theme::dim_style()))
}

fn render_binary_card(f: &mut Frame, area: Rect, config: &QemuConfig) {
    let block = card_block(" Binary ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Min(0), Constraint::Length(24)])
        .split(inner);

    let bin = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::raw(config.qemu_bin.clone()),
    ]));
    f.render_widget(bin, cols[0]);

    let version_line = match &config.qemu_version {
        Some(v) => Line::from(Span::styled(
            format!("v{} ", v),
            Style::default().fg(theme::GOOD),
        )),
        None => Line::from(Span::styled("(unknown) ", Style::default().fg(theme::DIM))),
    };
    f.render_widget(
        Paragraph::new(version_line).alignment(Alignment::Right),
        cols[1],
    );
}

fn render_args_card(f: &mut Frame, area: Rect, config: &QemuConfig) {
    let block = card_block(" Args ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    let lines: Vec<Line<'static>> = if config.args.is_empty() {
        vec![Line::from(vec![
            Span::raw("  "),
            Span::styled("(none)", theme::dim_style()),
        ])]
    } else {
        config
            .args
            .iter()
            .enumerate()
            .map(|(i, a)| {
                Line::from(vec![
                    Span::raw("  "),
                    Span::styled(format!("[{}]", i), theme::dim_style()),
                    Span::raw("  "),
                    Span::raw(a.clone()),
                ])
            })
            .collect()
    };
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_resources_card(f: &mut Frame, area: Rect, config: &QemuConfig) {
    let block = card_block(" Resources ");
    let inner = block.inner(area);
    f.render_widget(block, area);

    if config.resources.is_empty() {
        let line = Line::from(vec![
            Span::raw("  "),
            Span::styled("(none)", theme::dim_style()),
        ]);
        f.render_widget(Paragraph::new(line), inner);
        return;
    }

    let mut keys: Vec<&String> = config.resources.keys().collect();
    keys.sort();
    let mut lines: Vec<Line<'static>> = Vec::new();
    for k in keys {
        let r = &config.resources[k];
        lines.push(Line::from(vec![
            Span::raw("  "),
            Span::raw(k.clone()),
            Span::raw("  →  "),
            Span::raw(r.path.clone()),
        ]));
        let sha_span = match &r.sha256 {
            Some(_) => Span::styled("sha256 ✓", Style::default().fg(theme::GOOD)),
            None => Span::styled("sha256 (none)", theme::dim_style()),
        };
        lines.push(Line::from(vec![
            Span::raw("     ["),
            Span::styled(format!("{:?}", r.kind), theme::dim_style()),
            Span::styled(", ", theme::dim_style()),
            sha_span,
            Span::styled("]", theme::dim_style()),
        ]));
    }
    f.render_widget(Paragraph::new(lines), inner);
}

fn render_error_card(f: &mut Frame, area: Rect, error: &str) {
    let block = card_block(" Error ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let line = Line::from(vec![
        Span::raw("  "),
        Span::styled(error.to_string(), Style::default().fg(theme::BAD)),
    ]);
    f.render_widget(Paragraph::new(line), inner);
}

fn render_hint_card(f: &mut Frame, area: Rect, error: &str) {
    let block = card_block(" Hint ");
    let inner = block.inner(area);
    f.render_widget(block, area);
    let hint_text = hint_for_broken_error(error);
    let para = Paragraph::new(vec![Line::from(vec![
        Span::raw("  "),
        Span::raw(hint_text),
    ])])
    .wrap(Wrap { trim: false });
    f.render_widget(para, inner);
}

fn hint_for_broken_error(error: &str) -> &'static str {
    match error {
        "parse error" => "Open the file in a text editor to inspect the JSON syntax.",
        "io error" => "Check file permissions or whether the file still exists.",
        "empty file" => "The file is empty. Re-run `vex save` or restore from backup.",
        _ => "Check the file's contents and re-run `vex save` if needed.",
    }
}
