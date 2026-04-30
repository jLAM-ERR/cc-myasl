//! Adversarial tests for config::render and format::render_segment.

use super::*;
use crate::config::schema::{Config, FlexSegment, Line, Segment, TemplateSegment};
use crate::format::placeholders::RenderCtx;

fn tmpl_seg(template: &str, padding: u8, hide_when_absent: bool) -> Segment {
    Segment::Template(TemplateSegment {
        template: template.to_owned(),
        padding,
        hide_when_absent,
    })
}

fn flex_seg() -> Segment {
    Segment::Flex(FlexSegment { flex: true })
}

fn one_line(separator: &str, segments: Vec<Segment>) -> Config {
    Config {
        schema_url: None,
        lines: vec![Line {
            separator: separator.to_owned(),
            segments,
        }],
    }
}

// ── render_segment ────────────────────────────────────────────────────────────

/// render_segment("") has no tokens → returns Some(""), not None.
/// config::render then treats the empty string as absent, but render_segment
/// itself must not collapse.
#[test]
fn render_segment_empty_template_returns_some_empty() {
    let result = crate::format::render_segment("", &RenderCtx::default());
    assert_eq!(result, Some(String::new()));
}

/// Unknown placeholder is treated the same as an absent one: returns None.
#[test]
fn render_segment_unknown_placeholder_returns_none() {
    let result = crate::format::render_segment("{nonexistent_xyz}", &RenderCtx::default());
    assert!(result.is_none(), "unknown placeholder must return None");
}

/// Unknown placeholder mixed with plain text still collapses the whole segment.
#[test]
fn render_segment_unknown_placeholder_with_text_returns_none() {
    let result = crate::format::render_segment("prefix {nonexistent}", &RenderCtx::default());
    assert!(
        result.is_none(),
        "unknown placeholder must collapse whole segment"
    );
}

/// First placeholder present, second absent → whole segment None.
#[test]
fn render_segment_first_present_second_absent_returns_none() {
    let ctx = RenderCtx {
        model: Some("m".to_owned()),
        five_used: None,
        ..Default::default()
    };
    let result = crate::format::render_segment("{model} {five_left}", &ctx);
    assert!(
        result.is_none(),
        "absent second placeholder must collapse segment"
    );
}

/// Both placeholders present → Some with concatenated values.
#[test]
fn render_segment_both_present_returns_some() {
    let ctx = RenderCtx {
        model: Some("m".to_owned()),
        five_used: Some(30.0),
        ..Default::default()
    };
    assert_eq!(
        crate::format::render_segment("{model}-{five_left}", &ctx),
        Some("m-70".to_owned())
    );
}

/// Template with ONLY a collapsing optional block: outer segment returns Some("").
#[test]
fn render_segment_only_optional_block_absent_returns_some_empty() {
    let result = crate::format::render_segment("{? {five_left}% }", &RenderCtx::default());
    assert_eq!(
        result,
        Some(String::new()),
        "collapsed optional only → Some(\"\")"
    );
}

// ── render: structural ────────────────────────────────────────────────────────

#[test]
fn render_empty_config_returns_empty_string() {
    let config = Config {
        schema_url: None,
        lines: vec![],
    };
    assert_eq!(render(&config, &RenderCtx::default()), "");
}

#[test]
fn render_line_with_no_segments_returns_empty_string() {
    assert_eq!(render(&one_line("·", vec![]), &RenderCtx::default()), "");
}

#[test]
fn render_all_segments_hidden_returns_empty_string() {
    let config = one_line(
        " | ",
        vec![
            tmpl_seg("{model}", 0, true),
            tmpl_seg("{five_left}", 0, true),
        ],
    );
    assert_eq!(render(&config, &RenderCtx::default()), "");
}

/// Segment with hide_when_absent=false and absent placeholder → stays as empty
/// string, keeps its separator slots.
#[test]
fn render_not_hidden_absent_segment_contributes_empty_string() {
    let config = one_line(
        "|",
        vec![
            tmpl_seg("A", 0, false),
            tmpl_seg("{model}", 0, false), // absent, NOT hidden → ""
            tmpl_seg("B", 0, false),
        ],
    );
    let out = render(&config, &RenderCtx::default());
    assert_eq!(
        out, "A||B",
        "non-hidden absent segment must appear as empty; got: {out:?}"
    );
}

// ── render: separator edge cases ──────────────────────────────────────────────

/// [vis, hid, hid, vis] → exactly one separator between the two visible parts.
#[test]
fn render_two_consecutive_hidden_between_visible_single_separator() {
    let config = one_line(
        " · ",
        vec![
            tmpl_seg("A", 0, false),
            tmpl_seg("{model}", 0, true),
            tmpl_seg("{five_left}", 0, true),
            tmpl_seg("B", 0, false),
        ],
    );
    let out = render(&config, &RenderCtx::default());
    assert_eq!(
        out, "A · B",
        "two consecutive hidden must produce one separator; got: {out:?}"
    );
}

/// [hid, vis, vis] → no leading separator.
#[test]
fn render_leading_hidden_segment_no_leading_separator() {
    let config = one_line(
        " | ",
        vec![
            tmpl_seg("{model}", 0, true),
            tmpl_seg("X", 0, false),
            tmpl_seg("Y", 0, false),
        ],
    );
    let out = render(&config, &RenderCtx::default());
    assert_eq!(
        out, "X | Y",
        "hidden lead must not produce leading separator; got: {out:?}"
    );
}

/// [vis, vis, hid] → no trailing separator.
#[test]
fn render_trailing_hidden_segment_no_trailing_separator() {
    let config = one_line(
        " | ",
        vec![
            tmpl_seg("X", 0, false),
            tmpl_seg("Y", 0, false),
            tmpl_seg("{model}", 0, true),
        ],
    );
    let out = render(&config, &RenderCtx::default());
    assert_eq!(
        out, "X | Y",
        "hidden tail must not produce trailing separator; got: {out:?}"
    );
}

// ── render: line count ────────────────────────────────────────────────────────

/// Exactly MAX_LINES lines must all be rendered (no off-by-one truncation).
#[test]
fn render_exactly_max_lines_renders_all() {
    let line = Line {
        separator: "".to_owned(),
        segments: vec![tmpl_seg("x", 0, false)],
    };
    let config = Config {
        schema_url: None,
        lines: vec![line.clone(), line.clone(), line.clone()],
    };
    let out = render(&config, &RenderCtx::default());
    let count = out.split('\n').count();
    assert_eq!(
        count, 3,
        "exactly MAX_LINES must all render; got {count}: {out:?}"
    );
}

/// Multi-line output is joined with `\n`, never `\r\n`.
#[test]
fn render_multi_line_joined_with_newline_not_crlf() {
    let mk = |s: &str| Line {
        separator: "".to_owned(),
        segments: vec![tmpl_seg(s, 0, false)],
    };
    let config = Config {
        schema_url: None,
        lines: vec![mk("line1"), mk("line2")],
    };
    let out = render(&config, &RenderCtx::default());
    assert!(!out.contains("\r\n"), "must use \\n not \\r\\n");
    assert_eq!(out.split('\n').collect::<Vec<_>>(), vec!["line1", "line2"]);
}

// ── pad ───────────────────────────────────────────────────────────────────────

#[test]
fn pad_u8_max_does_not_panic() {
    let out = pad("x", u8::MAX);
    assert_eq!(out.len(), 1 + 2 * 255);
    assert_eq!(out.trim(), "x");
}

// ── visible_width ─────────────────────────────────────────────────────────────

/// Unterminated CSI (no 'm') must not panic; chars after \x1b[ count as 0.
#[test]
fn visible_width_unterminated_csi_no_panic() {
    assert_eq!(visible_width("\x1b[31"), 0, "unterminated CSI must count 0");
}

/// Lone ESC (no '[' following) counts as 1 regular byte.
#[test]
fn visible_width_lone_esc_counts_as_one() {
    assert_eq!(visible_width("\x1b"), 1);
}

/// Multiple consecutive ANSI sequences followed by ASCII.
#[test]
fn visible_width_multiple_consecutive_ansi_sequences() {
    assert_eq!(visible_width("\x1b[1m\x1b[31mhi\x1b[0m"), 2);
}

/// Phase-1 simplification: byte length, not grapheme clusters.
/// "aé" = 1 ASCII byte + 2 UTF-8 bytes = 3.
#[test]
fn visible_width_multibyte_counts_bytes_not_codepoints() {
    assert_eq!(visible_width("aé"), 3, "phase-1 counts UTF-8 bytes");
}

/// ANSI-wrapped text followed by plain text.
#[test]
fn visible_width_ansi_then_plain() {
    assert_eq!(visible_width("\x1b[32mgreen\x1b[0m plain"), 11);
}

// ── flex: edge cases ──────────────────────────────────────────────────────────

/// Flex is the only segment → fills entire terminal width.
#[test]
fn flex_only_segment_fills_entire_width() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    std::env::set_var("STATUSLINE_TEST_COLS", "20");
    let out = render(&one_line("", vec![flex_seg()]), &RenderCtx::default());
    match &prior {
        Some(v) => std::env::set_var("STATUSLINE_TEST_COLS", v),
        None => std::env::remove_var("STATUSLINE_TEST_COLS"),
    }
    assert_eq!(out.len(), 20, "flex-only must be 20 spaces; got {out:?}");
    assert!(out.chars().all(|c| c == ' '));
}

/// natural_width >= term_width → fill is exactly 1 (saturating_sub + max(1)).
#[test]
fn flex_natural_width_exceeds_terminal_width_produces_one_space() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    std::env::set_var("STATUSLINE_TEST_COLS", "5");
    let config = one_line("", vec![tmpl_seg("hello", 0, false), flex_seg()]);
    let out = render(&config, &RenderCtx::default());
    match &prior {
        Some(v) => std::env::set_var("STATUSLINE_TEST_COLS", v),
        None => std::env::remove_var("STATUSLINE_TEST_COLS"),
    }
    assert_eq!(
        out.len() - out.trim_end().len(),
        1,
        "overflow flex must be exactly 1 space; got {out:?}"
    );
}

/// STATUSLINE_TEST_COLS="0" → fill = max(0-n, 1) = 1.
#[test]
fn flex_test_cols_zero_produces_one_space() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    std::env::set_var("STATUSLINE_TEST_COLS", "0");
    let config = one_line("", vec![tmpl_seg("A", 0, false), flex_seg()]);
    let out = render(&config, &RenderCtx::default());
    match &prior {
        Some(v) => std::env::set_var("STATUSLINE_TEST_COLS", v),
        None => std::env::remove_var("STATUSLINE_TEST_COLS"),
    }
    assert_eq!(
        out.len() - out.trim_end_matches(' ').len(),
        1,
        "cols=0 must give 1 space; got {out:?}"
    );
}

/// Non-numeric STATUSLINE_TEST_COLS → parse fails → fallback (no panic).
#[test]
fn flex_test_cols_non_numeric_falls_back_gracefully() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    std::env::set_var("STATUSLINE_TEST_COLS", "abc");
    let config = one_line("", vec![tmpl_seg("A", 0, false), flex_seg()]);
    let out = render(&config, &RenderCtx::default());
    match &prior {
        Some(v) => std::env::set_var("STATUSLINE_TEST_COLS", v),
        None => std::env::remove_var("STATUSLINE_TEST_COLS"),
    }
    assert!(
        out.starts_with('A') && out.len() > 1,
        "must not panic; got {out:?}"
    );
}

/// Empty STATUSLINE_TEST_COLS → parse fails → fallback (no panic).
#[test]
fn flex_test_cols_empty_string_falls_back_gracefully() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    std::env::set_var("STATUSLINE_TEST_COLS", "");
    let config = one_line("", vec![tmpl_seg("B", 0, false), flex_seg()]);
    let out = render(&config, &RenderCtx::default());
    match &prior {
        Some(v) => std::env::set_var("STATUSLINE_TEST_COLS", v),
        None => std::env::remove_var("STATUSLINE_TEST_COLS"),
    }
    assert!(
        out.starts_with('B') && out.len() > 1,
        "must not panic; got {out:?}"
    );
}

/// Two flex segments (validation bypassed) must not panic and must not leave
/// NUL bytes (FLEX_MARKER remnants) in the output.
#[test]
fn flex_two_flex_segments_bypass_validation_safe() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    std::env::set_var("STATUSLINE_TEST_COLS", "20");
    let config = one_line(
        "",
        vec![
            tmpl_seg("A", 0, false),
            flex_seg(),
            tmpl_seg("B", 0, false),
            flex_seg(),
        ],
    );
    let out = render(&config, &RenderCtx::default());
    match &prior {
        Some(v) => std::env::set_var("STATUSLINE_TEST_COLS", v),
        None => std::env::remove_var("STATUSLINE_TEST_COLS"),
    }
    assert!(
        out.contains('A') && out.contains('B'),
        "A and B must appear; got {out:?}"
    );
    assert!(
        !out.contains('\x00'),
        "no NUL bytes must remain; got {out:?}"
    );
}

/// Flex with ANSI-colored surrounding segments: visible_width strips ANSI so
/// the fill is based on visible chars, not raw bytes.
#[test]
fn flex_with_ansi_colored_segments_correct_fill() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    std::env::set_var("STATUSLINE_TEST_COLS", "10");
    // "\x1b[32mAB\x1b[0m" → 2 visible chars; flex fills remaining 8
    let config = one_line(
        "",
        vec![tmpl_seg("\x1b[32mAB\x1b[0m", 0, false), flex_seg()],
    );
    let out = render(&config, &RenderCtx::default());
    match &prior {
        Some(v) => std::env::set_var("STATUSLINE_TEST_COLS", v),
        None => std::env::remove_var("STATUSLINE_TEST_COLS"),
    }
    let w = visible_width(&out);
    assert_eq!(w, 10, "visible width must be 10; got {w}, out={out:?}");
}
