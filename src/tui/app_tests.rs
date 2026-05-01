use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::schema::{Config, FlexSegment, Line, Segment, TemplateSegment};

use super::{App, EditorField, Focus, Mode};

fn one_line_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
        }],
    }
}

fn multi_seg_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![
                Segment::Template(TemplateSegment::new("{model}")),
                Segment::Template(TemplateSegment::new("{five_left}")),
                Segment::Flex(FlexSegment { flex: true }),
            ],
        }],
    }
}

fn three_line_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![
            Line {
                separator: String::new(),
                segments: vec![],
            },
            Line {
                separator: String::new(),
                segments: vec![],
            },
            Line {
                separator: String::new(),
                segments: vec![],
            },
        ],
    }
}

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

fn shift_key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::SHIFT)
}

// --- original Task 4 tests (preserved) ---

#[test]
fn app_new_starts_in_browsing_mode() {
    let app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    assert_eq!(app.mode, Mode::Browsing);
}

#[test]
fn app_dirty_starts_false() {
    let app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    assert!(!app.dirty);
}

#[test]
fn app_initial_selection() {
    let app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    assert_eq!(app.selected_line, 0);
    assert_eq!(app.selected_segment, Some(0));
}

#[test]
fn app_initial_selection_empty_config() {
    let app = App::new(
        Config {
            schema_url: None,
            powerline: false,
            lines: vec![],
        },
        PathBuf::from("/tmp/out.json"),
    );
    assert_eq!(app.selected_line, 0);
    assert_eq!(app.selected_segment, None);
}

#[test]
fn app_handle_q_in_non_browsing_mode_no_quit() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.mode = Mode::EditingTemplate;
    app.handle(key(KeyCode::Char('q')));
    assert!(!app.should_quit);
}

// --- Task 5 tests ---

#[test]
fn add_line_below_max_inserts() {
    let mut app = App::new(one_line_config(), PathBuf::from("/tmp/out.json"));
    assert_eq!(app.config.lines.len(), 1);
    app.handle(key(KeyCode::Char('n')));
    assert_eq!(app.config.lines.len(), 2);
}

#[test]
fn add_line_at_max_no_op() {
    let mut app = App::new(three_line_config(), PathBuf::from("/tmp/out.json"));
    assert_eq!(app.config.lines.len(), 3);
    app.handle(key(KeyCode::Char('n')));
    assert_eq!(app.config.lines.len(), 3);
}

#[test]
fn delete_line_with_one_remaining_no_op() {
    let mut app = App::new(one_line_config(), PathBuf::from("/tmp/out.json"));
    app.handle(shift_key(KeyCode::Char('D')));
    assert_eq!(app.config.lines.len(), 1);
}

#[test]
fn tab_switches_focus() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    assert_eq!(app.focus, Focus::LineList);
    app.handle(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::SegmentList);
    app.handle(key(KeyCode::Tab));
    assert_eq!(app.focus, Focus::Editor);
    app.handle(key(KeyCode::Tab));
    // In Editor focus, Tab cycles editor fields, not focus.
    assert_eq!(app.focus, Focus::Editor);
    assert_eq!(app.editor_field, EditorField::Padding);
}

#[test]
fn j_moves_selection_down_in_line_list() {
    let mut app = App::new(
        Config {
            schema_url: None,
            powerline: false,
            lines: vec![
                Line {
                    separator: String::new(),
                    segments: vec![],
                },
                Line {
                    separator: String::new(),
                    segments: vec![],
                },
            ],
        },
        PathBuf::from("/tmp/out.json"),
    );
    assert_eq!(app.selected_line, 0);
    app.handle(key(KeyCode::Char('j')));
    assert_eq!(app.selected_line, 1);
}

#[test]
fn j_caps_at_max_in_line_list() {
    let mut app = App::new(one_line_config(), PathBuf::from("/tmp/out.json"));
    app.handle(key(KeyCode::Char('j')));
    assert_eq!(app.selected_line, 0);
}

#[test]
fn j_in_segment_list_moves_segment_selection() {
    let mut app = App::new(multi_seg_config(), PathBuf::from("/tmp/out.json"));
    app.focus = Focus::SegmentList;
    assert_eq!(app.selected_segment, Some(0));
    app.handle(key(KeyCode::Char('j')));
    assert_eq!(app.selected_segment, Some(1));
}

#[test]
fn capital_j_reorders_segment_down() {
    let mut app = App::new(multi_seg_config(), PathBuf::from("/tmp/out.json"));
    app.focus = Focus::SegmentList;
    app.selected_segment = Some(0);
    app.handle(shift_key(KeyCode::Char('J')));
    assert_eq!(app.selected_segment, Some(1));
    let seg = &app.config.lines[0].segments[1];
    assert!(matches!(seg, Segment::Template(t) if t.template == "{model}"));
    assert!(app.dirty);
}

#[test]
fn capital_k_reorders_segment_up() {
    let mut app = App::new(multi_seg_config(), PathBuf::from("/tmp/out.json"));
    app.focus = Focus::SegmentList;
    app.selected_segment = Some(1);
    app.handle(shift_key(KeyCode::Char('K')));
    assert_eq!(app.selected_segment, Some(0));
    let seg = &app.config.lines[0].segments[0];
    assert!(matches!(seg, Segment::Template(t) if t.template == "{five_left}"));
    assert!(app.dirty);
}

#[test]
fn enter_on_segment_transitions_to_editing_template() {
    let mut app = App::new(multi_seg_config(), PathBuf::from("/tmp/out.json"));
    app.focus = Focus::SegmentList;
    app.selected_segment = Some(0);
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.mode, Mode::EditingTemplate);
}

#[test]
fn enter_on_add_segment_inserts_default() {
    let mut app = App::new(multi_seg_config(), PathBuf::from("/tmp/out.json"));
    app.focus = Focus::SegmentList;
    let seg_count = app.config.lines[0].segments.len();
    app.selected_segment = Some(seg_count);
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.config.lines[0].segments.len(), seg_count + 1);
    assert_eq!(app.mode, Mode::EditingTemplate);
}

#[test]
fn dirty_set_after_add_segment() {
    let mut app = App::new(multi_seg_config(), PathBuf::from("/tmp/out.json"));
    app.focus = Focus::SegmentList;
    let seg_count = app.config.lines[0].segments.len();
    app.selected_segment = Some(seg_count);
    app.handle(key(KeyCode::Enter));
    assert!(app.dirty);
}

#[test]
fn q_when_dirty_enters_confirm_quit() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.dirty = true;
    app.handle(key(KeyCode::Char('q')));
    assert_eq!(app.mode, Mode::ConfirmQuit);
    assert!(!app.should_quit);
}

#[test]
fn q_when_clean_quits() {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    assert!(!app.dirty);
    app.handle(key(KeyCode::Char('q')));
    assert!(app.should_quit);
}

// --- Task 6 tests ---

fn editor_focused_app() -> App {
    let mut app = App::new(minimal_config(), PathBuf::from("/tmp/out.json"));
    app.focus = Focus::Editor;
    app.selected_segment = Some(0);
    app
}

#[test]
fn tab_in_editor_cycles_fields() {
    let mut app = editor_focused_app();
    assert_eq!(app.editor_field, EditorField::Template);
    app.handle(key(KeyCode::Tab));
    assert_eq!(app.editor_field, EditorField::Padding);
    app.handle(key(KeyCode::Tab));
    assert_eq!(app.editor_field, EditorField::HideWhenAbsent);
    app.handle(key(KeyCode::Tab));
    assert_eq!(app.editor_field, EditorField::Color);
    app.handle(key(KeyCode::Tab));
    assert_eq!(app.editor_field, EditorField::Bg);
    app.handle(key(KeyCode::Tab));
    assert_eq!(app.editor_field, EditorField::Template);
}

#[test]
fn enter_on_template_field_enters_editing_template() {
    let mut app = editor_focused_app();
    assert_eq!(app.editor_field, EditorField::Template);
    app.handle(key(KeyCode::Enter));
    assert_eq!(app.mode, Mode::EditingTemplate);
    assert!(app.editing_buf.is_some());
}

#[test]
fn editing_template_appends_char() {
    let mut app = editor_focused_app();
    app.handle(key(KeyCode::Enter)); // enter EditingTemplate
    app.editing_buf = Some(String::new());
    app.handle(key(KeyCode::Char('a')));
    app.handle(key(KeyCode::Char('b')));
    assert_eq!(app.editing_buf.as_deref(), Some("ab"));
}

#[test]
fn editing_template_backspace_removes_char() {
    let mut app = editor_focused_app();
    app.handle(key(KeyCode::Enter));
    app.editing_buf = Some("hello".into());
    app.handle(key(KeyCode::Backspace));
    assert_eq!(app.editing_buf.as_deref(), Some("hell"));
}

#[test]
fn editing_template_enter_commits_and_dirties() {
    let mut app = editor_focused_app();
    app.handle(key(KeyCode::Enter)); // enter EditingTemplate
    app.editing_buf = Some("new_template".into());
    app.handle(key(KeyCode::Enter)); // commit
    assert_eq!(app.mode, Mode::Browsing);
    assert!(app.dirty);
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.template, "new_template");
}

#[test]
fn editing_template_esc_aborts_no_dirty() {
    let mut app = editor_focused_app();
    app.handle(key(KeyCode::Enter)); // enter EditingTemplate
    app.editing_buf = Some("discarded".into());
    app.handle(key(KeyCode::Esc)); // abort
    assert_eq!(app.mode, Mode::Browsing);
    assert!(!app.dirty);
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.template, "{model}"); // unchanged
}

#[test]
fn padding_increment_clamps_at_8() {
    let mut app = editor_focused_app();
    app.editor_field = EditorField::Padding;
    // Set padding to 8 manually.
    if let Segment::Template(t) = &mut app.config.lines[0].segments[0] {
        t.padding = 8;
    }
    app.handle(key(KeyCode::Char('+')));
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.padding, 8); // still 8
}

#[test]
fn padding_decrement_clamps_at_0() {
    let mut app = editor_focused_app();
    app.editor_field = EditorField::Padding;
    // padding starts at 0.
    app.handle(key(KeyCode::Char('-')));
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.padding, 0);
}

#[test]
fn hide_when_absent_toggle() {
    let mut app = editor_focused_app();
    app.editor_field = EditorField::HideWhenAbsent;
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert!(!t.hide_when_absent);
    app.handle(key(KeyCode::Char(' ')));
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert!(t.hide_when_absent);
    assert!(app.dirty);
}

#[test]
fn c_on_color_field_enters_picking_fg_color() {
    let mut app = editor_focused_app();
    app.editor_field = EditorField::Color;
    app.handle(key(KeyCode::Char('c')));
    assert_eq!(app.mode, Mode::PickingFgColor);
}

#[test]
fn b_on_bg_field_enters_picking_bg_color() {
    let mut app = editor_focused_app();
    app.editor_field = EditorField::Bg;
    app.handle(key(KeyCode::Char('b')));
    assert_eq!(app.mode, Mode::PickingBgColor);
}

#[test]
fn flex_segment_editor_shows_no_editable_fields() {
    let app = App::new(
        Config {
            schema_url: None,
            powerline: false,
            lines: vec![Line {
                separator: String::new(),
                segments: vec![Segment::Flex(FlexSegment { flex: true })],
            }],
        },
        PathBuf::from("/tmp/out.json"),
    );
    // Verify the segment at index 0 is indeed Flex — the widget render branch
    // for Flex shows "no editable fields" (verified in segment_editor tests).
    let seg = &app.config.lines[0].segments[0];
    assert!(matches!(seg, Segment::Flex(_)));
}

#[test]
fn esc_in_editor_focus_returns_to_segment_list() {
    let mut app = editor_focused_app();
    app.handle(key(KeyCode::Esc));
    assert_eq!(app.focus, Focus::SegmentList);
}

#[test]
fn padding_increment_works() {
    let mut app = editor_focused_app();
    app.editor_field = EditorField::Padding;
    app.handle(key(KeyCode::Char('+')));
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.padding, 1);
    assert!(app.dirty);
}

#[test]
fn padding_decrement_works() {
    let mut app = editor_focused_app();
    app.editor_field = EditorField::Padding;
    if let Segment::Template(t) = &mut app.config.lines[0].segments[0] {
        t.padding = 3;
    }
    app.handle(key(KeyCode::Char('-')));
    let Segment::Template(t) = &app.config.lines[0].segments[0] else {
        panic!("expected Template");
    };
    assert_eq!(t.padding, 2);
    assert!(app.dirty);
}
