use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::schema::{Config, Line, Segment, TemplateSegment};

use crate::tui::app::{App, Mode};

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

fn shift_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

#[test]
fn shift_p_toggles_powerline_off_to_on() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    assert!(!app.config.powerline);
    app.handle(shift_key(KeyCode::Char('P')));
    assert!(app.config.powerline);
    assert!(app.dirty);
}

#[test]
fn shift_p_toggles_powerline_on_to_off() {
    let mut app = App::new(
        Config {
            schema_url: None,
            powerline: true,
            lines: vec![Line {
                separator: String::new(),
                segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
            }],
        },
        PathBuf::from("/tmp/out.json"),
    );
    assert!(app.config.powerline);
    app.handle(shift_key(KeyCode::Char('P')));
    assert!(!app.config.powerline);
    assert!(app.dirty);
}

#[test]
fn shift_p_in_non_browsing_mode_does_nothing() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.mode = Mode::EditingTemplate;
    app.editing_buf = Some(String::new());
    app.handle(shift_key(KeyCode::Char('P')));
    // In EditingTemplate mode, 'P' is a literal char appended to the buffer.
    assert!(!app.config.powerline);
    assert!(!app.dirty);
    assert_eq!(app.editing_buf.as_deref(), Some("P"));
}

#[test]
fn powerline_toggle_round_trip_serde() {
    let config = Config {
        schema_url: None,
        powerline: true,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
        }],
    };
    let json = serde_json::to_string(&config).expect("serialize");
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    assert!(back.powerline);
}
