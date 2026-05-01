use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::schema::{Config, Line, Segment, TemplateSegment};

use crate::tui::app::{App, Focus, Mode};

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

fn ctrl_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn editing_app() -> App {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.focus = Focus::Editor;
    app.selected_segment = Some(0);
    app.editing_buf = Some(String::new());
    app.mode = Mode::EditingTemplate;
    app
}

#[test]
fn ctrl_p_in_editing_enters_picker_mode() {
    let mut app = editing_app();
    app.handle(ctrl_key(KeyCode::Char('p')));
    assert_eq!(app.mode, Mode::PickingPlaceholder);
    assert_eq!(app.picker_filter, "");
    assert_eq!(app.picker_selected, 0);
}

#[test]
fn picker_typing_filters() {
    let mut app = editing_app();
    app.handle(ctrl_key(KeyCode::Char('p')));
    app.handle(key(KeyCode::Char('g')));
    app.handle(key(KeyCode::Char('i')));
    app.handle(key(KeyCode::Char('t')));
    assert_eq!(app.picker_filter, "git");
    assert_eq!(app.mode, Mode::PickingPlaceholder);
}

#[test]
fn picker_backspace_removes_last_filter_char() {
    let mut app = editing_app();
    app.handle(ctrl_key(KeyCode::Char('p')));
    app.handle(key(KeyCode::Char('g')));
    app.handle(key(KeyCode::Char('i')));
    app.handle(key(KeyCode::Backspace));
    assert_eq!(app.picker_filter, "g");
}

#[test]
fn picker_enter_inserts_at_end_of_buffer_and_returns_to_editing() {
    let mut app = editing_app();
    app.editing_buf = Some("prefix_".into());
    app.handle(ctrl_key(KeyCode::Char('p')));
    for c in "model".chars() {
        app.handle(key(KeyCode::Char(c)));
    }
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.mode, Mode::EditingTemplate);
    let buf = app.editing_buf.as_deref().unwrap_or("");
    assert!(buf.starts_with("prefix_"), "buf was: {buf}");
    assert!(buf.contains("{model"), "buf was: {buf}");
}

#[test]
fn picker_esc_returns_without_insertion() {
    let mut app = editing_app();
    app.editing_buf = Some("unchanged".into());
    app.handle(ctrl_key(KeyCode::Char('p')));
    app.handle(key(KeyCode::Char('m')));
    app.handle(key(KeyCode::Esc));
    assert_eq!(app.mode, Mode::EditingTemplate);
    assert_eq!(app.editing_buf.as_deref(), Some("unchanged"));
}

#[test]
fn picker_up_down_moves_selection() {
    let mut app = editing_app();
    app.handle(ctrl_key(KeyCode::Char('p')));
    assert_eq!(app.picker_selected, 0);
    app.handle(key(KeyCode::Down));
    assert_eq!(app.picker_selected, 1);
    app.handle(key(KeyCode::Down));
    assert_eq!(app.picker_selected, 2);
    app.handle(key(KeyCode::Up));
    assert_eq!(app.picker_selected, 1);
}

#[test]
fn picker_selection_clamped_at_zero_on_up() {
    let mut app = editing_app();
    app.handle(ctrl_key(KeyCode::Char('p')));
    assert_eq!(app.picker_selected, 0);
    app.handle(key(KeyCode::Up));
    assert_eq!(app.picker_selected, 0);
}

#[test]
fn picker_filter_resets_selection_to_zero() {
    let mut app = editing_app();
    app.handle(ctrl_key(KeyCode::Char('p')));
    app.handle(key(KeyCode::Down));
    app.handle(key(KeyCode::Down));
    app.handle(key(KeyCode::Char('g')));
    assert_eq!(app.picker_selected, 0);
}
