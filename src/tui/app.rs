use super::event::AppEvent;
use super::scan::{self, ConfigEntry};
use crate::config::QemuConfig;

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

/// Identifies which field has keyboard focus in Edit mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EditField {
    Name,
    QemuBin,
    Description,
    Args,
}

impl EditField {
    /// Tab forward cycle: Name → QemuBin → Description → Args → Name → …
    pub fn next(self) -> Self {
        match self {
            EditField::Name => EditField::QemuBin,
            EditField::QemuBin => EditField::Description,
            EditField::Description => EditField::Args,
            EditField::Args => EditField::Name,
        }
    }

    /// Shift+Tab backward cycle.
    pub fn prev(self) -> Self {
        match self {
            EditField::Name => EditField::Args,
            EditField::QemuBin => EditField::Name,
            EditField::Description => EditField::QemuBin,
            EditField::Args => EditField::Description,
        }
    }
}

/// A single-line text input with cursor support. Cursor is a byte
/// offset into `value` and is always kept on a char boundary by every
/// mutator on this type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextField {
    pub value: String,
    pub cursor: usize,
}

impl TextField {
    pub fn new(initial: &str) -> Self {
        Self {
            value: initial.to_string(),
            cursor: initial.len(),
        }
    }

    pub fn insert_char(&mut self, c: char) {
        self.value.insert(self.cursor, c);
        self.cursor += c.len_utf8();
    }

    pub fn backspace(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut new_cursor = self.cursor - 1;
        while new_cursor > 0 && !self.value.is_char_boundary(new_cursor) {
            new_cursor -= 1;
        }
        self.value.replace_range(new_cursor..self.cursor, "");
        self.cursor = new_cursor;
    }

    pub fn move_left(&mut self) {
        if self.cursor == 0 {
            return;
        }
        let mut new_cursor = self.cursor - 1;
        while new_cursor > 0 && !self.value.is_char_boundary(new_cursor) {
            new_cursor -= 1;
        }
        self.cursor = new_cursor;
    }

    pub fn move_right(&mut self) {
        if self.cursor >= self.value.len() {
            return;
        }
        let mut new_cursor = self.cursor + 1;
        while new_cursor < self.value.len() && !self.value.is_char_boundary(new_cursor) {
            new_cursor += 1;
        }
        self.cursor = new_cursor;
    }
}

/// Whether the current Edit session creates a new configuration or
/// updates an existing one. `Update` carries the original name so rename
/// detection on save can locate the old file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditMode {
    Create,
    Update { original_name: String },
}

/// Sentinel for an in-progress "discard unsaved changes?" prompt.
/// Only one variant today; left as an enum for forward-compat.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExitConfirm {
    Pending,
}

/// Which large pane has keyboard focus in Edit mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum EditFocus {
    #[default]
    Editor,
    SnippetsDrawer,
}

/// One row of the Snippets drawer display.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawerRow {
    CategoryHeader {
        category: crate::snippets::SnippetCategory,
        collapsed: bool,
        snippet_count: usize,
    },
    Snippet {
        snippet_index: usize,
    },
}

/// Snippets-drawer filter input state.
#[derive(Debug, Clone, Default)]
pub struct SnippetFilter {
    pub active: bool,
    pub query: TextField,
}

/// State of the Snippets drawer (right pane of Edit mode).
#[derive(Debug, Clone)]
pub struct SnippetsDrawerState {
    pub snippets: Vec<crate::snippets::Snippet>,
    pub load_error: Option<String>,
    pub selected: usize,
    /// Categories the user has collapsed. Stored as a Vec (max 8 entries,
    /// linear lookup is fine) because SnippetCategory doesn't derive Hash
    /// and the snippets module is read-only at this phase.
    pub collapsed: Vec<crate::snippets::SnippetCategory>,
    pub filter: SnippetFilter,
}

impl SnippetsDrawerState {
    /// Load merged snippets. On user-file error, fall back to builtin
    /// and remember the error so the drawer title can surface it.
    pub fn load() -> Self {
        match crate::snippets::load_merged() {
            Ok(snippets) => Self {
                snippets,
                load_error: None,
                selected: 0,
                collapsed: Vec::new(),
                filter: SnippetFilter::default(),
            },
            Err(e) => Self {
                snippets: crate::snippets::builtin_snippets(),
                load_error: Some(format!("User snippets unavailable: {}", e)),
                selected: 0,
                collapsed: Vec::new(),
                filter: SnippetFilter::default(),
            },
        }
    }

    fn is_collapsed(&self, cat: &crate::snippets::SnippetCategory) -> bool {
        self.collapsed.contains(cat)
    }

    fn toggle_collapse(&mut self, cat: crate::snippets::SnippetCategory) {
        if let Some(idx) = self.collapsed.iter().position(|c| *c == cat) {
            self.collapsed.remove(idx);
        } else {
            self.collapsed.push(cat);
        }
    }

    /// Ordered list of display rows. Includes a CategoryHeader for every
    /// category that has at least one (filter-matched) snippet; if the
    /// category is not collapsed, also emits its snippet rows.
    pub fn visible_rows(&self) -> Vec<DrawerRow> {
        use crate::snippets::SnippetCategory;
        let categories = [
            SnippetCategory::Memory,
            SnippetCategory::Cpu,
            SnippetCategory::Machine,
            SnippetCategory::Storage,
            SnippetCategory::Network,
            SnippetCategory::Display,
            SnippetCategory::Debug,
            SnippetCategory::Kernel,
        ];

        let q = if self.filter.active && !self.filter.query.value.is_empty() {
            Some(self.filter.query.value.to_lowercase())
        } else {
            None
        };

        let snippet_matches = |s: &crate::snippets::Snippet| -> bool {
            let Some(q) = &q else { return true };
            if s.name.to_lowercase().contains(q) {
                return true;
            }
            s.description
                .as_ref()
                .map(|d| d.to_lowercase().contains(q))
                .unwrap_or(false)
        };

        let mut rows = Vec::new();
        for cat in &categories {
            let cat_snippets: Vec<(usize, &crate::snippets::Snippet)> = self
                .snippets
                .iter()
                .enumerate()
                .filter(|(_, s)| s.category == *cat)
                .collect();
            let total_count = cat_snippets.len();
            if total_count == 0 {
                continue;
            }
            let matching: Vec<(usize, &crate::snippets::Snippet)> = cat_snippets
                .into_iter()
                .filter(|(_, s)| snippet_matches(s))
                .collect();
            if q.is_some() && matching.is_empty() {
                continue;
            }

            let collapsed = self.is_collapsed(cat);
            rows.push(DrawerRow::CategoryHeader {
                category: *cat,
                collapsed,
                snippet_count: total_count,
            });
            if !collapsed {
                for (idx, _) in matching {
                    rows.push(DrawerRow::Snippet { snippet_index: idx });
                }
            }
        }
        rows
    }

    pub fn current_row(&self) -> Option<DrawerRow> {
        self.visible_rows().get(self.selected).cloned()
    }

    pub fn current_snippet(&self) -> Option<&crate::snippets::Snippet> {
        match self.current_row()? {
            DrawerRow::Snippet { snippet_index } => self.snippets.get(snippet_index),
            DrawerRow::CategoryHeader { .. } => None,
        }
    }

    pub fn clamp_selected(&mut self) {
        let len = self.visible_rows().len();
        if len == 0 {
            self.selected = 0;
        } else if self.selected >= len {
            self.selected = len - 1;
        }
    }
}

/// In-progress edit session buffer.
#[derive(Debug, Clone)]
pub struct EditState {
    pub mode: EditMode,
    pub name: TextField,
    pub qemu_bin: TextField,
    pub description: TextField,
    pub args: Vec<String>,
    pub focused_field: EditField,
    pub args_selected: usize,
    pub dirty: bool,
    pub exit_confirm: Option<ExitConfirm>,
    /// Which large pane has focus: Editor (text fields + args list) or
    /// SnippetsDrawer.
    pub focus: EditFocus,
    /// Snippets drawer state.
    pub snippets: SnippetsDrawerState,
    /// When `Some`, the currently-selected arg in the Args field is being
    /// edited in place via a TextField overlay. Set by `EditArgsAddEmpty`
    /// or `EditArgsEnterToken`; committed back to `args[args_selected]`
    /// by `commit_token_edit()`.
    pub token_edit: Option<TextField>,
    /// Original resources & qemu_version preserved so Update-mode save
    /// doesn't drop fields P4-7 doesn't yet edit.
    pub(crate) preserved_resources: std::collections::HashMap<String, crate::config::ResourceRef>,
    pub(crate) preserved_qemu_version: Option<String>,

    // Snapshots for dirty detection.
    initial_name: String,
    initial_qemu_bin: String,
    initial_description: String,
    initial_args: Vec<String>,
}

impl EditState {
    pub fn from_config(config: &QemuConfig, name: &str) -> Self {
        let description = config.desc.clone().unwrap_or_default();
        Self {
            mode: EditMode::Update {
                original_name: name.to_string(),
            },
            name: TextField::new(name),
            qemu_bin: TextField::new(&config.qemu_bin),
            description: TextField::new(&description),
            args: config.args.clone(),
            focused_field: EditField::Name,
            args_selected: 0,
            dirty: false,
            exit_confirm: None,
            focus: EditFocus::Editor,
            snippets: SnippetsDrawerState::load(),
            token_edit: None,
            preserved_resources: config.resources.clone(),
            preserved_qemu_version: config.qemu_version.clone(),
            initial_name: name.to_string(),
            initial_qemu_bin: config.qemu_bin.clone(),
            initial_description: description,
            initial_args: config.args.clone(),
        }
    }

    pub fn new_empty() -> Self {
        Self {
            mode: EditMode::Create,
            name: TextField::default(),
            qemu_bin: TextField::default(),
            description: TextField::default(),
            args: Vec::new(),
            focused_field: EditField::Name,
            args_selected: 0,
            dirty: false,
            exit_confirm: None,
            focus: EditFocus::Editor,
            snippets: SnippetsDrawerState::load(),
            token_edit: None,
            preserved_resources: std::collections::HashMap::new(),
            preserved_qemu_version: None,
            initial_name: String::new(),
            initial_qemu_bin: String::new(),
            initial_description: String::new(),
            initial_args: Vec::new(),
        }
    }

    pub fn recompute_dirty(&mut self) {
        self.dirty = self.name.value != self.initial_name
            || self.qemu_bin.value != self.initial_qemu_bin
            || self.description.value != self.initial_description
            || self.args != self.initial_args;
    }

    /// Commit the current `token_edit` buffer back into `args[args_selected]`
    /// and clear `token_edit`. No-op when `token_edit` is `None`.
    pub fn commit_token_edit(&mut self) {
        if let Some(token) = self.token_edit.take()
            && self.args_selected < self.args.len()
        {
            self.args[self.args_selected] = token.value;
            self.recompute_dirty();
        }
    }
}

// =========================================================================
// P4-9: Library mode types
// =========================================================================

/// Mirror of [`EditMode`] for Library-mode snippet edits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SnippetEditMode {
    Create,
    Update { original_name: String },
}

/// The four fields available in the Library snippet editor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnippetEditField {
    Name,
    Category,
    Description,
    Args,
}

impl SnippetEditField {
    pub fn next(self) -> Self {
        match self {
            Self::Name => Self::Category,
            Self::Category => Self::Description,
            Self::Description => Self::Args,
            Self::Args => Self::Name,
        }
    }
    pub fn prev(self) -> Self {
        match self {
            Self::Name => Self::Args,
            Self::Category => Self::Name,
            Self::Description => Self::Category,
            Self::Args => Self::Description,
        }
    }
}

/// In-progress user-snippet edit session (nested inside LibraryState).
#[derive(Debug, Clone)]
pub struct SnippetEditState {
    pub mode: SnippetEditMode,
    pub name: TextField,
    pub category: crate::snippets::SnippetCategory,
    pub description: TextField,
    pub args: Vec<String>,
    pub focused_field: SnippetEditField,
    pub args_selected: usize,
    /// When Some, the user is editing a single arg token inline.
    pub token_edit: Option<TextField>,
    pub dirty: bool,
    pub exit_confirm: Option<ExitConfirm>,

    initial_name: String,
    initial_category: crate::snippets::SnippetCategory,
    initial_description: String,
    initial_args: Vec<String>,
}

impl SnippetEditState {
    pub fn from_snippet(snippet: &crate::snippets::Snippet) -> Self {
        let desc = snippet.description.clone().unwrap_or_default();
        Self {
            mode: SnippetEditMode::Update {
                original_name: snippet.name.clone(),
            },
            name: TextField::new(&snippet.name),
            category: snippet.category,
            description: TextField::new(&desc),
            args: snippet.args.clone(),
            focused_field: SnippetEditField::Name,
            args_selected: 0,
            token_edit: None,
            dirty: false,
            exit_confirm: None,
            initial_name: snippet.name.clone(),
            initial_category: snippet.category,
            initial_description: desc,
            initial_args: snippet.args.clone(),
        }
    }

    pub fn new_empty() -> Self {
        Self {
            mode: SnippetEditMode::Create,
            name: TextField::default(),
            category: crate::snippets::SnippetCategory::Memory,
            description: TextField::default(),
            args: Vec::new(),
            focused_field: SnippetEditField::Name,
            args_selected: 0,
            token_edit: None,
            dirty: false,
            exit_confirm: None,
            initial_name: String::new(),
            initial_category: crate::snippets::SnippetCategory::Memory,
            initial_description: String::new(),
            initial_args: Vec::new(),
        }
    }

    pub fn recompute_dirty(&mut self) {
        self.dirty = self.name.value != self.initial_name
            || self.category != self.initial_category
            || self.description.value != self.initial_description
            || self.args != self.initial_args;
    }

    pub fn category_next(&mut self) {
        use crate::snippets::SnippetCategory::*;
        self.category = match self.category {
            Memory => Cpu,
            Cpu => Machine,
            Machine => Storage,
            Storage => Network,
            Network => Display,
            Display => Debug,
            Debug => Kernel,
            Kernel => Memory,
        };
        self.recompute_dirty();
    }

    pub fn category_prev(&mut self) {
        use crate::snippets::SnippetCategory::*;
        self.category = match self.category {
            Memory => Kernel,
            Cpu => Memory,
            Machine => Cpu,
            Storage => Machine,
            Network => Storage,
            Display => Network,
            Debug => Display,
            Kernel => Debug,
        };
        self.recompute_dirty();
    }

    pub fn commit_token_edit(&mut self) {
        if let Some(token) = self.token_edit.take()
            && self.args_selected < self.args.len()
        {
            self.args[self.args_selected] = token.value;
            self.recompute_dirty();
        }
    }
}

/// Pending deletion confirmation inside Library mode. `snippet_index`
/// is the position into `SnippetsDrawerState::snippets`; `snippet_name`
/// is denormalised here so the confirm prompt can render the name
/// without re-looking-up the row.
#[derive(Debug, Clone)]
pub struct DeleteConfirm {
    pub snippet_index: usize,
    pub snippet_name: String,
}

/// Library mode top-level state.
#[derive(Debug, Clone)]
pub struct LibraryState {
    pub snippets: SnippetsDrawerState,
    pub edit: Option<SnippetEditState>,
    pub delete_confirm: Option<DeleteConfirm>,
    /// Snapshot of the on-disk user snippets list. The persistence baseline
    /// for save/delete: we mutate this in place and write it back, instead
    /// of reconstructing the user list by filtering merged snippets through
    /// `is_builtin_snippet_name` (which would drop any user override whose
    /// name collides with a builtin).
    pub user_only: Vec<crate::snippets::Snippet>,
}

impl LibraryState {
    pub fn enter() -> Self {
        let user_only = crate::snippets::load_user_snippets().unwrap_or_default();
        Self {
            snippets: SnippetsDrawerState::load(),
            edit: None,
            delete_confirm: None,
            user_only,
        }
    }
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
    pub edit: Option<EditState>,
    pub library: Option<LibraryState>,
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

        // Edit mode gate: when an Edit session is active, only Edit-related
        // events apply. Ctrl+C (mapped to Quit) is the universal escape
        // hatch and discards the in-progress edit.
        if self.edit.is_some() {
            if matches!(event, AppEvent::Quit) {
                self.should_quit = true;
                self.edit = None;
                return true;
            }
            if !event.is_edit_event() {
                // Browse-only events (NavigateUp, EnterFilter, etc.) are
                // ignored. EnterEditExisting / EnterEditNew shouldn't reach
                // us here because translate_key only emits them when
                // app.edit is None.
                return true;
            }
        }

        // Library mode gate (P4-9). Ctrl+C escape-hatches as in Edit.
        if self.library.is_some() {
            if matches!(event, AppEvent::Quit) {
                self.should_quit = true;
                self.library = None;
                return true;
            }
            // Allow: library-specific events, shared drawer events, help.
            if !event.is_library_event()
                && !event.is_drawer_event()
                && !matches!(event, AppEvent::ToggleHelp | AppEvent::DismissHelp)
            {
                return true;
            }
        } else if event.is_library_event() && !matches!(event, AppEvent::EnterLibrary) {
            // Library-only event with no library active — drop silently.
            // EnterLibrary is the one exception: it BRINGS library into
            // existence and must be allowed through.
            return true;
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

                // Guard: refuse to launch if the current selection isn't
                // visible under the active filter. Protects users who accept
                // a filter with zero matches, and defends against any future
                // path that drifts selection out of view. Only runs in
                // filter mode — the Idle empty-entries path stays a no-op
                // to preserve P4-4 behaviour.
                if matches!(self.browse_sub, BrowseSubMode::Filtering { .. }) {
                    let visible = self.visible_indices();
                    if !visible.contains(&self.selected) {
                        if visible.is_empty() {
                            self.set_error("No configurations match the current filter");
                        } else {
                            self.set_error("Selected configuration is not visible");
                        }
                        return true;
                    }
                }

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
            // --- Edit mode entry from Browse -------------------------------
            AppEvent::EnterEditExisting => {
                if let Some(ConfigEntry::Ok { config, name, .. }) = self.entries.get(self.selected)
                {
                    self.edit = Some(EditState::from_config(config, name));
                } else {
                    self.set_error("Cannot edit: select an Ok configuration first");
                }
                true
            }
            AppEvent::EnterEditNew => {
                self.edit = Some(EditState::new_empty());
                true
            }
            // --- Edit mode internals -------------------------------------
            // P4-8: Tab and Shift+Tab swap the Editor ↔ SnippetsDrawer pane.
            // (Field cycling moved to ↑/↓ via EditFieldUp/EditFieldDown.)
            AppEvent::EditTab | AppEvent::EditShiftTab => {
                if let Some(edit) = &mut self.edit {
                    edit.focus = match edit.focus {
                        EditFocus::Editor => EditFocus::SnippetsDrawer,
                        EditFocus::SnippetsDrawer => EditFocus::Editor,
                    };
                    if edit.focus == EditFocus::Editor {
                        // Leaving the drawer drops any half-typed filter.
                        edit.snippets.filter.active = false;
                        edit.snippets.filter.query = TextField::default();
                        edit.snippets.clamp_selected();
                    }
                }
                true
            }
            AppEvent::EditFieldUp => {
                if let Some(edit) = &mut self.edit
                    && edit.focus == EditFocus::Editor
                {
                    edit.focused_field = edit.focused_field.prev();
                }
                true
            }
            AppEvent::EditFieldDown => {
                if let Some(edit) = &mut self.edit
                    && edit.focus == EditFocus::Editor
                {
                    edit.focused_field = edit.focused_field.next();
                }
                true
            }
            AppEvent::EditTextChar(c) => {
                if let Some(edit) = &mut self.edit {
                    match edit.focused_field {
                        EditField::Name => edit.name.insert_char(c),
                        EditField::QemuBin => edit.qemu_bin.insert_char(c),
                        EditField::Description => edit.description.insert_char(c),
                        EditField::Args => return true,
                    }
                    edit.recompute_dirty();
                }
                true
            }
            AppEvent::EditTextBackspace => {
                if let Some(edit) = &mut self.edit {
                    match edit.focused_field {
                        EditField::Name => edit.name.backspace(),
                        EditField::QemuBin => edit.qemu_bin.backspace(),
                        EditField::Description => edit.description.backspace(),
                        EditField::Args => return true,
                    }
                    edit.recompute_dirty();
                }
                true
            }
            AppEvent::EditTextLeft => {
                if let Some(edit) = &mut self.edit {
                    match edit.focused_field {
                        EditField::Name => edit.name.move_left(),
                        EditField::QemuBin => edit.qemu_bin.move_left(),
                        EditField::Description => edit.description.move_left(),
                        EditField::Args => {}
                    }
                }
                true
            }
            AppEvent::EditTextRight => {
                if let Some(edit) = &mut self.edit {
                    match edit.focused_field {
                        EditField::Name => edit.name.move_right(),
                        EditField::QemuBin => edit.qemu_bin.move_right(),
                        EditField::Description => edit.description.move_right(),
                        EditField::Args => {}
                    }
                }
                true
            }
            AppEvent::EditArgsUp => {
                if let Some(edit) = &mut self.edit
                    && !edit.args.is_empty()
                {
                    edit.args_selected = edit.args_selected.saturating_sub(1);
                }
                true
            }
            AppEvent::EditArgsDown => {
                if let Some(edit) = &mut self.edit
                    && !edit.args.is_empty()
                {
                    edit.args_selected = (edit.args_selected + 1).min(edit.args.len() - 1);
                }
                true
            }
            AppEvent::EditArgsMoveUp => {
                if let Some(edit) = &mut self.edit
                    && !edit.args.is_empty()
                    && edit.args_selected > 0
                {
                    edit.args.swap(edit.args_selected, edit.args_selected - 1);
                    edit.args_selected -= 1;
                    edit.recompute_dirty();
                }
                true
            }
            AppEvent::EditArgsMoveDown => {
                if let Some(edit) = &mut self.edit
                    && !edit.args.is_empty()
                    && edit.args_selected + 1 < edit.args.len()
                {
                    edit.args.swap(edit.args_selected, edit.args_selected + 1);
                    edit.args_selected += 1;
                    edit.recompute_dirty();
                }
                true
            }
            AppEvent::EditArgsDelete => {
                if let Some(edit) = &mut self.edit
                    && !edit.args.is_empty()
                {
                    edit.args.remove(edit.args_selected);
                    if edit.args.is_empty() {
                        edit.args_selected = 0;
                    } else if edit.args_selected >= edit.args.len() {
                        edit.args_selected = edit.args.len() - 1;
                    }
                    edit.recompute_dirty();
                }
                true
            }
            // P4-10: args token editing -----------------------------------
            AppEvent::EditArgsAddEmpty => {
                if let Some(edit) = &mut self.edit
                    && edit.focus == EditFocus::Editor
                    && edit.focused_field == EditField::Args
                {
                    let insert_at = if edit.args.is_empty() {
                        0
                    } else {
                        edit.args_selected + 1
                    };
                    edit.args.insert(insert_at, String::new());
                    edit.args_selected = insert_at;
                    edit.token_edit = Some(TextField::default());
                    edit.recompute_dirty();
                }
                true
            }
            AppEvent::EditArgsEnterToken => {
                if let Some(edit) = &mut self.edit
                    && edit.args_selected < edit.args.len()
                {
                    let current = edit.args[edit.args_selected].clone();
                    edit.token_edit = Some(TextField::new(&current));
                }
                true
            }
            AppEvent::EditTokenChar(c) => {
                if let Some(edit) = &mut self.edit
                    && let Some(field) = &mut edit.token_edit
                {
                    field.insert_char(c);
                }
                true
            }
            AppEvent::EditTokenBackspace => {
                if let Some(edit) = &mut self.edit
                    && let Some(field) = &mut edit.token_edit
                {
                    field.backspace();
                }
                true
            }
            AppEvent::EditTokenLeft => {
                if let Some(edit) = &mut self.edit
                    && let Some(field) = &mut edit.token_edit
                {
                    field.move_left();
                }
                true
            }
            AppEvent::EditTokenRight => {
                if let Some(edit) = &mut self.edit
                    && let Some(field) = &mut edit.token_edit
                {
                    field.move_right();
                }
                true
            }
            AppEvent::EditTokenCommit => {
                if let Some(edit) = &mut self.edit {
                    edit.commit_token_edit();
                }
                true
            }
            AppEvent::EditSave => {
                // Commit any in-progress token edit so Ctrl+S from inside the
                // token-edit overlay does not silently drop the buffered text.
                if let Some(edit) = &mut self.edit {
                    edit.commit_token_edit();
                }
                self.try_save_edit();
                true
            }
            AppEvent::EditCancel => {
                if let Some(edit) = &mut self.edit {
                    if edit.dirty {
                        edit.exit_confirm = Some(ExitConfirm::Pending);
                    } else {
                        self.edit = None;
                    }
                }
                true
            }
            AppEvent::EditConfirmSave => {
                if let Some(edit) = &mut self.edit {
                    edit.exit_confirm = None;
                }
                self.try_save_edit();
                true
            }
            AppEvent::EditConfirmDiscard => {
                self.edit = None;
                true
            }
            AppEvent::EditConfirmCancel => {
                if let Some(edit) = &mut self.edit {
                    edit.exit_confirm = None;
                }
                true
            }
            // --- Snippets drawer (shared between Edit mode + Library mode) ---
            AppEvent::SnippetsDrawerUp => {
                if let Some(d) = self.active_drawer_mut() {
                    d.selected = d.selected.saturating_sub(1);
                }
                true
            }
            AppEvent::SnippetsDrawerDown => {
                if let Some(d) = self.active_drawer_mut() {
                    let len = d.visible_rows().len();
                    if len > 0 && d.selected + 1 < len {
                        d.selected += 1;
                    }
                }
                true
            }
            AppEvent::SnippetsDrawerToggleCollapse => {
                if let Some(d) = self.active_drawer_mut() {
                    let cat_to_toggle = match d.current_row() {
                        Some(DrawerRow::CategoryHeader { category, .. }) => Some(category),
                        Some(DrawerRow::Snippet { snippet_index }) => {
                            d.snippets.get(snippet_index).map(|s| s.category)
                        }
                        None => None,
                    };
                    if let Some(cat) = cat_to_toggle {
                        d.toggle_collapse(cat);
                        let rows = d.visible_rows();
                        if let Some(idx) = rows.iter().position(|r| {
                            matches!(r, DrawerRow::CategoryHeader { category, .. } if *category == cat)
                        }) {
                            d.selected = idx;
                        } else {
                            d.clamp_selected();
                        }
                    }
                }
                true
            }
            AppEvent::SnippetsDrawerEnterFilter => {
                if let Some(d) = self.active_drawer_mut() {
                    d.filter.active = true;
                }
                true
            }
            AppEvent::SnippetsDrawerExitFilter => {
                if let Some(d) = self.active_drawer_mut() {
                    d.filter.active = false;
                    d.filter.query = TextField::default();
                    d.clamp_selected();
                }
                true
            }
            AppEvent::SnippetsDrawerFilterChar(c) => {
                if let Some(d) = self.active_drawer_mut() {
                    d.filter.query.insert_char(c);
                    d.clamp_selected();
                }
                true
            }
            AppEvent::SnippetsDrawerFilterBackspace => {
                if let Some(d) = self.active_drawer_mut() {
                    d.filter.query.backspace();
                    d.clamp_selected();
                }
                true
            }
            AppEvent::SnippetsDrawerInsert => {
                let to_insert: Option<crate::snippets::Snippet> =
                    self.edit
                        .as_ref()
                        .and_then(|edit| match edit.snippets.current_row()? {
                            DrawerRow::Snippet { snippet_index } => {
                                edit.snippets.snippets.get(snippet_index).cloned()
                            }
                            DrawerRow::CategoryHeader { .. } => None,
                        });
                if let Some(snippet) = to_insert {
                    if let Some(edit) = &mut self.edit {
                        let insert_at = if edit.args.is_empty() {
                            0
                        } else {
                            edit.args_selected + 1
                        };
                        for (offset, token) in snippet.args.iter().enumerate() {
                            edit.args.insert(insert_at + offset, token.clone());
                        }
                        if !snippet.args.is_empty() {
                            edit.args_selected = insert_at + snippet.args.len() - 1;
                        }
                        edit.recompute_dirty();
                    }
                    self.set_info(format!("Inserted snippet '{}'", snippet.name));
                }
                true
            }
            // --- P4-9: Library mode entry / exit ----------------------
            AppEvent::EnterLibrary => {
                self.library = Some(LibraryState::enter());
                true
            }
            AppEvent::ExitLibrary => {
                self.library = None;
                true
            }
            // --- P4-9: Library CRUD triggers --------------------------
            AppEvent::LibraryNew => {
                if let Some(lib) = &mut self.library {
                    lib.edit = Some(SnippetEditState::new_empty());
                }
                true
            }
            AppEvent::LibraryEditSelected => {
                // A merged row is "manageable" when an entry with the same name
                // exists in user_only — i.e. it is either a pure user snippet
                // or an override of a builtin. Pure builtins (no override yet)
                // are blocked so the user is nudged to create an override
                // explicitly. Classifying by name vs. the builtin set is wrong
                // because it would also block overrides whose name collides
                // with a builtin.
                let target: Option<(crate::snippets::Snippet, bool)> = self
                    .library
                    .as_ref()
                    .and_then(|lib| match lib.snippets.current_row()? {
                        DrawerRow::Snippet { snippet_index } => {
                            let s = lib.snippets.snippets.get(snippet_index)?.clone();
                            let is_pure_builtin = !lib.user_only.iter().any(|u| u.name == s.name);
                            Some((s, is_pure_builtin))
                        }
                        DrawerRow::CategoryHeader { .. } => None,
                    });
                match target {
                    Some((_, true)) => {
                        self.set_error(
                            "'e' and 'd' are disabled for builtin snippets. \
                             Press 'n' to create your own.",
                        );
                    }
                    Some((snippet, false)) => {
                        if let Some(lib) = &mut self.library {
                            lib.edit = Some(SnippetEditState::from_snippet(&snippet));
                        }
                    }
                    None => {}
                }
                true
            }
            AppEvent::LibraryDeleteSelected => {
                let target: Option<(usize, String, bool)> =
                    self.library
                        .as_ref()
                        .and_then(|lib| match lib.snippets.current_row()? {
                            DrawerRow::Snippet { snippet_index } => {
                                let s = lib.snippets.snippets.get(snippet_index)?.clone();
                                let is_pure_builtin =
                                    !lib.user_only.iter().any(|u| u.name == s.name);
                                Some((snippet_index, s.name, is_pure_builtin))
                            }
                            DrawerRow::CategoryHeader { .. } => None,
                        });
                match target {
                    Some((_, _, true)) => {
                        self.set_error(
                            "'e' and 'd' are disabled for builtin snippets. \
                             Press 'n' to create your own.",
                        );
                    }
                    Some((idx, name, false)) => {
                        if let Some(lib) = &mut self.library {
                            lib.delete_confirm = Some(DeleteConfirm {
                                snippet_index: idx,
                                snippet_name: name,
                            });
                        }
                    }
                    None => {}
                }
                true
            }
            AppEvent::LibraryDeleteConfirm => {
                let to_remove: Option<String> = self
                    .library
                    .as_mut()
                    .and_then(|lib| lib.delete_confirm.take().map(|c| c.snippet_name));
                if let Some(name) = to_remove {
                    let mut user_only: Vec<crate::snippets::Snippet> = self
                        .library
                        .as_ref()
                        .map(|lib| lib.user_only.clone())
                        .unwrap_or_default();
                    let pos = user_only.iter().position(|s| s.name == name);
                    match pos {
                        Some(idx) => {
                            user_only.remove(idx);
                            match crate::snippets::save_user_snippets(&user_only) {
                                Ok(()) => {
                                    if let Some(lib) = &mut self.library {
                                        lib.user_only = user_only;
                                        lib.snippets = SnippetsDrawerState::load();
                                    }
                                    self.set_info(format!("Deleted '{}'", name));
                                }
                                Err(e) => {
                                    self.set_error(format!("Failed to write snippets.json: {}", e));
                                }
                            }
                        }
                        None => {
                            // Defensive: LibraryDeleteSelected already blocks
                            // pure builtins before delete_confirm is set, so
                            // this branch should be unreachable. Refuse the
                            // operation rather than panic if the invariant
                            // ever breaks.
                            self.set_error(
                                "Cannot delete built-in snippet; create an override to customize.",
                            );
                        }
                    }
                }
                true
            }
            AppEvent::LibraryDeleteCancel => {
                if let Some(lib) = &mut self.library {
                    lib.delete_confirm = None;
                }
                true
            }
            // --- P4-9: SnippetEdit handlers ---------------------------
            AppEvent::SnippetEditFieldUp => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    s.focused_field = s.focused_field.prev();
                }
                true
            }
            AppEvent::SnippetEditFieldDown => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    s.focused_field = s.focused_field.next();
                }
                true
            }
            AppEvent::SnippetEditTextChar(c) => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    match s.focused_field {
                        SnippetEditField::Name => s.name.insert_char(c),
                        SnippetEditField::Description => s.description.insert_char(c),
                        _ => return true,
                    }
                    s.recompute_dirty();
                }
                true
            }
            AppEvent::SnippetEditTextBackspace => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    match s.focused_field {
                        SnippetEditField::Name => s.name.backspace(),
                        SnippetEditField::Description => s.description.backspace(),
                        _ => return true,
                    }
                    s.recompute_dirty();
                }
                true
            }
            AppEvent::SnippetEditTextLeft => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    match s.focused_field {
                        SnippetEditField::Name => s.name.move_left(),
                        SnippetEditField::Description => s.description.move_left(),
                        _ => {}
                    }
                }
                true
            }
            AppEvent::SnippetEditTextRight => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    match s.focused_field {
                        SnippetEditField::Name => s.name.move_right(),
                        SnippetEditField::Description => s.description.move_right(),
                        _ => {}
                    }
                }
                true
            }
            AppEvent::SnippetEditCategoryPrev => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    s.category_prev();
                }
                true
            }
            AppEvent::SnippetEditCategoryNext => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    s.category_next();
                }
                true
            }
            AppEvent::SnippetEditArgsUp => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && !s.args.is_empty()
                {
                    s.args_selected = s.args_selected.saturating_sub(1);
                }
                true
            }
            AppEvent::SnippetEditArgsDown => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && !s.args.is_empty()
                {
                    s.args_selected = (s.args_selected + 1).min(s.args.len() - 1);
                }
                true
            }
            AppEvent::SnippetEditArgsMoveUp => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && !s.args.is_empty()
                    && s.args_selected > 0
                {
                    s.args.swap(s.args_selected, s.args_selected - 1);
                    s.args_selected -= 1;
                    s.recompute_dirty();
                }
                true
            }
            AppEvent::SnippetEditArgsMoveDown => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && !s.args.is_empty()
                    && s.args_selected + 1 < s.args.len()
                {
                    s.args.swap(s.args_selected, s.args_selected + 1);
                    s.args_selected += 1;
                    s.recompute_dirty();
                }
                true
            }
            AppEvent::SnippetEditArgsDelete => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && !s.args.is_empty()
                {
                    s.args.remove(s.args_selected);
                    if s.args.is_empty() {
                        s.args_selected = 0;
                    } else if s.args_selected >= s.args.len() {
                        s.args_selected = s.args.len() - 1;
                    }
                    s.recompute_dirty();
                }
                true
            }
            AppEvent::SnippetEditArgsAddEmpty => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    let insert_at = if s.args.is_empty() {
                        0
                    } else {
                        s.args_selected + 1
                    };
                    s.args.insert(insert_at, String::new());
                    s.args_selected = insert_at;
                    s.token_edit = Some(TextField::default());
                    s.recompute_dirty();
                }
                true
            }
            AppEvent::SnippetEditArgsEnterToken => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && s.args_selected < s.args.len()
                {
                    let current = s.args[s.args_selected].clone();
                    s.token_edit = Some(TextField::new(&current));
                }
                true
            }
            AppEvent::SnippetEditTokenChar(c) => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && let Some(field) = &mut s.token_edit
                {
                    field.insert_char(c);
                }
                true
            }
            AppEvent::SnippetEditTokenBackspace => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && let Some(field) = &mut s.token_edit
                {
                    field.backspace();
                }
                true
            }
            AppEvent::SnippetEditTokenLeft => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && let Some(field) = &mut s.token_edit
                {
                    field.move_left();
                }
                true
            }
            AppEvent::SnippetEditTokenRight => {
                if let Some(s) = self.active_snippet_edit_mut()
                    && let Some(field) = &mut s.token_edit
                {
                    field.move_right();
                }
                true
            }
            AppEvent::SnippetEditTokenCommit => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    s.commit_token_edit();
                }
                true
            }
            AppEvent::SnippetEditSave => {
                // Commit any in-progress token edit so Ctrl+S from inside the
                // token-edit overlay does not silently drop the buffered text.
                if let Some(s) = self.active_snippet_edit_mut() {
                    s.commit_token_edit();
                }
                self.try_save_snippet_edit();
                true
            }
            AppEvent::SnippetEditCancel => {
                let should_confirm = self
                    .active_snippet_edit_mut()
                    .map(|s| s.dirty)
                    .unwrap_or(false);
                if let Some(s) = self.active_snippet_edit_mut() {
                    if should_confirm {
                        s.exit_confirm = Some(ExitConfirm::Pending);
                    } else if let Some(lib) = &mut self.library {
                        lib.edit = None;
                    }
                }
                true
            }
            AppEvent::SnippetEditConfirmSave => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    s.exit_confirm = None;
                }
                self.try_save_snippet_edit();
                true
            }
            AppEvent::SnippetEditConfirmDiscard => {
                if let Some(lib) = &mut self.library {
                    lib.edit = None;
                }
                true
            }
            AppEvent::SnippetEditConfirmCancel => {
                if let Some(s) = self.active_snippet_edit_mut() {
                    s.exit_confirm = None;
                }
                true
            }
            AppEvent::Noop => false,
        }
    }

    // --- P4-9: helpers ---------------------------------------------------

    /// Returns the active drawer state (Library mode first, then Edit mode).
    /// `app.library` and `app.edit` are mutually exclusive, so at most one
    /// branch is reachable.
    fn active_drawer_mut(&mut self) -> Option<&mut SnippetsDrawerState> {
        if let Some(lib) = &mut self.library {
            return Some(&mut lib.snippets);
        }
        if let Some(edit) = &mut self.edit {
            return Some(&mut edit.snippets);
        }
        None
    }

    fn active_snippet_edit_mut(&mut self) -> Option<&mut SnippetEditState> {
        self.library.as_mut()?.edit.as_mut()
    }

    /// Try to save the in-progress SnippetEditState. Writes to disk and
    /// reloads the drawer on success; leaves edit open with an error
    /// message on failure.
    fn try_save_snippet_edit(&mut self) {
        type SaveSnapshot = (
            String,
            crate::snippets::SnippetCategory,
            Option<String>,
            Vec<String>,
            SnippetEditMode,
        );
        let snapshot: Option<SaveSnapshot> = self.library.as_ref().and_then(|lib| {
            lib.edit.as_ref().map(|s| {
                (
                    s.name.value.trim().to_string(),
                    s.category,
                    if s.description.value.trim().is_empty() {
                        None
                    } else {
                        Some(s.description.value.trim().to_string())
                    },
                    s.args.iter().filter(|a| !a.is_empty()).cloned().collect(),
                    s.mode.clone(),
                )
            })
        });
        let Some((name, category, desc, args, mode)) = snapshot else {
            return;
        };

        if name.is_empty() {
            self.set_error("Snippet name cannot be empty");
            return;
        }
        if let Err(e) = crate::snippets::validate_snippet_name(&name) {
            self.set_error(format!("Invalid name: {}", e));
            return;
        }

        // Mutate the on-disk user list snapshot. Looking up entries by the
        // pre-edit name (original_name) is what lets renames work and is
        // also what makes overrides whose name collides with a builtin
        // survive a save — previously we filtered merged snippets through
        // is_builtin_snippet_name and silently dropped them.
        let mut user_only: Vec<crate::snippets::Snippet> = self
            .library
            .as_ref()
            .map(|lib| lib.user_only.clone())
            .unwrap_or_default();

        let new_snippet = crate::snippets::Snippet {
            name: name.clone(),
            args,
            category,
            description: desc,
        };
        match &mode {
            SnippetEditMode::Create => {
                if user_only.iter().any(|s| s.name == name) {
                    self.set_error(format!("Snippet '{}' already exists", name));
                    return;
                }
                user_only.push(new_snippet);
            }
            SnippetEditMode::Update { original_name } => {
                if name != *original_name && user_only.iter().any(|s| s.name == name) {
                    self.set_error(format!("Snippet '{}' already exists", name));
                    return;
                }
                if let Some(pos) = user_only.iter().position(|s| s.name == *original_name) {
                    user_only[pos] = new_snippet;
                } else {
                    // Editing a builtin without an existing override creates
                    // one — append it.
                    user_only.push(new_snippet);
                }
            }
        }

        match crate::snippets::save_user_snippets(&user_only) {
            Ok(()) => {
                if let Some(lib) = &mut self.library {
                    lib.user_only = user_only;
                    lib.edit = None;
                    lib.snippets = SnippetsDrawerState::load();
                    let rows = lib.snippets.visible_rows();
                    let target_idx = rows.iter().position(|r| {
                        matches!(r, DrawerRow::Snippet { snippet_index }
                            if lib.snippets.snippets[*snippet_index].name == name)
                    });
                    if let Some(idx) = target_idx {
                        lib.snippets.selected = idx;
                    }
                }
                self.set_info(format!("Saved snippet '{}'", name));
            }
            Err(e) => {
                self.set_error(format!("Failed to write snippets.json: {}", e));
            }
        }
    }

    /// Save the in-progress edit. On success: clear `self.edit`, rescan
    /// configs, select the saved entry, and set an info message. On
    /// failure: leave `self.edit` open and set an error message.
    fn try_save_edit(&mut self) {
        let Some(edit) = &self.edit else { return };

        let name = edit.name.value.trim().to_string();
        if let Err(e) = crate::config::validate_config_name(&name) {
            self.set_error(format!("Invalid name: {}", e));
            return;
        }

        let qemu_bin = edit.qemu_bin.value.trim().to_string();
        let description = if edit.description.value.trim().is_empty() {
            None
        } else {
            Some(edit.description.value.trim().to_string())
        };
        let config = QemuConfig {
            qemu_bin,
            args: edit.args.clone(),
            desc: description,
            qemu_version: edit.preserved_qemu_version.clone(),
            resources: edit.preserved_resources.clone(),
        };

        if let Err(e) = crate::config::validate_config(&config) {
            self.set_error(format!("Invalid config: {}", e));
            return;
        }

        let config_dir = match crate::config::config_dir() {
            Ok(p) => p,
            Err(e) => {
                self.set_error(format!("Cannot resolve config dir: {}", e));
                return;
            }
        };
        let new_path = config_dir.join(format!("{}.json", name));

        match &edit.mode {
            EditMode::Create => {
                if new_path.exists() {
                    self.set_error(format!("Config '{}' already exists", name));
                    return;
                }
            }
            EditMode::Update { original_name } => {
                if name != *original_name && new_path.exists() {
                    self.set_error(format!("Config '{}' already exists", name));
                    return;
                }
            }
        }

        let json = match serde_json::to_string_pretty(&config) {
            Ok(s) => s,
            Err(e) => {
                self.set_error(format!("Failed to serialize: {}", e));
                return;
            }
        };
        if let Err(e) = std::fs::write(&new_path, json) {
            self.set_error(format!("Failed to write config: {}", e));
            return;
        }
        if let EditMode::Update { original_name } = &edit.mode
            && name != *original_name
        {
            let old_path = config_dir.join(format!("{}.json", original_name));
            if let Err(e) = std::fs::remove_file(&old_path) {
                self.set_error(format!("Saved new config but failed to remove old: {}", e));
                return;
            }
        }

        let entry_name = name.clone();
        self.edit = None;

        match scan::scan_configs() {
            Ok(new_entries) => {
                self.entries = new_entries;
                self.selected = self
                    .entries
                    .iter()
                    .position(|e| e.name() == entry_name)
                    .unwrap_or(0);
            }
            Err(e) => {
                self.set_error(format!("Saved but rescan failed: {}", e));
                return;
            }
        }

        self.right_scroll = 0;
        self.set_info(format!("Saved '{}'", entry_name));
    }
}

/// Returns true if the given name matches a built-in snippet. P4-9 uses
/// this to gate edit/delete operations and to filter the user-only set
/// before persistence.
pub(crate) fn is_builtin_snippet_name(name: &str) -> bool {
    crate::snippets::builtin_snippets()
        .iter()
        .any(|s| s.name == name)
}
