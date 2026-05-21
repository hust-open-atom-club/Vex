use crate::tui::app::{
    App, BrowseSubMode, EditField, EditFocus, EditMode, EditState, ExitConfirm, Focus, MessageKind,
    TextField,
};
use crate::tui::event::{AppEvent, translate_key};
use crate::tui::scan::ConfigEntry;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use std::sync::Mutex;

/// Serializes tests that mutate VEX_CONFIG_DIR / set_var, which are
/// process-global. Phase 3 integration tests run subcommands as child
/// processes via escargot, so they don't conflict — this mutex covers
/// the in-process refresh tests added in P4-5.
static ENV_LOCK: Mutex<()> = Mutex::new(());

fn setup_test_config_dir(configs: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("create tempdir");
    // SAFETY: callers hold ENV_LOCK; only refresh tests touch VEX_CONFIG_DIR
    // in-process, and they all serialize through that lock.
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    for (name, qemu_bin) in configs {
        let path = dir.path().join(format!("{}.json", name));
        let json = format!(
            r#"{{"qemu_bin":"{}","args":[],"desc":null,"qemu_version":null}}"#,
            qemu_bin
        );
        std::fs::write(&path, json).expect("write config");
    }
    dir
}

// --- App state (baseline) --------------------------------------------------

#[test]
fn app_new_should_not_quit() {
    let app = App::new(vec![]);
    assert!(!app.should_quit);
}

#[test]
fn app_quit_event_sets_quit_flag() {
    let mut app = App::new(vec![]);
    app.handle_event(AppEvent::Quit);
    assert!(app.should_quit);
}

#[test]
fn app_noop_event_keeps_running() {
    let mut app = App::new(vec![]);
    app.handle_event(AppEvent::Noop);
    assert!(!app.should_quit);
}

// --- Key translation -------------------------------------------------------

fn press(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
    KeyEvent::new(code, mods)
}

#[test]
fn translate_q_to_quit() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Char('q'), KeyModifiers::NONE), &app),
        AppEvent::Quit
    );
}

#[test]
fn translate_esc_to_quit() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Esc, KeyModifiers::NONE), &app),
        AppEvent::Quit
    );
}

#[test]
fn translate_ctrl_c_to_quit() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Char('c'), KeyModifiers::CONTROL), &app),
        AppEvent::Quit
    );
}

#[test]
fn translate_random_key_to_noop() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Char('a'), KeyModifiers::NONE), &app),
        AppEvent::Noop
    );
}

#[test]
fn translate_j_to_navigate_down() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Char('j'), KeyModifiers::NONE), &app),
        AppEvent::NavigateDown
    );
}

#[test]
fn translate_k_to_navigate_up() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Char('k'), KeyModifiers::NONE), &app),
        AppEvent::NavigateUp
    );
}

#[test]
fn translate_g_to_navigate_top() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Char('g'), KeyModifiers::NONE), &app),
        AppEvent::NavigateTop
    );
}

#[test]
fn translate_shift_g_to_navigate_bottom() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Char('G'), KeyModifiers::SHIFT), &app),
        AppEvent::NavigateBottom
    );
}

#[test]
fn translate_tab_to_toggle_focus() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Tab, KeyModifiers::NONE), &app),
        AppEvent::ToggleFocus
    );
}

#[test]
fn translate_release_is_noop() {
    let app = App::default();
    let key = KeyEvent::new_with_kind_and_state(
        KeyCode::Char('q'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
        KeyEventState::NONE,
    );
    assert_eq!(translate_key(key, &app), AppEvent::Noop);
}

// --- L1 navigation state transitions --------------------------------------

fn fixture_entries(n: usize) -> Vec<ConfigEntry> {
    use crate::config::QemuConfig;
    (0..n)
        .map(|i| ConfigEntry::Ok {
            name: format!("cfg{}", i),
            config: QemuConfig {
                qemu_bin: "/bin/true".to_string(),
                args: vec![],
                desc: None,
                qemu_version: None,
                resources: Default::default(),
            },
            path: std::path::PathBuf::from(format!("/tmp/cfg{}.json", i)),
        })
        .collect()
}

#[test]
fn app_navigate_down_increments_selected() {
    let mut app = App::new(fixture_entries(3));
    assert_eq!(app.selected, 0);
    app.handle_event(AppEvent::NavigateDown);
    assert_eq!(app.selected, 1);
}

#[test]
fn app_navigate_down_clamps_at_end() {
    let mut app = App::new(fixture_entries(3));
    app.handle_event(AppEvent::NavigateDown);
    app.handle_event(AppEvent::NavigateDown);
    app.handle_event(AppEvent::NavigateDown);
    app.handle_event(AppEvent::NavigateDown);
    assert_eq!(app.selected, 2);
}

#[test]
fn app_navigate_up_saturating_at_zero() {
    let mut app = App::new(fixture_entries(3));
    app.handle_event(AppEvent::NavigateUp);
    assert_eq!(app.selected, 0);
}

#[test]
fn app_navigate_top_resets_to_zero() {
    let mut app = App::new(fixture_entries(5));
    app.handle_event(AppEvent::NavigateBottom);
    assert_eq!(app.selected, 4);
    app.handle_event(AppEvent::NavigateTop);
    assert_eq!(app.selected, 0);
}

#[test]
fn app_navigate_bottom_jumps_to_last() {
    let mut app = App::new(fixture_entries(5));
    app.handle_event(AppEvent::NavigateBottom);
    assert_eq!(app.selected, 4);
}

#[test]
fn app_navigate_resets_right_scroll() {
    let mut app = App::new(fixture_entries(3));
    // Simulate a stale scroll offset on the right pane while focus is left.
    app.right_scroll = 7;
    app.handle_event(AppEvent::NavigateDown);
    assert_eq!(app.right_scroll, 0);
}

#[test]
fn app_toggle_focus_swaps_left_right() {
    let mut app = App::new(fixture_entries(1));
    assert_eq!(app.focus, Focus::Left);
    app.handle_event(AppEvent::ToggleFocus);
    assert_eq!(app.focus, Focus::Right);
    app.handle_event(AppEvent::ToggleFocus);
    assert_eq!(app.focus, Focus::Left);
}

#[test]
fn app_right_focus_scrolls_right_panel() {
    let mut app = App::new(fixture_entries(2));
    app.handle_event(AppEvent::ToggleFocus);
    assert_eq!(app.focus, Focus::Right);
    let before = app.selected;
    app.handle_event(AppEvent::NavigateDown);
    assert_eq!(app.right_scroll, 1);
    assert_eq!(app.selected, before, "right scroll must not move selection");
}

// --- L2 render snapshot (TestBackend) --------------------------------------

fn render_to_buffer(app: &mut App, width: u16, height: u16) -> ratatui::buffer::Buffer {
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| crate::tui::runner::draw(f, app)).unwrap();
    terminal.backend().buffer().clone()
}

fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
    buf.content.iter().map(|c| c.symbol()).collect()
}

fn buffer_row_to_string(buf: &ratatui::buffer::Buffer, row: u16) -> String {
    let w = buf.area.width as usize;
    let start = row as usize * w;
    buf.content
        .iter()
        .skip(start)
        .take(w)
        .map(|c| c.symbol())
        .collect()
}

#[test]
fn render_empty_state() {
    let mut app = App::new(vec![]);
    let buf = render_to_buffer(&mut app, 60, 12);
    let s = buffer_to_string(&buf);
    assert!(s.contains("No configurations"), "buffer: {}", s);
}

#[test]
fn render_ok_entry_shows_name_in_list() {
    let mut app = App::new(fixture_entries(1));
    let buf = render_to_buffer(&mut app, 80, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("cfg0"), "buffer: {}", s);
}

#[test]
fn render_broken_entry_marked() {
    let entries = vec![ConfigEntry::Broken {
        name: "bad".to_string(),
        path: std::path::PathBuf::from("/tmp/bad.json"),
        error: "parse error".to_string(),
    }];
    let mut app = App::new(entries);
    let buf = render_to_buffer(&mut app, 80, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("<broken>"), "buffer: {}", s);
    assert!(s.contains("bad"), "buffer: {}", s);
}

#[test]
fn render_status_bar_broken_count() {
    // In P4-5.2 the broken count moved from the bottom status bar into
    // the top status bar. The assertion is updated to require the more
    // specific "1 broken" string (stronger than the prior "broken"
    // substring), still verifying that a broken entry surfaces visibly.
    let mut entries = fixture_entries(2);
    entries.push(ConfigEntry::Broken {
        name: "zbad".to_string(),
        path: std::path::PathBuf::from("/tmp/zbad.json"),
        error: "parse error".to_string(),
    });
    let mut app = App::new(entries);
    let buf = render_to_buffer(&mut app, 100, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("1 broken"), "buffer: {}", s);
}

#[test]
fn render_focus_indicator_on_left_default() {
    let mut app = App::new(fixture_entries(1));
    let buf = render_to_buffer(&mut app, 80, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Configurations"), "buffer: {}", s);
}

// --- L1 Launch state machine ----------------------------------------------

fn broken_entry(name: &str) -> ConfigEntry {
    ConfigEntry::Broken {
        name: name.to_string(),
        path: std::path::PathBuf::from(format!("/tmp/{}.json", name)),
        error: "parse error".to_string(),
    }
}

#[test]
fn launch_on_ok_entry_sets_pending_launch() {
    let mut app = App::new(fixture_entries(2));
    app.handle_event(AppEvent::Launch);
    assert_eq!(app.pending_launch, Some(0));
    assert!(app.last_message.is_none());
}

#[test]
fn launch_on_broken_entry_sets_error_message() {
    let mut app = App::new(vec![broken_entry("bad")]);
    app.handle_event(AppEvent::Launch);
    assert!(app.pending_launch.is_none());
    let msg = app.last_message.as_ref().expect("expected error message");
    assert_eq!(msg.kind, MessageKind::Error);
    assert!(
        msg.text.to_lowercase().contains("broken"),
        "expected 'broken' in {:?}",
        msg.text
    );
}

#[test]
fn launch_on_empty_entries_is_noop() {
    let mut app = App::new(vec![]);
    app.handle_event(AppEvent::Launch);
    assert!(app.pending_launch.is_none());
    assert!(app.last_message.is_none());
}

#[test]
fn take_pending_launch_consumes() {
    let mut app = App::new(fixture_entries(1));
    app.pending_launch = Some(0);
    assert_eq!(app.take_pending_launch(), Some(0));
    assert_eq!(app.take_pending_launch(), None);
}

#[test]
fn any_non_noop_event_clears_message() {
    let mut app = App::new(fixture_entries(3));
    app.set_error("stale");
    app.handle_event(AppEvent::NavigateDown);
    assert!(app.last_message.is_none());
}

#[test]
fn noop_event_preserves_message() {
    let mut app = App::new(fixture_entries(1));
    app.set_info("hello");
    app.handle_event(AppEvent::Noop);
    assert!(app.last_message.is_some());
}

#[test]
fn enter_translates_to_launch() {
    let app = App::default();
    assert_eq!(
        translate_key(press(KeyCode::Enter, KeyModifiers::NONE), &app),
        AppEvent::Launch
    );
}

// --- L2 message rendering -------------------------------------------------

#[test]
fn render_error_message_in_status_bar() {
    let mut app = App::new(fixture_entries(1));
    app.set_error("Test error");
    let buf = render_to_buffer(&mut app, 100, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Test error"), "buffer: {}", s);
    assert!(s.contains("press any key to dismiss"), "buffer: {}", s);
}

#[test]
fn render_info_message_in_status_bar() {
    let mut app = App::new(fixture_entries(1));
    app.set_info("Test info");
    let buf = render_to_buffer(&mut app, 100, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Test info"), "buffer: {}", s);
}

#[test]
fn message_overrides_broken_count() {
    // In P4-5.2 the broken count is in the top bar and the bottom status
    // bar shows bracketed key hints by default. A live last_message must
    // still override the bottom row — verified here by checking that the
    // message text appears and the default "[q]" bracket hint does NOT.
    let mut entries = fixture_entries(1);
    entries.push(broken_entry("zbad"));
    let mut app = App::new(entries);
    app.set_error("Override");
    let buf = render_to_buffer(&mut app, 100, 20);
    let status_row = buffer_row_to_string(&buf, 19);
    assert!(
        status_row.contains("Override"),
        "status row: {}",
        status_row
    );
    assert!(
        !status_row.contains("[q]"),
        "default bracket hint should be hidden when a message is active: {}",
        status_row
    );
}

// --- P4-5.1: Launch guards under filter ----------------------------------

#[test]
fn launch_with_empty_filter_result_sets_error_message() {
    let mut app = App::new(fixture_entries(3));
    app.browse_sub = BrowseSubMode::Filtering {
        query: "xyzzzz".to_string(),
        accepted: true,
    };
    app.handle_event(AppEvent::Launch);
    assert!(app.pending_launch.is_none());
    let msg = app.last_message.as_ref().expect("expected error message");
    assert_eq!(msg.kind, MessageKind::Error);
    assert!(
        msg.text.contains("No configurations match"),
        "unexpected text: {:?}",
        msg.text
    );
}

#[test]
fn launch_with_filter_visible_still_works() {
    let mut app = App::new(fixture_entries(3));
    app.browse_sub = BrowseSubMode::Filtering {
        query: "cfg".to_string(),
        accepted: true,
    };
    // All three fixture entries are named cfg0/cfg1/cfg2, so they all match.
    app.selected = 0;
    app.handle_event(AppEvent::Launch);
    assert_eq!(app.pending_launch, Some(0));
    assert!(app.last_message.is_none());
}

#[test]
fn launch_with_filter_idle_still_works() {
    let mut app = App::new(fixture_entries(3));
    app.selected = 1;
    app.handle_event(AppEvent::Launch);
    assert_eq!(app.pending_launch, Some(1));
    assert!(app.last_message.is_none());
}

// --- L1 Filter sub-mode ---------------------------------------------------

fn ok_entry_with_desc(name: &str, desc: Option<&str>) -> ConfigEntry {
    use crate::config::QemuConfig;
    ConfigEntry::Ok {
        name: name.to_string(),
        config: QemuConfig {
            qemu_bin: "/bin/true".to_string(),
            args: vec![],
            desc: desc.map(|s| s.to_string()),
            qemu_version: None,
            resources: Default::default(),
        },
        path: std::path::PathBuf::from(format!("/tmp/{}.json", name)),
    }
}

#[test]
fn enter_filter_from_idle_starts_empty_query() {
    let mut app = App::new(fixture_entries(2));
    app.handle_event(AppEvent::EnterFilter);
    match &app.browse_sub {
        BrowseSubMode::Filtering { query, accepted } => {
            assert_eq!(query, "");
            assert!(!accepted);
        }
        other => panic!("expected Filtering, got {:?}", other),
    }
}

#[test]
fn filter_char_appends_to_query() {
    let mut app = App::new(fixture_entries(2));
    app.handle_event(AppEvent::EnterFilter);
    app.handle_event(AppEvent::FilterChar('c'));
    app.handle_event(AppEvent::FilterChar('f'));
    match &app.browse_sub {
        BrowseSubMode::Filtering { query, .. } => assert_eq!(query, "cf"),
        _ => panic!("expected Filtering"),
    }
}

#[test]
fn filter_backspace_pops_query() {
    let mut app = App::new(fixture_entries(2));
    app.handle_event(AppEvent::EnterFilter);
    app.handle_event(AppEvent::FilterChar('a'));
    app.handle_event(AppEvent::FilterChar('b'));
    app.handle_event(AppEvent::FilterBackspace);
    match &app.browse_sub {
        BrowseSubMode::Filtering { query, .. } => assert_eq!(query, "a"),
        _ => panic!("expected Filtering"),
    }
}

#[test]
fn accept_filter_marks_accepted_true() {
    let mut app = App::new(fixture_entries(2));
    app.handle_event(AppEvent::EnterFilter);
    app.handle_event(AppEvent::FilterChar('c'));
    app.handle_event(AppEvent::AcceptFilter);
    match &app.browse_sub {
        BrowseSubMode::Filtering { accepted, query } => {
            assert!(*accepted);
            assert_eq!(query, "c");
        }
        _ => panic!("expected Filtering"),
    }
}

#[test]
fn exit_filter_returns_to_idle_and_resets_selected() {
    let mut app = App::new(fixture_entries(3));
    app.selected = 2;
    app.handle_event(AppEvent::EnterFilter);
    app.handle_event(AppEvent::ExitFilter);
    assert!(matches!(app.browse_sub, BrowseSubMode::Idle));
    assert_eq!(app.selected, 0);
    assert_eq!(app.right_scroll, 0);
}

#[test]
fn enter_filter_again_preserves_existing_query_when_accepted() {
    let mut app = App::new(fixture_entries(2));
    app.handle_event(AppEvent::EnterFilter);
    app.handle_event(AppEvent::FilterChar('c'));
    app.handle_event(AppEvent::FilterChar('f'));
    app.handle_event(AppEvent::AcceptFilter);
    app.handle_event(AppEvent::EnterFilter);
    match &app.browse_sub {
        BrowseSubMode::Filtering { query, accepted } => {
            assert_eq!(query, "cf");
            assert!(!accepted);
        }
        _ => panic!("expected Filtering"),
    }
}

#[test]
fn visible_indices_filters_by_name() {
    let entries = vec![
        ok_entry_with_desc("alpha", None),
        ok_entry_with_desc("beta", None),
        ok_entry_with_desc("alphabet", None),
    ];
    let mut app = App::new(entries);
    app.browse_sub = BrowseSubMode::Filtering {
        query: "alp".to_string(),
        accepted: false,
    };
    let v = app.visible_indices();
    assert_eq!(v, vec![0, 2]);
}

#[test]
fn visible_indices_filters_by_description() {
    let entries = vec![
        ok_entry_with_desc("one", Some("linux server")),
        ok_entry_with_desc("two", Some("windows")),
        ok_entry_with_desc("three", None),
    ];
    let mut app = App::new(entries);
    app.browse_sub = BrowseSubMode::Filtering {
        query: "linux".to_string(),
        accepted: false,
    };
    let v = app.visible_indices();
    assert_eq!(v, vec![0]);
}

#[test]
fn visible_indices_case_insensitive() {
    let entries = vec![
        ok_entry_with_desc("AlphaBox", None),
        ok_entry_with_desc("zeta", None),
    ];
    let mut app = App::new(entries);
    app.browse_sub = BrowseSubMode::Filtering {
        query: "ALPHA".to_string(),
        accepted: false,
    };
    let v = app.visible_indices();
    assert_eq!(v, vec![0]);
}

#[test]
fn visible_indices_empty_query_returns_all() {
    let entries = fixture_entries(3);
    let mut app = App::new(entries);
    app.browse_sub = BrowseSubMode::Filtering {
        query: String::new(),
        accepted: false,
    };
    let v = app.visible_indices();
    assert_eq!(v, vec![0, 1, 2]);
}

#[test]
fn reselect_preserves_name_when_still_visible() {
    let entries = vec![
        ok_entry_with_desc("alpha", None),
        ok_entry_with_desc("alphabet", None),
        ok_entry_with_desc("beta", None),
    ];
    let mut app = App::new(entries);
    app.selected = 1; // "alphabet"
    app.handle_event(AppEvent::EnterFilter);
    app.handle_event(AppEvent::FilterChar('a'));
    app.handle_event(AppEvent::FilterChar('l'));
    // "alphabet" still matches; selected should follow it.
    assert_eq!(app.entries[app.selected].name(), "alphabet");
}

#[test]
fn reselect_falls_back_to_first_when_name_filtered_out() {
    let entries = vec![
        ok_entry_with_desc("alpha", None),
        ok_entry_with_desc("beta", None),
        ok_entry_with_desc("gamma", None),
    ];
    let mut app = App::new(entries);
    app.selected = 2; // "gamma"
    app.handle_event(AppEvent::EnterFilter);
    app.handle_event(AppEvent::FilterChar('b'));
    // "gamma" no longer visible; should fall back to first visible (beta).
    assert_eq!(app.entries[app.selected].name(), "beta");
}

// --- L1 Help overlay ------------------------------------------------------

#[test]
fn toggle_help_flips_show_help() {
    let mut app = App::new(fixture_entries(1));
    assert!(!app.show_help);
    app.handle_event(AppEvent::ToggleHelp);
    assert!(app.show_help);
    app.handle_event(AppEvent::ToggleHelp);
    assert!(!app.show_help);
}

#[test]
fn dismiss_help_clears_show_help() {
    let mut app = App::new(fixture_entries(1));
    app.show_help = true;
    app.handle_event(AppEvent::DismissHelp);
    assert!(!app.show_help);
}

// --- L1 Refresh -----------------------------------------------------------

#[test]
fn refresh_success_updates_entries_and_sets_info() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _dir = setup_test_config_dir(&[("vmA", "/bin/true"), ("vmB", "/bin/true")]);
    let mut app = App::new(vec![]); // start empty
    app.handle_event(AppEvent::Refresh);
    assert_eq!(app.entries.len(), 2);
    let msg = app.last_message.as_ref().expect("expected info message");
    assert_eq!(msg.kind, MessageKind::Info);
    assert!(msg.text.to_lowercase().contains("reloaded"));
}

#[test]
fn refresh_preserves_selection_by_name() {
    let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let _dir = setup_test_config_dir(&[
        ("alpha", "/bin/true"),
        ("beta", "/bin/true"),
        ("gamma", "/bin/true"),
    ]);
    // Pre-populate with the same set so initial selection points at "beta".
    let initial = vec![
        ok_entry_with_desc("alpha", None),
        ok_entry_with_desc("beta", None),
        ok_entry_with_desc("gamma", None),
    ];
    let mut app = App::new(initial);
    app.selected = 1; // beta
    app.handle_event(AppEvent::Refresh);
    assert_eq!(app.entries[app.selected].name(), "beta");
}

// --- L2 Filter + Help rendering -------------------------------------------

#[test]
fn render_filter_input_visible_when_filtering() {
    let mut app = App::new(fixture_entries(2));
    app.browse_sub = BrowseSubMode::Filtering {
        query: "abc".to_string(),
        accepted: false,
    };
    let buf = render_to_buffer(&mut app, 100, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("/ abc"), "buffer: {}", s);
}

#[test]
fn render_filtered_list_shows_only_matches() {
    let entries = vec![
        ok_entry_with_desc("alpha", None),
        ok_entry_with_desc("beta", None),
        ok_entry_with_desc("gamma", None),
    ];
    let mut app = App::new(entries);
    app.browse_sub = BrowseSubMode::Filtering {
        query: "bet".to_string(),
        accepted: false,
    };
    // Reselect so app.selected is on a visible row.
    app.selected = 1;
    let buf = render_to_buffer(&mut app, 100, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("beta"), "buffer: {}", s);
    // alpha/gamma should NOT appear as list rows. They can still appear in
    // the title or hint, so check the list area rows (rows 3..18).
    let mut list_text = String::new();
    for r in 3..18 {
        list_text.push_str(&buffer_row_to_string(&buf, r));
        list_text.push('\n');
    }
    assert!(!list_text.contains("alpha"), "list rows: {}", list_text);
    assert!(!list_text.contains("gamma"), "list rows: {}", list_text);
}

#[test]
fn render_help_overlay_blocks_underlying_content() {
    let mut app = App::new(fixture_entries(1));
    app.show_help = true;
    let buf = render_to_buffer(&mut app, 80, 24);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Help"), "buffer: {}", s);
    assert!(s.contains("Navigation"), "buffer: {}", s);
    assert!(s.contains("launch selected QEMU"), "buffer: {}", s);
}

#[test]
fn render_help_can_be_dismissed() {
    let mut app = App::new(fixture_entries(1));
    app.show_help = true;
    app.handle_event(AppEvent::DismissHelp);
    assert!(!app.show_help);
    let buf = render_to_buffer(&mut app, 80, 24);
    let s = buffer_to_string(&buf);
    assert!(
        !s.contains("Press any key to close this help"),
        "buffer: {}",
        s
    );
}

#[test]
fn render_filter_accepted_shows_in_title() {
    let mut app = App::new(fixture_entries(2));
    app.browse_sub = BrowseSubMode::Filtering {
        query: "cfg".to_string(),
        accepted: true,
    };
    let buf = render_to_buffer(&mut app, 100, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("/cfg"), "buffer: {}", s);
}

// --- P4-5.2: top-bar / cards / bracket-style status bar -------------------

#[test]
fn render_top_bar_shows_counts() {
    let mut entries = fixture_entries(3);
    entries.push(broken_entry("zbad"));
    let mut app = App::new(entries);
    let buf = render_to_buffer(&mut app, 100, 24);
    let s = buffer_to_string(&buf);
    assert!(s.contains("4 configurations"), "buffer: {}", s);
    assert!(s.contains("3 ok"), "buffer: {}", s);
    assert!(s.contains("⚠ 1 broken"), "buffer: {}", s);
}

#[test]
fn render_top_bar_filter_matched_count() {
    let entries = vec![
        ok_entry_with_desc("alpha", None),
        ok_entry_with_desc("beta", None),
        ok_entry_with_desc("gamma", None),
        ok_entry_with_desc("delta", None),
        ok_entry_with_desc("epsilon", None),
    ];
    let mut app = App::new(entries);
    // "alpha" matches only the "alpha" entry → 1/5 matched.
    app.browse_sub = BrowseSubMode::Filtering {
        query: "alpha".to_string(),
        accepted: false,
    };
    let buf = render_to_buffer(&mut app, 120, 24);
    let s = buffer_to_string(&buf);
    assert!(s.contains("1/5"), "buffer: {}", s);
    assert!(s.contains("matched"), "buffer: {}", s);
}

#[test]
fn render_right_pane_has_three_cards_for_ok() {
    let mut app = App::new(fixture_entries(1));
    let buf = render_to_buffer(&mut app, 120, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Binary"), "buffer: {}", s);
    assert!(s.contains("Args"), "buffer: {}", s);
    assert!(s.contains("Resources"), "buffer: {}", s);
}

#[test]
fn render_right_pane_broken_has_error_and_hint() {
    let mut app = App::new(vec![broken_entry("bad")]);
    let buf = render_to_buffer(&mut app, 120, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Error"), "buffer: {}", s);
    assert!(s.contains("Hint"), "buffer: {}", s);
    // Default broken_entry uses "parse error" → hint mentions JSON.
    assert!(s.contains("JSON"), "buffer: {}", s);
}

// --- P4-5.3: right-pane card scroll ---------------------------------------

/// Build an Ok entry with `n` args so its Args card overflows a small viewport.
fn ok_entry_with_many_args(name: &str, n: usize) -> ConfigEntry {
    use crate::config::QemuConfig;
    let args = (0..n).map(|i| format!("--arg{}", i)).collect();
    ConfigEntry::Ok {
        name: name.to_string(),
        config: QemuConfig {
            qemu_bin: "/bin/true".to_string(),
            args,
            desc: None,
            qemu_version: None,
            resources: Default::default(),
        },
        path: std::path::PathBuf::from(format!("/tmp/{}.json", name)),
    }
}

#[test]
fn render_right_pane_scroll_zero_shows_first_card() {
    let mut app = App::new(vec![ok_entry_with_many_args("long", 30)]);
    // scroll defaults to 0 in App::new.
    let buf = render_to_buffer(&mut app, 120, 15);
    let s = buffer_to_string(&buf);
    assert!(
        s.contains("Binary"),
        "Binary card should be visible at scroll=0: {}",
        s
    );
}

#[test]
fn render_right_pane_scroll_changes_visible_content() {
    let mut app = App::new(vec![ok_entry_with_many_args("long", 30)]);
    // Strategy A is line-precise: Binary card occupies virtual rows 0..3.
    // A scroll of 4 puts it entirely above the viewport, so the card must
    // disappear from the buffer.
    app.right_scroll = 4;
    let buf = render_to_buffer(&mut app, 120, 15);
    let s = buffer_to_string(&buf);
    assert!(
        !s.contains("Binary"),
        "Binary card should be scrolled off-screen at scroll=4: {}",
        s
    );
}

#[test]
fn render_right_pane_scroll_clamped_does_not_panic() {
    let mut app = App::new(vec![ok_entry_with_many_args("long", 30)]);
    app.right_scroll = u16::MAX;
    let buf = render_to_buffer(&mut app, 120, 15);
    let s = buffer_to_string(&buf);
    // u16::MAX is clamped to the content-aware max internally. At least one
    // card must remain visible — Resources is the last card, so at max
    // scroll it lands inside the viewport.
    assert!(
        s.contains("Resources"),
        "at max scroll the last card should still render: {}",
        s
    );
}

// --- P4-5.4: Ctrl+C priority + right_scroll clamp write-back --------------

#[test]
fn translate_ctrl_c_with_help_shown_still_quits() {
    let app = App {
        show_help: true,
        ..Default::default()
    };
    let ev = translate_key(press(KeyCode::Char('c'), KeyModifiers::CONTROL), &app);
    assert_eq!(
        ev,
        AppEvent::Quit,
        "Ctrl+C must escape the help overlay immediately"
    );
}

#[test]
fn render_clamps_right_scroll_when_exceeds_max() {
    let mut app = App::new(vec![ok_entry_with_many_args("long", 30)]);
    app.right_scroll = u16::MAX;
    let _ = render_to_buffer(&mut app, 120, 15);
    assert!(
        app.right_scroll < u16::MAX,
        "render should write back the clamped value: scroll = {}",
        app.right_scroll
    );
    // Sanity check: clamped value should be small for a 30-arg config in a
    // 15-row backend. We don't need the exact figure, only that it's no
    // longer the sentinel.
    assert!(
        app.right_scroll < 1000,
        "clamped scroll should be much smaller than u16::MAX: {}",
        app.right_scroll
    );
}

#[test]
fn render_does_not_change_right_scroll_when_within_bounds() {
    let mut app = App::new(vec![ok_entry_with_many_args("long", 30)]);
    app.right_scroll = 0;
    let _ = render_to_buffer(&mut app, 120, 15);
    assert_eq!(
        app.right_scroll, 0,
        "render must not perturb an in-bounds scroll value"
    );
}

// --- P4-5.5: help overlay panic safety + filter zero-match suppression ---

#[test]
fn render_help_does_not_panic_on_tiny_terminal() {
    let mut app = App {
        show_help: true,
        ..Default::default()
    };
    // 30×10 is well below the prior 40×12 lower bound — must not panic.
    let _ = render_to_buffer(&mut app, 30, 10);
}

#[test]
fn render_help_does_not_panic_on_huge_terminal() {
    let mut app = App {
        show_help: true,
        ..Default::default()
    };
    // Sanity check the opposite extreme — width/height far above the
    // hard ceilings. Must not panic and the overlay should render.
    let _ = render_to_buffer(&mut app, 200, 80);
}

#[test]
fn render_with_filter_zero_match_shows_no_match_message() {
    // Build three entries whose qemu_bin is a distinctive string so we can
    // assert the right pane does NOT leak its details when filtered to zero.
    use crate::config::QemuConfig;
    let entries: Vec<ConfigEntry> = (0..3)
        .map(|i| ConfigEntry::Ok {
            name: format!("alpha{}", i),
            config: QemuConfig {
                qemu_bin: "qemu-system-aarch64".to_string(),
                args: vec![],
                desc: None,
                qemu_version: None,
                resources: Default::default(),
            },
            path: std::path::PathBuf::from(format!("/tmp/alpha{}.json", i)),
        })
        .collect();
    let mut app = App::new(entries);
    app.browse_sub = BrowseSubMode::Filtering {
        query: "xyzzz".to_string(),
        accepted: true,
    };
    let buf = render_to_buffer(&mut app, 120, 24);
    let s = buffer_to_string(&buf);
    assert!(s.contains("No matching"), "expected hint in buffer: {}", s);
    assert!(
        !s.contains("qemu-system-aarch64"),
        "right pane must not show details of filtered-out entries: {}",
        s
    );
}

#[test]
fn render_status_bar_uses_bracket_style() {
    let mut app = App::new(fixture_entries(1));
    let buf = render_to_buffer(&mut app, 120, 24);
    let s = buffer_to_string(&buf);
    assert!(s.contains("[q]"), "buffer: {}", s);
    assert!(s.contains("[Tab]"), "buffer: {}", s);
    assert!(s.contains("[Enter]"), "buffer: {}", s);
}

#[test]
fn render_status_bar_no_broken_count() {
    let mut entries = fixture_entries(1);
    entries.push(broken_entry("zbad"));
    let mut app = App::new(entries);
    let buf = render_to_buffer(&mut app, 120, 24);
    // Status bar is the last row.
    let status_row = buffer_row_to_string(&buf, 23);
    assert!(
        !status_row.contains("broken"),
        "broken count should live in top bar, not status bar: {}",
        status_row
    );
}

#[test]
fn render_empty_state_still_has_top_bar() {
    let mut app = App::new(vec![]);
    let buf = render_to_buffer(&mut app, 80, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("0 configurations"), "buffer: {}", s);
}

// --- L3 headless state machine --------------------------------------------

#[test]
fn headless_quit_event_terminates_loop() {
    use crate::tui::runner::run_state_machine;
    let mut app = App::new(fixture_entries(2));
    run_state_machine(
        &mut app,
        [
            AppEvent::NavigateDown,
            AppEvent::Quit,
            AppEvent::NavigateDown,
        ],
    )
    .unwrap();
    assert!(app.should_quit);
    // The second NavigateDown after Quit should NOT have been processed.
    assert_eq!(app.selected, 1);
}

#[test]
fn headless_multi_event_navigation_sequence() {
    use crate::tui::runner::run_state_machine;
    let mut app = App::new(fixture_entries(5));
    run_state_machine(
        &mut app,
        [
            AppEvent::NavigateDown,
            AppEvent::NavigateDown,
            AppEvent::NavigateDown,
            AppEvent::NavigateUp,
        ],
    )
    .unwrap();
    assert_eq!(app.selected, 2);
}

#[test]
fn headless_filter_workflow() {
    use crate::tui::runner::run_state_machine;
    let entries = vec![
        ok_entry_with_desc("alpha", None),
        ok_entry_with_desc("beta", None),
        ok_entry_with_desc("alphabet", None),
    ];
    let mut app = App::new(entries);
    run_state_machine(
        &mut app,
        [
            AppEvent::EnterFilter,
            AppEvent::FilterChar('a'),
            AppEvent::FilterChar('l'),
            AppEvent::AcceptFilter,
        ],
    )
    .unwrap();
    match &app.browse_sub {
        BrowseSubMode::Filtering { query, accepted } => {
            assert_eq!(query, "al");
            assert!(*accepted);
        }
        _ => panic!("expected Filtering"),
    }
    let visible = app.visible_indices();
    assert_eq!(visible.len(), 2);
}

// --- End-to-end smoke test (L3) -------------------------------------------

#[test]
#[ignore = "requires TTY for crossterm raw_mode"]
fn tui_command_exits_cleanly_after_init() {
    use escargot::CargoBuild;
    let vex = CargoBuild::new()
        .bin("vex")
        .current_release()
        .run()
        .unwrap();
    let output = vex
        .command()
        .args(["tui", "--exit-after-init"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "vex tui --exit-after-init failed: {:?}",
        output
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.to_lowercase().contains("panic"),
        "stderr contained panic: {}",
        stderr
    );
}

// =========================================================================
// P4-7: Edit mode tests
// =========================================================================

// --- TextField unit tests -------------------------------------------------

#[test]
fn text_field_new_initial_cursor_at_end() {
    let f = TextField::new("hello");
    assert_eq!(f.value, "hello");
    assert_eq!(f.cursor, 5);
}

#[test]
fn text_field_insert_char_advances_cursor() {
    let mut f = TextField::new("ab");
    f.cursor = 1;
    f.insert_char('X');
    assert_eq!(f.value, "aXb");
    assert_eq!(f.cursor, 2);
}

#[test]
fn text_field_backspace_removes_prev_char() {
    let mut f = TextField::new("abc");
    f.cursor = 3;
    f.backspace();
    assert_eq!(f.value, "ab");
    assert_eq!(f.cursor, 2);
    f.cursor = 0;
    f.backspace();
    assert_eq!(f.value, "ab", "backspace at cursor=0 is no-op");
}

#[test]
fn text_field_move_left_right_clamps() {
    let mut f = TextField::new("ab");
    f.cursor = 2;
    f.move_right();
    assert_eq!(f.cursor, 2);
    f.move_left();
    f.move_left();
    f.move_left();
    assert_eq!(f.cursor, 0);
}

#[test]
fn text_field_handles_unicode() {
    // "αβ" is 4 bytes total (2 bytes each in UTF-8). Cursor positions
    // must align with char boundaries.
    let mut f = TextField::new("αβ");
    assert_eq!(f.cursor, 4);
    f.backspace();
    assert_eq!(f.value, "α");
    assert_eq!(f.cursor, 2);
    f.insert_char('γ');
    assert_eq!(f.value, "αγ");
    assert_eq!(f.cursor, 4);
    f.move_left();
    assert_eq!(f.cursor, 2);
    f.move_left();
    assert_eq!(f.cursor, 0);
}

// --- EditState tests -----------------------------------------------------

fn ok_entry_for_edit(name: &str, qemu_bin: &str) -> crate::tui::scan::ConfigEntry {
    use crate::config::QemuConfig;
    crate::tui::scan::ConfigEntry::Ok {
        name: name.to_string(),
        config: QemuConfig {
            qemu_bin: qemu_bin.to_string(),
            args: vec!["-m".to_string(), "1G".to_string()],
            desc: Some("hello".to_string()),
            qemu_version: None,
            resources: Default::default(),
        },
        path: std::path::PathBuf::from(format!("/tmp/{}.json", name)),
    }
}

#[test]
fn edit_from_config_clones_fields() {
    use crate::config::QemuConfig;
    let cfg = QemuConfig {
        qemu_bin: "/usr/bin/qemu".to_string(),
        args: vec!["-m".to_string(), "1G".to_string()],
        desc: Some("hi".to_string()),
        qemu_version: None,
        resources: Default::default(),
    };
    let state = EditState::from_config(&cfg, "myvm");
    assert_eq!(state.name.value, "myvm");
    assert_eq!(state.qemu_bin.value, "/usr/bin/qemu");
    assert_eq!(state.description.value, "hi");
    assert_eq!(state.args, vec!["-m", "1G"]);
    assert!(matches!(state.mode, EditMode::Update { .. }));
    assert!(!state.dirty);
}

#[test]
fn edit_new_empty_starts_at_name_field() {
    let state = EditState::new_empty();
    assert!(matches!(state.mode, EditMode::Create));
    assert_eq!(state.focused_field, EditField::Name);
    assert!(state.name.value.is_empty());
    assert!(!state.dirty);
}

#[test]
fn edit_tab_switches_pane() {
    // P4-8: Tab now swaps Editor ↔ SnippetsDrawer; field cycling moved
    // to ↑/↓ (EditFieldUp/EditFieldDown).
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    assert_eq!(app.edit.as_ref().unwrap().focus, EditFocus::Editor);
    app.handle_event(AppEvent::EditTab);
    assert_eq!(app.edit.as_ref().unwrap().focus, EditFocus::SnippetsDrawer);
    app.handle_event(AppEvent::EditTab);
    assert_eq!(app.edit.as_ref().unwrap().focus, EditFocus::Editor);
}

#[test]
fn edit_shift_tab_switches_pane() {
    // P4-8: Shift+Tab is equivalent to Tab in the 2-pane layout.
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditShiftTab);
    assert_eq!(app.edit.as_ref().unwrap().focus, EditFocus::SnippetsDrawer);
    app.handle_event(AppEvent::EditShiftTab);
    assert_eq!(app.edit.as_ref().unwrap().focus, EditFocus::Editor);
}

#[test]
fn edit_text_input_marks_dirty() {
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    assert!(!app.edit.as_ref().unwrap().dirty);
    app.handle_event(AppEvent::EditTextChar('X'));
    assert!(app.edit.as_ref().unwrap().dirty);
}

#[test]
fn edit_args_move_up_swaps() {
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditFieldDown); // Name → QemuBin
    app.handle_event(AppEvent::EditFieldDown); // → Description
    app.handle_event(AppEvent::EditFieldDown); // → Args
    // Args = ["-m", "1G"], selected = 0. Move down then up.
    app.handle_event(AppEvent::EditArgsDown);
    assert_eq!(app.edit.as_ref().unwrap().args_selected, 1);
    app.handle_event(AppEvent::EditArgsMoveUp);
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.args, vec!["1G", "-m"]);
    assert_eq!(edit.args_selected, 0);
    assert!(edit.dirty);
}

#[test]
fn edit_args_move_down_swaps() {
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditFieldDown);
    app.handle_event(AppEvent::EditFieldDown);
    app.handle_event(AppEvent::EditFieldDown);
    app.handle_event(AppEvent::EditArgsMoveDown);
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.args, vec!["1G", "-m"]);
    assert_eq!(edit.args_selected, 1);
}

#[test]
fn edit_args_delete_decrements_selected_if_last() {
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditFieldDown);
    app.handle_event(AppEvent::EditFieldDown);
    app.handle_event(AppEvent::EditFieldDown);
    app.handle_event(AppEvent::EditArgsDown); // selected = 1 (last)
    app.handle_event(AppEvent::EditArgsDelete);
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.args, vec!["-m"]);
    assert_eq!(
        edit.args_selected, 0,
        "deleting the last entry must clamp selection"
    );
}

#[test]
fn edit_cancel_without_changes_exits_immediately() {
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditCancel);
    assert!(app.edit.is_none());
}

#[test]
fn edit_cancel_with_changes_enters_exit_confirm() {
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditTextChar('X'));
    app.handle_event(AppEvent::EditCancel);
    let edit = app.edit.as_ref().expect("edit still active");
    assert_eq!(edit.exit_confirm, Some(ExitConfirm::Pending));
}

#[test]
fn edit_confirm_discard_exits() {
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditTextChar('X'));
    app.handle_event(AppEvent::EditCancel);
    app.handle_event(AppEvent::EditConfirmDiscard);
    assert!(app.edit.is_none());
}

#[test]
fn edit_confirm_cancel_returns_to_normal() {
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditTextChar('X'));
    app.handle_event(AppEvent::EditCancel);
    app.handle_event(AppEvent::EditConfirmCancel);
    let edit = app.edit.as_ref().expect("edit still active");
    assert_eq!(edit.exit_confirm, None);
    assert!(edit.dirty);
}

#[test]
fn edit_ctrl_c_during_edit_quits_and_discards() {
    let mut app = App::new(vec![ok_entry_for_edit("a", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditTextChar('X'));
    // Ctrl+C in translate_key maps to AppEvent::Quit; the Edit gate routes
    // it through the special escape path.
    app.handle_event(AppEvent::Quit);
    assert!(app.should_quit);
    assert!(app.edit.is_none());
}

// --- L2 Edit-pane render tests -------------------------------------------

#[test]
fn render_edit_pane_shows_editing_title() {
    let mut app = App::new(vec![ok_entry_for_edit("alpha", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    let buf = render_to_buffer(&mut app, 120, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Editing: alpha"), "buffer: {}", s);
}

#[test]
fn render_edit_pane_focused_field_highlighted() {
    let mut app = App::new(vec![ok_entry_for_edit("alpha", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    // Default focus = Name. The cursor block "█" must appear in the buffer
    // since the Name field is focused.
    let buf = render_to_buffer(&mut app, 120, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("█"), "expected cursor glyph: {}", s);
    assert!(s.contains("Name"), "label expected: {}", s);
}

#[test]
fn render_edit_pane_dirty_indicator_visible() {
    let mut app = App::new(vec![ok_entry_for_edit("alpha", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditTextChar('X'));
    let buf = render_to_buffer(&mut app, 120, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("unsaved"), "expected dirty indicator: {}", s);
}

#[test]
fn render_edit_pane_exit_confirm_prompt() {
    let mut app = App::new(vec![ok_entry_for_edit("alpha", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditTextChar('X'));
    app.handle_event(AppEvent::EditCancel);
    let buf = render_to_buffer(&mut app, 120, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Save changes?"), "buffer: {}", s);
    assert!(s.contains("[y] save"), "buffer: {}", s);
    assert!(s.contains("[n] discard"), "buffer: {}", s);
}

#[test]
fn render_top_bar_badge_changes_to_edit() {
    let mut app = App::new(vec![ok_entry_for_edit("alpha", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    let buf = render_to_buffer(&mut app, 120, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("EDIT"), "expected EDIT badge: {}", s);
    assert!(!s.contains("BROWSE"), "BROWSE badge must be hidden: {}", s);
}

// --- Save flow integration (uses tempdir + VEX_CONFIG_DIR) ---------------

fn write_config_file(dir: &std::path::Path, name: &str, qemu_bin: &str) {
    let json = format!(
        r#"{{"qemu_bin":"{}","args":[],"desc":null,"qemu_version":null}}"#,
        qemu_bin
    );
    std::fs::write(dir.join(format!("{}.json", name)), json).unwrap();
}

#[test]
fn save_valid_config_writes_file_and_exits_edit() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    // SAFETY: env mutation serialized via ENV_LOCK above.
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let mut app = App::new(vec![]);
    app.handle_event(AppEvent::EnterEditNew);
    // Type a name and binary.
    for c in "newvm".chars() {
        app.handle_event(AppEvent::EditTextChar(c));
    }
    app.handle_event(AppEvent::EditFieldDown); // → QemuBin
    for c in "/bin/true".chars() {
        app.handle_event(AppEvent::EditTextChar(c));
    }
    app.handle_event(AppEvent::EditSave);
    assert!(app.edit.is_none(), "edit should close on successful save");
    assert!(dir.path().join("newvm.json").exists());
    // After save, entries should include the new config.
    assert!(
        app.entries.iter().any(
            |e| matches!(e, crate::tui::scan::ConfigEntry::Ok { name, .. } if name == "newvm")
        )
    );
    let msg = app.last_message.as_ref().expect("info message expected");
    assert_eq!(msg.kind, MessageKind::Info);
    assert!(msg.text.contains("Saved"));
}

#[test]
fn save_invalid_name_keeps_edit_open_with_error() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let mut app = App::new(vec![]);
    app.handle_event(AppEvent::EnterEditNew);
    // Name with a path separator is invalid.
    for c in "bad/name".chars() {
        app.handle_event(AppEvent::EditTextChar(c));
    }
    app.handle_event(AppEvent::EditSave);
    assert!(
        app.edit.is_some(),
        "edit must stay open on validation error"
    );
    let msg = app.last_message.as_ref().expect("error expected");
    assert_eq!(msg.kind, MessageKind::Error);
}

#[test]
fn save_name_collision_in_create_mode_errors() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    write_config_file(dir.path(), "taken", "/bin/true");

    let mut app = App::new(vec![]);
    app.handle_event(AppEvent::EnterEditNew);
    for c in "taken".chars() {
        app.handle_event(AppEvent::EditTextChar(c));
    }
    app.handle_event(AppEvent::EditFieldDown);
    for c in "/bin/true".chars() {
        app.handle_event(AppEvent::EditTextChar(c));
    }
    app.handle_event(AppEvent::EditSave);
    assert!(app.edit.is_some());
    let msg = app.last_message.as_ref().expect("error expected");
    assert_eq!(msg.kind, MessageKind::Error);
    assert!(msg.text.contains("already exists"), "got: {}", msg.text);
}

#[test]
fn save_rename_in_update_mode_removes_old_file() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    write_config_file(dir.path(), "old", "/bin/true");

    use crate::tui::scan::ConfigEntry;
    let entries = vec![ConfigEntry::Ok {
        name: "old".to_string(),
        config: crate::config::QemuConfig {
            qemu_bin: "/bin/true".to_string(),
            args: vec![],
            desc: None,
            qemu_version: None,
            resources: Default::default(),
        },
        path: dir.path().join("old.json"),
    }];
    let mut app = App::new(entries);
    app.handle_event(AppEvent::EnterEditExisting);
    // Wipe name field and retype "new".
    while app.edit.as_ref().unwrap().name.cursor > 0 {
        app.handle_event(AppEvent::EditTextBackspace);
    }
    for c in "new".chars() {
        app.handle_event(AppEvent::EditTextChar(c));
    }
    app.handle_event(AppEvent::EditSave);
    assert!(app.edit.is_none());
    assert!(!dir.path().join("old.json").exists(), "old file removed");
    assert!(dir.path().join("new.json").exists(), "new file written");
}

#[test]
fn save_preserves_resources_from_original_config() {
    use crate::config::{QemuConfig, ResourceKind, ResourceRef};
    use std::collections::HashMap;

    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let target_path = dir.path().join("disk.img");
    std::fs::write(&target_path, b"fake").unwrap();

    let mut resources = HashMap::new();
    resources.insert(
        "disk".to_string(),
        ResourceRef {
            path: target_path.to_string_lossy().into_owned(),
            kind: ResourceKind::Image,
            sha256: None,
            size: None,
            url: None,
        },
    );
    let original = QemuConfig {
        qemu_bin: "/bin/true".to_string(),
        args: vec![],
        desc: None,
        qemu_version: Some("9.0.0".to_string()),
        resources,
    };
    // Write the original config first so storage / rescan sees it.
    let json = serde_json::to_string_pretty(&original).unwrap();
    std::fs::write(dir.path().join("withres.json"), json).unwrap();

    use crate::tui::scan::ConfigEntry;
    let entries = vec![ConfigEntry::Ok {
        name: "withres".to_string(),
        config: original.clone(),
        path: dir.path().join("withres.json"),
    }];
    let mut app = App::new(entries);
    app.handle_event(AppEvent::EnterEditExisting);
    // Tweak description so save proceeds with a meaningful change.
    app.handle_event(AppEvent::EditFieldDown);
    app.handle_event(AppEvent::EditFieldDown); // → Description
    for c in "new desc".chars() {
        app.handle_event(AppEvent::EditTextChar(c));
    }
    app.handle_event(AppEvent::EditSave);
    assert!(app.edit.is_none());

    // Round-trip read from disk and verify the resources are still there.
    let written = std::fs::read_to_string(dir.path().join("withres.json")).unwrap();
    let parsed: QemuConfig = serde_json::from_str(&written).unwrap();
    assert!(parsed.resources.contains_key("disk"));
    assert_eq!(parsed.qemu_version.as_deref(), Some("9.0.0"));
    assert_eq!(parsed.desc.as_deref(), Some("new desc"));
}

// =========================================================================
// P4-8: Snippets drawer tests
// =========================================================================

use crate::snippets::SnippetCategory;
use crate::tui::app::{DrawerRow, SnippetsDrawerState};

fn enter_edit_with_snippets() -> App {
    let mut app = App::new(vec![ok_entry_for_edit("alpha", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app
}

// --- L1 EditFocus / EditFieldUp / EditFieldDown -------------------------

#[test]
fn edit_field_up_cycles_backward() {
    let mut app = enter_edit_with_snippets();
    // From Name, ↑ → Args (prev cycle).
    app.handle_event(AppEvent::EditFieldUp);
    assert_eq!(app.edit.as_ref().unwrap().focused_field, EditField::Args);
}

#[test]
fn edit_field_down_cycles_forward() {
    let mut app = enter_edit_with_snippets();
    app.handle_event(AppEvent::EditFieldDown);
    assert_eq!(app.edit.as_ref().unwrap().focused_field, EditField::QemuBin);
}

#[test]
fn edit_field_up_down_noop_in_snippets_focus() {
    let mut app = enter_edit_with_snippets();
    app.handle_event(AppEvent::EditTab); // → SnippetsDrawer
    let before = app.edit.as_ref().unwrap().focused_field;
    app.handle_event(AppEvent::EditFieldUp);
    app.handle_event(AppEvent::EditFieldDown);
    assert_eq!(app.edit.as_ref().unwrap().focused_field, before);
}

// --- L1 DrawerRow / visible_rows ----------------------------------------

#[test]
fn drawer_visible_rows_has_8_headers_when_no_filter() {
    let drawer = SnippetsDrawerState::load();
    let rows = drawer.visible_rows();
    let header_count = rows
        .iter()
        .filter(|r| matches!(r, DrawerRow::CategoryHeader { .. }))
        .count();
    assert_eq!(header_count, 8);
    // All 42 snippets are visible plus 8 headers = 50 total rows.
    assert_eq!(rows.len(), 50);
}

#[test]
fn drawer_visible_rows_collapsed_category_keeps_header() {
    let mut drawer = SnippetsDrawerState::load();
    drawer.collapsed.push(SnippetCategory::Memory);
    let rows = drawer.visible_rows();
    // Memory header should still appear, but no Memory snippet rows.
    let memory_header_present = rows.iter().any(|r| {
        matches!(
            r,
            DrawerRow::CategoryHeader {
                category: SnippetCategory::Memory,
                collapsed: true,
                ..
            }
        )
    });
    assert!(memory_header_present);
    let memory_snippets = rows
        .iter()
        .filter_map(|r| match r {
            DrawerRow::Snippet { snippet_index } => Some(*snippet_index),
            _ => None,
        })
        .filter(|idx| drawer.snippets[*idx].category == SnippetCategory::Memory)
        .count();
    assert_eq!(memory_snippets, 0);
}

#[test]
fn drawer_visible_rows_filter_hides_unmatched_categories() {
    let mut drawer = SnippetsDrawerState::load();
    drawer.filter.active = true;
    drawer.filter.query = TextField::new("alloc");
    let rows = drawer.visible_rows();
    // Only Memory descriptions contain "Allocate"; the rest should be hidden.
    let categories: Vec<SnippetCategory> = rows
        .iter()
        .filter_map(|r| match r {
            DrawerRow::CategoryHeader { category, .. } => Some(*category),
            _ => None,
        })
        .collect();
    assert_eq!(categories.len(), 1);
    assert!(categories.contains(&SnippetCategory::Memory));
}

#[test]
fn drawer_visible_rows_filter_with_collapse() {
    let mut drawer = SnippetsDrawerState::load();
    drawer.filter.active = true;
    drawer.filter.query = TextField::new("alloc");
    drawer.collapsed.push(SnippetCategory::Memory);
    let rows = drawer.visible_rows();
    // Memory header present + collapsed, no snippets emitted.
    assert_eq!(rows.len(), 1);
    assert!(matches!(
        &rows[0],
        DrawerRow::CategoryHeader {
            category: SnippetCategory::Memory,
            collapsed: true,
            ..
        }
    ));
}

// --- L1 Toggle collapse / clamp ----------------------------------------

#[test]
fn drawer_toggle_collapse_on_snippet_keeps_selected_on_header() {
    let mut app = enter_edit_with_snippets();
    app.handle_event(AppEvent::EditTab); // → SnippetsDrawer
    // Move selection onto first Memory snippet (row 1 — header is row 0).
    app.handle_event(AppEvent::SnippetsDrawerDown);
    assert!(matches!(
        app.edit.as_ref().unwrap().snippets.current_row(),
        Some(DrawerRow::Snippet { .. })
    ));
    app.handle_event(AppEvent::SnippetsDrawerToggleCollapse);
    // After collapse, selection should snap back onto the header row.
    assert!(matches!(
        app.edit.as_ref().unwrap().snippets.current_row(),
        Some(DrawerRow::CategoryHeader {
            category: SnippetCategory::Memory,
            collapsed: true,
            ..
        })
    ));
}

#[test]
fn drawer_toggle_collapse_on_header_unfolds_category() {
    let mut app = enter_edit_with_snippets();
    app.handle_event(AppEvent::EditTab);
    // Currently selected = row 0 = Memory header (expanded). Collapse it.
    app.handle_event(AppEvent::SnippetsDrawerToggleCollapse);
    assert!(matches!(
        app.edit.as_ref().unwrap().snippets.current_row(),
        Some(DrawerRow::CategoryHeader {
            collapsed: true,
            ..
        })
    ));
    // Toggle again → unfolds.
    app.handle_event(AppEvent::SnippetsDrawerToggleCollapse);
    assert!(matches!(
        app.edit.as_ref().unwrap().snippets.current_row(),
        Some(DrawerRow::CategoryHeader {
            collapsed: false,
            ..
        })
    ));
}

#[test]
fn drawer_clamp_selected_when_filter_changes() {
    let mut drawer = SnippetsDrawerState::load();
    drawer.selected = 40; // valid in unfiltered (50 rows)
    drawer.filter.active = true;
    drawer.filter.query = TextField::new("alloc");
    drawer.clamp_selected();
    let len = drawer.visible_rows().len();
    assert!(drawer.selected < len);
}

// --- L1 Insert ---------------------------------------------------------

#[test]
fn drawer_insert_appends_to_empty_args() {
    let mut app = App::new(vec![]);
    app.handle_event(AppEvent::EnterEditNew);
    app.handle_event(AppEvent::EditTab); // → SnippetsDrawer
    // First visible row is Memory header; move down to the first snippet
    // ("512M memory").
    app.handle_event(AppEvent::SnippetsDrawerDown);
    app.handle_event(AppEvent::SnippetsDrawerInsert);
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.args, vec!["-m", "512M"]);
    assert!(edit.dirty);
}

#[test]
fn drawer_insert_inserts_after_selected_position() {
    // Start with args = ["-X", "-Y"], args_selected = 0.
    use crate::config::QemuConfig;
    let entry = crate::tui::scan::ConfigEntry::Ok {
        name: "z".to_string(),
        config: QemuConfig {
            qemu_bin: "/bin/true".to_string(),
            args: vec!["-X".to_string(), "-Y".to_string()],
            desc: None,
            qemu_version: None,
            resources: Default::default(),
        },
        path: std::path::PathBuf::from("/tmp/z.json"),
    };
    let mut app = App::new(vec![entry]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditTab); // → SnippetsDrawer
    app.handle_event(AppEvent::SnippetsDrawerDown); // → "512M memory"
    app.handle_event(AppEvent::SnippetsDrawerInsert);
    let edit = app.edit.as_ref().unwrap();
    // args_selected was 0 → insert at index 1.
    assert_eq!(edit.args, vec!["-X", "-m", "512M", "-Y"]);
}

#[test]
fn drawer_insert_on_header_row_is_noop() {
    let mut app = App::new(vec![]);
    app.handle_event(AppEvent::EnterEditNew);
    app.handle_event(AppEvent::EditTab);
    // Selected row 0 = Memory header — Insert must do nothing.
    let before = app.edit.as_ref().unwrap().args.clone();
    app.handle_event(AppEvent::SnippetsDrawerInsert);
    assert_eq!(app.edit.as_ref().unwrap().args, before);
    assert!(!app.edit.as_ref().unwrap().dirty);
}

#[test]
fn drawer_insert_marks_dirty() {
    let mut app = App::new(vec![]);
    app.handle_event(AppEvent::EnterEditNew);
    app.handle_event(AppEvent::EditTab);
    app.handle_event(AppEvent::SnippetsDrawerDown);
    app.handle_event(AppEvent::SnippetsDrawerInsert);
    assert!(app.edit.as_ref().unwrap().dirty);
}

// --- L2 render ----------------------------------------------------------

#[test]
fn render_edit_pane_shows_snippets_drawer() {
    let mut app = enter_edit_with_snippets();
    let buf = render_to_buffer(&mut app, 140, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Snippets"), "buffer: {}", s);
}

#[test]
fn render_snippets_drawer_shows_category_headers() {
    let mut app = enter_edit_with_snippets();
    let buf = render_to_buffer(&mut app, 140, 50);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Memory"), "buffer: {}", s);
    assert!(s.contains("Cpu"), "buffer: {}", s);
}

#[test]
fn render_snippets_drawer_collapsed_chevron_visible() {
    let mut app = enter_edit_with_snippets();
    app.handle_event(AppEvent::EditTab);
    app.handle_event(AppEvent::SnippetsDrawerToggleCollapse);
    let buf = render_to_buffer(&mut app, 140, 30);
    let s = buffer_to_string(&buf);
    // ▶ (U+25B6) is the collapsed-chevron glyph; the suffix "(5 hidden)"
    // also signals collapse.
    assert!(s.contains("hidden"), "expected hidden suffix: {}", s);
}

#[test]
fn render_snippets_drawer_user_badge() {
    use crate::config::QemuConfig;
    // Construct an edit state with an injected user snippet.
    let entry = crate::tui::scan::ConfigEntry::Ok {
        name: "u".to_string(),
        config: QemuConfig {
            qemu_bin: "/bin/true".to_string(),
            args: vec![],
            desc: None,
            qemu_version: None,
            resources: Default::default(),
        },
        path: std::path::PathBuf::from("/tmp/u.json"),
    };
    let mut app = App::new(vec![entry]);
    app.handle_event(AppEvent::EnterEditExisting);
    // Inject a user snippet into the drawer state.
    let edit = app.edit.as_mut().unwrap();
    edit.snippets.snippets.push(crate::snippets::Snippet {
        name: "mycustom".to_string(),
        args: vec!["-X".to_string()],
        category: SnippetCategory::Debug,
        description: None,
    });
    let buf = render_to_buffer(&mut app, 140, 60);
    let s = buffer_to_string(&buf);
    assert!(s.contains("[user]"), "expected user badge: {}", s);
}

// --- Integration: SnippetsDrawerState::load() ---------------------------

#[test]
fn drawer_load_merges_user_file_when_present() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    // Write a user snippets file with one override + one unique entry.
    let file = crate::snippets::SnippetFile {
        schema_version: crate::snippets::SnippetFile::CURRENT_VERSION,
        snippets: vec![
            crate::snippets::Snippet {
                name: "1G memory".to_string(),
                args: vec!["-m".to_string(), "1024M".to_string()],
                category: SnippetCategory::Memory,
                description: None,
            },
            crate::snippets::Snippet {
                name: "my unique".to_string(),
                args: vec!["-z".to_string()],
                category: SnippetCategory::Debug,
                description: None,
            },
        ],
    };
    std::fs::write(
        dir.path().join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();

    let drawer = SnippetsDrawerState::load();
    assert!(drawer.load_error.is_none(), "expected clean load");
    assert_eq!(
        drawer.snippets.len(),
        43,
        "42 builtins minus 1 override + 2 user"
    );
}

#[test]
fn drawer_load_fallback_when_user_file_corrupt() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    std::fs::write(dir.path().join("snippets.json"), "not valid json").unwrap();

    let drawer = SnippetsDrawerState::load();
    assert!(
        drawer.load_error.is_some(),
        "expected load_error on corrupt file"
    );
    assert_eq!(drawer.snippets.len(), 42, "fall back to builtin-only");
}

// =========================================================================
// P4-9: Library mode tests
// =========================================================================

use crate::tui::app::{DeleteConfirm, SnippetEditField, SnippetEditMode};

// --- L1 LibraryState ----------------------------------------------------

#[test]
fn enter_library_initializes_state() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    let lib = app.library.as_ref().expect("library should be active");
    assert!(lib.edit.is_none());
    assert!(lib.delete_confirm.is_none());
    assert!(!lib.snippets.snippets.is_empty());
}

#[test]
fn exit_library_clears_state() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    app.handle_event(AppEvent::ExitLibrary);
    assert!(app.library.is_none());
}

#[test]
fn library_disables_browse_events() {
    let mut app = App::new(fixture_entries(3));
    app.handle_event(AppEvent::EnterLibrary);
    // Browse-only NavigateDown should not move the entries cursor.
    let before = app.selected;
    app.handle_event(AppEvent::NavigateDown);
    assert_eq!(app.selected, before);
}

// --- L1 Library CRUD triggers -------------------------------------------

#[test]
fn library_new_opens_empty_edit() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    app.handle_event(AppEvent::LibraryNew);
    let lib = app.library.as_ref().unwrap();
    let s = lib.edit.as_ref().expect("snippet edit should be open");
    assert!(matches!(s.mode, SnippetEditMode::Create));
    assert!(s.name.value.is_empty());
}

#[test]
fn library_edit_user_snippet_opens_edit() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    // Pre-seed a user snippet on disk.
    let file = crate::snippets::SnippetFile {
        schema_version: crate::snippets::SnippetFile::CURRENT_VERSION,
        snippets: vec![crate::snippets::Snippet {
            name: "mycustom".to_string(),
            args: vec!["-X".to_string()],
            category: crate::snippets::SnippetCategory::Debug,
            description: None,
        }],
    };
    std::fs::write(
        dir.path().join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();

    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    // Navigate to the "mycustom" row — it's appended at the end (after
    // 42 builtins). Headers are interleaved, so let's find it via the
    // drawer's visible rows.
    let lib = app.library.as_ref().unwrap();
    let target_idx = lib
        .snippets
        .visible_rows()
        .iter()
        .position(|r| {
            matches!(r, crate::tui::app::DrawerRow::Snippet { snippet_index }
                if lib.snippets.snippets[*snippet_index].name == "mycustom")
        })
        .expect("my custom should be visible");
    app.library.as_mut().unwrap().snippets.selected = target_idx;
    app.handle_event(AppEvent::LibraryEditSelected);
    let lib = app.library.as_ref().unwrap();
    let s = lib.edit.as_ref().expect("edit should be open");
    assert_eq!(s.name.value, "mycustom");
    assert!(matches!(s.mode, SnippetEditMode::Update { .. }));
}

#[test]
fn library_edit_builtin_snippet_sets_error() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    // Default selection is the Memory header (row 0); move down to first
    // Memory snippet ("512M memory"), a builtin.
    app.handle_event(AppEvent::SnippetsDrawerDown);
    app.handle_event(AppEvent::LibraryEditSelected);
    assert!(app.library.as_ref().unwrap().edit.is_none());
    let msg = app.last_message.as_ref().expect("error expected");
    assert_eq!(msg.kind, MessageKind::Error);
    assert!(msg.text.contains("builtin"));
}

#[test]
fn library_delete_user_snippet_opens_confirm() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let file = crate::snippets::SnippetFile {
        schema_version: crate::snippets::SnippetFile::CURRENT_VERSION,
        snippets: vec![crate::snippets::Snippet {
            name: "to-delete".to_string(),
            args: vec![],
            category: crate::snippets::SnippetCategory::Debug,
            description: None,
        }],
    };
    std::fs::write(
        dir.path().join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    let lib = app.library.as_ref().unwrap();
    let target_idx = lib
        .snippets
        .visible_rows()
        .iter()
        .position(|r| {
            matches!(r, crate::tui::app::DrawerRow::Snippet { snippet_index }
                if lib.snippets.snippets[*snippet_index].name == "to-delete")
        })
        .unwrap();
    app.library.as_mut().unwrap().snippets.selected = target_idx;
    app.handle_event(AppEvent::LibraryDeleteSelected);
    let lib = app.library.as_ref().unwrap();
    let confirm = lib.delete_confirm.as_ref().expect("confirm open");
    assert_eq!(confirm.snippet_name, "to-delete");
}

#[test]
fn library_delete_builtin_snippet_sets_error() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    app.handle_event(AppEvent::SnippetsDrawerDown);
    app.handle_event(AppEvent::LibraryDeleteSelected);
    assert!(app.library.as_ref().unwrap().delete_confirm.is_none());
    let msg = app.last_message.as_ref().expect("error expected");
    assert_eq!(msg.kind, MessageKind::Error);
}

#[test]
fn library_delete_confirm_y_persists_and_reloads() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let file = crate::snippets::SnippetFile {
        schema_version: crate::snippets::SnippetFile::CURRENT_VERSION,
        snippets: vec![crate::snippets::Snippet {
            name: "delete-me".to_string(),
            args: vec![],
            category: crate::snippets::SnippetCategory::Debug,
            description: None,
        }],
    };
    std::fs::write(
        dir.path().join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    // Trigger delete confirm manually for simplicity.
    app.library.as_mut().unwrap().delete_confirm = Some(DeleteConfirm {
        snippet_index: 0,
        snippet_name: "delete-me".to_string(),
    });
    app.handle_event(AppEvent::LibraryDeleteConfirm);
    // Verify the file on disk no longer contains "delete-me".
    let content = std::fs::read_to_string(dir.path().join("snippets.json")).unwrap();
    assert!(
        !content.contains("delete-me"),
        "snippets.json should not contain deleted entry: {}",
        content
    );
}

#[test]
fn library_delete_confirm_n_cancels() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    app.library.as_mut().unwrap().delete_confirm = Some(DeleteConfirm {
        snippet_index: 0,
        snippet_name: "x".to_string(),
    });
    app.handle_event(AppEvent::LibraryDeleteCancel);
    assert!(app.library.as_ref().unwrap().delete_confirm.is_none());
}

// --- L1 SnippetEditState -------------------------------------------------

fn enter_library_new() -> App {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    app.handle_event(AppEvent::LibraryNew);
    app
}

#[test]
fn snippet_edit_field_cycle() {
    let mut app = enter_library_new();
    let f0 = app
        .library
        .as_ref()
        .unwrap()
        .edit
        .as_ref()
        .unwrap()
        .focused_field;
    assert_eq!(f0, SnippetEditField::Name);
    app.handle_event(AppEvent::SnippetEditFieldDown);
    assert_eq!(
        app.library
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap()
            .focused_field,
        SnippetEditField::Category
    );
    app.handle_event(AppEvent::SnippetEditFieldUp);
    app.handle_event(AppEvent::SnippetEditFieldUp);
    assert_eq!(
        app.library
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap()
            .focused_field,
        SnippetEditField::Args
    );
}

#[test]
fn snippet_edit_category_next_prev() {
    let mut app = enter_library_new();
    app.handle_event(AppEvent::SnippetEditCategoryNext);
    assert_eq!(
        app.library
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap()
            .category,
        crate::snippets::SnippetCategory::Cpu
    );
    app.handle_event(AppEvent::SnippetEditCategoryPrev);
    assert_eq!(
        app.library
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap()
            .category,
        crate::snippets::SnippetCategory::Memory
    );
}

#[test]
fn snippet_edit_text_input_marks_dirty() {
    let mut app = enter_library_new();
    app.handle_event(AppEvent::SnippetEditTextChar('a'));
    assert!(app.library.as_ref().unwrap().edit.as_ref().unwrap().dirty);
}

#[test]
fn snippet_edit_args_enter_token_opens_token_edit() {
    let mut app = enter_library_new();
    // Navigate to Args field, add an empty arg (auto-opens token edit).
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditArgsAddEmpty);
    // Commit (Esc), then re-enter via Enter.
    app.handle_event(AppEvent::SnippetEditTokenCommit);
    app.handle_event(AppEvent::SnippetEditArgsEnterToken);
    assert!(
        app.library
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap()
            .token_edit
            .is_some()
    );
}

#[test]
fn snippet_edit_token_commit_updates_args() {
    let mut app = enter_library_new();
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditArgsAddEmpty);
    // Type into the token buffer.
    app.handle_event(AppEvent::SnippetEditTokenChar('-'));
    app.handle_event(AppEvent::SnippetEditTokenChar('m'));
    app.handle_event(AppEvent::SnippetEditTokenCommit);
    let s = &app.library.as_ref().unwrap().edit.as_ref().unwrap();
    assert_eq!(s.args, vec!["-m"]);
    assert!(s.token_edit.is_none());
}

#[test]
fn snippet_edit_token_cancel_discards() {
    // commit_token_edit is the same as cancel in this prototype: both
    // take() the buffer. We model "cancel" as "commit without applying" —
    // here we just verify token_edit becomes None and args length stays
    // the same after the new-arg auto-add path.
    let mut app = enter_library_new();
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditArgsAddEmpty);
    assert!(
        app.library
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap()
            .token_edit
            .is_some()
    );
    app.handle_event(AppEvent::SnippetEditTokenCommit);
    assert!(
        app.library
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap()
            .token_edit
            .is_none()
    );
}

#[test]
fn snippet_edit_add_empty_arg_enters_token_edit() {
    let mut app = enter_library_new();
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditFieldDown);
    app.handle_event(AppEvent::SnippetEditArgsAddEmpty);
    let s = app.library.as_ref().unwrap().edit.as_ref().unwrap();
    assert_eq!(s.args.len(), 1);
    assert!(s.token_edit.is_some());
}

#[test]
fn snippet_edit_save_writes_file_and_reloads() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let mut app = enter_library_new();
    // Type the name.
    for c in "mysnip".chars() {
        app.handle_event(AppEvent::SnippetEditTextChar(c));
    }
    app.handle_event(AppEvent::SnippetEditSave);
    assert!(app.library.as_ref().unwrap().edit.is_none());
    let content = std::fs::read_to_string(dir.path().join("snippets.json")).unwrap();
    assert!(content.contains("mysnip"));
}

#[test]
fn snippet_edit_save_name_collision_sets_error() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let file = crate::snippets::SnippetFile {
        schema_version: crate::snippets::SnippetFile::CURRENT_VERSION,
        snippets: vec![crate::snippets::Snippet {
            name: "taken".to_string(),
            args: vec![],
            category: crate::snippets::SnippetCategory::Debug,
            description: None,
        }],
    };
    std::fs::write(
        dir.path().join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();
    let mut app = enter_library_new();
    for c in "taken".chars() {
        app.handle_event(AppEvent::SnippetEditTextChar(c));
    }
    app.handle_event(AppEvent::SnippetEditSave);
    assert!(app.library.as_ref().unwrap().edit.is_some());
    let msg = app.last_message.as_ref().expect("error expected");
    assert_eq!(msg.kind, MessageKind::Error);
    assert!(msg.text.contains("already exists"));
}

#[test]
fn snippet_edit_cancel_with_changes_enters_exit_confirm() {
    let mut app = enter_library_new();
    app.handle_event(AppEvent::SnippetEditTextChar('a'));
    app.handle_event(AppEvent::SnippetEditCancel);
    assert_eq!(
        app.library
            .as_ref()
            .unwrap()
            .edit
            .as_ref()
            .unwrap()
            .exit_confirm,
        Some(crate::tui::app::ExitConfirm::Pending)
    );
}

#[test]
fn snippet_edit_confirm_discard_exits() {
    let mut app = enter_library_new();
    app.handle_event(AppEvent::SnippetEditTextChar('a'));
    app.handle_event(AppEvent::SnippetEditCancel);
    app.handle_event(AppEvent::SnippetEditConfirmDiscard);
    assert!(app.library.as_ref().unwrap().edit.is_none());
}

// --- L2 render ----------------------------------------------------------

#[test]
fn render_library_pane_shows_tree_and_detail() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    let buf = render_to_buffer(&mut app, 140, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Snippets Library"), "buffer: {}", s);
    assert!(s.contains("Memory"), "buffer: {}", s);
}

#[test]
fn render_library_pane_user_badge_in_detail() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let file = crate::snippets::SnippetFile {
        schema_version: crate::snippets::SnippetFile::CURRENT_VERSION,
        snippets: vec![crate::snippets::Snippet {
            name: "alpha-user".to_string(),
            args: vec!["-Z".to_string()],
            category: crate::snippets::SnippetCategory::Debug,
            description: None,
        }],
    };
    std::fs::write(
        dir.path().join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    let lib = app.library.as_ref().unwrap();
    let target = lib
        .snippets
        .visible_rows()
        .iter()
        .position(|r| {
            matches!(r, crate::tui::app::DrawerRow::Snippet { snippet_index }
                if lib.snippets.snippets[*snippet_index].name == "alpha-user")
        })
        .unwrap();
    app.library.as_mut().unwrap().snippets.selected = target;
    let buf = render_to_buffer(&mut app, 140, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("[user]"), "expected user badge: {}", s);
    assert!(s.contains("snippets.json"), "expected source line: {}", s);
}

#[test]
fn render_library_pane_builtin_source_readonly() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    // Move down to first builtin snippet.
    app.handle_event(AppEvent::SnippetsDrawerDown);
    let buf = render_to_buffer(&mut app, 140, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("builtin (read-only)"), "buffer: {}", s);
}

#[test]
fn render_library_pane_in_snippet_edit_shows_editor() {
    let mut app = enter_library_new();
    let buf = render_to_buffer(&mut app, 140, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Editing: new snippet"), "buffer: {}", s);
}

#[test]
fn render_top_bar_library_badge() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    let buf = render_to_buffer(&mut app, 140, 30);
    let s = buffer_to_string(&buf);
    assert!(s.contains("LIBRARY"), "expected LIBRARY badge: {}", s);
}

// --- Integration: full create/edit/delete roundtrips --------------------

#[test]
fn library_full_create_save_reload_roundtrip() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let mut app = enter_library_new();
    for c in "newone".chars() {
        app.handle_event(AppEvent::SnippetEditTextChar(c));
    }
    app.handle_event(AppEvent::SnippetEditSave);

    // Read back and confirm the snippet survived a fresh load.
    let drawer = crate::tui::app::SnippetsDrawerState::load();
    assert!(drawer.snippets.iter().any(|s| s.name == "newone"));
}

#[test]
fn library_full_edit_save_reload_roundtrip() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let file = crate::snippets::SnippetFile {
        schema_version: crate::snippets::SnippetFile::CURRENT_VERSION,
        snippets: vec![crate::snippets::Snippet {
            name: "before".to_string(),
            args: vec!["-X".to_string()],
            category: crate::snippets::SnippetCategory::Debug,
            description: None,
        }],
    };
    std::fs::write(
        dir.path().join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();

    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    let lib = app.library.as_ref().unwrap();
    let idx = lib
        .snippets
        .visible_rows()
        .iter()
        .position(|r| {
            matches!(r, crate::tui::app::DrawerRow::Snippet { snippet_index }
                if lib.snippets.snippets[*snippet_index].name == "before")
        })
        .unwrap();
    app.library.as_mut().unwrap().snippets.selected = idx;
    app.handle_event(AppEvent::LibraryEditSelected);
    // Rename: delete "before", type "after".
    for _ in 0.."before".len() {
        app.handle_event(AppEvent::SnippetEditTextBackspace);
    }
    for c in "after".chars() {
        app.handle_event(AppEvent::SnippetEditTextChar(c));
    }
    app.handle_event(AppEvent::SnippetEditSave);

    let drawer = crate::tui::app::SnippetsDrawerState::load();
    assert!(drawer.snippets.iter().any(|s| s.name == "after"));
    assert!(!drawer.snippets.iter().any(|s| s.name == "before"));
}

#[test]
fn library_full_delete_save_reload_roundtrip() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    let file = crate::snippets::SnippetFile {
        schema_version: crate::snippets::SnippetFile::CURRENT_VERSION,
        snippets: vec![crate::snippets::Snippet {
            name: "doomed".to_string(),
            args: vec![],
            category: crate::snippets::SnippetCategory::Debug,
            description: None,
        }],
    };
    std::fs::write(
        dir.path().join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();

    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    app.library.as_mut().unwrap().delete_confirm = Some(DeleteConfirm {
        snippet_index: 0,
        snippet_name: "doomed".to_string(),
    });
    app.handle_event(AppEvent::LibraryDeleteConfirm);

    let drawer = crate::tui::app::SnippetsDrawerState::load();
    assert!(!drawer.snippets.iter().any(|s| s.name == "doomed"));
}

// =========================================================================
// P4-10 subtask 1: args token edit in Edit mode
// =========================================================================

fn enter_edit_args_field() -> App {
    let mut app = App::new(vec![ok_entry_for_edit("alpha", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.handle_event(AppEvent::EditFieldDown); // → QemuBin
    app.handle_event(AppEvent::EditFieldDown); // → Description
    app.handle_event(AppEvent::EditFieldDown); // → Args
    app
}

#[test]
fn edit_args_add_empty_inserts_and_enters_token_edit() {
    let mut app = enter_edit_args_field();
    let before_len = app.edit.as_ref().unwrap().args.len();
    app.handle_event(AppEvent::EditArgsAddEmpty);
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.args.len(), before_len + 1);
    assert!(edit.token_edit.is_some());
    // Inserted at args_selected + 1 (new arg becomes the selection).
    assert_eq!(edit.args[edit.args_selected], "");
}

#[test]
fn edit_args_enter_token_opens_token_edit_with_current_value() {
    let mut app = enter_edit_args_field();
    // Default fixture args = ["-m", "1G"], selected = 0.
    app.handle_event(AppEvent::EditArgsEnterToken);
    let edit = app.edit.as_ref().unwrap();
    let token = edit.token_edit.as_ref().expect("token_edit open");
    assert_eq!(token.value, "-m");
}

#[test]
fn edit_token_char_input_modifies_buffer() {
    let mut app = enter_edit_args_field();
    app.handle_event(AppEvent::EditArgsEnterToken);
    app.handle_event(AppEvent::EditTokenChar('X'));
    let token = app.edit.as_ref().unwrap().token_edit.as_ref().unwrap();
    assert_eq!(token.value, "-mX");
}

#[test]
fn edit_token_commit_writes_back_to_args() {
    let mut app = enter_edit_args_field();
    app.handle_event(AppEvent::EditArgsEnterToken);
    app.handle_event(AppEvent::EditTokenChar('!'));
    app.handle_event(AppEvent::EditTokenCommit);
    let edit = app.edit.as_ref().unwrap();
    assert_eq!(edit.args[0], "-m!");
    assert!(edit.token_edit.is_none());
    assert!(edit.dirty);
}

#[test]
fn edit_token_backspace_deletes_from_buffer() {
    let mut app = enter_edit_args_field();
    app.handle_event(AppEvent::EditArgsEnterToken);
    app.handle_event(AppEvent::EditTokenBackspace);
    let token = app.edit.as_ref().unwrap().token_edit.as_ref().unwrap();
    assert_eq!(token.value, "-");
}

#[test]
fn edit_token_commit_clears_token_edit_state() {
    let mut app = enter_edit_args_field();
    app.handle_event(AppEvent::EditArgsAddEmpty);
    assert!(app.edit.as_ref().unwrap().token_edit.is_some());
    app.handle_event(AppEvent::EditTokenCommit);
    assert!(app.edit.as_ref().unwrap().token_edit.is_none());
}

#[test]
fn render_edit_args_token_edit_shows_cursor() {
    let mut app = enter_edit_args_field();
    app.handle_event(AppEvent::EditArgsEnterToken);
    app.handle_event(AppEvent::EditTokenChar('Z'));
    let buf = render_to_buffer(&mut app, 140, 30);
    let s = buffer_to_string(&buf);
    // The cursor glyph appears in the args list line for the edited token.
    assert!(s.contains("█"), "expected cursor glyph: {}", s);
    // The new token text appears too.
    assert!(s.contains("-mZ"), "expected edited token text: {}", s);
}

// --- P4-10 subtask 2: help overlay completeness -------------------------

#[test]
fn render_help_browse_mode_contains_ctrl_l_library() {
    let mut app = App::new(fixture_entries(1));
    app.show_help = true;
    let buf = render_to_buffer(&mut app, 80, 28);
    let s = buffer_to_string(&buf);
    assert!(
        s.contains("Ctrl+L"),
        "Browse help should mention Ctrl+L: {}",
        s
    );
    assert!(s.contains("snippets library"), "buffer: {}", s);
}

#[test]
fn render_help_edit_mode_contains_a_and_enter_token() {
    let mut app = App::new(vec![ok_entry_for_edit("alpha", "/bin/true")]);
    app.handle_event(AppEvent::EnterEditExisting);
    app.show_help = true;
    let buf = render_to_buffer(&mut app, 80, 28);
    let s = buffer_to_string(&buf);
    assert!(
        s.contains("add empty arg"),
        "Edit help should mention 'a' / add empty arg: {}",
        s
    );
    assert!(
        s.contains("edit selected arg token"),
        "Edit help should mention Enter / edit token: {}",
        s
    );
    assert!(
        s.contains("Token edit"),
        "Edit help should have a Token edit section: {}",
        s
    );
}

// --- P4-10 subtask 3: top bar consistency review -----------------------

#[test]
fn render_top_bar_library_to_edit_transition() {
    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    let buf = render_to_buffer(&mut app, 140, 28);
    let s = buffer_to_string(&buf);
    assert!(s.contains("LIBRARY"), "outer mode badge: {}", s);

    // Open snippet edit → badge should flip to EDIT, stats to "Editing".
    app.handle_event(AppEvent::LibraryNew);
    let buf = render_to_buffer(&mut app, 140, 28);
    let s = buffer_to_string(&buf);
    assert!(
        s.contains("EDIT"),
        "nested sub-edit should swap badge to EDIT: {}",
        s
    );
    assert!(
        s.contains("Editing new snippet"),
        "stats should reflect snippet edit: {}",
        s
    );
}

// =========================================================================
// P4-10.1: Codex Round 1 regression coverage
// =========================================================================

/// Seed snippets.json with two overrides whose names collide with builtins
/// plus one pure user entry. Reused by the save / delete regression tests.
fn seed_overrides_and_pure_user(dir: &std::path::Path) {
    let file = crate::snippets::SnippetFile {
        schema_version: crate::snippets::SnippetFile::CURRENT_VERSION,
        snippets: vec![
            crate::snippets::Snippet {
                name: "1G memory".to_string(),
                args: vec!["-m".to_string(), "1024M".to_string()],
                category: crate::snippets::SnippetCategory::Memory,
                description: Some("override-1g".to_string()),
            },
            crate::snippets::Snippet {
                name: "4G memory".to_string(),
                args: vec!["-m".to_string(), "4096M".to_string()],
                category: crate::snippets::SnippetCategory::Memory,
                description: Some("override-4g".to_string()),
            },
            crate::snippets::Snippet {
                name: "mycustom".to_string(),
                args: vec!["-X".to_string()],
                category: crate::snippets::SnippetCategory::Debug,
                description: None,
            },
        ],
    };
    std::fs::write(
        dir.join("snippets.json"),
        serde_json::to_string(&file).unwrap(),
    )
    .unwrap();
}

#[test]
fn library_save_edit_preserves_other_overrides() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    seed_overrides_and_pure_user(dir.path());

    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    // Edit the *pure user* entry. Pre-fix, save_snippet_edits rebuilt the
    // user list by filtering merged through is_builtin_snippet_name, so the
    // two builtin-name overrides would silently vanish after this save.
    let target = crate::snippets::Snippet {
        name: "mycustom".to_string(),
        args: vec!["-X".to_string()],
        category: crate::snippets::SnippetCategory::Debug,
        description: Some("updated".to_string()),
    };
    let edit = crate::tui::app::SnippetEditState::from_snippet(&target);
    app.library.as_mut().unwrap().edit = Some(edit);
    app.handle_event(AppEvent::SnippetEditSave);

    let content = std::fs::read_to_string(dir.path().join("snippets.json")).unwrap();
    let parsed: crate::snippets::SnippetFile = serde_json::from_str(&content).unwrap();
    let names: Vec<&str> = parsed.snippets.iter().map(|s| s.name.as_str()).collect();
    assert!(
        names.contains(&"1G memory"),
        "builtin-name override must survive an unrelated save: {:?}",
        names
    );
    assert!(
        names.contains(&"4G memory"),
        "builtin-name override must survive an unrelated save: {:?}",
        names
    );
    assert!(
        names.contains(&"mycustom"),
        "edited entry must remain: {:?}",
        names
    );
    let edited = parsed
        .snippets
        .iter()
        .find(|s| s.name == "mycustom")
        .unwrap();
    assert_eq!(edited.description.as_deref(), Some("updated"));
}

#[test]
fn library_delete_preserves_other_overrides() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    seed_overrides_and_pure_user(dir.path());

    let mut app = App::default();
    app.handle_event(AppEvent::EnterLibrary);
    // Bypass navigation: stage delete_confirm directly. snippet_index is
    // unused by the handler post-Sub-3 (lookup is by name).
    app.library.as_mut().unwrap().delete_confirm = Some(DeleteConfirm {
        snippet_index: 0,
        snippet_name: "1G memory".to_string(),
    });
    app.handle_event(AppEvent::LibraryDeleteConfirm);

    let content = std::fs::read_to_string(dir.path().join("snippets.json")).unwrap();
    let parsed: crate::snippets::SnippetFile = serde_json::from_str(&content).unwrap();
    let names: Vec<&str> = parsed.snippets.iter().map(|s| s.name.as_str()).collect();
    assert!(
        !names.contains(&"1G memory"),
        "deleted entry must be gone: {:?}",
        names
    );
    assert!(
        names.contains(&"4G memory"),
        "other override must be preserved: {:?}",
        names
    );
    assert!(
        names.contains(&"mycustom"),
        "pure user entry must be preserved: {:?}",
        names
    );
}

/// Sub-4 fallback A: collision-rejection preserves both files. Verifies the
/// "rename rejected → old preserved" invariant on the Update path without
/// having to mock a write failure.
#[test]
fn edit_rename_collision_keeps_old_and_new_files() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = tempfile::tempdir().unwrap();
    unsafe {
        std::env::set_var("VEX_CONFIG_DIR", dir.path());
    }
    // Two pre-existing configs: we'll try to rename "old" → "taken".
    write_config_file(dir.path(), "old", "/bin/true");
    let taken_marker = r#"{"qemu_bin":"/bin/marker","args":[],"desc":null,"qemu_version":null}"#;
    std::fs::write(dir.path().join("taken.json"), taken_marker).unwrap();

    let entries = vec![ConfigEntry::Ok {
        name: "old".to_string(),
        config: crate::config::QemuConfig {
            qemu_bin: "/bin/true".to_string(),
            args: vec![],
            desc: None,
            qemu_version: None,
            resources: Default::default(),
        },
        path: dir.path().join("old.json"),
    }];
    let mut app = App::new(entries);
    app.handle_event(AppEvent::EnterEditExisting);
    while app.edit.as_ref().unwrap().name.cursor > 0 {
        app.handle_event(AppEvent::EditTextBackspace);
    }
    for c in "taken".chars() {
        app.handle_event(AppEvent::EditTextChar(c));
    }
    app.handle_event(AppEvent::EditSave);

    // Edit stays open with an error; both files survive untouched.
    assert!(app.edit.is_some(), "edit must stay open on collision");
    let msg = app.last_message.as_ref().expect("error expected");
    assert_eq!(msg.kind, MessageKind::Error);
    assert!(msg.text.contains("already exists"), "got: {}", msg.text);
    assert!(dir.path().join("old.json").exists(), "old file preserved");
    let surviving = std::fs::read_to_string(dir.path().join("taken.json")).unwrap();
    assert_eq!(
        surviving, taken_marker,
        "taken.json must be untouched by a rejected rename"
    );
}
