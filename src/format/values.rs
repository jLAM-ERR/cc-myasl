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

// ── helper: current unix time ────────────────────────────────────────────────

/// Return the current Unix timestamp in seconds (best-effort; 0 on error).
#[allow(dead_code)]
pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

// ── tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── bar ──────────────────────────────────────────────────────────────────

    #[test]
    fn bar_zero_width_returns_empty_brackets() {
        assert_eq!(bar(50.0, 0), "[]");
    }

    #[test]
    fn bar_zero_percent_width_5() {
        // 0 % → all empty
        assert_eq!(bar(0.0, 5), "[░░░░░]");
    }

    #[test]
    fn bar_hundred_percent_width_5() {
        // 100 % → all filled
        assert_eq!(bar(100.0, 5), "[█████]");
    }

    #[test]
    fn bar_fifty_percent_width_1() {
        // 50 % of 1 = 0.5, rounds to 1 → "[█]"
        assert_eq!(bar(50.0, 1), "[█]");
    }

    #[test]
    fn bar_fifty_percent_width_20() {
        let s = bar(50.0, 20);
        // 10 filled + 10 empty
        assert_eq!(s, "[██████████░░░░░░░░░░]");
    }

    #[test]
    fn bar_zero_percent_width_1() {
        assert_eq!(bar(0.0, 1), "[░]");
    }

    #[test]
    fn bar_hundred_percent_width_20() {
        let s = bar(100.0, 20);
        assert_eq!(s, "[████████████████████]");
    }

    #[test]
    fn bar_clamps_negative_percent() {
        // -10 % → treated as 0 %
        assert_eq!(bar(-10.0, 5), "[░░░░░]");
    }

    #[test]
    fn bar_clamps_over_100_percent() {
        // 150 % → treated as 100 %
        assert_eq!(bar(150.0, 5), "[█████]");
    }

    #[test]
    fn bar_width_1_at_various_percents() {
        // 0 % rounds to 0 filled
        assert_eq!(bar(0.0, 1), "[░]");
        // 49 % rounds to 0 filled (0.49 rounds to 0)
        assert_eq!(bar(49.0, 1), "[░]");
        // 50 % rounds to 1 filled
        assert_eq!(bar(50.0, 1), "[█]");
        // 100 % → all filled
        assert_eq!(bar(100.0, 1), "[█]");
    }

    // ── percent_int ──────────────────────────────────────────────────────────

    #[test]
    fn percent_int_rounds_half_up() {
        assert_eq!(percent_int(23.5), "24");
        assert_eq!(percent_int(23.4), "23");
    }

    #[test]
    fn percent_int_exact() {
        assert_eq!(percent_int(0.0), "0");
        assert_eq!(percent_int(100.0), "100");
    }

    #[test]
    fn percent_int_rounds_negative() {
        // -0.5 rounds towards zero in Rust's f64::round (ties round away from 0)
        assert_eq!(percent_int(-0.5), "-1"); // f64::round rounds away from 0
        assert_eq!(percent_int(-1.4), "-1");
    }

    // ── percent_decimal ──────────────────────────────────────────────────────

    #[test]
    fn percent_decimal_one_place() {
        assert_eq!(percent_decimal(23.456), "23.5");
        assert_eq!(percent_decimal(0.0), "0.0");
        assert_eq!(percent_decimal(100.0), "100.0");
    }

    #[test]
    fn percent_decimal_already_one_place() {
        assert_eq!(percent_decimal(42.1), "42.1");
    }

    // ── clock_local (UTC stub) ────────────────────────────────────────────────

    /// Verify that `clock_local` returns a string matching `HH:MM`.
    fn is_hhmm(s: &str) -> bool {
        if s.len() != 5 {
            return false;
        }
        let bytes = s.as_bytes();
        bytes[0].is_ascii_digit()
            && bytes[1].is_ascii_digit()
            && bytes[2] == b':'
            && bytes[3].is_ascii_digit()
            && bytes[4].is_ascii_digit()
    }

    #[test]
    fn clock_local_epoch_zero_is_hhmm() {
        // Epoch 0 == 1970-01-01 00:00:00 UTC → "00:00"
        let s = clock_local(0);
        assert!(is_hhmm(&s), "expected HH:MM, got {s:?}");
        assert_eq!(s, "00:00");
    }

    #[test]
    fn clock_local_noon_utc() {
        // 12 * 3600 = 43200
        let s = clock_local(43_200);
        assert!(is_hhmm(&s), "expected HH:MM, got {s:?}");
        assert_eq!(s, "12:00");
    }

    #[test]
    fn clock_local_just_before_midnight() {
        // 23:59 = (23*3600 + 59*60) = 82800 + 3540 = 86340
        let s = clock_local(86_340);
        assert!(is_hhmm(&s), "expected HH:MM, got {s:?}");
        assert_eq!(s, "23:59");
    }

    #[test]
    fn clock_local_arbitrary_epoch() {
        // 1_000_000 seconds past epoch; just verify format
        let s = clock_local(1_000_000);
        assert!(is_hhmm(&s), "expected HH:MM, got {s:?}");
    }

    // ── countdown ────────────────────────────────────────────────────────────

    #[test]
    fn countdown_zero_seconds() {
        assert_eq!(countdown(100, 100), "0m");
    }

    #[test]
    fn countdown_target_before_now() {
        assert_eq!(countdown(50, 100), "0m");
    }

    #[test]
    fn countdown_30_seconds() {
        // < 1 min → "0m"
        assert_eq!(countdown(130, 100), "0m");
    }

    #[test]
    fn countdown_exactly_60_seconds() {
        assert_eq!(countdown(160, 100), "1m");
    }

    #[test]
    fn countdown_exactly_1_hour() {
        assert_eq!(countdown(100 + 3_600, 100), "1h");
    }

    #[test]
    fn countdown_1_hour_30_minutes() {
        assert_eq!(countdown(100 + 3_600 + 1_800, 100), "1h30m");
    }

    #[test]
    fn countdown_exactly_1_day() {
        assert_eq!(countdown(100 + 86_400, 100), "1d");
    }

    #[test]
    fn countdown_1_day_2_hours_13_minutes() {
        let delta = 86_400 + 2 * 3_600 + 13 * 60;
        assert_eq!(countdown(100 + delta, 100), "1d2h13m");
    }

    #[test]
    fn countdown_drops_zero_hours() {
        // 1d0h30m → "1d30m"
        let delta = 86_400 + 30 * 60;
        assert_eq!(countdown(100 + delta, 100), "1d30m");
    }

    #[test]
    fn countdown_drops_zero_minutes() {
        // 1d2h0m → "1d2h"
        let delta = 86_400 + 2 * 3_600;
        assert_eq!(countdown(100 + delta, 100), "1d2h");
    }

    #[test]
    fn countdown_only_minutes() {
        assert_eq!(countdown(100 + 45 * 60, 100), "45m");
    }

    #[test]
    fn countdown_only_seconds_shows_0m() {
        // 59 seconds — less than 1 minute → "0m"
        assert_eq!(countdown(159, 100), "0m");
    }

    // ── format_duration_ms ───────────────────────────────────────────────────

    #[test]
    fn duration_ms_zero() {
        assert_eq!(format_duration_ms(0), "0s");
    }

    #[test]
    fn duration_ms_999_sub_second() {
        assert_eq!(format_duration_ms(999), "0s");
    }

    #[test]
    fn duration_ms_1000_one_second() {
        assert_eq!(format_duration_ms(1_000), "1s");
    }

    #[test]
    fn duration_ms_4500() {
        assert_eq!(format_duration_ms(4_500), "4s");
    }

    #[test]
    fn duration_ms_59999_just_under_one_minute() {
        assert_eq!(format_duration_ms(59_999), "59s");
    }

    #[test]
    fn duration_ms_60000_one_minute() {
        assert_eq!(format_duration_ms(60_000), "1m");
    }

    #[test]
    fn duration_ms_60500_rounds_down() {
        assert_eq!(format_duration_ms(60_500), "1m");
    }

    #[test]
    fn duration_ms_3599999_just_under_one_hour() {
        assert_eq!(format_duration_ms(3_599_999), "59m");
    }

    #[test]
    fn duration_ms_3600000_one_hour() {
        assert_eq!(format_duration_ms(3_600_000), "1h0m");
    }

    #[test]
    fn duration_ms_3661000_one_hour_one_minute() {
        assert_eq!(format_duration_ms(3_661_000), "1h1m");
    }

    #[test]
    fn duration_ms_86399999_just_under_one_day() {
        assert_eq!(format_duration_ms(86_399_999), "23h59m");
    }

    #[test]
    fn duration_ms_86400000_one_day() {
        assert_eq!(format_duration_ms(86_400_000), "1d0h");
    }

    #[test]
    fn duration_ms_u64_max_does_not_panic() {
        // Should not panic; exact value not pinned.
        let _ = format_duration_ms(u64::MAX);
    }
}
