use super::event::AppEvent;
use super::scan::{self, ConfigEntry};

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

/// Browse-mode sub-state. Scope is limited to the Browse top-level mode —
/// Edit / Library modes (introduced in P4-7 / P4-9) will own their own
/// sub-state enums and must not reuse this one.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub enum BrowseSubMode {
    #[default]
    Idle,
    Filtering {
        query: String,
        accepted: bool,
    },
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
    pub browse_sub: BrowseSubMode,
    pub show_help: bool,
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

    /// Indices into `entries` of items that match the current filter.
    /// Idle or empty-query returns every index in order.
    pub fn visible_indices(&self) -> Vec<usize> {
        let q = match &self.browse_sub {
            BrowseSubMode::Filtering { query, .. } if !query.is_empty() => query.to_lowercase(),
            _ => return (0..self.entries.len()).collect(),
        };
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, e)| Self::matches_filter(e, &q))
            .map(|(i, _)| i)
            .collect()
    }

    fn matches_filter(entry: &ConfigEntry, query_lower: &str) -> bool {
        match entry {
            ConfigEntry::Ok { name, config, .. } => {
                if name.to_lowercase().contains(query_lower) {
                    return true;
                }
                config
                    .desc
                    .as_ref()
                    .map(|d| d.to_lowercase().contains(query_lower))
                    .unwrap_or(false)
            }
            ConfigEntry::Broken { name, .. } => name.to_lowercase().contains(query_lower),
        }
    }

    /// After the filter changes: keep selection on the same named entry if
    /// it is still visible, else fall back to the first visible entry.
    fn reselect_after_filter_change(&mut self) {
        let current_name = self.current().map(|e| e.name().to_string());
        let visible = self.visible_indices();
        if let Some(name) = current_name
            && let Some(&idx) = visible.iter().find(|&&i| self.entries[i].name() == name)
        {
            self.selected = idx;
            return;
        }
        self.selected = visible.first().copied().unwrap_or(0);
        self.right_scroll = 0;
    }

    fn navigate_in_visible(&mut self, delta: i32) -> bool {
        let visible = self.visible_indices();
        if visible.is_empty() {
            return false;
        }
        let pos = visible
            .iter()
            .position(|&i| i == self.selected)
            .unwrap_or(0);
        let new_pos = if delta >= 0 {
            (pos + delta as usize).min(visible.len() - 1)
        } else {
            pos.saturating_sub((-delta) as usize)
        };
        self.selected = visible[new_pos];
        true
    }

    fn jump_in_visible(&mut self, to_end: bool) -> bool {
        let visible = self.visible_indices();
        if visible.is_empty() {
            return false;
        }
        self.selected = if to_end {
            *visible.last().unwrap()
        } else {
            visible[0]
        };
        true
    }

    /// Apply an AppEvent. Return true if a redraw is needed.
    pub fn handle_event(&mut self, event: AppEvent) -> bool {
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
                    None => {}
                }
                true
            }
            AppEvent::NavigateUp => match self.focus {
                Focus::Left => {
                    let changed = self.navigate_in_visible(-1);
                    if changed {
                        self.right_scroll = 0;
                    }
                    changed
                }
                Focus::Right => {
                    self.right_scroll = self.right_scroll.saturating_sub(1);
                    true
                }
            },
            AppEvent::NavigateDown => match self.focus {
                Focus::Left => {
                    let changed = self.navigate_in_visible(1);
                    if changed {
                        self.right_scroll = 0;
                    }
                    changed
                }
                Focus::Right => {
                    self.right_scroll = self.right_scroll.saturating_add(1);
                    true
                }
            },
            AppEvent::NavigateTop => match self.focus {
                Focus::Left => {
                    let changed = self.jump_in_visible(false);
                    if changed {
                        self.right_scroll = 0;
                    }
                    changed
                }
                Focus::Right => {
                    self.right_scroll = 0;
                    true
                }
            },
            AppEvent::NavigateBottom => match self.focus {
                Focus::Left => {
                    let changed = self.jump_in_visible(true);
                    if changed {
                        self.right_scroll = 0;
                    }
                    changed
                }
                Focus::Right => {
                    self.right_scroll = u16::MAX;
                    true
                }
            },
            AppEvent::EnterFilter => {
                let existing = match &self.browse_sub {
                    BrowseSubMode::Filtering { query, .. } => query.clone(),
                    BrowseSubMode::Idle => String::new(),
                };
                self.browse_sub = BrowseSubMode::Filtering {
                    query: existing,
                    accepted: false,
                };
                true
            }
            AppEvent::ExitFilter => {
                self.browse_sub = BrowseSubMode::Idle;
                self.selected = 0;
                self.right_scroll = 0;
                true
            }
            AppEvent::AcceptFilter => {
                if let BrowseSubMode::Filtering { accepted, .. } = &mut self.browse_sub {
                    *accepted = true;
                }
                true
            }
            AppEvent::FilterChar(c) => {
                let mut changed = false;
                if let BrowseSubMode::Filtering { query, accepted } = &mut self.browse_sub
                    && !*accepted
                {
                    query.push(c);
                    changed = true;
                }
                if changed {
                    self.reselect_after_filter_change();
                }
                true
            }
            AppEvent::FilterBackspace => {
                let mut changed = false;
                if let BrowseSubMode::Filtering { query, accepted } = &mut self.browse_sub
                    && !*accepted
                {
                    query.pop();
                    changed = true;
                }
                if changed {
                    self.reselect_after_filter_change();
                }
                true
            }
            AppEvent::ToggleHelp => {
                self.show_help = !self.show_help;
                true
            }
            AppEvent::DismissHelp => {
                self.show_help = false;
                true
            }
            AppEvent::Refresh => {
                match scan::scan_configs() {
                    Ok(new_entries) => {
                        let current_name = self.current().map(|e| e.name().to_string());
                        let new_len = new_entries.len();
                        self.entries = new_entries;
                        self.right_scroll = 0;
                        self.selected = if let Some(name) = current_name {
                            self.entries
                                .iter()
                                .position(|e| e.name() == name)
                                .unwrap_or(0)
                        } else {
                            0
                        };
                        if matches!(self.browse_sub, BrowseSubMode::Filtering { .. }) {
                            self.reselect_after_filter_change();
                        }
                        self.set_info(format!("Reloaded: {} configurations", new_len));
                    }
                    Err(e) => {
                        self.set_error(format!("Refresh failed: {}", e));
                    }
                }
                true
            }
            AppEvent::Noop => false,
        }
    }
}
