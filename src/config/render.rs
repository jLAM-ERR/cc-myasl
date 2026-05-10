//! Multi-line config renderer with flex-spacer support.

use crate::config::schema::{Config, MAX_LINES, Segment};
use crate::format::values::{ansi_bg, ansi_fg};
use crate::format::{self, placeholders::RenderCtx};

const FLEX_MARKER: &str = "\x00FLEX\x00";

/// Render `config` against `ctx`, returning a newline-joined string of all lines.
///
/// Lines are capped at `MAX_LINES`.  Hidden segments (None result + `hide_when_absent`)
/// are dropped along with their separator slots.  A `Segment::Flex` is replaced
/// with enough spaces to fill the terminal width.
pub fn render(config: &Config, ctx: &RenderCtx) -> String {
    let mut out_lines: Vec<String> = Vec::new();

    let term_width = if config.powerline {
        test_cols_override()
            .or_else(|| {
                terminal_size::terminal_size().map(|(terminal_size::Width(w), _)| w as usize)
            })
            .unwrap_or(80)
    } else {
        0 // unused in non-powerline path
    };

    for line in config.lines.iter().take(MAX_LINES) {
        if config.powerline {
            out_lines.push(super::render_powerline::render_powerline_line(
                line, ctx, term_width,
            ));
            continue;
        }

        let mut parts: Vec<Option<String>> = Vec::new();

        for seg in &line.segments {
            match seg {
                Segment::Template(t) => {
                    let result = format::render_segment(&t.template, ctx);
                    match result {
                        Some(s) if !s.is_empty() => {
                            let padded = pad(&s, t.padding);
                            let colored =
                                apply_colors(&padded, t.color.as_deref(), t.bg.as_deref());
                            parts.push(Some(colored));
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

/// Wrap `value` with ANSI fg/bg codes if either is Some. Appends `\x1b[0m` reset.
fn apply_colors(value: &str, fg: Option<&str>, bg: Option<&str>) -> String {
    if fg.is_none() && bg.is_none() {
        return value.to_owned();
    }
    let mut s = String::new();
    if let Some(name) = fg {
        s.push_str(ansi_fg(name));
    }
    if let Some(name) = bg {
        s.push_str(ansi_bg(name));
    }
    s.push_str(value);
    s.push_str("\x1b[0m");
    s
}

/// Apply symmetric padding around a string value (left + right by `n` spaces).
pub(crate) fn pad(s: &str, n: u8) -> String {
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
#[path = "render_color_tests.rs"]
mod render_color_tests;

#[cfg(test)]
#[path = "render_inline_tests.rs"]
mod tests;
