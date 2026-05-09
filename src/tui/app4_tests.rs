use std::path::PathBuf;

use crate::config::schema::{Config, Line, MAX_LINES};
use crate::tui::builder::{BuilderLine, BuilderSegment, BuilderState};
use crate::tui::catalog::Category;

use super::{App, Cursor, Focus, Mode};

// ── helpers ───────────────────────────────────────────────────────────────────

pub(super) fn empty_config() -> Config {
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

pub(super) fn mk_app() -> App {
    App::new(empty_config(), PathBuf::from("/tmp/test.json"))
}

pub(super) fn app_with_builder(state: BuilderState) -> App {
    let mut a = mk_app();
    a.builder = state;
    a
}

pub(super) fn three_line_state() -> BuilderState {
    BuilderState {
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
    }
}

pub(super) fn two_line_state(sep_a: &str, sep_b: &str) -> BuilderState {
    BuilderState {
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
    }
}

// ── App::new ──────────────────────────────────────────────────────────────────

#[test]
fn new_initial_focus_top() {
    assert_eq!(mk_app().focus, Focus::Top);
}

#[test]
fn new_initial_cursor_gutter() {
    assert_eq!(mk_app().cursor, Cursor::Gutter);
}

#[test]
fn new_initial_active_line_zero() {
    assert_eq!(mk_app().active_line, 0);
}

#[test]
fn new_initial_tab_workspace() {
    assert_eq!(mk_app().active_tab, Category::Workspace);
}

#[test]
fn new_not_dirty() {
    assert!(!mk_app().dirty);
}

#[test]
fn new_should_not_quit() {
    assert!(!mk_app().should_quit);
}

// ── focus cycle ───────────────────────────────────────────────────────────────

#[test]
fn focus_forward_top_to_middle() {
    let mut a = mk_app();
    a.focus_cycle_forward();
    assert_eq!(a.focus, Focus::Middle);
}

#[test]
fn focus_forward_middle_to_bottom() {
    let mut a = mk_app();
    a.focus = Focus::Middle;
    a.focus_cycle_forward();
    assert_eq!(a.focus, Focus::Bottom);
}

#[test]
fn focus_forward_bottom_to_top() {
    let mut a = mk_app();
    a.focus = Focus::Bottom;
    a.focus_cycle_forward();
    assert_eq!(a.focus, Focus::Top);
}

#[test]
fn focus_backward_top_to_bottom() {
    let mut a = mk_app();
    a.focus_cycle_backward();
    assert_eq!(a.focus, Focus::Bottom);
}

#[test]
fn focus_backward_middle_to_top() {
    let mut a = mk_app();
    a.focus = Focus::Middle;
    a.focus_cycle_backward();
    assert_eq!(a.focus, Focus::Top);
}

#[test]
fn focus_backward_bottom_to_middle() {
    let mut a = mk_app();
    a.focus = Focus::Bottom;
    a.focus_cycle_backward();
    assert_eq!(a.focus, Focus::Middle);
}

#[test]
fn focus_forward_full_round_trip() {
    let mut a = mk_app();
    for _ in 0..3 {
        a.focus_cycle_forward();
    }
    assert_eq!(a.focus, Focus::Top);
}

// ── tab cycle ─────────────────────────────────────────────────────────────────

#[test]
fn tab_next_workspace_to_git() {
    let mut a = mk_app();
    a.tab_cycle_next();
    assert_eq!(a.active_tab, Category::Git);
}

#[test]
fn tab_next_wraps_appearance_to_workspace() {
    let mut a = mk_app();
    a.active_tab = Category::Appearance;
    a.tab_cycle_next();
    assert_eq!(a.active_tab, Category::Workspace);
}

#[test]
fn tab_prev_workspace_to_appearance() {
    let mut a = mk_app();
    a.tab_cycle_prev();
    assert_eq!(a.active_tab, Category::Appearance);
}

#[test]
fn tab_prev_git_to_workspace() {
    let mut a = mk_app();
    a.active_tab = Category::Git;
    a.tab_cycle_prev();
    assert_eq!(a.active_tab, Category::Workspace);
}

#[test]
fn tab_all_8_categories_round_trip() {
    let mut a = mk_app();
    for _ in 0..8 {
        a.tab_cycle_next();
    }
    assert_eq!(a.active_tab, Category::Workspace);
}

// ── cursor walks ──────────────────────────────────────────────────────────────

#[test]
fn cursor_right_gutter_no_segments_stays() {
    let mut a = mk_app();
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Gutter);
}

#[test]
fn cursor_right_gutter_moves_to_segment0() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.cursor = Cursor::Gutter;
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Segment(0));
}

#[test]
fn cursor_right_at_last_segment_no_op() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.cursor = Cursor::Segment(0);
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Segment(0));
}

#[test]
fn cursor_left_segment0_to_gutter() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.cursor = Cursor::Segment(0);
    a.cursor_left();
    assert_eq!(a.cursor, Cursor::Gutter);
}

#[test]
fn cursor_left_segment1_to_segment0() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.toggle_preset(Category::SessionModel, 1);
    a.cursor = Cursor::Segment(1);
    a.cursor_left();
    assert_eq!(a.cursor, Cursor::Segment(0));
}

#[test]
fn cursor_left_gutter_no_op() {
    let mut a = mk_app();
    a.cursor_left();
    assert_eq!(a.cursor, Cursor::Gutter);
}

#[test]
fn cursor_down_from_last_line_shows_virtual() {
    let mut a = mk_app();
    a.cursor_down_line();
    assert_eq!(a.cursor, Cursor::VirtualNewLine);
}

#[test]
fn cursor_down_virtual_no_op() {
    let mut a = mk_app();
    a.cursor = Cursor::VirtualNewLine;
    a.cursor_down_line();
    assert_eq!(a.cursor, Cursor::VirtualNewLine);
}

#[test]
fn cursor_up_from_virtual_goes_to_last_real_gutter() {
    let mut a = mk_app();
    a.cursor = Cursor::VirtualNewLine;
    a.cursor_up_line();
    assert_eq!(a.active_line, 0);
    assert_eq!(a.cursor, Cursor::Gutter);
}

#[test]
fn cursor_up_from_line0_no_op() {
    let mut a = mk_app();
    a.cursor_up_line();
    assert_eq!(a.active_line, 0);
}

#[test]
fn cursor_down_no_virtual_at_max_lines() {
    let mut a = app_with_builder(three_line_state());
    a.active_line = MAX_LINES - 1;
    a.cursor_down_line();
    assert_ne!(a.cursor, Cursor::VirtualNewLine);
    assert_eq!(a.active_line, MAX_LINES - 1);
}

#[test]
fn cursor_down_walks_real_lines() {
    let mut a = app_with_builder(three_line_state());
    a.active_line = 0;
    a.cursor_down_line();
    assert_eq!(a.active_line, 1);
    assert_eq!(a.cursor, Cursor::Gutter);
}

#[test]
fn cursor_right_advances_through_multi_segment_line() {
    // 2 segments: Gutter → Seg(0) → Seg(1) → no-op at Seg(1)
    let mut a = mk_app();
    a.toggle_preset(Category::Workspace, 0); // cwd_basename
    a.toggle_preset(Category::SessionModel, 0); // model_name
    assert_eq!(a.builder.lines[0].segments.len(), 2);
    a.cursor = Cursor::Gutter;
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Segment(0));
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Segment(1));
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Segment(1), "no-op at last segment");

    // cursor_left walks back: Seg(1) → Seg(0) → Gutter → no-op
    a.cursor_left();
    assert_eq!(a.cursor, Cursor::Segment(0));
    a.cursor_left();
    assert_eq!(a.cursor, Cursor::Gutter);
    a.cursor_left();
    assert_eq!(a.cursor, Cursor::Gutter, "no-op at Gutter");
}

#[test]
fn cursor_right_advances_through_three_segment_line() {
    // 3 segments: confirm correct boundary at Seg(2)
    let mut a = mk_app();
    a.toggle_preset(Category::Workspace, 0);
    a.toggle_preset(Category::SessionModel, 0);
    a.toggle_preset(Category::SessionModel, 1);
    assert_eq!(a.builder.lines[0].segments.len(), 3);
    a.cursor = Cursor::Gutter;
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Segment(0));
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Segment(1));
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Segment(2));
    a.cursor_right();
    assert_eq!(a.cursor, Cursor::Segment(2), "no-op at last segment");
}

// ── add_line ──────────────────────────────────────────────────────────────────

#[test]
fn add_line_pushes_and_moves_cursor() {
    let mut a = mk_app();
    a.add_line();
    assert_eq!(a.builder.lines.len(), 2);
    assert_eq!(a.active_line, 1);
    assert_eq!(a.cursor, Cursor::Gutter);
    assert!(a.dirty);
}

#[test]
fn add_line_at_max_sets_status() {
    let mut a = app_with_builder(three_line_state());
    a.add_line();
    assert_eq!(a.builder.lines.len(), 3);
    let (msg, _) = a.status_message.as_ref().unwrap();
    assert_eq!(msg, "max 3 lines");
}

// ── edit_separator / ConfirmQuit / status_message ────────────────────────────

#[test]
fn edit_separator_enters_mode() {
    let mut a = mk_app();
    a.edit_separator();
    assert_eq!(a.mode, Mode::EditingSeparator);
}

#[test]
fn request_quit_clean() {
    let mut a = mk_app();
    a.request_quit();
    assert!(a.should_quit);
}

#[test]
fn request_quit_dirty_enters_confirm() {
    let mut a = mk_app();
    a.dirty = true;
    a.request_quit();
    assert_eq!(a.mode, Mode::ConfirmQuit);
    assert!(!a.should_quit);
}

#[test]
fn confirm_quit_yes() {
    let mut a = mk_app();
    a.mode = Mode::ConfirmQuit;
    a.confirm_quit_yes();
    assert!(a.should_quit);
    assert_eq!(a.mode, Mode::Browsing);
}

#[test]
fn confirm_quit_no() {
    let mut a = mk_app();
    a.mode = Mode::ConfirmQuit;
    a.confirm_quit_no();
    assert_eq!(a.mode, Mode::Browsing);
    assert!(!a.should_quit);
}

#[test]
fn cannot_remove_last_line_sets_status() {
    let mut a = mk_app();
    a.delete_line();
    assert_eq!(a.builder.lines.len(), 1);
    let (msg, expiry) = a.status_message.as_ref().unwrap();
    assert_eq!(msg, "cannot remove last line");
    assert!(*expiry > 0);
}

// ── toggle_preset ─────────────────────────────────────────────────────────────

#[test]
fn toggle_preset_adds() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    assert_eq!(a.builder.lines[0].segments.len(), 1);
    assert!(a.dirty);
}

#[test]
fn toggle_preset_removes() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.dirty = false;
    a.toggle_preset(Category::SessionModel, 0);
    assert_eq!(a.builder.lines[0].segments.len(), 0);
    assert!(a.dirty);
}

#[test]
fn toggle_preset_out_of_range_no_op() {
    let mut a = mk_app();
    a.toggle_preset(Category::Workspace, 9999);
    assert_eq!(a.builder.lines[0].segments.len(), 0);
    assert!(!a.dirty);
}

#[test]
fn toggle_preset_custom_protection() {
    use crate::tui::catalog::by_category;
    let preset = by_category(Category::SessionModel).next().unwrap();
    let mut a = app_with_builder(BuilderState {
        lines: vec![BuilderLine {
            separator: " | ".into(),
            segments: vec![BuilderSegment::Custom {
                template: preset.template.to_owned(),
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
    });
    a.toggle_preset(Category::SessionModel, 0);
    assert_eq!(a.builder.lines[0].segments.len(), 2);
    assert!(
        a.builder.lines[0]
            .segments
            .iter()
            .any(|s| matches!(s, BuilderSegment::Custom { .. }))
    );
    assert!(
        a.builder.lines[0]
            .segments
            .iter()
            .any(|s| matches!(s, BuilderSegment::Preset { id, .. } if *id == preset.id))
    );
}
