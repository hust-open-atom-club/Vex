use super::event::AppEvent;
use super::scan::ConfigEntry;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum Focus {
    #[default]
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageKind {
    Error,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppMessage {
    pub text: String,
    pub kind: MessageKind,
}

#[derive(Debug, Default)]
pub struct App {
    pub should_quit: bool,
    pub entries: Vec<ConfigEntry>,
    pub selected: usize,
    pub focus: Focus,
    pub right_scroll: u16,
    pub last_message: Option<AppMessage>,
    pub pending_launch: Option<usize>,
}

impl App {
    pub fn new(entries: Vec<ConfigEntry>) -> Self {
        Self {
            entries,
            ..Default::default()
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn current(&self) -> Option<&ConfigEntry> {
        self.entries.get(self.selected)
    }

    pub fn take_pending_launch(&mut self) -> Option<usize> {
        self.pending_launch.take()
    }

    pub fn set_error(&mut self, text: impl Into<String>) {
        self.last_message = Some(AppMessage {
            text: text.into(),
            kind: MessageKind::Error,
        });
    }

    pub fn set_info(&mut self, text: impl Into<String>) {
        self.last_message = Some(AppMessage {
            text: text.into(),
            kind: MessageKind::Info,
        });
    }

    /// Apply an AppEvent. Return true if a redraw is needed.
    pub fn handle_event(&mut self, event: AppEvent) -> bool {
        // Any non-Noop event clears the transient message. Done BEFORE
        // dispatching so Launch can set a new message on the same tick.
        if !matches!(event, AppEvent::Noop) {
            self.last_message = None;
        }

        match event {
            AppEvent::Quit => {
                self.should_quit = true;
                true
            }
            AppEvent::ToggleFocus => {
                self.focus = match self.focus {
                    Focus::Left => Focus::Right,
                    Focus::Right => Focus::Left,
                };
                true
            }
            AppEvent::Launch => {
                // Redundant safety: also clear here. See doc above.
                self.last_message = None;
                match self.entries.get(self.selected) {
                    Some(ConfigEntry::Ok { .. }) => {
                        self.pending_launch = Some(self.selected);
                    }
                    Some(ConfigEntry::Broken { name, .. }) => {
                        self.set_error(format!(
                            "Cannot launch '{}': configuration is broken",
                            name
                        ));
                    }
                    None => {
                        // Empty entries — no-op.
                    }
                }
                true
            }
            AppEvent::NavigateUp => match self.focus {
                Focus::Left => {
                    if self.is_empty() {
                        return false;
                    }
                    self.selected = self.selected.saturating_sub(1);
                    self.right_scroll = 0;
                    true
                }
                Focus::Right => {
                    self.right_scroll = self.right_scroll.saturating_sub(1);
                    true
                }
            },
            AppEvent::NavigateDown => match self.focus {
                Focus::Left => {
                    if self.is_empty() {
                        return false;
                    }
                    let last = self.len().saturating_sub(1);
                    self.selected = (self.selected + 1).min(last);
                    self.right_scroll = 0;
                    true
                }
                Focus::Right => {
                    self.right_scroll = self.right_scroll.saturating_add(1);
                    true
                }
            },
            AppEvent::NavigateTop => match self.focus {
                Focus::Left => {
                    if self.is_empty() {
                        return false;
                    }
                    self.selected = 0;
                    self.right_scroll = 0;
                    true
                }
                Focus::Right => {
                    self.right_scroll = 0;
                    true
                }
            },
            AppEvent::NavigateBottom => match self.focus {
                Focus::Left => {
                    if self.is_empty() {
                        return false;
                    }
                    self.selected = self.len().saturating_sub(1);
                    self.right_scroll = 0;
                    true
                }
                Focus::Right => {
                    self.right_scroll = u16::MAX;
                    true
                }
            },
            AppEvent::Noop => false,
        }
    }
}
