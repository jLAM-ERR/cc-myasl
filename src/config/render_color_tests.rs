//! Tests for fg/bg color rendering in config::render.

use super::*;
use crate::config::schema::{FlexSegment, Line, Segment, TemplateSegment};
use crate::format::placeholders::RenderCtx;

fn one_line(separator: &str, segments: Vec<Segment>) -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: separator.to_owned(),
            segments,
        }],
    }
}

fn tmpl_seg(template: &str) -> Segment {
    Segment::Template(TemplateSegment::new(template))
}

fn flex_seg() -> Segment {
    Segment::Flex(FlexSegment { flex: true })
}

fn colored_seg(template: &str, fg: Option<&str>, bg: Option<&str>) -> Segment {
    let mut t = TemplateSegment::new(template);
    t.color = fg.map(str::to_owned);
    t.bg = bg.map(str::to_owned);
    Segment::Template(t)
}

#[test]
fn segment_with_fg_color_wraps_with_escape_and_reset() {
    let out = render(
        &one_line("", vec![colored_seg("hello", Some("red"), None)]),
        &RenderCtx::default(),
    );
    assert!(out.contains("\x1b[31m") && out.contains("\x1b[0m") && out.contains("hello"));
}

#[test]
fn segment_with_fg_and_bg_wraps_both_escapes() {
    let out = render(
        &one_line("", vec![colored_seg("hi", Some("red"), Some("blue"))]),
        &RenderCtx::default(),
    );
    assert!(out.contains("\x1b[31m"), "fg red; got {out:?}");
    assert!(out.contains("\x1b[44m"), "bg blue; got {out:?}");
    assert!(
        out.find("\x1b[31m") < out.find("\x1b[44m"),
        "fg before bg; got {out:?}"
    );
    assert!(out.contains("\x1b[0m"), "reset; got {out:?}");
}

#[test]
fn segment_without_color_emits_unchanged() {
    let out = render(
        &one_line("", vec![tmpl_seg("plain")]),
        &RenderCtx::default(),
    );
    assert_eq!(out, "plain");
}

#[test]
fn flex_spacer_width_correct_with_colored_adjacent_segment() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    unsafe { std::env::set_var("STATUSLINE_TEST_COLS", "10") };
    let config = one_line(
        "",
        vec![colored_seg("AB", Some("red"), Some("blue")), flex_seg()],
    );
    let out = render(&config, &RenderCtx::default());
    match &prior {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_TEST_COLS", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_TEST_COLS") },
    }
    let w = visible_width(&out);
    assert_eq!(
        w, 10,
        "flex must account for ANSI escapes; got {w}, out={out:?}"
    );
}

#[test]
fn five_color_placeholder_still_works() {
    let ctx = RenderCtx {
        five_used: Some(10.0),
        ..Default::default()
    };
    let out = render(&one_line("", vec![tmpl_seg("{five_color}")]), &ctx);
    assert!(out.contains("\x1b[32m"), "green for 90% left; got {out:?}");
}

#[test]
fn visible_width_strips_bg_escape_code() {
    // bg blue \x1b[44m is a CSI sequence stripped by the generic CSI parser
    assert_eq!(visible_width("\x1b[44mAB\x1b[0m"), 2);
}
