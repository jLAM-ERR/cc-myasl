//! Tests for Powerline rendering (Task 3).

use crate::config::render::COLS_MUTEX;
use crate::config::schema::{Config, FlexSegment, Line, Segment, TemplateSegment};
use crate::format::placeholders::RenderCtx;

// ── helpers ──────────────────────────────────────────────────────────────────

fn pl_line(segments: Vec<Segment>) -> Config {
    Config {
        schema_url: None,
        powerline: true,
        lines: vec![Line {
            separator: "|".to_owned(), // must be overridden by Powerline mode
            segments,
        }],
    }
}

fn tmpl(template: &str) -> Segment {
    Segment::Template(TemplateSegment::new(template))
}

fn tmpl_bg(template: &str, bg: &str) -> Segment {
    let mut t = TemplateSegment::new(template);
    t.bg = Some(bg.to_owned());
    Segment::Template(t)
}

fn tmpl_fg_bg(template: &str, fg: &str, bg: &str) -> Segment {
    let mut t = TemplateSegment::new(template);
    t.color = Some(fg.to_owned());
    t.bg = Some(bg.to_owned());
    Segment::Template(t)
}

fn tmpl_hide(template: &str) -> Segment {
    Segment::Template(TemplateSegment::new(template).with_hide_when_absent())
}

fn flex_seg() -> Segment {
    Segment::Flex(FlexSegment { flex: true })
}

fn render(config: &Config) -> String {
    crate::config::render::render(config, &RenderCtx::default())
}

fn render_with_ctx(config: &Config, ctx: &RenderCtx) -> String {
    crate::config::render::render(config, ctx)
}

fn count_chevrons(s: &str) -> usize {
    // CHEVRON is 3 bytes (U+E0B0 = \xEE\x82\xB0)
    let chevron = "\u{E0B0}";
    s.matches(chevron).count()
}

// ── 3-segment: exactly 3 chevrons ────────────────────────────────────────────

#[test]
fn three_segments_produce_exactly_three_chevrons() {
    let config = pl_line(vec![tmpl("A"), tmpl("B"), tmpl("C")]);
    let out = render(&config);
    assert_eq!(
        count_chevrons(&out),
        3,
        "3 segments → 2 between + 1 trailing = 3 chevrons; got {out:?}"
    );
}

// ── no leading chevron ────────────────────────────────────────────────────────

#[test]
fn no_leading_chevron_before_first_segment() {
    let config = pl_line(vec![tmpl("A"), tmpl("B")]);
    let out = render(&config);
    // The output must not start with a chevron (ANSI codes may precede content
    // but a chevron before the first segment is forbidden).
    let chevron = "\u{E0B0}";
    // Strip leading ANSI sequences to find the first non-ANSI char.
    let stripped = strip_ansi_naive(&out);
    assert!(
        !stripped.starts_with(chevron),
        "must not have leading chevron; got {out:?}"
    );
}

/// Strip obvious ANSI CSI sequences for assertion purposes only.
fn strip_ansi_naive(s: &str) -> String {
    let mut out = String::new();
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\x1b' && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            i += 2;
            while i < bytes.len() && bytes[i] != b'm' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1;
            }
        } else {
            out.push(bytes[i] as char);
            i += 1;
        }
    }
    out
}

// ── trailing chevron guarantee ────────────────────────────────────────────────

#[test]
fn single_segment_has_one_trailing_chevron() {
    let config = pl_line(vec![tmpl("X")]);
    let out = render(&config);
    assert_eq!(
        count_chevrons(&out),
        1,
        "1 segment → 0 between + 1 trailing; got {out:?}"
    );
}

#[test]
fn two_segments_produce_exactly_two_chevrons() {
    let config = pl_line(vec![tmpl("A"), tmpl("B")]);
    let out = render(&config);
    assert_eq!(
        count_chevrons(&out),
        2,
        "2 segments → 1 between + 1 trailing; got {out:?}"
    );
}

// ── chevron colors: same bg makes chevron visually invisible ─────────────────

#[test]
fn chevron_between_same_bg_segments_has_matching_fg_and_bg() {
    // Both segments have bg="blue"; the chevron between them should have
    // fg=blue and bg=blue (fg=\x1b[34m, bg=\x1b[44m).
    let config = pl_line(vec![tmpl_bg("A", "blue"), tmpl_bg("B", "blue")]);
    let out = render(&config);
    // Both ANSI codes must appear. The between-chevron uses RESET+fg(prev)+bg(cur).
    // Since prev=blue and cur=blue, we expect \x1b[34m...\x1b[44m around the chevron.
    assert!(
        out.contains("\x1b[34m"),
        "fg blue present for same-bg chevron; got {out:?}"
    );
    assert!(
        out.contains("\x1b[44m"),
        "bg blue present for same-bg chevron; got {out:?}"
    );
}

// ── hidden middle segment skips its chevron slot ─────────────────────────────

#[test]
fn hidden_middle_segment_gives_two_chevrons_not_three() {
    // A, {model}(hidden), C — model absent → hidden, so only A+C visible → 2 chevrons.
    let config = pl_line(vec![tmpl("A"), tmpl_hide("{model}"), tmpl("C")]);
    let out = render_with_ctx(&config, &RenderCtx::default());
    assert_eq!(
        count_chevrons(&out),
        2,
        "hidden middle → only 2 visible segs → 2 chevrons; got {out:?}"
    );
}

#[test]
fn hidden_middle_segment_no_double_chevron() {
    // Same as above: verify text flows A → C with no adjacent chevrons.
    let config = pl_line(vec![tmpl("A"), tmpl_hide("{model}"), tmpl("C")]);
    let out = render_with_ctx(&config, &RenderCtx::default());
    let chevron = "\u{E0B0}";
    let double = format!("{chevron}{chevron}");
    assert!(
        !out.contains(&double),
        "no double-chevron when middle is hidden; got {out:?}"
    );
}

// ── separator overridden in powerline mode ────────────────────────────────────

#[test]
fn separator_is_not_rendered_in_powerline_mode() {
    let config = pl_line(vec![tmpl("A"), tmpl("B")]);
    let out = render(&config);
    // The separator "|" must not appear in the output.
    assert!(
        !out.contains('|'),
        "separator must be overridden by chevrons; got {out:?}"
    );
}

// ── fg color on segment ───────────────────────────────────────────────────────

#[test]
fn segment_with_fg_red_emits_fg_red_escape() {
    let config = pl_line(vec![tmpl_fg_bg("hello", "red", "blue")]);
    let out = render(&config);
    assert!(out.contains("\x1b[31m"), "fg red expected; got {out:?}");
    assert!(out.contains("\x1b[44m"), "bg blue expected; got {out:?}");
}

// ── flex spacer in powerline mode ─────────────────────────────────────────────

#[test]
fn flex_spacer_in_powerline_produces_correct_chevron_count() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    unsafe { std::env::set_var("STATUSLINE_TEST_COLS", "40") };

    let config = pl_line(vec![tmpl("A"), flex_seg(), tmpl("B")]);
    let out = render_with_ctx(&config, &RenderCtx::default());

    match &prior {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_TEST_COLS", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_TEST_COLS") },
    }

    // 3 items (A, Flex, B) → 3 chevrons.
    assert_eq!(
        count_chevrons(&out),
        3,
        "flex counts as an item → 3 chevrons; got {out:?}"
    );
}

#[test]
fn flex_spacer_uses_default_bg_for_surrounding_chevrons() {
    let _guard = COLS_MUTEX.lock().unwrap();
    let prior = std::env::var("STATUSLINE_TEST_COLS").ok();
    unsafe { std::env::set_var("STATUSLINE_TEST_COLS", "40") };

    // A(bg=red) flex B(bg=blue)
    // Chevron before flex: fg=red (\x1b[31m), bg=default (\x1b[49m).
    // Chevron after flex: fg=default (\x1b[39m), bg=blue (\x1b[44m).
    let config = pl_line(vec![tmpl_bg("A", "red"), flex_seg(), tmpl_bg("B", "blue")]);
    let out = render_with_ctx(&config, &RenderCtx::default());

    match &prior {
        Some(v) => unsafe { std::env::set_var("STATUSLINE_TEST_COLS", v) },
        None => unsafe { std::env::remove_var("STATUSLINE_TEST_COLS") },
    }

    // bg default appears for the flex-region chevrons.
    assert!(
        out.contains("\x1b[49m"),
        "default bg expected for flex region chevron; got {out:?}"
    );
}

// ── non-powerline regression guard ───────────────────────────────────────────

#[test]
fn powerline_false_uses_separator_not_chevrons() {
    let config = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "|".to_owned(),
            segments: vec![tmpl("A"), tmpl("B")],
        }],
    };
    let out = render(&config);
    assert_eq!(
        count_chevrons(&out),
        0,
        "non-powerline must have no chevrons; got {out:?}"
    );
    assert!(
        out.contains('|'),
        "non-powerline must use separator; got {out:?}"
    );
}

// ── empty line ────────────────────────────────────────────────────────────────

#[test]
fn empty_powerline_line_returns_empty_string() {
    let config = pl_line(vec![]);
    let out = render(&config);
    assert!(out.is_empty(), "empty line must render empty; got {out:?}");
}

// ── all segments hidden → empty output ───────────────────────────────────────

#[test]
fn all_hidden_segments_render_empty() {
    let config = pl_line(vec![tmpl_hide("{model}"), tmpl_hide("{model}")]);
    let out = render_with_ctx(&config, &RenderCtx::default());
    assert!(out.is_empty(), "all hidden → empty; got {out:?}");
}
