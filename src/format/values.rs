//! Pure rendering helpers for the status-line format engine.
//!
//! All functions are stateless, stdlib-only, and have no side effects
//! beyond returning a `String`.  They do NOT depend on `crate::api` or
//! `crate::cache`.
//!
//! `clock_local` is a UTC-only stub; the full local-timezone
//! implementation lands in `time.rs` (Task 11a) and will replace this.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

// ── bar ─────────────────────────────────────────────────────────────────────

/// Render an ASCII block-fill progress bar.
///
/// Format: `[` + `█` × filled + `░` × empty + `]`.
///
/// `percent` is clamped to `0.0..=100.0`.  `width == 0` returns `"[]"`.
pub fn bar(percent: f64, width: usize) -> String {
    if width == 0 {
        return "[]".to_owned();
    }
    let clamped = percent.clamp(0.0, 100.0);
    let filled = (clamped / 100.0 * width as f64).round() as usize;
    let filled = filled.min(width);
    let empty = width - filled;
    let mut s = String::with_capacity(2 + width * 3); // each char is 3 bytes
    s.push('[');
    for _ in 0..filled {
        s.push('█');
    }
    for _ in 0..empty {
        s.push('░');
    }
    s.push(']');
    s
}

// ── percent helpers ──────────────────────────────────────────────────────────

/// Format `p` as an integer string (rounded), without a `%` sign.
///
/// e.g. `23.5` → `"24"`, `23.4` → `"23"`.
pub fn percent_int(p: f64) -> String {
    format!("{}", p.round() as i64)
}

/// Format `p` to one decimal place, without a `%` sign.
///
/// e.g. `23.456` → `"23.5"`.
pub fn percent_decimal(p: f64) -> String {
    format!("{:.1}", p)
}

// ── clock_local ──────────────────────────────────────────────────────────────

/// Format a Unix timestamp as `HH:MM`.
///
/// # Note — UTC-only stub
///
/// This implementation uses UTC.  The full local-timezone version
/// (`time::format_clock_local`) lands in Task 11a and will supersede
/// this function.  Tests here validate the `HH:MM` format contract only.
pub fn clock_local(unix_secs: u64) -> String {
    // Build a SystemTime from the epoch offset.
    let dt = UNIX_EPOCH + Duration::from_secs(unix_secs);
    // Duration since epoch gives us total seconds.
    let total_secs = dt
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs();
    let hours = (total_secs % 86_400) / 3_600;
    let minutes = (total_secs % 3_600) / 60;
    format!("{:02}:{:02}", hours, minutes)
}

// ── countdown ────────────────────────────────────────────────────────────────

/// Produce a human-readable countdown string like `"2h13m"`.
///
/// If `target_unix <= now_unix`, returns `"0m"`.  Leading zero-value
/// units are dropped (e.g. `"1d2h"` if minutes are 0).  At least one
/// unit is always shown (`"0m"` if delta is 0 s).
pub fn countdown(target_unix: u64, now_unix: u64) -> String {
    if target_unix <= now_unix {
        return "0m".to_owned();
    }
    let total = target_unix - now_unix;
    let days = total / 86_400;
    let hours = (total % 86_400) / 3_600;
    let mins = (total % 3_600) / 60;

    let mut s = String::new();
    if days > 0 {
        s.push_str(&format!("{days}d"));
    }
    if hours > 0 {
        s.push_str(&format!("{hours}h"));
    }
    if mins > 0 {
        s.push_str(&format!("{mins}m"));
    }
    if s.is_empty() {
        s.push_str("0m");
    }
    s
}

// ── format_duration_ms ───────────────────────────────────────────────────────

/// Format a millisecond duration as a compact human-readable string.
///
/// - Sub-second: `"0s"`
/// - Seconds only (< 60 s): `"Xs"`
/// - Minutes (< 1 h): `"Xm"` (seconds dropped)
/// - Hours (< 1 d): `"XhYm"` (always show minutes, even when 0)
/// - Days: `"XdYh"` (always show hours, even when 0)
pub fn format_duration_ms(ms: u64) -> String {
    let total_secs = ms / 1_000;
    if total_secs == 0 {
        return "0s".to_owned();
    }
    let days = total_secs / 86_400;
    let hours = (total_secs % 86_400) / 3_600;
    let mins = (total_secs % 3_600) / 60;
    let secs = total_secs % 60;

    if days > 0 {
        format!("{days}d{hours}h")
    } else if hours > 0 {
        format!("{hours}h{mins}m")
    } else if mins > 0 {
        format!("{mins}m")
    } else {
        format!("{secs}s")
    }
}

// ── format_count ─────────────────────────────────────────────────────────────

/// Format a token count as a compact human-readable string.
///
/// Rounding: values are divided by the unit magnitude and formatted with
/// `{:.1}`, which uses standard "round half to even" (banker's rounding)
/// as implemented by Rust's `format!`.  In practice the difference is
/// only visible at exact .5 boundaries — callers should treat the
/// displayed value as an approximation.
///
/// Thresholds (exclusive lower bound for the *next* suffix):
/// - `< 1_000` → raw integer (no suffix), e.g. `999` → `"999"`
/// - `< 1_000_000` → `"X.Xk"`, e.g. `1_234` → `"1.2k"`
/// - `< 1_000_000_000` → `"X.XM"`, e.g. `1_234_567` → `"1.2M"`
/// - `≥ 1_000_000_000` → `"X.XG"`, e.g. `1_000_000_000` → `"1.0G"`
pub fn format_count(n: u64) -> String {
    if n < 1_000 {
        return n.to_string();
    }
    if n < 1_000_000 {
        return format!("{:.1}k", n as f64 / 1_000.0);
    }
    if n < 1_000_000_000 {
        return format!("{:.1}M", n as f64 / 1_000_000.0);
    }
    format!("{:.1}G", n as f64 / 1_000_000_000.0)
}

// ── helper: current unix time ────────────────────────────────────────────────

/// Return the current Unix timestamp in seconds (best-effort; 0 on error).
#[allow(dead_code)]
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

// ── ansi color helpers ────────────────────────────────────────────────────────

/// ANSI foreground escape for a named color. `""` for unknown names.
pub fn ansi_fg(name: &str) -> &'static str {
    match name {
        "red" => "\x1b[31m",
        "green" => "\x1b[32m",
        "yellow" => "\x1b[33m",
        "blue" => "\x1b[34m",
        "magenta" => "\x1b[35m",
        "cyan" => "\x1b[36m",
        "white" => "\x1b[37m",
        "default" => "\x1b[39m",
        _ => "",
    }
}

/// ANSI background escape for a named color. `""` for unknown names.
pub fn ansi_bg(name: &str) -> &'static str {
    match name {
        "red" => "\x1b[41m",
        "green" => "\x1b[42m",
        "yellow" => "\x1b[43m",
        "blue" => "\x1b[44m",
        "magenta" => "\x1b[45m",
        "cyan" => "\x1b[46m",
        "white" => "\x1b[47m",
        "default" => "\x1b[49m",
        _ => "",
    }
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "values_color_tests.rs"]
mod color_tests;

#[cfg(test)]
#[path = "values_tests.rs"]
mod tests;
