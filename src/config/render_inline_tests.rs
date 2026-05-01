//! Core unit tests for config::render.

use super::*;
use crate::config::schema::{Config, FlexSegment, Line, Segment, TemplateSegment};
use crate::format::placeholders::RenderCtx;
use std::path::PathBuf;

fn full_ctx() -> RenderCtx {
    RenderCtx {
        model: Some("claude-opus-4".to_owned()),
        cwd: Some(PathBuf::from("/tmp/project")),
        five_used: Some(30.0),
        five_reset_unix: Some(3600),
        seven_used: Some(60.0),
        seven_reset_unix: Some(90000),
        extra_enabled: Some(false),
        extra_used: None,
        extra_limit: None,
        extra_pct: None,
        now_unix: 0,
        ..Default::default()
    }
}

fn tmpl_seg(template: &str, padding: u8, hide_when_absent: bool) -> Segment {
    let mut t = TemplateSegment::new(template).with_padding(padding);
    if hide_when_absent {
        t = t.with_hide_when_absent();
    }
    Segment::Template(t)
}

fn flex_seg() -> Segment {
    Segment::Flex(FlexSegment { flex: true })
}

// ── happy-path: 2-line config ────────────────────────────────────────────

#[test]
fn two_line_config_renders_with_newline() {
    let config = Config {
        schema_url: None,
        powerline: false,
        lines: vec![
            Line {
                separator: "".to_owned(),
                segments: vec![tmpl_seg("{model}", 0, false)],
            },
            Line {
                separator: "".to_owned(),
                segments: vec![tmpl_seg("5h:{five_left}%", 0, false)],
            },
        ],
    };
    let ctx = full_ctx();
    let out = render(&config, &ctx);
    let lines: Vec<&str> = out.split('\n').collect();
    assert_eq!(lines.len(), 2, "expected exactly one \\n; got: {out:?}");
    assert!(lines[0].contains("claude-opus-4"), "line 0: {}", lines[0]);
    assert!(lines[1].contains("5h:70%"), "line 1: {}", lines[1]);
}

// ── separator drop with hidden segment ──────────────────────────────────

#[test]
fn hidden_middle_segment_drops_separator() {
    let config = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "|".to_owned(),
            segments: vec![
                tmpl_seg("A", 0, false),
                // {seven_left} absent with empty ctx → None → hidden
                tmpl_seg("{seven_left}", 0, true),
                tmpl_seg("C", 0, false),
            ],
        }],
    };
    // Use empty ctx so seven_left is None.
    let ctx = RenderCtx::default();
    let out = render(&config, &ctx);
    // "A" and "C" both visible; middle hidden → exactly one separator
    assert_eq!(out, "A|C", "got: {out:?}");
    assert_eq!(out.matches('|').count(), 1, "expected 1 separator");
}

#[test]
fn no_trailing_separator_when_last_segment_hidden() {
    let config = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "|".to_owned(),
            segments: vec![tmpl_seg("A", 0, false), tmpl_seg("{model}", 0, true)],
        }],
    };
    // model is None → segment hidden
    let ctx = RenderCtx::default();
    let out = render(&config, &ctx);
    assert_eq!(out, "A", "trailing separator must not appear; got: {out:?}");
}

// ── padding ──────────────────────────────────────────────────────────────

#[test]
fn padding_two_wraps_value() {
    let out = pad("x", 2);
    assert_eq!(out, "  x  ");
}

#[test]
fn padding_zero_returns_unchanged() {
    let out = pad("x", 0);
    assert_eq!(out, "x");
}

#[test]
fn segment_with_padding_in_render() {
    let config = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![tmpl_seg("{model}", 2, false)],
        }],
    };
    let ctx = full_ctx();
    let out = render(&config, &ctx);
    assert_eq!(out, "  claude-opus-4  ", "got: {out:?}");
}

// ── ANSI-stripped visible_width ───────────────────────────────────────────

#[test]
fn visible_width_plain_ascii() {
    assert_eq!(visible_width("hello"), 5);
}

#[test]
fn visible_width_strips_ansi_csi() {
    // "\x1b[31mred\x1b[0m" — 3 visible chars
    let s = "\x1b[31mred\x1b[0m";
    assert_eq!(visible_width(s), 3, "ANSI bytes must not count");
}

#[test]
fn visible_width_empty() {
    assert_eq!(visible_width(""), 0);
}

// ── flex spacer ──────────────────────────────────────────────────────────

#[test]
fn flex_with_test_cols_80_fills_to_width() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    unsafe { std::env::set_var("STATUSLINE_TEST_COLS", "80") };

    let config = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![
                tmpl_seg("hello", 0, false), // 5 chars
                flex_seg(),
                tmpl_seg("world", 0, false), // 5 chars
            ],
        }],
    };
    let ctx = RenderCtx::default();
    let out = render(&config, &ctx);

    // Restore before assert (guard released after)
    match prior {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_TEST_COLS", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_TEST_COLS") },
    }

    let w = visible_width(&out);
    assert_eq!(w, 80, "output width must be 80; got {w}, out={out:?}");
}

#[test]
fn flex_with_test_cols_10_content_5_filler_5() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    unsafe { std::env::set_var("STATUSLINE_TEST_COLS", "10") };

    let config = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![
                tmpl_seg("abcde", 0, false), // 5 chars visible
                flex_seg(),
            ],
        }],
    };
    let ctx = RenderCtx::default();
    let out = render(&config, &ctx);

    match prior {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_TEST_COLS", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_TEST_COLS") },
    }

    let trailing_spaces = out.len() - out.trim_end().len();
    assert_eq!(trailing_spaces, 5, "filler must be 5 spaces; out={out:?}");
}

#[test]
fn flex_without_width_set_degrades_to_one_space() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    unsafe { std::env::remove_var("STATUSLINE_TEST_COLS") };

    // Build a line whose content alone is wider than any terminal we'd see
    // (to guarantee fallback-to-80 triggers if terminal_size returns None).
    // We'll just confirm there's at least one space in the flex position.
    let config = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![tmpl_seg("A", 0, false), flex_seg(), tmpl_seg("B", 0, false)],
        }],
    };
    let ctx = RenderCtx::default();
    let out = render(&config, &ctx);

    match prior {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_TEST_COLS", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_TEST_COLS") },
    }

    // The flex region must be at least 1 space.
    let between = out.trim_start_matches('A').trim_end_matches('B');
    assert!(
        between.chars().all(|c| c == ' ') && !between.is_empty(),
        "flex must produce ≥1 space; out={out:?}"
    );
}

// ── MAX_LINES truncation guard ───────────────────────────────────────────

#[test]
fn max_lines_truncation_no_panic() {
    // Bypass validation by constructing 5 lines directly.
    let line = Line {
        separator: "".to_owned(),
        segments: vec![tmpl_seg("x", 0, false)],
    };
    let config = Config {
        schema_url: None,
        powerline: false,
        lines: vec![
            line.clone(),
            line.clone(),
            line.clone(),
            line.clone(),
            line.clone(),
        ],
    };
    let ctx = RenderCtx::default();
    let out = render(&config, &ctx);
    let line_count = out.split('\n').count();
    assert!(
        line_count <= MAX_LINES,
        "render must cap at MAX_LINES={MAX_LINES}; got {line_count}"
    );
}

// ── render_segment unit tests ────────────────────────────────────────────

#[test]
fn render_segment_returns_none_for_absent_placeholder() {
    let ctx = RenderCtx::default(); // model is None
    let result = format::render_segment("{model}", &ctx);
    assert!(result.is_none(), "absent placeholder must return None");
}

#[test]
fn render_segment_returns_some_when_all_present() {
    let ctx = full_ctx();
    let result = format::render_segment("{model}", &ctx);
    assert_eq!(result, Some("claude-opus-4".to_owned()));
}

#[test]
fn render_segment_returns_none_when_placeholder_is_empty_string() {
    // cwd_basename with empty cwd path → returns None from placeholder
    let ctx = RenderCtx {
        cwd: Some(PathBuf::from("")),
        ..Default::default()
    };
    let result = format::render_segment("{cwd_basename}", &ctx);
    assert!(result.is_none(), "empty placeholder must return None");
}

#[test]
fn render_segment_plain_text_returns_some() {
    let ctx = RenderCtx::default();
    let result = format::render_segment("hello", &ctx);
    assert_eq!(result, Some("hello".to_owned()));
}

#[test]
fn render_segment_optional_block_collapse_does_not_propagate_none() {
    // Top-level is plain text + optional block. Optional collapses (five_left absent)
    // but does not make the whole segment None.
    let ctx = RenderCtx::default();
    let result = format::render_segment("prefix{? /{five_left}}", &ctx);
    assert_eq!(
        result,
        Some("prefix".to_owned()),
        "collapsed optional block must not make outer None"
    );
}
