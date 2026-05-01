use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::schema::{Config, Line, Segment, TemplateSegment};
use crate::tui::app::{App, Mode};

fn valid_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
        }],
    }
}

fn ctrl(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn ctrl_s_in_browsing_calls_save_and_clears_dirty() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut app = App::new(valid_config(), path.clone());
    app.dirty = true;
    app.handle(ctrl('s'));
    assert!(!app.dirty, "dirty must be cleared after successful save");
    assert!(path.exists(), "config file must be written");
    assert!(
        app.status_message.is_some(),
        "status message must be set on save"
    );
    let (msg, _ts) = app.status_message.as_ref().unwrap();
    assert!(msg.contains("Saved"), "message should mention Saved");
}

#[test]
fn ctrl_s_with_invalid_config_enters_saving_mode() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut cfg = valid_config();
    // Force 4 lines — bypasses validate_and_clamp.
    cfg.lines.push(cfg.lines[0].clone());
    cfg.lines.push(cfg.lines[0].clone());
    cfg.lines.push(cfg.lines[0].clone());
    let mut app = App::new(cfg, path.clone());
    app.handle(ctrl('s'));
    assert_eq!(app.mode, Mode::Saving);
    assert!(
        !app.last_save_errors.is_empty(),
        "last_save_errors must be populated"
    );
    assert!(
        !path.exists(),
        "file must not be written on validation error"
    );
}

#[test]
fn q_when_dirty_enters_confirm_quit() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut app = App::new(valid_config(), path);
    app.dirty = true;
    app.handle(key(KeyCode::Char('q')));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);
}

#[test]
fn confirm_quit_y_quits_without_saving() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut app = App::new(valid_config(), path.clone());
    app.dirty = true;
    app.mode = Mode::ConfirmQuit;
    app.handle(key(KeyCode::Char('y')));
    assert!(app.should_quit);
    assert!(!path.exists(), "no save should happen on y");
}

#[test]
fn confirm_quit_capital_y_quits_without_saving() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut app = App::new(valid_config(), path.clone());
    app.dirty = true;
    app.mode = Mode::ConfirmQuit;
    app.handle(KeyEvent::new(KeyCode::Char('Y'), KeyModifiers::SHIFT));
    assert!(app.should_quit);
    assert!(!path.exists());
}

#[test]
fn confirm_quit_n_returns_to_browsing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut app = App::new(valid_config(), path);
    app.dirty = true;
    app.mode = Mode::ConfirmQuit;
    app.handle(key(KeyCode::Char('n')));
    assert_eq!(app.mode, Mode::Browsing);
    assert!(!app.should_quit);
}

#[test]
fn confirm_quit_s_saves_then_quits() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut app = App::new(valid_config(), path.clone());
    app.dirty = true;
    app.mode = Mode::ConfirmQuit;
    app.handle(key(KeyCode::Char('s')));
    assert!(app.should_quit, "must quit after save");
    assert!(!app.dirty, "dirty cleared after save");
    assert!(path.exists(), "file must be written");
}

#[test]
fn confirm_quit_s_with_invalid_config_surfaces_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("config.json");
    let mut cfg = valid_config();
    cfg.lines.push(cfg.lines[0].clone());
    cfg.lines.push(cfg.lines[0].clone());
    cfg.lines.push(cfg.lines[0].clone());
    let mut app = App::new(cfg, path.clone());
    app.mode = Mode::ConfirmQuit;
    app.handle(key(KeyCode::Char('s')));
    assert!(!app.should_quit, "must NOT quit on save failure");
    assert_eq!(app.mode, Mode::Saving);
    assert!(!app.last_save_errors.is_empty());
}
