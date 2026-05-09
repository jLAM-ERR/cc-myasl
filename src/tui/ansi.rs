//! ANSI-to-ratatui-Spans parser.
//!
//! # Contract
//! [`ansi_to_lines`] parses an ANSI-escaped string and returns
//! `Vec<ratatui::text::Line<'static>>` where every [`Span`] owns its text
//! (`Cow::Owned(String)`).  The input can be freed immediately after the call.
//!
//! Callers may mutate `Span::style` in place after receipt — e.g. to overlay
//! `Modifier::DIM` on custom segments or `Modifier::REVERSED` on the cursor
//! segment — because `Span<'static>` owns its fields entirely.
//!
//! # Supported escape codes
//! Only `CSI … m` (SGR) sequences are interpreted:
//! - 0  → reset all attributes
//! - 1  → bold (`Modifier::BOLD`)
//! - 2  → dim (`Modifier::DIM`)
//! - 7  → reverse (`Modifier::REVERSED`)
//! - 22 → remove bold/dim
//! - 27 → remove reverse
//! - 30-37 → fg standard (Black…White)
//! - 39 → default fg
//! - 40-47 → bg standard (Black…White)
//! - 49 → default bg
//! - 90-97 → fg bright (DarkGray…White)
//! - 100-107 → bg bright
//!
//! Unknown CSI commands and non-`[` ESC continuations are silently skipped.
//! Unterminated CSI sequences (no final byte before end-of-string) are dropped.

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

/// Standard ANSI color order (codes 30-37 / 40-47).
const STANDARD: [Color; 8] = [
    Color::Black,
    Color::Red,
    Color::Green,
    Color::Yellow,
    Color::Blue,
    Color::Magenta,
    Color::Cyan,
    Color::White,
];

/// Bright ANSI color order (codes 90-97 / 100-107).
const BRIGHT: [Color; 8] = [
    Color::DarkGray,
    Color::LightRed,
    Color::LightGreen,
    Color::LightYellow,
    Color::LightBlue,
    Color::LightMagenta,
    Color::LightCyan,
    Color::White,
];

/// Apply a single SGR numeric code to `style`.
fn apply_code(style: &mut Style, code: u16) {
    match code {
        0 => *style = Style::default(),
        1 => *style = style.add_modifier(Modifier::BOLD),
        2 => *style = style.add_modifier(Modifier::DIM),
        7 => *style = style.add_modifier(Modifier::REVERSED),
        22 => *style = style.remove_modifier(Modifier::BOLD | Modifier::DIM),
        27 => *style = style.remove_modifier(Modifier::REVERSED),
        30..=37 => *style = style.fg(STANDARD[(code - 30) as usize]),
        39 => *style = style.fg(Color::Reset),
        40..=47 => *style = style.bg(STANDARD[(code - 40) as usize]),
        49 => *style = style.bg(Color::Reset),
        90..=97 => *style = style.fg(BRIGHT[(code - 90) as usize]),
        100..=107 => *style = style.bg(BRIGHT[(code - 100) as usize]),
        _ => {} // silently ignore unknown codes
    }
}

/// Parse an ANSI-escaped string into ratatui [`Line`]s with `'static` spans.
///
/// `\n` ends the current line and starts a new one; style state is preserved
/// across newlines (matching real terminal behaviour).
pub fn ansi_to_lines(s: &str) -> Vec<Line<'static>> {
    let bytes = s.as_bytes();
    let len = bytes.len();

    let mut lines: Vec<Line<'static>> = Vec::new();
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut current_style = Style::default();
    let mut buf = String::new();
    let mut i = 0;

    // Flush `buf` as a span with `current_style` into `spans`.
    macro_rules! flush {
        () => {
            if !buf.is_empty() {
                let text = std::mem::take(&mut buf);
                spans.push(Span::styled(text, current_style));
            }
        };
    }

    while i < len {
        if bytes[i] == b'\x1b' {
            // ESC found — peek at next byte.
            if i + 1 < len && bytes[i + 1] == b'[' {
                // CSI sequence: accumulate until we hit a letter (command byte).
                i += 2; // skip ESC and '['
                let csi_start = i;
                while i < len && !bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                if i >= len {
                    // Unterminated CSI — drop silently, stop parsing.
                    break;
                }
                let command = bytes[i];
                i += 1;
                if command == b'm' {
                    // SGR: parse semicolon-separated codes.
                    let params = &s[csi_start..i - 1]; // everything between '[' and 'm'
                    flush!();
                    if params.is_empty() {
                        // `\x1b[m` is equivalent to reset.
                        apply_code(&mut current_style, 0);
                    } else {
                        for part in params.split(';') {
                            let code: u16 = part.parse().unwrap_or(0);
                            apply_code(&mut current_style, code);
                        }
                    }
                }
                // Non-'m' CSI commands are skipped (i already advanced past them).
            } else {
                // Standalone ESC not followed by '[': skip just the ESC byte.
                i += 1;
            }
        } else if bytes[i] == b'\n' {
            flush!();
            lines.push(Line::from(std::mem::take(&mut spans)));
            i += 1;
        } else {
            buf.push(bytes[i] as char);
            i += 1;
        }
    }

    // Flush any remaining buffered text.
    flush!();
    // Always emit a final line (even if empty, to represent the last segment).
    lines.push(Line::from(spans));

    lines
}

#[cfg(test)]
#[path = "ansi_tests.rs"]
mod tests;
