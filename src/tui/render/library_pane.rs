use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Wrap},
};

use crate::tui::app::{
    App, DrawerRow, LibraryState, SnippetEditField, SnippetEditMode, SnippetEditState,
    SnippetsDrawerState, TextField,
};
use crate::tui::theme;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(lib) = &app.library else {
        return;
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    render_left_tree(f, cols[0], lib);

    if let Some(sedit) = &lib.edit {
        render_snippet_editor(f, cols[1], sedit);
    } else {
        render_right_detail(f, cols[1], lib);
    }
}

fn render_left_tree(f: &mut Frame, area: Rect, lib: &LibraryState) {
    let focused = lib.edit.is_none() && lib.delete_confirm.is_none();
    let border_style = if focused {
        Style::default().fg(theme::BADGE_LIBRARY_BG)
    } else {
        theme::border_style()
    };
    let title_style = if focused {
        Style::default()
            .fg(theme::BADGE_LIBRARY_BG)
            .add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style().add_modifier(Modifier::BOLD)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Span::styled(" Snippets Library ", title_style));
    let inner = block.inner(area);
    f.render_widget(block, area);

    if lib.snippets.filter.active {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);
        render_filter_input(f, layout[1], &lib.snippets.filter.query);
        let sep: String = "─".repeat(layout[2].width as usize);
        f.render_widget(Paragraph::new(sep).style(theme::dim_style()), layout[2]);
        render_drawer_list(f, layout[3], &lib.snippets, &lib.user_only, focused);
    } else {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);
        render_drawer_list(f, layout[1], &lib.snippets, &lib.user_only, focused);
    }
}

fn render_filter_input(f: &mut Frame, area: Rect, query: &TextField) {
    let line = Line::from(vec![
        Span::styled("  / ", theme::accent_style()),
        Span::raw(query.value.clone()),
        Span::styled("█", theme::accent_style()),
    ]);
    f.render_widget(Paragraph::new(line), area);
}

fn render_drawer_list(
    f: &mut Frame,
    area: Rect,
    drawer: &SnippetsDrawerState,
    user_only: &[crate::snippets::Snippet],
    focused: bool,
) {
    let rows = drawer.visible_rows();
    if rows.is_empty() {
        let msg = if drawer.snippets.is_empty() {
            "  (no snippets loaded)"
        } else if drawer.filter.active && !drawer.filter.query.value.is_empty() {
            "  (no matching snippets)"
        } else {
            "  (no snippets to show)"
        };
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(msg, theme::dim_style()))),
            area,
        );
        return;
    }
    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_selected = focused && i == drawer.selected;
            match row {
                DrawerRow::CategoryHeader {
                    category,
                    collapsed,
                    snippet_count,
                } => {
                    let chevron = if *collapsed { "▶" } else { "▼" };
                    let indicator = if is_selected { "▸ " } else { "  " };
                    let count_suffix = if *collapsed {
                        format!(" ({} hidden)", snippet_count)
                    } else {
                        String::new()
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled(indicator, theme::accent_style()),
                        Span::styled(format!("{} ", chevron), theme::dim_style()),
                        Span::styled(
                            format!("{:?}", category),
                            theme::accent_style().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(count_suffix, theme::dim_style()),
                    ]))
                }
                DrawerRow::Snippet { snippet_index } => {
                    let snippet = &drawer.snippets[*snippet_index];
                    let indicator = if is_selected { "▸ " } else { "  " };
                    let is_user_owned = user_only.iter().any(|u| u.name == snippet.name);
                    let badge = if is_user_owned {
                        Span::styled(" [user]", Style::default().fg(theme::BADGE_LIBRARY_BG))
                    } else {
                        Span::styled(" [builtin]", theme::dim_style())
                    };
                    ListItem::new(Line::from(vec![
                        Span::styled("    ", theme::dim_style()),
                        Span::styled(indicator, theme::accent_style()),
                        Span::raw(snippet.name.clone()),
                        badge,
                    ]))
                }
            }
        })
        .collect();
    let mut state = ListState::default();
    state.select(Some(drawer.selected));
    f.render_stateful_widget(List::new(items), area, &mut state);
}

fn render_right_detail(f: &mut Frame, area: Rect, lib: &LibraryState) {
    if let Some(confirm) = &lib.delete_confirm {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(theme::BAD))
            .title(Span::styled(
                " Confirm deletion ",
                Style::default().fg(theme::BAD).add_modifier(Modifier::BOLD),
            ));
        let inner = block.inner(area);
        f.render_widget(block, area);
        let lines = vec![
            Line::from(""),
            Line::from(Span::styled(
                format!("  Delete user snippet '{}' ?", confirm.snippet_name),
                Style::default().fg(theme::BAD).add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from(Span::styled(
                "  [y] delete    [n] cancel    (any other key to cancel)",
                theme::dim_style(),
            )),
        ];
        f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), inner);
        return;
    }

    let snippet = match lib.snippets.current_row() {
        Some(DrawerRow::Snippet { snippet_index }) => lib.snippets.snippets.get(snippet_index),
        _ => None,
    };
    let Some(snippet) = snippet else {
        let p = Paragraph::new(Line::from(Span::styled(
            "  (select a snippet to view details)",
            theme::dim_style(),
        )));
        f.render_widget(p, area);
        return;
    };
    let is_builtin = !lib.user_only.iter().any(|u| u.name == snippet.name);

    let mut lines: Vec<Line<'static>> = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                format!("  {}", snippet.name),
                Style::default()
                    .fg(theme::BADGE_LIBRARY_BG)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            if is_builtin {
                Span::styled("[builtin]", theme::dim_style())
            } else {
                Span::styled("[user]", Style::default().fg(theme::BADGE_LIBRARY_BG))
            },
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Category     ", theme::dim_style()),
            Span::raw(format!("{:?}", snippet.category)),
        ]),
        Line::from(vec![
            Span::styled("  Description  ", theme::dim_style()),
            Span::raw(
                snippet
                    .description
                    .clone()
                    .unwrap_or_else(|| "(none)".to_string()),
            ),
        ]),
        Line::from(""),
        Line::from(Span::styled("  Args", theme::dim_style())),
    ];
    if snippet.args.is_empty() {
        lines.push(Line::from(Span::styled("    (none)", theme::dim_style())));
    } else {
        for (i, arg) in snippet.args.iter().enumerate() {
            lines.push(Line::from(vec![
                Span::raw("    "),
                Span::styled(format!("[{}]  ", i), theme::dim_style()),
                Span::raw(arg.clone()),
            ]));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(vec![
        Span::styled("  Source       ", theme::dim_style()),
        if is_builtin {
            Span::styled("builtin (read-only)", theme::dim_style())
        } else {
            Span::styled(
                "user (~/.vex/snippets.json)",
                Style::default().fg(theme::BADGE_LIBRARY_BG),
            )
        },
    ]));
    f.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

// --- Snippet editor (right pane when lib.edit is Some) ------------------

fn render_snippet_editor(f: &mut Frame, area: Rect, sedit: &SnippetEditState) {
    let title_text = match &sedit.mode {
        SnippetEditMode::Create => " Editing: new snippet ".to_string(),
        SnippetEditMode::Update { original_name } => {
            format!(" Editing: {} ", original_name)
        }
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(theme::BADGE_EDIT_BG))
        .title(Span::styled(
            title_text,
            Style::default()
                .fg(theme::BADGE_EDIT_BG)
                .add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(area);
    f.render_widget(block, area);

    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Min(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(inner);

    render_text_field(
        f,
        rows[1],
        "Name",
        &sedit.name,
        sedit.focused_field == SnippetEditField::Name,
    );
    render_category_field(f, rows[2], sedit);
    render_text_field(
        f,
        rows[3],
        "Description",
        &sedit.description,
        sedit.focused_field == SnippetEditField::Description,
    );

    let args_focused = sedit.focused_field == SnippetEditField::Args;
    let args_label_style = if args_focused {
        Style::default()
            .fg(theme::BADGE_EDIT_BG)
            .add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style()
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("  Args", args_label_style))),
        rows[4],
    );
    render_args_field(f, rows[5], sedit);

    f.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled("  Source       ", theme::dim_style()),
            Span::styled(
                "user (~/.vex/snippets.json)",
                Style::default().fg(theme::BADGE_LIBRARY_BG),
            ),
        ])),
        rows[6],
    );

    if sedit.exit_confirm == Some(crate::tui::app::ExitConfirm::Pending) {
        let prompt = Line::from(vec![
            Span::styled(
                "  Save changes? ",
                Style::default()
                    .fg(theme::WARN)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("[y] save  [n] discard  ", theme::dim_style()),
            Span::styled("(any other key to cancel)", theme::dim_style()),
        ]);
        f.render_widget(Paragraph::new(prompt), rows[7]);
    } else if sedit.dirty {
        let dirty = Line::from(vec![
            Span::styled("  ●", Style::default().fg(theme::WARN)),
            Span::styled(" unsaved changes", theme::dim_style()),
        ]);
        f.render_widget(Paragraph::new(dirty), rows[7]);
    }
}

fn render_text_field(f: &mut Frame, area: Rect, label: &str, field: &TextField, focused: bool) {
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    let label_style = if focused {
        Style::default()
            .fg(theme::BADGE_EDIT_BG)
            .add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style()
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            format!("  {}", label),
            label_style,
        ))),
        rows[0],
    );
    let mut spans: Vec<Span<'static>> = vec![Span::styled("  [ ", theme::dim_style())];
    if focused {
        let cursor = field.cursor.min(field.value.len());
        spans.push(Span::raw(field.value[..cursor].to_string()));
        spans.push(Span::styled("█", Style::default().fg(theme::BADGE_EDIT_BG)));
        spans.push(Span::raw(field.value[cursor..].to_string()));
    } else {
        spans.push(Span::raw(field.value.clone()));
    }
    spans.push(Span::styled(" ]", theme::dim_style()));
    f.render_widget(Paragraph::new(Line::from(spans)), rows[1]);
}

fn render_category_field(f: &mut Frame, area: Rect, sedit: &SnippetEditState) {
    let focused = sedit.focused_field == SnippetEditField::Category;
    let label_style = if focused {
        Style::default()
            .fg(theme::BADGE_EDIT_BG)
            .add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style()
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("  Category", label_style))),
        rows[0],
    );
    let arrow_style = if focused {
        Style::default().fg(theme::BADGE_EDIT_BG)
    } else {
        theme::dim_style()
    };
    let line = Line::from(vec![
        Span::raw("  "),
        Span::styled("◂ ", arrow_style),
        Span::raw(format!("{:?}", sedit.category)),
        Span::styled(" ▸", arrow_style),
    ]);
    f.render_widget(Paragraph::new(line), rows[1]);
}

fn render_args_field(f: &mut Frame, area: Rect, sedit: &SnippetEditState) {
    if sedit.args.is_empty() {
        let placeholder = Line::from(vec![
            Span::raw("    "),
            Span::styled("(no args — press 'a' to add)", theme::dim_style()),
        ]);
        f.render_widget(Paragraph::new(placeholder), area);
        return;
    }
    let focus_active = sedit.focused_field == SnippetEditField::Args;
    let edit_style = Style::default().fg(theme::BADGE_EDIT_BG);
    let items: Vec<ListItem> = sedit
        .args
        .iter()
        .enumerate()
        .map(|(i, arg)| {
            let is_selected = focus_active && i == sedit.args_selected;
            let indicator = if is_selected { "▸ " } else { "  " };
            let mut line_spans: Vec<Span<'static>> = vec![
                Span::raw("  "),
                Span::styled(indicator, edit_style),
                Span::styled(format!("[{}] ", i), theme::dim_style()),
            ];
            if let Some(token) = sedit.token_edit.as_ref().filter(|_| is_selected) {
                let cursor = token.cursor.min(token.value.len());
                line_spans.push(Span::raw(token.value[..cursor].to_string()));
                line_spans.push(Span::styled("█", edit_style));
                line_spans.push(Span::raw(token.value[cursor..].to_string()));
            } else {
                line_spans.push(Span::raw(arg.clone()));
            }
            ListItem::new(Line::from(line_spans))
        })
        .collect();
    let list = List::new(items);
    f.render_widget(list, area);
}
