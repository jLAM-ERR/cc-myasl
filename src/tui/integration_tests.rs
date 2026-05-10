//! Phase 4 integration tests — drive App::handle synthetically; no terminal.

use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

use crate::config::schema::{Config, Line};
use crate::tui::app4::{App, Focus, Mode};
use crate::tui::builder::{BuilderSegment, to_config};
use crate::tui::catalog::Category;

// ── helpers ───────────────────────────────────────────────────────────────────

fn fresh_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![Line {
            separator: " | ".into(),
            segments: vec![],
        }],
    }
}

fn fresh_app() -> App {
    App::new(fresh_config(), PathBuf::from("/tmp/integration_test.json"))
}

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn press_mod(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
    KeyEvent {
        code,
        modifiers,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

// ── Test 1: navigate pane→pane, tab→tab, toggle preset, then Ctrl+S ──────────

#[test]
fn navigate_and_save() {
    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("config.json");

    let mut app = App::new(fresh_config(), out_path.clone());

    // Start at Top focus.
    assert_eq!(app.focus, Focus::Top);

    // Tab → Middle.
    app.handle(press(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Middle);

    // ] three times to reach the Rates tab (index 6 from Workspace=0).
    // Order: Workspace(0), Git(1), SessionModel(2), Context(3), Tokens(4), Cost(5), Rates(6), Appearance(7).
    for _ in 0..6 {
        app.handle(press(KeyCode::Char(']')));
    }
    assert_eq!(app.active_tab, Category::Rates);

    // Space to toggle the first preset (picker_selected == 0).
    app.handle(press(KeyCode::Char(' ')));

    // The active line should now have 1 segment.
    assert_eq!(app.builder.lines[0].segments.len(), 1);
    assert!(app.dirty);

    // Tab → Top, then Tab → Middle → Bottom → Tab back to Top to verify cycling.
    app.handle(press(KeyCode::Tab)); // Middle → Bottom
    assert_eq!(app.focus, Focus::Bottom);
    app.handle(press(KeyCode::Tab)); // Bottom → Top
    assert_eq!(app.focus, Focus::Top);

    // Ctrl+S → mode = Saving.
    app.handle(press_mod(KeyCode::Char('s'), KeyModifiers::CONTROL));
    assert_eq!(app.mode, Mode::Saving);

    // Simulate the run4 save-handler: save synchronously, then reset mode.
    let cfg = to_config(&app.builder);
    let result = crate::tui::overlays::save::save(&app.output_path, &cfg);
    assert!(result.is_ok(), "save must succeed: {:?}", result);
    app.set_status_ok(format!("saved → {}", result.unwrap().display()));
    app.dirty = false;
    app.mode = Mode::Browsing;

    // Verify the file was written and contains the toggled preset.
    let saved_json = std::fs::read_to_string(&out_path).expect("saved file must exist");
    let saved_cfg: crate::config::schema::Config =
        serde_json::from_str(&saved_json).expect("saved JSON must be valid Config");
    assert_eq!(saved_cfg.lines.len(), 1);
    // The line must have exactly 1 segment (the toggled preset).
    assert_eq!(saved_cfg.lines[0].segments.len(), 1);
}

// ── Test 2: non-TTY invocation returns NotATty ────────────────────────────────

#[test]
fn non_tty_returns_not_a_tty() {
    use crate::error::Error;
    use std::io::IsTerminal;

    // In cargo test, stdout is not a TTY — run4 should return Err(NotATty).
    // If somehow stdout IS a TTY in this environment, skip the assertion
    // (this can happen when tests run inside a PTY emulator).
    if std::io::stdout().is_terminal() {
        // Can't assert NotATty if stdout actually is a TTY; just verify run4 is callable.
        return;
    }

    let tmp = tempfile::tempdir().unwrap();
    let out_path = tmp.path().join("config.json");
    let result = crate::tui::run4(fresh_config(), out_path);
    assert!(
        matches!(result, Err(Error::NotATty)),
        "expected NotATty, got: {:?}",
        result.map(|_| ())
    );
}

// ── Test 3: dirty-quit confirm flow ──────────────────────────────────────────

#[test]
fn dirty_quit_confirms() {
    let mut app = fresh_app();
    app.dirty = true;

    // 'q' with dirty state → ConfirmQuit mode.
    app.handle(press(KeyCode::Char('q')));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);

    // 'n' → back to Browsing.
    app.handle(press(KeyCode::Char('n')));
    assert_eq!(app.mode, Mode::Browsing);
    assert!(!app.should_quit);

    // Mark dirty again, send 'q' again.
    app.dirty = true;
    app.handle(press(KeyCode::Char('q')));
    assert_eq!(app.mode, Mode::ConfirmQuit);

    // 'y' → should_quit = true.
    app.handle(press(KeyCode::Char('y')));
    assert!(app.should_quit);
}

// ── Test 4: separator editing via handle ──────────────────────────────────────

#[test]
fn separator_edit_via_handle() {
    let mut app = fresh_app();
    assert_eq!(app.focus, Focus::Top);

    // 's' on Gutter → EditingSeparator, pre-fills buffer.
    app.handle(press(KeyCode::Char('s')));
    assert_eq!(app.mode, Mode::EditingSeparator);

    // Clear existing buffer, type new separator.
    while !app.picker_filter.is_empty() {
        app.handle(press(KeyCode::Backspace));
    }
    app.handle(press(KeyCode::Char('-')));
    app.handle(press(KeyCode::Char('-')));

    // Enter commits.
    app.handle(press(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.builder.lines[0].separator, "--");
    assert!(app.dirty);
}

// ── Test 5: separator edit cancel leaves state unchanged ──────────────────────

#[test]
fn separator_edit_cancel_unchanged() {
    let mut app = fresh_app();
    let orig = app.builder.lines[0].separator.clone();

    app.handle(press(KeyCode::Char('s')));
    assert_eq!(app.mode, Mode::EditingSeparator);

    app.handle(press(KeyCode::Char('X')));
    // Esc cancels.
    app.handle(press(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.builder.lines[0].separator, orig);
    assert!(!app.dirty);
}

// ── Test 6: help overlay dismissed on any key ─────────────────────────────────

#[test]
fn help_dismissed_on_any_key() {
    let mut app = fresh_app();
    app.mode = Mode::Help;

    // Any key in Help mode → Browsing.
    app.handle(press(KeyCode::Char('a')));
    assert_eq!(app.mode, Mode::Browsing);
}

// ── Test 7: Ctrl+C always quits ───────────────────────────────────────────────

#[test]
fn ctrl_c_quits_unconditionally() {
    let mut app = fresh_app();
    app.dirty = true;

    app.handle(press_mod(KeyCode::Char('c'), KeyModifiers::CONTROL));
    assert!(app.should_quit);
}

// ── Test 8: toggle preset on/off via Space in middle pane ────────────────────

#[test]
fn space_toggles_preset_in_middle() {
    let mut app = fresh_app();
    // Move to Middle focus.
    app.handle(press(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Middle);
    assert_eq!(app.active_tab, Category::Workspace);

    // Toggle first preset on.
    app.handle(press(KeyCode::Char(' ')));
    assert_eq!(app.builder.lines[0].segments.len(), 1);

    // Toggle first preset off.
    app.handle(press(KeyCode::Char(' ')));
    assert_eq!(app.builder.lines[0].segments.len(), 0);
}

// ── Test 9: set_active_line helper ───────────────────────────────────────────

#[test]
fn set_active_line_out_of_bounds_sets_status() {
    let mut app = fresh_app();
    // Only 1 line; line 1 (idx 1) doesn't exist.
    app.set_active_line(1);
    assert_eq!(app.active_line, 0, "active_line must not change");
    assert!(app.status_message.is_some(), "status message must be set");
}

// ── Test 10: filter in middle pane ───────────────────────────────────────────

#[test]
fn filter_mode_enter_and_commit() {
    let mut app = fresh_app();
    // Move to Middle.
    app.handle(press(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Middle);

    // '/' opens filter.
    app.handle(press(KeyCode::Char('/')));
    assert_eq!(app.mode, Mode::Filter);

    // Type a char.
    app.handle(press(KeyCode::Char('m')));
    assert_eq!(app.picker_filter, "m");

    // Enter commits, mode returns to Browsing, filter stays.
    app.handle(press(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.picker_filter, "m");

    // '/' again clears filter.
    app.handle(press(KeyCode::Char('/')));
    assert_eq!(app.picker_filter, "");
    assert_eq!(app.mode, Mode::Filter);
}

// ── Test 11: picker_select_up/down clamp at boundaries ───────────────────────

#[test]
fn picker_select_clamps() {
    let mut app = fresh_app();
    // Move to Middle (Workspace tab has presets).
    app.handle(press(KeyCode::Tab));

    // Up at position 0 stays at 0.
    app.picker_selected = 0;
    app.picker_select_up();
    assert_eq!(app.picker_selected, 0);

    // Down advances.
    app.picker_select_down();
    assert_eq!(app.picker_selected, 1);
}

// ── Test 12: BuilderSegment::Preset present after toggle ─────────────────────

#[test]
fn toggled_preset_is_preset_variant() {
    let mut app = fresh_app();
    app.handle(press(KeyCode::Tab)); // → Middle
    app.handle(press(KeyCode::Char(' '))); // toggle first Workspace preset

    match &app.builder.lines[0].segments[0] {
        BuilderSegment::Preset { .. } => {}
        other => panic!("expected Preset variant, got {:?}", other),
    }
}
