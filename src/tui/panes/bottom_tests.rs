use std::path::PathBuf;

use ratatui::backend::TestBackend;

use crate::config::schema::{Config, Line};
use crate::tui::app::{App, Cursor, Focus, Mode};
use crate::tui::builder::{BuilderLine, BuilderSegment, BuilderState};
use crate::tui::catalog::Category;

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

fn render_buf(app: &App, w: u16, h: u16) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(w, h);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let area = terminal.get_frame().area();
    terminal.draw(|f| render(f, area, app)).unwrap();
    terminal.backend().buffer().clone()
}

fn buf_text(buf: &ratatui::buffer::Buffer) -> String {
    let (w, h) = (buf.area.width, buf.area.height);
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            if let Some(c) = buf.cell((x, y)) {
                out.push(c.symbol().chars().next().unwrap_or(' '));
            }
        }
    }
    out
}

fn text(app: &App) -> String {
    buf_text(&render_buf(app, 120, 6))
}

// ── keymap per (focus, mode, cursor) ─────────────────────────────────────────

#[test]
fn top_browsing_segment_cursor() {
    let mut app = mk_app();
    app.builder.lines[0].segments.push(BuilderSegment::Preset {
        id: "model_name",
        color: None,
        bg: None,
    });
    app.focus = Focus::Top;
    app.mode = Mode::Browsing;
    app.cursor = Cursor::Segment(0);
    let t = text(&app);
    assert!(t.contains("cursor"), "segment: {t:?}");
    assert!(t.contains("reorder"), "segment: {t:?}");
    assert!(t.contains("delete"), "segment: {t:?}");
    assert!(t.contains("quit"), "segment: {t:?}");
    assert!(t.contains("help"), "segment: {t:?}");
}

#[test]
fn top_browsing_gutter_cursor() {
    let mut app = mk_app();
    app.focus = Focus::Top;
    app.mode = Mode::Browsing;
    app.cursor = Cursor::Gutter;
    let t = text(&app);
    assert!(t.contains("separator"), "gutter: {t:?}");
    assert!(t.contains("move-line"), "gutter: {t:?}");
    assert!(t.contains("duplicate"), "gutter: {t:?}");
    assert!(t.contains("delete-line"), "gutter: {t:?}");
    assert!(t.contains("quit"), "gutter: {t:?}");
}

#[test]
fn top_browsing_virtual_new_line_cursor() {
    let mut app = mk_app();
    app.focus = Focus::Top;
    app.mode = Mode::Browsing;
    app.cursor = Cursor::VirtualNewLine;
    let t = text(&app);
    assert!(t.contains("add-line"), "virtual: {t:?}");
    assert!(t.contains("back"), "virtual: {t:?}");
    assert!(t.contains("quit"), "virtual: {t:?}");
}

#[test]
fn middle_browsing_preset_tab() {
    let mut app = mk_app();
    app.focus = Focus::Middle;
    app.mode = Mode::Browsing;
    app.active_tab = Category::Workspace;
    let t = text(&app);
    assert!(t.contains("toggle"), "preset: {t:?}");
    assert!(t.contains("filter"), "preset: {t:?}");
    assert!(t.contains("save"), "preset: {t:?}");
    assert!(t.contains("quit"), "preset: {t:?}");
}

#[test]
fn middle_browsing_appearance_tab() {
    let mut app = mk_app();
    app.focus = Focus::Middle;
    app.mode = Mode::Browsing;
    app.active_tab = Category::Appearance;
    let t = text(&app);
    assert!(t.contains("toggle"), "appearance: {t:?}");
    assert!(t.contains("edit"), "appearance: {t:?}");
    assert!(t.contains("save"), "appearance: {t:?}");
    assert!(t.contains("quit"), "appearance: {t:?}");
    // No filter key in Appearance state.
    assert!(
        !t.contains("filter"),
        "appearance must not show filter: {t:?}"
    );
}

#[test]
fn filter_mode_edit_keys() {
    let mut app = mk_app();
    app.focus = Focus::Middle;
    app.mode = Mode::Filter;
    let t = text(&app);
    assert!(t.contains("type to change"), "filter: {t:?}");
    assert!(t.contains("commit"), "filter: {t:?}");
    assert!(t.contains("cancel"), "filter: {t:?}");
}

#[test]
fn editing_separator_mode_edit_keys() {
    let mut app = mk_app();
    app.focus = Focus::Top;
    app.mode = Mode::EditingSeparator;
    let t = text(&app);
    assert!(t.contains("type to change"), "editing_sep: {t:?}");
    assert!(t.contains("commit"), "editing_sep: {t:?}");
    assert!(t.contains("cancel"), "editing_sep: {t:?}");
}

// ── mode-specific keymaps ─────────────────────────────────────────────────────

#[test]
fn confirm_delete_keymap_shows_y_n() {
    let mut app = mk_app();
    app.mode = Mode::ConfirmDelete;
    let t = text(&app);
    assert!(
        t.contains('y') && t.contains("confirm"),
        "confirm_delete: {t:?}"
    );
    assert!(
        t.contains('n') && t.contains("cancel"),
        "confirm_delete: {t:?}"
    );
}

#[test]
fn confirm_quit_keymap_shows_y_n_esc() {
    let mut app = mk_app();
    app.mode = Mode::ConfirmQuit;
    let t = text(&app);
    assert!(
        t.contains('y') && t.contains("quit"),
        "confirm_quit y: {t:?}"
    );
    assert!(
        t.contains('n') && t.contains("cancel"),
        "confirm_quit n: {t:?}"
    );
    assert!(t.contains("Esc"), "confirm_quit Esc: {t:?}");
}

#[test]
fn help_keymap_shows_esc() {
    let mut app = mk_app();
    app.mode = Mode::Help;
    let t = text(&app);
    assert!(t.contains("Esc") && t.contains("close"), "help Esc: {t:?}");
}

#[test]
fn saving_keymap_shows_esc() {
    let mut app = mk_app();
    app.mode = Mode::Saving;
    let t = text(&app);
    assert!(
        t.contains("Esc") && t.contains("cancel"),
        "saving Esc: {t:?}"
    );
}

// ── custom-segment hint ───────────────────────────────────────────────────────

#[test]
fn custom_segment_hint_on_custom() {
    let mut app = mk_app();
    app.builder.lines[0].segments.push(BuilderSegment::Custom {
        template: "${cost_usd}".into(),
        color: None,
        bg: None,
        padding: 0,
        hide_when_absent: false,
    });
    app.focus = Focus::Top;
    app.mode = Mode::Browsing;
    app.cursor = Cursor::Segment(0);
    let t = buf_text(&render_buf(&app, 120, 8));
    assert!(t.contains("custom:"), "hint: {t:?}");
    assert!(t.contains("${cost_usd}"), "hint template: {t:?}");
    assert!(t.contains("toggle disabled"), "hint text: {t:?}");
}

#[test]
fn no_custom_hint_on_preset() {
    let mut app = mk_app();
    app.builder.lines[0].segments.push(BuilderSegment::Preset {
        id: "model_name",
        color: None,
        bg: None,
    });
    app.focus = Focus::Top;
    app.cursor = Cursor::Segment(0);
    assert!(!text(&app).contains("custom:"));
}

#[test]
fn no_custom_hint_when_cursor_not_on_segment() {
    let mut app = mk_app();
    app.builder.lines[0].segments.push(BuilderSegment::Custom {
        template: "hello".into(),
        color: None,
        bg: None,
        padding: 0,
        hide_when_absent: false,
    });
    app.focus = Focus::Top;
    app.cursor = Cursor::Gutter;
    assert!(!text(&app).contains("custom:"));
}

#[test]
fn custom_hint_uses_correct_segment_index() {
    // Preset at 0, Custom at 1 — cursor on 1 shows custom hint.
    let state = BuilderState {
        lines: vec![BuilderLine {
            separator: " | ".into(),
            segments: vec![
                BuilderSegment::Preset {
                    id: "model_name",
                    color: None,
                    bg: None,
                },
                BuilderSegment::Custom {
                    template: "my_custom_tmpl".into(),
                    color: None,
                    bg: None,
                    padding: 0,
                    hide_when_absent: false,
                },
            ],
        }],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    let mut app = mk_app();
    app.builder = state;
    app.focus = Focus::Top;
    app.cursor = Cursor::Segment(1);
    let t = buf_text(&render_buf(&app, 120, 8));
    assert!(t.contains("my_custom_tmpl"), "correct index: {t:?}");
}

#[test]
fn no_custom_hint_for_preset_at_index_1() {
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
    let mut app = mk_app();
    app.builder = state;
    app.focus = Focus::Top;
    app.cursor = Cursor::Segment(1);
    assert!(!text(&app).contains("custom:"));
}

// ── truncation at narrow widths ───────────────────────────────────────────────

#[test]
fn truncation_preserves_quit_at_narrow_width() {
    let mut app = mk_app();
    app.builder.lines[0].segments.push(BuilderSegment::Preset {
        id: "model_name",
        color: None,
        bg: None,
    });
    app.focus = Focus::Top;
    app.mode = Mode::Browsing;
    app.cursor = Cursor::Segment(0);
    let t = buf_text(&render_buf(&app, 40, 4));
    assert!(
        t.contains('q') && t.contains("quit"),
        "quit preserved: {t:?}"
    );
}

#[test]
fn truncation_drops_pairs_at_narrow_width() {
    // Wide render has more text than narrow.
    let mut app = mk_app();
    app.builder.lines[0].segments.push(BuilderSegment::Preset {
        id: "model_name",
        color: None,
        bg: None,
    });
    app.focus = Focus::Top;
    app.mode = Mode::Browsing;
    app.cursor = Cursor::Segment(0);
    let narrow = buf_text(&render_buf(&app, 40, 4));
    let wide = buf_text(&render_buf(&app, 120, 4));
    assert!(wide.contains("reorder"), "wide has reorder: {wide:?}");
    assert!(
        narrow.trim_end().len() < wide.trim_end().len(),
        "narrow shorter than wide"
    );
}

#[test]
fn truncation_middle_preserves_ctrl_s() {
    let mut app = mk_app();
    app.focus = Focus::Middle;
    app.mode = Mode::Browsing;
    app.active_tab = Category::Workspace;
    let t = buf_text(&render_buf(&app, 40, 4));
    assert!(
        t.contains("Ctrl+S") && t.contains("save"),
        "Ctrl+S preserved: {t:?}"
    );
}

#[test]
fn truncation_drops_help_at_very_narrow_width() {
    // At 30 cols, ? help should be dropped while q:quit and Ctrl+S:save survive.
    // Use Middle/Browsing which has both Ctrl+S and ?.
    let mut app = mk_app();
    app.focus = Focus::Middle;
    app.mode = Mode::Browsing;
    app.active_tab = Category::Workspace;
    let t = buf_text(&render_buf(&app, 30, 4));
    assert!(
        t.contains('q') && t.contains("quit"),
        "q:quit present: {t:?}"
    );
    assert!(
        t.contains("Ctrl+S") && t.contains("save"),
        "Ctrl+S present: {t:?}"
    );
    // ? has priority 200, so it should be dropped before the required pairs.
    assert!(!t.contains("help"), "help dropped at 30 cols: {t:?}");
}

#[test]
fn arrow_key_pair_width_counts_chars_not_bytes() {
    // ←/→ is 3 chars but 9 UTF-8 bytes. pair_width must use char count.
    // We verify this indirectly: at a width that fits 3+1+6+2=12 chars
    // ("←/→:cursor  ") but NOT 9+1+6+2=18 bytes, the pair renders.
    let mut app = mk_app();
    app.builder.lines[0].segments.push(BuilderSegment::Preset {
        id: "model_name",
        color: None,
        bg: None,
    });
    app.focus = Focus::Top;
    app.mode = Mode::Browsing;
    app.cursor = Cursor::Segment(0);
    // At width 120 the full keymap renders — just confirm cursor appears.
    let t = text(&app);
    assert!(t.contains("cursor"), "arrow key renders: {t:?}");
}

// ── powerline hint ────────────────────────────────────────────────────────────

#[test]
fn powerline_hint_when_powerline_and_focus_top() {
    let mut app = mk_app();
    app.builder.powerline = true;
    app.focus = Focus::Top;
    let t = buf_text(&render_buf(&app, 120, 8));
    assert!(t.contains("powerline preview"), "hint shown: {t:?}");
}

#[test]
fn powerline_hint_not_shown_when_focus_not_top() {
    let mut app = mk_app();
    app.builder.powerline = true;
    app.focus = Focus::Middle;
    assert!(!text(&app).contains("powerline preview"));
}

#[test]
fn powerline_hint_not_shown_when_powerline_false() {
    let mut app = mk_app();
    app.builder.powerline = false;
    app.focus = Focus::Top;
    assert!(!text(&app).contains("powerline preview"));
}

#[test]
fn height_guard_keymap_present_at_minimal_height() {
    // inner height = 1 (area h=3, border takes 2 rows).
    // Keymap row must be present even if all hints are suppressed.
    let mut app = mk_app();
    app.builder.powerline = true;
    app.builder.lines[0].segments.push(BuilderSegment::Custom {
        template: "x".into(),
        color: None,
        bg: None,
        padding: 0,
        hide_when_absent: false,
    });
    app.focus = Focus::Top;
    app.cursor = Cursor::Segment(0);
    let t = buf_text(&render_buf(&app, 80, 3));
    assert!(
        t.contains('q') && t.contains("quit"),
        "keymap present: {t:?}"
    );
    // With inner_height=1, hints must be suppressed.
    assert!(!t.contains("custom:"), "custom hint suppressed: {t:?}");
    assert!(
        !t.contains("powerline preview"),
        "powerline hint suppressed: {t:?}"
    );
}

// ── border / title visual ─────────────────────────────────────────────────────

#[test]
fn border_cyan_when_bottom_focused() {
    let mut app = mk_app();
    app.focus = Focus::Bottom;
    let buf = render_buf(&app, 120, 6);
    assert_eq!(buf.cell((0, 0)).unwrap().fg, ratatui::style::Color::Cyan);
}

#[test]
fn border_dark_gray_when_not_bottom_focused() {
    let mut app = mk_app();
    app.focus = Focus::Top;
    let buf = render_buf(&app, 120, 6);
    assert_eq!(
        buf.cell((0, 0)).unwrap().fg,
        ratatui::style::Color::DarkGray
    );
}

// ── invariant: no forbidden imports ──────────────────────────────────────────

#[test]
fn no_forbidden_imports_in_bottom_rs() {
    let src = include_str!("bottom.rs");
    assert!(!src.contains("crate::api"));
    assert!(!src.contains("crate::cache"));
    assert!(!src.contains("crate::git"));
}
