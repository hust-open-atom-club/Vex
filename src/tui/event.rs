#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppEvent {
    Quit,
    NavigateUp,
    NavigateDown,
    NavigateTop,
    NavigateBottom,
    ToggleFocus,
    Launch,
    Noop,
}

pub fn translate_key(key: crossterm::event::KeyEvent) -> AppEvent {
    use crossterm::event::{KeyCode, KeyEventKind, KeyModifiers};

    if key.kind != KeyEventKind::Press {
        return AppEvent::Noop;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) => AppEvent::Quit,
        (KeyCode::Esc, _) => AppEvent::Quit,
        (KeyCode::Char('c'), m) if m.contains(KeyModifiers::CONTROL) => AppEvent::Quit,
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
