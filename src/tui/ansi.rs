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
//! - 38;5;N → 256-color fg (`Color::Indexed(N)`)
//! - 48;5;N → 256-color bg (`Color::Indexed(N)`)
//! - 38;2;R;G;B → RGB fg (`Color::Rgb(R,G,B)`)
//! - 48;2;R;G;B → RGB bg (`Color::Rgb(R,G,B)`)
//!
//! Unknown CSI commands and non-`[` ESC continuations are silently skipped.
//! If a CSI sequence is unterminated (no command byte before EOF), parsing
//! stops at the unterminated escape and the rest of the input is dropped.

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

/// Apply SGR codes from a parsed slice, handling 256-color and RGB sub-sequences.
fn apply_sgr_params(style: &mut Style, params: &str) {
    if params.is_empty() {
        // `\x1b[m` or `\x1b[;m` with only empty parts → reset.
        *style = Style::default();
        return;
    }

    let parts: Vec<&str> = params.split(';').collect();
    let mut idx = 0;
    while idx < parts.len() {
        let code: u32 = match parts[idx].parse() {
            Ok(n) => n,
            Err(_) => {
                // Parse failure (empty part, overflow, garbage) — skip silently.
                idx += 1;
                continue;
            }
        };

        match code {
            38 | 48 => {
                // 256-color: 38;5;N  or  RGB: 38;2;R;G;B
                if idx + 2 < parts.len() {
                    let sub: u32 = match parts[idx + 1].parse() {
                        Ok(n) => n,
                        Err(_) => {
                            idx += 1;
                            continue;
                        }
                    };
                    if sub == 5 {
                        if let Ok(n) = parts[idx + 2].parse::<u8>() {
                            let color = Color::Indexed(n);
                            if code == 38 {
                                *style = style.fg(color);
                            } else {
                                *style = style.bg(color);
                            }
                        }
                        idx += 3;
                        continue;
                    } else if sub == 2 && idx + 4 < parts.len() {
                        let r = parts[idx + 2].parse::<u8>();
                        let g = parts[idx + 3].parse::<u8>();
                        let b = parts[idx + 4].parse::<u8>();
                        if let (Ok(r), Ok(g), Ok(b)) = (r, g, b) {
                            let color = Color::Rgb(r, g, b);
                            if code == 38 {
                                *style = style.fg(color);
                            } else {
                                *style = style.bg(color);
                            }
                        }
                        idx += 5;
                        continue;
                    }
                }
                // Truncated sub-sequence — drop silently.
                idx += 1;
            }
            n => {
                apply_code(style, n as u16);
                idx += 1;
            }
        }
    }
}

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
    // Byte offset of the start of the current plain-text run in `s`.
    let mut text_start: usize = 0;
    // Byte offset of the end of the current plain-text run (exclusive).
    let mut text_end: usize = 0;
    let mut i = 0;

    // push_span emits the current text slice as a span; resets text_start to text_end.
    // The trailing `let _ = text_start;` suppresses unused-assignment when the
    // caller reassigns text_start immediately after (ESC/newline branches).
    macro_rules! push_span {
        () => {{
            if text_end > text_start {
                let text = s[text_start..text_end].to_owned();
                spans.push(Span::styled(text, current_style));
            }
            #[allow(unused_assignments)]
            {
                text_start = text_end;
            }
        }};
    }

    while i < len {
        if bytes[i] == b'\x1b' {
            push_span!();
            // ESC found — peek at next byte.
            if i + 1 < len && bytes[i + 1] == b'[' {
                // CSI sequence: accumulate until we hit a letter (command byte).
                i += 2; // skip ESC and '['
                let csi_start = i;
                while i < len && !bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                if i >= len {
                    // Unterminated CSI — stop parsing.
                    break;
                }
                let command = bytes[i];
                i += 1;
                if command == b'm' {
                    let params = &s[csi_start..i - 1]; // everything between '[' and 'm'
                    apply_sgr_params(&mut current_style, params);
                }
                // Non-'m' CSI commands are skipped (i already advanced past them).
            } else {
                // Standalone ESC not followed by '[': skip just the ESC byte.
                i += 1;
            }
            text_start = i;
            text_end = i;
        } else if bytes[i] == b'\n' {
            push_span!();
            lines.push(Line::from(std::mem::take(&mut spans)));
            i += 1;
            text_start = i;
            text_end = i;
        } else {
            // Advance text_end by the full UTF-8 char length.
            let ch_len = char_len_at(bytes, i);
            i += ch_len;
            text_end += ch_len;
        }
    }

    // Flush any remaining buffered text.
    push_span!();
    // Always emit a final line (even if empty, to represent the last segment).
    lines.push(Line::from(spans));

    lines
}

/// Return the byte length of the UTF-8 character starting at `bytes[i]`.
/// Falls back to 1 for continuation bytes or lone bytes (best-effort).
fn char_len_at(bytes: &[u8], i: usize) -> usize {
    let b = bytes[i];
    if b < 0x80 {
        1
    } else if b & 0xE0 == 0xC0 {
        2
    } else if b & 0xF0 == 0xE0 {
        3
    } else if b & 0xF8 == 0xF0 {
        4
    } else {
        1 // continuation byte or lone byte
    }
}

#[cfg(test)]
#[path = "ansi_tests.rs"]
mod tests;
