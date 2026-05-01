use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::schema::{Config, Line, Segment, TemplateSegment};
use crate::tui::app::{App, EditorField, Focus, Mode};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn app_in_fg_picker() -> App {
    let mut app = App::new(
        Config {
            schema_url: None,
            powerline: false,
            lines: vec![Line {
                separator: String::new(),
                segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
            }],
        },
        PathBuf::from("/tmp/out.json"),
    );
    app.focus = Focus::Editor;
    app.selected_segment = Some(0);
    app.editor_field = EditorField::Color;
    // Enter picker — resets selection.
    app.handle(key(KeyCode::Char('c')));
    assert_eq!(app.mode, Mode::PickingFgColor);
    app
}

fn app_in_bg_picker() -> App {
    let mut app = App::new(
        Config {
            schema_url: None,
            powerline: false,
            lines: vec![Line {
                separator: String::new(),
                segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
            }],
        },
        PathBuf::from("/tmp/out.json"),
    );
    app.focus = Focus::Editor;
    app.selected_segment = Some(0);
    app.editor_field = EditorField::Bg;
    // Enter picker — resets selection.
    app.handle(key(KeyCode::Char('b')));
    assert_eq!(app.mode, Mode::PickingBgColor);
    app
}

#[test]
fn enter_fg_picker_resets_selection_to_zero() {
    let mut app = App::new(
        Config {
            schema_url: None,
            powerline: false,
            lines: vec![Line {
                separator: String::new(),
                segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
            }],
        },
        PathBuf::from("/tmp/out.json"),
    );
    app.focus = Focus::Editor;
    app.selected_segment = Some(0);
    app.editor_field = EditorField::Color;
    // Pre-set to a non-zero value.
    app.color_picker_selected = 5;
    app.handle(key(KeyCode::Char('c')));
    assert_eq!(app.color_picker_selected, 0);
    assert_eq!(app.mode, Mode::PickingFgColor);
}

#[test]
fn fg_picker_j_moves_down() {
    let mut app = app_in_fg_picker();
    assert_eq!(app.color_picker_selected, 0);
    app.handle(key(KeyCode::Char('j')));
    assert_eq!(app.color_picker_selected, 1);
    app.handle(key(KeyCode::Down));
    assert_eq!(app.color_picker_selected, 2);
}

#[test]
fn fg_picker_k_moves_up() {
    let mut app = app_in_fg_picker();
    app.color_picker_selected = 3;
    app.handle(key(KeyCode::Char('k')));
    assert_eq!(app.color_picker_selected, 2);
    app.handle(key(KeyCode::Up));
    assert_eq!(app.color_picker_selected, 1);
}

#[test]
fn fg_picker_up_clamped_at_zero() {
    let mut app = app_in_fg_picker();
    assert_eq!(app.color_picker_selected, 0);
    app.handle(key(KeyCode::Up));
    assert_eq!(app.color_picker_selected, 0);
}

#[test]
fn fg_picker_down_clamped_at_max() {
    let mut app = app_in_fg_picker();
    // 9 entries (8 colors + none), max index = 8.
    for _ in 0..20 {
        app.handle(key(KeyCode::Down));
    }
    assert_eq!(app.color_picker_selected, 8);
}

#[test]
fn fg_picker_enter_writes_selected_color_and_dirties() {
    let mut app = app_in_fg_picker();
    // Move to "green" (index 1 in NAMED_COLORS = ["red","green",...]).
    app.handle(key(KeyCode::Char('j')));
    assert_eq!(app.color_picker_selected, 1);
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Browsing);
    assert!(app.dirty);
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.color.as_deref(), Some("green"));
    assert!(t.bg.is_none()); // bg unchanged
}

#[test]
fn fg_picker_enter_first_entry_writes_red() {
    let mut app = app_in_fg_picker();
    // selection is 0 = "red"
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Browsing);
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.color.as_deref(), Some("red"));
}

#[test]
fn fg_picker_enter_with_none_selected_writes_none() {
    let mut app = app_in_fg_picker();
    // "(none)" is at index 8 (last entry).
    app.color_picker_selected = 8;
    // Pre-populate color so we can verify it gets cleared.
    if let Segment::Template(t) = &mut app.config.lines[0].segments[0] {
        t.color = Some("red".into());
    }
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Browsing);
    assert!(app.dirty);
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert!(t.color.is_none());
}

#[test]
fn fg_picker_esc_returns_without_change() {
    let mut app = app_in_fg_picker();
    if let Segment::Template(t) = &mut app.config.lines[0].segments[0] {
        t.color = Some("cyan".into());
    }
    app.handle(key(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Browsing);
    assert!(!app.dirty);
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.color.as_deref(), Some("cyan")); // unchanged
}

#[test]
fn bg_picker_writes_bg_field_not_color() {
    let mut app = app_in_bg_picker();
    // Move to "blue" (index 3).
    for _ in 0..3 {
        app.handle(key(KeyCode::Char('j')));
    }
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Browsing);
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.bg.as_deref(), Some("blue"));
    assert!(t.color.is_none()); // color unchanged
}

#[test]
fn bg_picker_independent_from_fg_state() {
    let mut fg_app = app_in_fg_picker();
    let mut bg_app = app_in_bg_picker();

    fg_app.color_picker_selected = 2; // yellow
    bg_app.color_picker_selected = 5; // cyan

    fg_app.handle(key(KeyCode::Enter));
    bg_app.handle(key(KeyCode::Enter));

    let Segment::Template(ft) = &fg_app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    let Segment::Template(bt) = &bg_app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };

    assert_eq!(ft.color.as_deref(), Some("yellow"));
    assert!(ft.bg.is_none());
    assert_eq!(bt.bg.as_deref(), Some("cyan"));
    assert!(bt.color.is_none());
}

#[test]
fn color_picker_serde_round_trip_after_pick() {
    let mut app = app_in_fg_picker();
    app.handle(key(KeyCode::Enter)); // pick "red" (index 0)

    let json = serde_json::to_string(&app.config).expect("serialize");
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    let Segment::Template(t) = &back.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.color.as_deref(), Some("red"));
}
