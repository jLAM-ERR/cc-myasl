use std::path::PathBuf;

use ratatui::backend::TestBackend;

use crate::config::named_color::NamedColor;
use crate::config::schema::{Config, Line};
use crate::tui::app4::{App, Focus};
use crate::tui::builder::{BuilderLine, BuilderState};

use super::{AppearanceRow, render, rows};

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

fn two_line_app(sep_a: &str, sep_b: &str) -> App {
    let state = BuilderState {
        lines: vec![
            BuilderLine {
                separator: sep_a.into(),
                segments: vec![],
            },
            BuilderLine {
                separator: sep_b.into(),
                segments: vec![],
            },
        ],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    app_with_state(state)
}

fn render_to_buffer(app: &App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(60, 10);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let area = terminal.get_frame().area();
    terminal.draw(|f| render(f, area, app)).unwrap();
    terminal.backend().buffer().clone()
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
fn powerline_toggle_flips_and_dirties() {
    let mut app = mk_app();
    assert!(!app.builder.powerline);
    assert!(!app.dirty);

    app.toggle_powerline();
    assert!(app.builder.powerline);
    assert!(app.dirty);

    app.toggle_powerline();
    assert!(!app.builder.powerline);
    assert!(app.dirty);
}

#[test]
fn separator_set_updates_right_line() {
    let mut app = two_line_app(" | ", " · ");

    app.set_separator(1, " - ".to_string());

    assert_eq!(app.builder.lines[1].separator, " - ");
    assert_eq!(
        app.builder.lines[0].separator, " | ",
        "line 0 must be unchanged"
    );
    assert!(app.dirty);
}

#[test]
fn separator_two_lines_only_two_sep_rows_visible() {
    let app = two_line_app(" | ", " · ");
    let visible = rows(&app);

    assert_eq!(
        visible,
        vec![
            AppearanceRow::Powerline,
            AppearanceRow::DefaultFg,
            AppearanceRow::DefaultBg,
            AppearanceRow::Separator(0),
            AppearanceRow::Separator(1),
        ],
        "two-line app must expose exactly 5 rows"
    );
}

#[test]
fn one_line_only_one_sep_row_visible() {
    let app = mk_app();
    let visible = rows(&app);

    assert_eq!(visible.len(), 4);
    assert_eq!(visible[3], AppearanceRow::Separator(0));
}

#[test]
fn three_lines_three_sep_rows_visible() {
    let state = BuilderState {
        lines: vec![
            BuilderLine {
                separator: "a".into(),
                segments: vec![],
            },
            BuilderLine {
                separator: "b".into(),
                segments: vec![],
            },
            BuilderLine {
                separator: "c".into(),
                segments: vec![],
            },
        ],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    let app = app_with_state(state);
    let visible = rows(&app);

    assert_eq!(visible.len(), 6);
    assert_eq!(visible[5], AppearanceRow::Separator(2));
}

#[test]
fn appearance_render_does_not_panic() {
    let app = mk_app();
    let _buf = render_to_buffer(&app);
}

#[test]
fn default_fg_set_dirties() {
    let mut app = mk_app();
    assert!(!app.dirty);

    app.set_default_fg(Some(NamedColor::Red));
    assert_eq!(app.builder.default_fg, Some(NamedColor::Red));
    assert!(app.dirty);
}

#[test]
fn default_bg_set_dirties() {
    let mut app = mk_app();
    app.set_default_bg(Some(NamedColor::Cyan));
    assert_eq!(app.builder.default_bg, Some(NamedColor::Cyan));
    assert!(app.dirty);
}

#[test]
fn powerline_row_shows_on_when_true() {
    let mut app = mk_app();
    app.toggle_powerline();

    let buf = render_to_buffer(&app);
    let text = row_text(&buf, 0);
    assert!(
        text.contains("on"),
        "powerline=true must show 'on': {text:?}"
    );
    assert!(
        text.contains("[x]"),
        "powerline=true must show [x]: {text:?}"
    );
}

#[test]
fn powerline_row_shows_off_when_false() {
    let app = mk_app();
    let buf = render_to_buffer(&app);
    let text = row_text(&buf, 0);
    assert!(
        text.contains("off"),
        "powerline=false must show 'off': {text:?}"
    );
    assert!(
        text.contains("[ ]"),
        "powerline=false must show [ ]: {text:?}"
    );
}

#[test]
fn separator_value_quoted_in_render() {
    let app = two_line_app(" | ", " · ");
    let buf = render_to_buffer(&app);

    // Row 3 = Line 1 separator.
    let text = row_text(&buf, 3);
    assert!(
        text.contains('"'),
        "separator row must display quoted value: {text:?}"
    );
}

#[test]
fn selected_row_reversed_when_middle_focused() {
    let mut app = mk_app();
    app.focus = Focus::Middle;
    app.picker_selected = 0;

    let buf = render_to_buffer(&app);

    // Row 0 = Powerline row (selected).
    let cell = buf.cell((0, 0)).expect("cell exists");
    assert!(
        cell.modifier.contains(ratatui::style::Modifier::REVERSED),
        "selected row must have REVERSED modifier when focus=Middle"
    );
}

#[test]
fn non_selected_row_not_reversed_when_middle_focused() {
    let mut app = mk_app();
    app.focus = Focus::Middle;
    app.picker_selected = 0;

    let buf = render_to_buffer(&app);

    // Row 1 = DefaultFg row (not selected).
    let cell = buf.cell((0, 1)).expect("cell exists");
    assert!(
        !cell.modifier.contains(ratatui::style::Modifier::REVERSED),
        "non-selected row must NOT have REVERSED"
    );
}

#[test]
fn fg_color_name_shown_when_set() {
    let mut app = mk_app();
    app.set_default_fg(Some(NamedColor::Cyan));

    let buf = render_to_buffer(&app);
    // Row 1 = DefaultFg row.
    let text = row_text(&buf, 1);
    assert!(
        text.contains("cyan"),
        "fg row must show color name when set: {text:?}"
    );
    assert!(
        text.contains("[x]"),
        "fg row must show [x] when fg is set: {text:?}"
    );
}

#[test]
fn fg_shows_none_when_unset() {
    let app = mk_app();
    let buf = render_to_buffer(&app);
    let text = row_text(&buf, 1);
    assert!(
        text.contains("none"),
        "fg row must show 'none' when unset: {text:?}"
    );
    assert!(
        text.contains("[ ]"),
        "fg row must show [ ] when unset: {text:?}"
    );
}
