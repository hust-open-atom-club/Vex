use super::app::{App, BrowseSubMode};

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
    Noop,
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
                // Reject control modifier combos to avoid eating Ctrl+X etc.
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

    // 5. Idle, or Filtering { accepted: true }.
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
