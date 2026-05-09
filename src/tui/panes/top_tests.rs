use std::path::PathBuf;

use ratatui::{backend::TestBackend, style::Modifier};

use crate::config::schema::{Config, Line, MAX_LINES};
use crate::tui::app4::{App, Cursor, Focus};
use crate::tui::builder::{BuilderLine, BuilderSegment, BuilderState};

use super::render;

// ── helpers ───────────────────────────────────────────────────────────────────

fn empty_config() -> Config {
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

fn mk_app() -> App {
    App::new(empty_config(), PathBuf::from("/tmp/test.json"))
}

fn app_with_state(state: BuilderState) -> App {
    let mut a = mk_app();
    a.builder = state;
    a
}

fn render_to_buffer(app: &App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(80, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let area = terminal.get_frame().area();
    terminal.draw(|f| render(f, area, app)).unwrap();
    terminal.backend().buffer().clone()
}

fn has_modifier_at(buf: &ratatui::buffer::Buffer, x: u16, y: u16, m: Modifier) -> bool {
    buf.cell((x, y))
        .map(|c| c.modifier.contains(m))
        .unwrap_or(false)
}

fn row_text(buf: &ratatui::buffer::Buffer, y: u16) -> String {
    let width = buf.area.width;
    (0..width)
        .map(|x| {
            buf.cell((x, y))
                .map(|c| c.symbol().chars().next().unwrap_or(' '))
                .unwrap_or(' ')
        })
        .collect()
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[test]
fn cursor_segment_reversed() {
    // App with one line containing two preset segments; cursor on segment 1.
    let state = BuilderState {
        lines: vec![BuilderLine {
            separator: " | ".into(),
            segments: vec![
                BuilderSegment::Preset {
                    id: "model_name",
                    color: None,
                    bg: None,
                },
                BuilderSegment::Preset {
                    id: "cost_usd",
                    color: None,
                    bg: None,
                },
            ],
        }],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    let mut app = app_with_state(state);
    app.focus = Focus::Top;
    app.cursor = Cursor::Segment(1);
    app.active_line = 0;

    let buf = render_to_buffer(&app);

    // Find a cell that belongs to the second segment (past gutter + separator).
    // We look for REVERSED modifier in row 1 (first data row after border).
    let row = 1u16; // first content row inside the block border
    let found_reversed = (2..80u16).any(|x| has_modifier_at(&buf, x, row, Modifier::REVERSED));
    assert!(
        found_reversed,
        "cursor segment (index 1) must have REVERSED modifier"
    );
}

#[test]
fn custom_segment_dim() {
    let state = BuilderState {
        lines: vec![BuilderLine {
            separator: " | ".into(),
            segments: vec![BuilderSegment::Custom {
                template: "hello".into(),
                color: None,
                bg: None,
                padding: 0,
                hide_when_absent: false,
            }],
        }],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    let mut app = app_with_state(state);
    // Move cursor away from the custom segment so DIM is not masked by REVERSED.
    app.focus = Focus::Middle;

    let buf = render_to_buffer(&app);

    // Row 1 = first data line inside border.
    let row = 1u16;
    // Skip gutter (2 chars at x=1..2), look for DIM in the content area.
    let found_dim = (3..80u16).any(|x| has_modifier_at(&buf, x, row, Modifier::DIM));
    assert!(found_dim, "Custom segment must render with Modifier::DIM");
}

#[test]
fn virtual_new_line_visible_when_less_than_max() {
    // One line → virtual row must appear.
    let state = BuilderState {
        lines: vec![BuilderLine {
            separator: " | ".into(),
            segments: vec![],
        }],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    let app = app_with_state(state);
    let buf = render_to_buffer(&app);

    // Row 2 (after border row 0 and line row 1) should contain "+ new line".
    let text2 = row_text(&buf, 2);
    assert!(
        text2.contains('+') && text2.contains('n'),
        "virtual row must be visible when lines < {MAX_LINES}: got {text2:?}"
    );
}

#[test]
fn virtual_new_line_hidden_when_at_max() {
    // Three lines → virtual row must NOT appear.
    let state = BuilderState {
        lines: vec![
            BuilderLine {
                separator: " | ".into(),
                segments: vec![],
            },
            BuilderLine {
                separator: " | ".into(),
                segments: vec![],
            },
            BuilderLine {
                separator: " | ".into(),
                segments: vec![],
            },
        ],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    let app = app_with_state(state);
    let buf = render_to_buffer(&app);

    // Row 4 would be the virtual row position; confirm `+` does not appear.
    let text4 = row_text(&buf, 4);
    assert!(
        !text4.contains('+'),
        "virtual row must NOT appear at MAX_LINES=3: row 4 = {text4:?}"
    );
}

#[test]
fn gutter_marker_only_on_active_line() {
    let state = BuilderState {
        lines: vec![
            BuilderLine {
                separator: " | ".into(),
                segments: vec![],
            },
            BuilderLine {
                separator: " | ".into(),
                segments: vec![],
            },
        ],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    let mut app = app_with_state(state);
    app.active_line = 0;

    let buf = render_to_buffer(&app);

    // Row 1 = line 0 (active) → should have `>`.
    // Row 2 = line 1 (inactive) → should have ` ` in gutter.
    let active_cell = buf
        .cell((1, 1))
        .map(|c| c.symbol().to_owned())
        .unwrap_or_default();
    let inactive_cell = buf
        .cell((1, 2))
        .map(|c| c.symbol().to_owned())
        .unwrap_or_default();

    assert_eq!(active_cell, ">", "active line must show `>` in gutter");
    assert_eq!(inactive_cell, " ", "inactive line must show ` ` in gutter");
}

#[test]
fn border_color_cyan_when_top_focused() {
    let mut app = mk_app();
    app.focus = Focus::Top;

    let buf = render_to_buffer(&app);

    // Top-left corner of the block border should be cyan.
    let cell = buf.cell((0, 0)).expect("cell must exist");
    assert_eq!(
        cell.fg,
        ratatui::style::Color::Cyan,
        "border must be Cyan when focus=Top"
    );
}

#[test]
fn border_color_dark_gray_when_not_top_focused() {
    let mut app = mk_app();
    app.focus = Focus::Middle;

    let buf = render_to_buffer(&app);

    let cell = buf.cell((0, 0)).expect("cell must exist");
    assert_eq!(
        cell.fg,
        ratatui::style::Color::DarkGray,
        "border must be DarkGray when focus!=Top"
    );
}

#[test]
fn virtual_new_line_reversed_when_cursor_on_it() {
    let state = BuilderState {
        lines: vec![BuilderLine {
            separator: " | ".into(),
            segments: vec![],
        }],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    let mut app = app_with_state(state);
    app.focus = Focus::Top;
    app.cursor = Cursor::VirtualNewLine;

    let buf = render_to_buffer(&app);

    // Row 2 = virtual new-line row; check for REVERSED modifier.
    let found = (0..80u16).any(|x| has_modifier_at(&buf, x, 2, Modifier::REVERSED));
    assert!(found, "virtual row must be REVERSED when cursor is on it");
}
