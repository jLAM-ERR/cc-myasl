//! Phase 4 golden integration tests.
//!
//! 1. Round-trip: every builtin → BuilderState → Config → serde_json::Value equality.
//! 2. Multi-step synthetic session: navigate, toggle preset, edit separator, Ctrl+S.
//! 3. Delete-line + dirty-quit-confirm: file NOT modified after Esc.

use cc_myasl::config::builtins::{ALL_NAMES, lookup as builtin_lookup};
use cc_myasl::config::schema::{Config, Line, Segment, TemplateSegment};
use cc_myasl::tui::app::{App, Focus, Mode};
use cc_myasl::tui::builder::{from_config, to_config};
use cc_myasl::tui::catalog::Category;
use cc_myasl::tui::overlays::save::save;
use cc_myasl::tui::process_save_if_needed;
use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};
use tempfile::TempDir;

// ── helpers ───────────────────────────────────────────────────────────────────

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::CONTROL,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    }
}

fn two_line_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![
            Line {
                separator: " | ".into(),
                segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
            },
            Line {
                separator: " | ".into(),
                segments: vec![Segment::Template(TemplateSegment::new("{five_left}%"))],
            },
        ],
    }
}

// ── Test 1: round-trip every builtin ─────────────────────────────────────────

#[test]
fn round_trip_all_builtins() {
    for name in ALL_NAMES {
        let c = builtin_lookup(name).unwrap_or_else(|| panic!("builtin '{}' not found", name));
        let builder = from_config(&c);
        let c2 = to_config(&builder);

        let json1 = serde_json::to_string_pretty(&c).unwrap();
        let json2 = serde_json::to_string_pretty(&c2).unwrap();
        let v1: serde_json::Value = serde_json::from_str(&json1).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&json2).unwrap();
        assert_eq!(v1, v2, "Builtin '{}' fails round-trip", name);
    }
}

#[test]
fn round_trip_covers_all_nine_builtins() {
    assert_eq!(ALL_NAMES.len(), 9, "expected 9 builtins");
}

// ── Test 2: multi-step synthetic session ─────────────────────────────────────

#[test]
fn multi_step_session_save() {
    let tmp = TempDir::new().unwrap();
    let out_path = tmp.path().join("config.json");

    // Start with default empty config.
    let cfg = Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![Line {
            separator: " | ".into(),
            segments: vec![],
        }],
    };
    let mut app = App::new(cfg, out_path.clone());

    // Tab → Middle (Top → Middle).
    app.handle(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Middle);

    // ] × 6 to reach Rates tab.
    // Category::ordered(): Workspace(0) Git(1) SessionModel(2) Context(3) Tokens(4) Cost(5) Rates(6) Appearance(7)
    for _ in 0..6 {
        app.handle(key(KeyCode::Char(']')));
    }
    assert_eq!(app.active_tab, Category::Rates);

    // Space on row 0 → toggles five_left_pct.
    app.handle(key(KeyCode::Char(' ')));
    assert_eq!(app.builder.lines[0].segments.len(), 1);
    assert!(app.dirty);

    // Tab → Bottom → Top.
    app.handle(key(KeyCode::Tab)); // Middle → Bottom
    assert_eq!(app.focus, Focus::Bottom);
    app.handle(key(KeyCode::Tab)); // Bottom → Top
    assert_eq!(app.focus, Focus::Top);

    // 's' on Gutter → EditingSeparator (pre-filled with current separator).
    app.handle(key(KeyCode::Char('s')));
    assert_eq!(app.mode, Mode::EditingSeparator);

    // Clear existing buffer.
    while !app.picker_filter.is_empty() {
        app.handle(key(KeyCode::Backspace));
    }
    // Type " | " (space, pipe, space).
    app.handle(key(KeyCode::Char(' ')));
    app.handle(key(KeyCode::Char('|')));
    app.handle(key(KeyCode::Char(' ')));
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.builder.lines[0].separator, " | ");

    // Ctrl+S → Saving.
    app.handle(ctrl(KeyCode::Char('s')));
    assert_eq!(app.mode, Mode::Saving);

    // Drive the save.
    process_save_if_needed(&mut app);
    assert_eq!(app.mode, Mode::Browsing);
    assert!(!app.dirty);

    // Read saved JSON and assert structure.
    let saved_json = std::fs::read_to_string(&out_path).expect("saved file must exist");
    let saved_cfg: Config = serde_json::from_str(&saved_json).expect("valid JSON");

    assert_eq!(saved_cfg.lines.len(), 1, "one line");
    assert_eq!(saved_cfg.lines[0].separator, " | ");
    assert_eq!(saved_cfg.lines[0].segments.len(), 1, "one segment");
    // The segment must be the five_left_pct preset template.
    match &saved_cfg.lines[0].segments[0] {
        Segment::Template(t) => {
            assert!(
                t.template.contains("five_left"),
                "expected five_left_pct template, got: {:?}",
                t.template
            );
        }
        Segment::Flex(_) => panic!("expected Template segment"),
    }
}

// ── Test 3: delete-line + dirty-quit-confirm; file NOT modified after Esc ────

#[test]
fn delete_dirty_quit_esc_file_unchanged() {
    let tmp = TempDir::new().unwrap();
    let out_path = tmp.path().join("config.json");

    // Create a 2-line config, save baseline.
    let baseline = two_line_config();
    save(&out_path, &baseline).expect("baseline save must succeed");
    let baseline_json = std::fs::read_to_string(&out_path).unwrap();

    // Build app from that same config with clean state.
    let mut app = App::new(two_line_config(), out_path.clone());
    assert!(!app.dirty);

    // Toggle a preset to dirty the state without touching the file.
    app.handle(key(KeyCode::Tab)); // Top → Middle
    app.handle(key(KeyCode::Char(' '))); // toggle Workspace[0]
    assert!(app.dirty);

    // 'q' with dirty → ConfirmQuit.
    app.handle(key(KeyCode::Char('q')));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);

    // Esc → back to Browsing, no quit.
    app.handle(key(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Browsing);
    assert!(!app.should_quit);

    // File must be the baseline (dirty changes never written).
    let current_json = std::fs::read_to_string(&out_path).unwrap();
    assert_eq!(
        current_json, baseline_json,
        "file must not be modified after Esc on confirm-quit"
    );
}

// ── Test 4: round-trip preserves separator on multi-line config ───────────────

#[test]
fn round_trip_preserves_separator() {
    let cfg = Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![
            Line {
                separator: " :: ".into(),
                segments: vec![],
            },
            Line {
                separator: " -- ".into(),
                segments: vec![],
            },
        ],
    };

    let builder = from_config(&cfg);
    let c2 = to_config(&builder);

    let v1: serde_json::Value =
        serde_json::from_str(&serde_json::to_string_pretty(&cfg).unwrap()).unwrap();
    let v2: serde_json::Value =
        serde_json::from_str(&serde_json::to_string_pretty(&c2).unwrap()).unwrap();
    assert_eq!(v1, v2);
}

// ── Test 5: delete-line confirm-yes removes line ──────────────────────────────

#[test]
fn delete_line_confirm_yes_removes_line() {
    let tmp = TempDir::new().unwrap();
    let out_path = tmp.path().join("config.json");

    // Start with 2-line config.
    let mut app = App::new(two_line_config(), out_path.clone());
    assert_eq!(app.builder.lines.len(), 2);

    // Line 0 has 1 segment ("{model}"); 'x' on gutter → ConfirmDelete.
    app.handle(key(KeyCode::Char('x')));
    assert_eq!(app.mode, Mode::ConfirmDelete);

    // 'y' commits delete.
    app.handle(key(KeyCode::Char('y')));
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.builder.lines.len(), 1);
    assert!(app.dirty);
}

// ── Test 6: ConfirmDelete 'n' leaves state unchanged ─────────────────────────

#[test]
fn delete_line_confirm_no_leaves_unchanged() {
    let tmp = TempDir::new().unwrap();
    let out_path = tmp.path().join("config.json");

    let mut app = App::new(two_line_config(), out_path);
    assert_eq!(app.builder.lines.len(), 2);

    app.handle(key(KeyCode::Char('x')));
    assert_eq!(app.mode, Mode::ConfirmDelete);

    app.handle(key(KeyCode::Char('n')));
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.builder.lines.len(), 2, "line count unchanged after 'n'");
}
