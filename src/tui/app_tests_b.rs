use crate::tui::builder::{BuilderLine, BuilderSegment, BuilderState};
use crate::tui::catalog::Category;

use super::tests::{app_with_builder, mk_app, three_line_state, two_line_state};
use super::{App, Cursor, Mode};

// ── delete_line ───────────────────────────────────────────────────────────────

#[test]
fn delete_line_empty_removes_immediately() {
    let mut a = app_with_builder(BuilderState {
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
    });
    a.active_line = 1;
    a.delete_line();
    assert_eq!(a.builder.lines.len(), 1);
    assert_eq!(a.mode, Mode::Browsing);
    assert!(a.dirty);
}

fn two_line_with_preset() -> App {
    app_with_builder(BuilderState {
        lines: vec![
            BuilderLine {
                separator: " | ".into(),
                segments: vec![],
            },
            BuilderLine {
                separator: " | ".into(),
                segments: vec![BuilderSegment::Preset {
                    id: "model_name",
                    color: None,
                    bg: None,
                }],
            },
        ],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    })
}

#[test]
fn delete_line_with_segments_enters_confirm_delete() {
    let mut a = two_line_with_preset();
    a.active_line = 1;
    a.delete_line();
    assert_eq!(a.mode, Mode::ConfirmDelete);
    assert_eq!(a.builder.lines.len(), 2);
}

#[test]
fn confirm_delete_yes_removes_line() {
    let mut a = two_line_with_preset();
    a.active_line = 1;
    a.mode = Mode::ConfirmDelete;
    a.confirm_delete_yes();
    assert_eq!(a.builder.lines.len(), 1);
    assert_eq!(a.mode, Mode::Browsing);
    assert!(a.dirty);
}

#[test]
fn confirm_delete_no_no_change() {
    let mut a = two_line_with_preset();
    a.active_line = 1;
    a.mode = Mode::ConfirmDelete;
    a.confirm_delete_no();
    assert_eq!(a.builder.lines.len(), 2);
    assert_eq!(a.mode, Mode::Browsing);
}

// ── move_line_up/down ─────────────────────────────────────────────────────────

#[test]
fn move_line_up_swaps() {
    let mut a = app_with_builder(two_line_state("A", "B"));
    a.active_line = 1;
    a.move_line_up();
    assert_eq!(a.active_line, 0);
    assert_eq!(a.builder.lines[0].separator, "B");
    assert_eq!(a.builder.lines[1].separator, "A");
    assert!(a.dirty);
}

#[test]
fn move_line_down_swaps() {
    let mut a = app_with_builder(two_line_state("A", "B"));
    a.active_line = 0;
    a.move_line_down();
    assert_eq!(a.active_line, 1);
    assert_eq!(a.builder.lines[0].separator, "B");
    assert_eq!(a.builder.lines[1].separator, "A");
    assert!(a.dirty);
}

#[test]
fn move_line_up_at_first_no_op() {
    let mut a = mk_app();
    a.move_line_up();
    assert_eq!(a.active_line, 0);
    assert!(!a.dirty);
}

#[test]
fn move_line_down_at_last_no_op() {
    let mut a = mk_app();
    a.move_line_down();
    assert_eq!(a.active_line, 0);
    assert!(!a.dirty);
}

// ── duplicate_line ────────────────────────────────────────────────────────────

#[test]
fn duplicate_line_inserts_copy_below() {
    let mut a = app_with_builder(BuilderState {
        lines: vec![BuilderLine {
            separator: "X".into(),
            segments: vec![],
        }],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    });
    a.duplicate_line();
    assert_eq!(a.builder.lines.len(), 2);
    assert_eq!(a.builder.lines[1].separator, "X");
    assert_eq!(a.active_line, 1);
    assert!(a.dirty);
}

#[test]
fn duplicate_line_at_max_sets_status() {
    let mut a = app_with_builder(three_line_state());
    a.duplicate_line();
    assert_eq!(a.builder.lines.len(), 3);
    assert!(a.status_message.is_some());
}

// ── delete_segment ────────────────────────────────────────────────────────────

#[test]
fn delete_segment_removes_and_adjusts_cursor() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.toggle_preset(Category::SessionModel, 1);
    a.cursor = Cursor::Segment(0);
    a.delete_segment();
    assert_eq!(a.builder.lines[0].segments.len(), 1);
    assert_eq!(a.cursor, Cursor::Segment(0));
    assert!(a.dirty);
}

#[test]
fn delete_segment_last_moves_to_gutter() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.cursor = Cursor::Segment(0);
    a.delete_segment();
    assert_eq!(a.builder.lines[0].segments.len(), 0);
    assert_eq!(a.cursor, Cursor::Gutter);
}

#[test]
fn delete_segment_on_gutter_no_op() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.cursor = Cursor::Gutter;
    a.delete_segment();
    assert_eq!(a.builder.lines[0].segments.len(), 1);
}

// ── reorder_left/right ────────────────────────────────────────────────────────

#[test]
fn reorder_right_moves_and_updates_cursor() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.toggle_preset(Category::SessionModel, 1);
    a.cursor = Cursor::Segment(0);
    let id_before = match &a.builder.lines[0].segments[0] {
        BuilderSegment::Preset { id, .. } => *id,
        _ => panic!(),
    };
    a.reorder_right();
    assert_eq!(a.cursor, Cursor::Segment(1));
    let id_after = match &a.builder.lines[0].segments[0] {
        BuilderSegment::Preset { id, .. } => *id,
        _ => panic!(),
    };
    assert_ne!(id_before, id_after);
    assert!(a.dirty);
}

#[test]
fn reorder_left_moves_and_updates_cursor() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.toggle_preset(Category::SessionModel, 1);
    a.cursor = Cursor::Segment(1);
    a.reorder_left();
    assert_eq!(a.cursor, Cursor::Segment(0));
    assert!(a.dirty);
}

#[test]
fn reorder_left_at_boundary_no_op() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.cursor = Cursor::Segment(0);
    a.reorder_left();
    assert_eq!(a.cursor, Cursor::Segment(0));
}

#[test]
fn reorder_right_at_boundary_no_op() {
    let mut a = mk_app();
    a.toggle_preset(Category::SessionModel, 0);
    a.cursor = Cursor::Segment(0);
    a.reorder_right();
    assert_eq!(a.cursor, Cursor::Segment(0));
}
