use super::app::{App, BrowseSubMode, EditField, EditFocus, ExitConfirm, SnippetEditField};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Quit,
    NavigateUp,
    NavigateDown,
    NavigateTop,
    NavigateBottom,
    ToggleFocus,
    Launch,
    EnterFilter,
    ExitFilter,
    AcceptFilter,
    FilterChar(char),
    FilterBackspace,
    ToggleHelp,
    DismissHelp,
    Refresh,
    // Edit mode entry (from Browse).
    EnterEditExisting,
    EnterEditNew,
    // Edit mode internals.
    EditTab,
    EditShiftTab,
    EditTextChar(char),
    EditTextBackspace,
    EditTextLeft,
    EditTextRight,
    EditArgsUp,
    EditArgsDown,
    EditArgsMoveUp,
    EditArgsMoveDown,
    EditArgsDelete,
    EditSave,
    EditCancel,
    EditConfirmSave,
    EditConfirmDiscard,
    EditConfirmCancel,
    // P4-8: Editor-pane field cycling via ↑/↓ (Tab was repurposed to
    // switch large panes).
    EditFieldUp,
    EditFieldDown,
    // P4-8: Snippets drawer events.
    SnippetsDrawerUp,
    SnippetsDrawerDown,
    SnippetsDrawerToggleCollapse,
    SnippetsDrawerEnterFilter,
    SnippetsDrawerExitFilter,
    SnippetsDrawerFilterChar(char),
    SnippetsDrawerFilterBackspace,
    SnippetsDrawerInsert,
    // P4-10: Args token editing in Edit mode (mirrors SnippetEditToken*).
    EditArgsAddEmpty,
    EditArgsEnterToken,
    EditTokenChar(char),
    EditTokenBackspace,
    EditTokenLeft,
    EditTokenRight,
    EditTokenCommit,
    // P4-9: Library mode entry/exit + CRUD triggers
    EnterLibrary,
    ExitLibrary,
    LibraryNew,
    LibraryEditSelected,
    LibraryDeleteSelected,
    LibraryDeleteConfirm,
    LibraryDeleteCancel,
    // P4-9: SnippetEdit (nested in Library) operations
    SnippetEditFieldUp,
    SnippetEditFieldDown,
    SnippetEditTextChar(char),
    SnippetEditTextBackspace,
    SnippetEditTextLeft,
    SnippetEditTextRight,
    SnippetEditCategoryPrev,
    SnippetEditCategoryNext,
    SnippetEditArgsUp,
    SnippetEditArgsDown,
    SnippetEditArgsMoveUp,
    SnippetEditArgsMoveDown,
    SnippetEditArgsDelete,
    SnippetEditArgsAddEmpty,
    SnippetEditArgsEnterToken,
    SnippetEditTokenChar(char),
    SnippetEditTokenBackspace,
    SnippetEditTokenLeft,
    SnippetEditTokenRight,
    SnippetEditTokenCommit,
    SnippetEditSave,
    SnippetEditCancel,
    SnippetEditConfirmSave,
    SnippetEditConfirmDiscard,
    SnippetEditConfirmCancel,
    Noop,
}

impl AppEvent {
    /// True if this event should only be processed while in Edit mode.
    /// `handle_event` uses this to gate Browse-only events out of Edit
    /// and vice versa.
    pub fn is_edit_event(&self) -> bool {
        matches!(
            self,
            AppEvent::EditTab
                | AppEvent::EditShiftTab
                | AppEvent::EditTextChar(_)
                | AppEvent::EditTextBackspace
                | AppEvent::EditTextLeft
                | AppEvent::EditTextRight
                | AppEvent::EditArgsUp
                | AppEvent::EditArgsDown
                | AppEvent::EditArgsMoveUp
                | AppEvent::EditArgsMoveDown
                | AppEvent::EditArgsDelete
                | AppEvent::EditSave
                | AppEvent::EditCancel
                | AppEvent::EditConfirmSave
                | AppEvent::EditConfirmDiscard
                | AppEvent::EditConfirmCancel
                | AppEvent::EditFieldUp
                | AppEvent::EditFieldDown
                | AppEvent::SnippetsDrawerUp
                | AppEvent::SnippetsDrawerDown
                | AppEvent::SnippetsDrawerToggleCollapse
                | AppEvent::SnippetsDrawerEnterFilter
                | AppEvent::SnippetsDrawerExitFilter
                | AppEvent::SnippetsDrawerFilterChar(_)
                | AppEvent::SnippetsDrawerFilterBackspace
                | AppEvent::SnippetsDrawerInsert
                | AppEvent::EditArgsAddEmpty
                | AppEvent::EditArgsEnterToken
                | AppEvent::EditTokenChar(_)
                | AppEvent::EditTokenBackspace
                | AppEvent::EditTokenLeft
                | AppEvent::EditTokenRight
                | AppEvent::EditTokenCommit
        )
    }

    /// True if this event should only be processed while Library mode is
    /// active. SnippetsDrawer events are excluded here because they are
    /// shared with Edit mode (`is_edit_event` already covers them).
    pub fn is_library_event(&self) -> bool {
        matches!(
            self,
            AppEvent::EnterLibrary
                | AppEvent::ExitLibrary
                | AppEvent::LibraryNew
                | AppEvent::LibraryEditSelected
                | AppEvent::LibraryDeleteSelected
                | AppEvent::LibraryDeleteConfirm
                | AppEvent::LibraryDeleteCancel
                | AppEvent::SnippetEditFieldUp
                | AppEvent::SnippetEditFieldDown
                | AppEvent::SnippetEditTextChar(_)
                | AppEvent::SnippetEditTextBackspace
                | AppEvent::SnippetEditTextLeft
                | AppEvent::SnippetEditTextRight
                | AppEvent::SnippetEditCategoryPrev
                | AppEvent::SnippetEditCategoryNext
                | AppEvent::SnippetEditArgsUp
                | AppEvent::SnippetEditArgsDown
                | AppEvent::SnippetEditArgsMoveUp
                | AppEvent::SnippetEditArgsMoveDown
                | AppEvent::SnippetEditArgsDelete
                | AppEvent::SnippetEditArgsAddEmpty
                | AppEvent::SnippetEditArgsEnterToken
                | AppEvent::SnippetEditTokenChar(_)
                | AppEvent::SnippetEditTokenBackspace
                | AppEvent::SnippetEditTokenLeft
                | AppEvent::SnippetEditTokenRight
                | AppEvent::SnippetEditTokenCommit
                | AppEvent::SnippetEditSave
                | AppEvent::SnippetEditCancel
                | AppEvent::SnippetEditConfirmSave
                | AppEvent::SnippetEditConfirmDiscard
                | AppEvent::SnippetEditConfirmCancel
        )
    }

    /// Drawer events shared between Edit mode (P4-8) and Library mode (P4-9).
    pub fn is_drawer_event(&self) -> bool {
        matches!(
            self,
            AppEvent::SnippetsDrawerUp
                | AppEvent::SnippetsDrawerDown
                | AppEvent::SnippetsDrawerToggleCollapse
                | AppEvent::SnippetsDrawerEnterFilter
                | AppEvent::SnippetsDrawerExitFilter
                | AppEvent::SnippetsDrawerFilterChar(_)
                | AppEvent::SnippetsDrawerFilterBackspace
                | AppEvent::SnippetsDrawerInsert
        )
    }
}

pub fn translate_key(key: crossterm::event::KeyEvent, app: &App) -> AppEvent {
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

    // 1. Ctrl+C is ALWAYS Quit — must come before any mode/overlay swallow.
    //    Kind::Press guard included so a Release event doesn't trigger Quit.
    if key.kind == KeyEventKind::Press
        && matches!(key.code, KeyCode::Char('c'))
        && key.modifiers.contains(KeyModifiers::CONTROL)
    {
        return AppEvent::Quit;
    }

    // 2. KeyEventKind::Press guard (windows release/repeat dedup).
    if key.kind != KeyEventKind::Press {
        return AppEvent::Noop;
    }

    // 3. Help overlay swallows every remaining Press.
    if app.show_help {
        return AppEvent::DismissHelp;
    }

    // 4. Filtering with editable query: characters go into the query.
    if matches!(
        &app.browse_sub,
        BrowseSubMode::Filtering {
            accepted: false,
            ..
        }
    ) {
        return match key.code {
            KeyCode::Esc => AppEvent::ExitFilter,
            KeyCode::Enter => AppEvent::AcceptFilter,
            KeyCode::Backspace => AppEvent::FilterBackspace,
            KeyCode::Up => AppEvent::NavigateUp,
            KeyCode::Down => AppEvent::NavigateDown,
            KeyCode::Char(c) => {
                if key
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT)
                {
                    AppEvent::Noop
                } else {
                    AppEvent::FilterChar(c)
                }
            }
            _ => AppEvent::Noop,
        };
    }

    // 5. Edit mode dispatch (P4-7 + P4-8). Only when an Edit session is active.
    if let Some(edit) = &app.edit {
        // [5.1] Exit-confirm overrides everything (Ctrl+C already handled above).
        if edit.exit_confirm == Some(ExitConfirm::Pending) {
            return match key.code {
                KeyCode::Char('y') => AppEvent::EditConfirmSave,
                KeyCode::Char('n') => AppEvent::EditConfirmDiscard,
                _ => AppEvent::EditConfirmCancel,
            };
        }

        // [5.2] Snippets-drawer filter input is modal within the drawer.
        if edit.focus == EditFocus::SnippetsDrawer && edit.snippets.filter.active {
            return match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => AppEvent::SnippetsDrawerExitFilter,
                (KeyCode::Backspace, _) => AppEvent::SnippetsDrawerFilterBackspace,
                (KeyCode::Up, _) => AppEvent::SnippetsDrawerUp,
                (KeyCode::Down, _) => AppEvent::SnippetsDrawerDown,
                (KeyCode::Char(c), m)
                    if !m.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    AppEvent::SnippetsDrawerFilterChar(c)
                }
                _ => AppEvent::Noop,
            };
        }

        // [5.3] Common Edit shortcuts: save / cancel / pane swap.
        // Esc in token-edit mode commits the token instead of cancelling
        // the whole edit session.
        let in_token_edit = edit.focus == EditFocus::Editor
            && edit.focused_field == EditField::Args
            && edit.token_edit.is_some();
        match (key.code, key.modifiers) {
            (KeyCode::Char('s'), m) if m.contains(KeyModifiers::CONTROL) => {
                return AppEvent::EditSave;
            }
            (KeyCode::Esc, _) if in_token_edit => return AppEvent::EditTokenCommit,
            (KeyCode::Esc, _) => return AppEvent::EditCancel,
            (KeyCode::BackTab, _) => return AppEvent::EditShiftTab,
            (KeyCode::Tab, m) if m.contains(KeyModifiers::SHIFT) => {
                return AppEvent::EditShiftTab;
            }
            (KeyCode::Tab, _) => return AppEvent::EditTab,
            _ => {}
        }

        // [5.4] Per-pane dispatch.
        return match edit.focus {
            EditFocus::Editor => match edit.focused_field {
                EditField::Args => {
                    // P4-10: token edit modal — takes precedence inside Args.
                    if edit.token_edit.is_some() {
                        return match (key.code, key.modifiers) {
                            (KeyCode::Enter, _) | (KeyCode::Esc, _) => AppEvent::EditTokenCommit,
                            (KeyCode::Backspace, _) => AppEvent::EditTokenBackspace,
                            (KeyCode::Left, _) => AppEvent::EditTokenLeft,
                            (KeyCode::Right, _) => AppEvent::EditTokenRight,
                            (KeyCode::Char(c), m)
                                if !m.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                            {
                                AppEvent::EditTokenChar(c)
                            }
                            _ => AppEvent::Noop,
                        };
                    }
                    match (key.code, key.modifiers) {
                        (KeyCode::Up, _) => AppEvent::EditArgsUp,
                        (KeyCode::Down, _) => AppEvent::EditArgsDown,
                        (KeyCode::Char('K'), _) => AppEvent::EditArgsMoveUp,
                        (KeyCode::Char('J'), _) => AppEvent::EditArgsMoveDown,
                        (KeyCode::Char('d'), m) if !m.contains(KeyModifiers::CONTROL) => {
                            AppEvent::EditArgsDelete
                        }
                        // P4-10: new args operations.
                        (KeyCode::Char('a'), m) if !m.contains(KeyModifiers::CONTROL) => {
                            AppEvent::EditArgsAddEmpty
                        }
                        (KeyCode::Enter, _) => AppEvent::EditArgsEnterToken,
                        _ => AppEvent::Noop,
                    }
                }
                _ => match (key.code, key.modifiers) {
                    (KeyCode::Up, _) => AppEvent::EditFieldUp,
                    (KeyCode::Down, _) => AppEvent::EditFieldDown,
                    (KeyCode::Backspace, _) => AppEvent::EditTextBackspace,
                    (KeyCode::Left, _) => AppEvent::EditTextLeft,
                    (KeyCode::Right, _) => AppEvent::EditTextRight,
                    (KeyCode::Char(c), m)
                        if !m.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        AppEvent::EditTextChar(c)
                    }
                    _ => AppEvent::Noop,
                },
            },
            EditFocus::SnippetsDrawer => match (key.code, key.modifiers) {
                (KeyCode::Up, _) => AppEvent::SnippetsDrawerUp,
                (KeyCode::Down, _) => AppEvent::SnippetsDrawerDown,
                (KeyCode::Char(' '), _) => AppEvent::SnippetsDrawerToggleCollapse,
                (KeyCode::Char('/'), _) => AppEvent::SnippetsDrawerEnterFilter,
                (KeyCode::Right, _) => AppEvent::SnippetsDrawerInsert,
                _ => AppEvent::Noop,
            },
        };
    }

    // 6. Library mode dispatch (P4-9). Only when a Library session is active.
    if let Some(lib) = &app.library {
        // [6.1] Nested SnippetEdit overrides Library browse.
        if let Some(sedit) = &lib.edit {
            // [6.1.1] Token edit modal.
            if sedit.token_edit.is_some() {
                return match (key.code, key.modifiers) {
                    (KeyCode::Enter, _) | (KeyCode::Esc, _) => AppEvent::SnippetEditTokenCommit,
                    (KeyCode::Backspace, _) => AppEvent::SnippetEditTokenBackspace,
                    (KeyCode::Left, _) => AppEvent::SnippetEditTokenLeft,
                    (KeyCode::Right, _) => AppEvent::SnippetEditTokenRight,
                    (KeyCode::Char(c), m)
                        if !m.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        AppEvent::SnippetEditTokenChar(c)
                    }
                    _ => AppEvent::Noop,
                };
            }
            // [6.1.2] Exit-confirm modal.
            if sedit.exit_confirm == Some(ExitConfirm::Pending) {
                return match key.code {
                    KeyCode::Char('y') => AppEvent::SnippetEditConfirmSave,
                    KeyCode::Char('n') => AppEvent::SnippetEditConfirmDiscard,
                    _ => AppEvent::SnippetEditConfirmCancel,
                };
            }
            // [6.1.3] Common SnippetEdit shortcuts.
            match (key.code, key.modifiers) {
                (KeyCode::Char('s'), m) if m.contains(KeyModifiers::CONTROL) => {
                    return AppEvent::SnippetEditSave;
                }
                (KeyCode::Esc, _) => return AppEvent::SnippetEditCancel,
                _ => {}
            }
            // [6.1.4] Per-field dispatch.
            return match sedit.focused_field {
                SnippetEditField::Args => match (key.code, key.modifiers) {
                    (KeyCode::Up, _) => AppEvent::SnippetEditArgsUp,
                    (KeyCode::Down, _) => AppEvent::SnippetEditArgsDown,
                    (KeyCode::Char('K'), _) => AppEvent::SnippetEditArgsMoveUp,
                    (KeyCode::Char('J'), _) => AppEvent::SnippetEditArgsMoveDown,
                    (KeyCode::Char('d'), m) if !m.contains(KeyModifiers::CONTROL) => {
                        AppEvent::SnippetEditArgsDelete
                    }
                    (KeyCode::Char('a'), m) if !m.contains(KeyModifiers::CONTROL) => {
                        AppEvent::SnippetEditArgsAddEmpty
                    }
                    (KeyCode::Enter, _) => AppEvent::SnippetEditArgsEnterToken,
                    _ => AppEvent::Noop,
                },
                SnippetEditField::Category => match (key.code, key.modifiers) {
                    (KeyCode::Up, _) => AppEvent::SnippetEditFieldUp,
                    (KeyCode::Down, _) => AppEvent::SnippetEditFieldDown,
                    (KeyCode::Left, _) => AppEvent::SnippetEditCategoryPrev,
                    (KeyCode::Right, _) => AppEvent::SnippetEditCategoryNext,
                    _ => AppEvent::Noop,
                },
                _ => match (key.code, key.modifiers) {
                    (KeyCode::Up, _) => AppEvent::SnippetEditFieldUp,
                    (KeyCode::Down, _) => AppEvent::SnippetEditFieldDown,
                    (KeyCode::Backspace, _) => AppEvent::SnippetEditTextBackspace,
                    (KeyCode::Left, _) => AppEvent::SnippetEditTextLeft,
                    (KeyCode::Right, _) => AppEvent::SnippetEditTextRight,
                    (KeyCode::Char(c), m)
                        if !m.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                    {
                        AppEvent::SnippetEditTextChar(c)
                    }
                    _ => AppEvent::Noop,
                },
            };
        }
        // [6.2] Delete-confirm modal.
        if lib.delete_confirm.is_some() {
            return match key.code {
                KeyCode::Char('y') => AppEvent::LibraryDeleteConfirm,
                _ => AppEvent::LibraryDeleteCancel,
            };
        }
        // [6.3] Library snippets-drawer filter modal.
        if lib.snippets.filter.active {
            return match (key.code, key.modifiers) {
                (KeyCode::Esc, _) => AppEvent::SnippetsDrawerExitFilter,
                (KeyCode::Backspace, _) => AppEvent::SnippetsDrawerFilterBackspace,
                (KeyCode::Up, _) => AppEvent::SnippetsDrawerUp,
                (KeyCode::Down, _) => AppEvent::SnippetsDrawerDown,
                (KeyCode::Char(c), m)
                    if !m.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
                {
                    AppEvent::SnippetsDrawerFilterChar(c)
                }
                _ => AppEvent::Noop,
            };
        }
        // [6.4] Library browse dispatch.
        return match (key.code, key.modifiers) {
            (KeyCode::Char('l'), m) if m.contains(KeyModifiers::CONTROL) => AppEvent::ExitLibrary,
            (KeyCode::Esc, _) => AppEvent::ExitLibrary,
            (KeyCode::Up, _) => AppEvent::SnippetsDrawerUp,
            (KeyCode::Down, _) => AppEvent::SnippetsDrawerDown,
            (KeyCode::Char(' '), _) => AppEvent::SnippetsDrawerToggleCollapse,
            (KeyCode::Char('/'), _) => AppEvent::SnippetsDrawerEnterFilter,
            (KeyCode::Char('n'), m) if !m.contains(KeyModifiers::CONTROL) => AppEvent::LibraryNew,
            (KeyCode::Char('e'), m) if !m.contains(KeyModifiers::CONTROL) => {
                AppEvent::LibraryEditSelected
            }
            (KeyCode::Char('d'), m) if !m.contains(KeyModifiers::CONTROL) => {
                AppEvent::LibraryDeleteSelected
            }
            (KeyCode::Char('?'), _) => AppEvent::ToggleHelp,
            _ => AppEvent::Noop,
        };
    }

    // 7. Idle, or Filtering { accepted: true }.
    let accepted_filter = matches!(
        &app.browse_sub,
        BrowseSubMode::Filtering { accepted: true, .. }
    );

    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) => AppEvent::Quit,
        (KeyCode::Esc, _) => {
            if accepted_filter {
                AppEvent::ExitFilter
            } else {
                AppEvent::Quit
            }
        }
        (KeyCode::Char('?'), _) => AppEvent::ToggleHelp,
        (KeyCode::Char('/'), _) => AppEvent::EnterFilter,
        (KeyCode::Char('r'), _) => AppEvent::Refresh,
        (KeyCode::Char('l'), m) if m.contains(KeyModifiers::CONTROL) => AppEvent::EnterLibrary,
        (KeyCode::Char('e'), _) => AppEvent::EnterEditExisting,
        (KeyCode::Char('n'), _) => AppEvent::EnterEditNew,
        (KeyCode::Char('j'), _) => AppEvent::NavigateDown,
        (KeyCode::Down, _) => AppEvent::NavigateDown,
        (KeyCode::Char('k'), _) => AppEvent::NavigateUp,
        (KeyCode::Up, _) => AppEvent::NavigateUp,
        (KeyCode::Char('g'), m) if !m.contains(KeyModifiers::SHIFT) => AppEvent::NavigateTop,
        (KeyCode::Char('G'), _) => AppEvent::NavigateBottom,
        (KeyCode::Tab, _) => AppEvent::ToggleFocus,
        (KeyCode::Enter, _) => AppEvent::Launch,
        _ => AppEvent::Noop,
    }
}
