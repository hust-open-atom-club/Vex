use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, List, ListItem, ListState, Paragraph},
};

use crate::tui::app::{
    App, DrawerRow, EditField, EditFocus, EditMode, EditState, ExitConfirm, TextField,
};
use crate::tui::theme;

pub(super) fn render(f: &mut Frame, area: Rect, app: &App) {
    let Some(edit) = &app.edit else {
        return;
    };

    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(area);

    render_editor_pane(f, cols[0], edit);
    render_snippets_drawer(f, cols[1], edit);
}

fn render_editor_pane(f: &mut Frame, area: Rect, edit: &EditState) {
    let editor_focused = edit.focus == EditFocus::Editor;
    let border_style = if editor_focused {
        theme::accent_style()
    } else {
        theme::border_style()
    };
    let title_text = match &edit.mode {
        EditMode::Create => " Editing: new configuration ".to_string(),
        EditMode::Update { original_name } => format!(" Editing: {} ", original_name),
    };
    let title_style = if editor_focused {
        theme::accent_style().add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style().add_modifier(Modifier::BOLD)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(Span::styled(title_text, title_style));
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
        &edit.name,
        editor_focused && edit.focused_field == EditField::Name,
    );
    render_text_field(
        f,
        rows[2],
        "QEMU binary",
        &edit.qemu_bin,
        editor_focused && edit.focused_field == EditField::QemuBin,
    );
    render_text_field(
        f,
        rows[3],
        "Description",
        &edit.description,
        editor_focused && edit.focused_field == EditField::Description,
    );

    let args_focused = editor_focused && edit.focused_field == EditField::Args;
    let args_label_style = if args_focused {
        theme::accent_style().add_modifier(Modifier::BOLD)
    } else {
        theme::dim_style()
    };
    f.render_widget(
        Paragraph::new(Line::from(Span::styled("  Args", args_label_style))),
        rows[4],
    );

    render_args_list(
        f,
        rows[5],
        &edit.args,
        edit.args_selected,
        args_focused,
        edit.token_edit.as_ref(),
    );

    let res_line = if edit.preserved_resources.is_empty() {
        Line::from(vec![
            Span::styled("  Resources  ", theme::dim_style()),
            Span::styled("(none)", theme::dim_style()),
            Span::styled("    ", theme::dim_style()),
            Span::styled("(read-only, edit via CLI)", theme::dim_style()),
        ])
    } else {
        let mut keys: Vec<&String> = edit.preserved_resources.keys().collect();
        keys.sort();
        let key_list = keys
            .iter()
            .map(|k| k.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        Line::from(vec![
            Span::styled("  Resources  ", theme::dim_style()),
            Span::raw(key_list),
            Span::styled("    ", theme::dim_style()),
            Span::styled("(read-only, edit via CLI)", theme::dim_style()),
        ])
    };
    f.render_widget(Paragraph::new(res_line), rows[6]);

    if edit.exit_confirm == Some(ExitConfirm::Pending) {
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
    } else if edit.dirty {
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
        theme::accent_style().add_modifier(Modifier::BOLD)
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
        spans.push(Span::styled("█", theme::accent_style()));
        spans.push(Span::raw(field.value[cursor..].to_string()));
    } else {
        spans.push(Span::raw(field.value.clone()));
    }
    spans.push(Span::styled(" ]", theme::dim_style()));
    f.render_widget(Paragraph::new(Line::from(spans)), rows[1]);
}

fn render_args_list(
    f: &mut Frame,
    area: Rect,
    args: &[String],
    selected: usize,
    focused: bool,
    token_edit: Option<&TextField>,
) {
    if args.is_empty() {
        let placeholder = Line::from(vec![
            Span::styled("    ", theme::dim_style()),
            Span::styled(
                "(no args — Tab to Snippets drawer, → to insert one; or 'a' to add)",
                theme::dim_style(),
            ),
        ]);
        f.render_widget(Paragraph::new(placeholder), area);
        return;
    }

    let items: Vec<ListItem> = args
        .iter()
        .enumerate()
        .map(|(i, arg)| {
            let is_selected = focused && i == selected;
            let indicator = if is_selected { "▸ " } else { "  " };
            let mut line_spans: Vec<Span<'static>> = vec![
                Span::raw("  "),
                Span::styled(indicator, theme::accent_style()),
                Span::styled(format!("[{}] ", i), theme::dim_style()),
            ];
            if let Some(token) = token_edit.filter(|_| is_selected) {
                let cursor = token.cursor.min(token.value.len());
                line_spans.push(Span::raw(token.value[..cursor].to_string()));
                line_spans.push(Span::styled("█", theme::accent_style()));
                line_spans.push(Span::raw(token.value[cursor..].to_string()));
            } else {
                line_spans.push(Span::raw(arg.clone()));
            }
            ListItem::new(Line::from(line_spans))
        })
        .collect();

    let list = List::new(items);
    let mut state = ListState::default();
    if focused {
        state.select(Some(selected));
    }
    f.render_stateful_widget(list, area, &mut state);
}

// --- Snippets drawer -----------------------------------------------------

fn render_snippets_drawer(f: &mut Frame, area: Rect, edit: &EditState) {
    let drawer_focused = edit.focus == EditFocus::SnippetsDrawer;
    let border_style = if drawer_focused {
        theme::accent_style()
    } else {
        theme::border_style()
    };
    let title = if let Some(err) = &edit.snippets.load_error {
        Span::styled(
            format!(" Snippets · {} ", err),
            Style::default().fg(theme::WARN),
        )
    } else if edit.snippets.filter.active {
        Span::styled(
            " Snippets · filter ".to_string(),
            theme::accent_style().add_modifier(Modifier::BOLD),
        )
    } else {
        let title_style = if drawer_focused {
            theme::accent_style().add_modifier(Modifier::BOLD)
        } else {
            theme::dim_style().add_modifier(Modifier::BOLD)
        };
        Span::styled(" Snippets ".to_string(), title_style)
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(border_style)
        .title(title);
    let inner = block.inner(area);
    f.render_widget(block, area);

    if edit.snippets.filter.active {
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
        render_filter_input(f, layout[1], &edit.snippets.filter.query);
        let sep: String = "─".repeat(layout[2].width as usize);
        f.render_widget(Paragraph::new(sep).style(theme::dim_style()), layout[2]);
        render_snippets_list(f, layout[3], edit);
    } else {
        let layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1),
                Constraint::Min(0),
                Constraint::Length(1),
            ])
            .split(inner);
        render_snippets_list(f, layout[1], edit);
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

fn render_snippets_list(f: &mut Frame, area: Rect, edit: &EditState) {
    let rows = edit.snippets.visible_rows();

    if rows.is_empty() {
        let msg = if edit.snippets.snippets.is_empty() {
            "  (no snippets loaded)"
        } else if edit.snippets.filter.active && !edit.snippets.filter.query.value.is_empty() {
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

    let drawer_focused = edit.focus == EditFocus::SnippetsDrawer;
    let selected = edit.snippets.selected;

    let items: Vec<ListItem> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| {
            let is_selected = drawer_focused && i == selected;
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
                    let line = Line::from(vec![
                        Span::styled(indicator, theme::accent_style()),
                        Span::styled(format!("{} ", chevron), theme::dim_style()),
                        Span::styled(
                            format!("{:?}", category),
                            theme::accent_style().add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(count_suffix, theme::dim_style()),
                    ]);
                    ListItem::new(line)
                }
                DrawerRow::Snippet { snippet_index } => {
                    let snippet = &edit.snippets.snippets[*snippet_index];
                    let indicator = if is_selected { "▸ " } else { "  " };
                    let is_user_owned = edit
                        .user_only_snippets
                        .iter()
                        .any(|u| u.name == snippet.name);
                    let badge = if is_user_owned {
                        Span::styled(" [user]", Style::default().fg(theme::BADGE_LIBRARY_BG))
                    } else {
                        Span::styled(" [builtin]", theme::dim_style())
                    };
                    let line = Line::from(vec![
                        Span::styled("    ", theme::dim_style()),
                        Span::styled(indicator, theme::accent_style()),
                        Span::raw(snippet.name.clone()),
                        badge,
                    ]);
                    ListItem::new(line)
                }
            }
        })
        .collect();

    f.render_widget(List::new(items), area);
}
