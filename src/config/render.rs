//! Multi-line config renderer with flex-spacer support.

use crate::config::schema::{Config, MAX_LINES, Segment};
use crate::format::{self, placeholders::RenderCtx};

const FLEX_MARKER: &str = "\x00FLEX\x00";

/// Render `config` against `ctx`, returning a newline-joined string of all lines.
///
/// Lines are capped at `MAX_LINES`.  Hidden segments (None result + `hide_when_absent`)
/// are dropped along with their separator slots.  A `Segment::Flex` is replaced
/// with enough spaces to fill the terminal width.
pub fn render(config: &Config, ctx: &RenderCtx) -> String {
    let mut out_lines: Vec<String> = Vec::new();

    for line in config.lines.iter().take(MAX_LINES) {
        let mut parts: Vec<Option<String>> = Vec::new();

        for seg in &line.segments {
            match seg {
                Segment::Template(t) => {
                    let result = format::render_segment(&t.template, ctx);
                    match result {
                        Some(s) if !s.is_empty() => {
                            parts.push(Some(pad(&s, t.padding)));
                        }
                        _ => {
                            if t.hide_when_absent {
                                parts.push(None);
                            } else {
                                // Not hidden: push padded empty string
                                parts.push(Some(pad("", t.padding)));
                            }
                        }
                    }
                }
                Segment::Flex(_) => {
                    parts.push(Some(FLEX_MARKER.to_owned()));
                }
            }
        }

        // Join visible (Some) parts with the line separator.
        let line_str: String = parts
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(&line.separator);

        let line_str = if line_str.contains(FLEX_MARKER) {
            resolve_flex(line_str)
        } else {
            line_str
        };

        out_lines.push(line_str);
    }

    out_lines.join("\n")
}

/// Replace the flex marker with spaces to fill the terminal width.
///
/// Defense-in-depth: only the FIRST marker gets the real fill; any additional
/// markers (which validation should have rejected) become a single space each.
fn resolve_flex(line_str: String) -> String {
    let without_flex = line_str.replace(FLEX_MARKER, "");
    let natural = visible_width(&without_flex);

    let term_width = test_cols_override()
        .or_else(|| terminal_size::terminal_size().map(|(terminal_size::Width(w), _)| w as usize))
        .unwrap_or(80);

    // Count extra markers beyond the first; each will become 1 space.
    let extra_markers = line_str.matches(FLEX_MARKER).count().saturating_sub(1);
    let fill = term_width
        .saturating_sub(natural)
        .saturating_sub(extra_markers)
        .max(1);
    // Replace only the first occurrence with the computed fill.
    let after_first = line_str.replacen(FLEX_MARKER, &" ".repeat(fill), 1);
    // Any remaining markers (validation bypass) become a single space.
    after_first.replace(FLEX_MARKER, " ")
}

/// Returns the value of `STATUSLINE_TEST_COLS` when compiled in test mode.
/// Always returns `None` in production builds.
#[cfg(test)]
fn test_cols_override() -> Option<usize> {
    std::env::var("STATUSLINE_TEST_COLS")
        .ok()
        .and_then(|s| s.parse::<usize>().ok())
}

#[cfg(not(test))]
fn test_cols_override() -> Option<usize> {
    None
}

/// Apply symmetric padding around a string value (left + right by `n` spaces).
fn pad(s: &str, n: u8) -> String {
    if n == 0 {
        return s.to_owned();
    }
    let spaces = " ".repeat(n as usize);
    format!("{spaces}{s}{spaces}")
}

/// Count the visible (non-ANSI) byte length of `s`.
///
/// Strips CSI sequences of the form `\x1b[<args>m` before counting.
/// Phase-1 simplification: byte length after stripping (correct for
/// ASCII content + ANSI colour escapes; no full grapheme-cluster counting).
pub fn visible_width(s: &str) -> usize {
    let mut width = 0usize;
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\x1b' && i + 1 < bytes.len() && bytes[i + 1] == b'[' {
            // Skip CSI sequence: \x1b[ ... m
            i += 2;
            while i < bytes.len() && bytes[i] != b'm' {
                i += 1;
            }
            if i < bytes.len() {
                i += 1; // skip 'm'
            }
        } else {
            width += 1;
            i += 1;
        }
    }
    width
}

/// Mutex serializing tests that read or write `STATUSLINE_TEST_COLS`.
///
/// Every test that touches this env var MUST:
///   1. acquire `COLS_MUTEX`
///   2. set `STATUSLINE_TEST_COLS` as needed
///   3. restore (or remove) prior value before releasing the guard
///
/// Pattern mirrors `format::ENV_MUTEX` and `creds::HOME_MUTEX`.
#[cfg(test)]
pub(crate) static COLS_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[path = "render_tests.rs"]
mod render_tests;

#[cfg(test)]
mod tests {
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
        Segment::Template(TemplateSegment {
            template: template.to_owned(),
            padding,
            hide_when_absent,
        })
    }

    fn flex_seg() -> Segment {
        Segment::Flex(FlexSegment { flex: true })
    }

    // ── happy-path: 2-line config ────────────────────────────────────────────

    #[test]
    fn two_line_config_renders_with_newline() {
        let config = Config {
            schema_url: None,
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
}
