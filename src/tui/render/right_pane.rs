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
    render_cards_area(f, rows[3], entry);
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

fn render_cards_area(f: &mut Frame, area: Rect, entry: &ConfigEntry) {
    match entry {
        ConfigEntry::Ok { config, .. } => render_ok_cards(f, area, config),
        ConfigEntry::Broken { error, .. } => render_broken_cards(f, area, error),
    }
}

fn render_ok_cards(f: &mut Frame, area: Rect, config: &QemuConfig) {
    let binary_h: u16 = 3;
    let args_h: u16 = 2 + config.args.len().max(1) as u16;
    let resources_h: u16 = if config.resources.is_empty() {
        3
    } else {
        // 2 lines per resource (key→path + metadata), + 2 for borders.
        2 + (config.resources.len() as u16) * 2
    };

    let constraints = vec![
        Constraint::Length(binary_h),
        Constraint::Length(1),
        Constraint::Length(args_h),
        Constraint::Length(1),
        Constraint::Length(resources_h),
        Constraint::Min(0),
    ];
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints(constraints)
        .split(area);

    render_binary_card(f, chunks[0], config);
    render_args_card(f, chunks[2], config);
    render_resources_card(f, chunks[4], config);
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

fn render_broken_cards(f: &mut Frame, area: Rect, error: &str) {
    let error_h: u16 = 3;
    let hint_h: u16 = 4;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(error_h),
            Constraint::Length(1),
            Constraint::Length(hint_h),
            Constraint::Min(0),
        ])
        .split(area);

    let err_block = card_block(" Error ");
    let err_inner = err_block.inner(chunks[0]);
    f.render_widget(err_block, chunks[0]);
    let err_line = Line::from(vec![
        Span::raw("  "),
        Span::styled(error.to_string(), Style::default().fg(theme::BAD)),
    ]);
    f.render_widget(Paragraph::new(err_line), err_inner);

    let hint_block = card_block(" Hint ");
    let hint_inner = hint_block.inner(chunks[2]);
    f.render_widget(hint_block, chunks[2]);
    let hint_text = hint_for_broken_error(error);
    let hint_para = Paragraph::new(vec![Line::from(vec![
        Span::raw("  "),
        Span::raw(hint_text),
    ])])
    .wrap(Wrap { trim: false });
    f.render_widget(hint_para, hint_inner);
}

fn hint_for_broken_error(error: &str) -> &'static str {
    match error {
        "parse error" => "Open the file in a text editor to inspect the JSON syntax.",
        "io error" => "Check file permissions or whether the file still exists.",
        "empty file" => "The file is empty. Re-run `vex save` or restore from backup.",
        _ => "Check the file's contents and re-run `vex save` if needed.",
    }
}
