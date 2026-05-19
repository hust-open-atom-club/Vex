use crate::tui::app::{App, BrowseSubMode, Focus, MessageKind};
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
