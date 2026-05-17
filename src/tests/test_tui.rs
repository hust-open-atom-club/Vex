use crate::tui::app::{App, Focus, MessageKind};
use crate::tui::event::{AppEvent, translate_key};
use crate::tui::scan::ConfigEntry;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

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
    assert_eq!(
        translate_key(press(KeyCode::Char('q'), KeyModifiers::NONE)),
        AppEvent::Quit
    );
}

#[test]
fn translate_esc_to_quit() {
    assert_eq!(
        translate_key(press(KeyCode::Esc, KeyModifiers::NONE)),
        AppEvent::Quit
    );
}

#[test]
fn translate_ctrl_c_to_quit() {
    assert_eq!(
        translate_key(press(KeyCode::Char('c'), KeyModifiers::CONTROL)),
        AppEvent::Quit
    );
}

#[test]
fn translate_random_key_to_noop() {
    assert_eq!(
        translate_key(press(KeyCode::Char('a'), KeyModifiers::NONE)),
        AppEvent::Noop
    );
}

#[test]
fn translate_j_to_navigate_down() {
    assert_eq!(
        translate_key(press(KeyCode::Char('j'), KeyModifiers::NONE)),
        AppEvent::NavigateDown
    );
}

#[test]
fn translate_k_to_navigate_up() {
    assert_eq!(
        translate_key(press(KeyCode::Char('k'), KeyModifiers::NONE)),
        AppEvent::NavigateUp
    );
}

#[test]
fn translate_g_to_navigate_top() {
    assert_eq!(
        translate_key(press(KeyCode::Char('g'), KeyModifiers::NONE)),
        AppEvent::NavigateTop
    );
}

#[test]
fn translate_shift_g_to_navigate_bottom() {
    assert_eq!(
        translate_key(press(KeyCode::Char('G'), KeyModifiers::SHIFT)),
        AppEvent::NavigateBottom
    );
}

#[test]
fn translate_tab_to_toggle_focus() {
    assert_eq!(
        translate_key(press(KeyCode::Tab, KeyModifiers::NONE)),
        AppEvent::ToggleFocus
    );
}

#[test]
fn translate_release_is_noop() {
    let key = KeyEvent::new_with_kind_and_state(
        KeyCode::Char('q'),
        KeyModifiers::NONE,
        KeyEventKind::Release,
        KeyEventState::NONE,
    );
    assert_eq!(translate_key(key), AppEvent::Noop);
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

fn render_to_buffer(app: &App, width: u16, height: u16) -> ratatui::buffer::Buffer {
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
    let app = App::new(vec![]);
    let buf = render_to_buffer(&app, 60, 12);
    let s = buffer_to_string(&buf);
    assert!(s.contains("No configurations"), "buffer: {}", s);
}

#[test]
fn render_ok_entry_shows_name_in_list() {
    let app = App::new(fixture_entries(1));
    let buf = render_to_buffer(&app, 80, 20);
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
    let app = App::new(entries);
    let buf = render_to_buffer(&app, 80, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("<broken>"), "buffer: {}", s);
    assert!(s.contains("bad"), "buffer: {}", s);
}

#[test]
fn render_status_bar_broken_count() {
    let mut entries = fixture_entries(2);
    entries.push(ConfigEntry::Broken {
        name: "zbad".to_string(),
        path: std::path::PathBuf::from("/tmp/zbad.json"),
        error: "parse error".to_string(),
    });
    let app = App::new(entries);
    let buf = render_to_buffer(&app, 100, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("broken"), "buffer: {}", s);
}

#[test]
fn render_focus_indicator_on_left_default() {
    let app = App::new(fixture_entries(1));
    let buf = render_to_buffer(&app, 80, 20);
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
    assert_eq!(
        translate_key(press(KeyCode::Enter, KeyModifiers::NONE)),
        AppEvent::Launch
    );
}

// --- L2 message rendering -------------------------------------------------

#[test]
fn render_error_message_in_status_bar() {
    let mut app = App::new(fixture_entries(1));
    app.set_error("Test error");
    let buf = render_to_buffer(&app, 100, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Test error"), "buffer: {}", s);
    assert!(s.contains("press any key to dismiss"), "buffer: {}", s);
}

#[test]
fn render_info_message_in_status_bar() {
    let mut app = App::new(fixture_entries(1));
    app.set_info("Test info");
    let buf = render_to_buffer(&app, 100, 20);
    let s = buffer_to_string(&buf);
    assert!(s.contains("Test info"), "buffer: {}", s);
}

#[test]
fn message_overrides_broken_count() {
    let mut entries = fixture_entries(1);
    entries.push(broken_entry("zbad"));
    let mut app = App::new(entries);
    app.set_error("Override");
    let buf = render_to_buffer(&app, 100, 20);
    // The status bar is the last row; the broken count must be hidden there.
    let status_row = buffer_row_to_string(&buf, 19);
    assert!(
        status_row.contains("Override"),
        "status row: {}",
        status_row
    );
    assert!(
        !status_row.contains("broken"),
        "broken count should be hidden in status bar: {}",
        status_row
    );
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
