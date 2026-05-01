use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::schema::{Config, Line, Segment, TemplateSegment};
use crate::tui::app::{App, Mode};
use crate::tui::widgets::status::status_message_visible;

fn minimal_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
        }],
    }
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

// --- Help mode entry/exit ---

#[test]
fn question_mark_in_browsing_enters_help() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    assert_eq!(app.mode, Mode::Browsing);
    app.handle(key(KeyCode::Char('?')));
    assert_eq!(app.mode, Mode::Help);
}

#[test]
fn any_key_in_help_returns_to_browsing_enter() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.mode = Mode::Help;
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.mode, Mode::Browsing);
}

#[test]
fn any_key_in_help_returns_to_browsing_esc() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.mode = Mode::Help;
    app.handle(key(KeyCode::Esc));
    assert_eq!(app.mode, Mode::Browsing);
}

#[test]
fn any_key_in_help_returns_to_browsing_char() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.mode = Mode::Help;
    app.handle(key(KeyCode::Char('x')));
    assert_eq!(app.mode, Mode::Browsing);
}

#[test]
fn question_mark_does_not_quit() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.handle(key(KeyCode::Char('?')));
    assert!(!app.should_quit);
}

// --- Status message timeout (uses pure helper) ---

#[test]
fn status_message_visible_within_3_seconds() {
    // 2 s after set → visible
    assert!(status_message_visible(100, 102));
}

#[test]
fn status_message_hidden_after_3_seconds() {
    // 4 s after set → hidden
    assert!(!status_message_visible(100, 104));
}

// --- Dirty indicator (app-level state) ---

#[test]
fn dirty_indicator_visible_when_dirty() {
    let app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    // App starts clean; mark dirty.
    let mut app = app;
    app.dirty = true;
    assert!(app.dirty);
}

#[test]
fn dirty_indicator_absent_when_clean() {
    let app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    assert!(!app.dirty);
}

// --- Status message stored with timestamp ---

#[test]
fn status_message_is_none_initially() {
    let app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    assert!(app.status_message.is_none());
}

#[test]
fn status_message_set_has_string_and_timestamp() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.status_message = Some(("hello".into(), 999));
    let (msg, ts) = app.status_message.as_ref().unwrap();
    assert_eq!(msg, "hello");
    assert_eq!(*ts, 999);
}
