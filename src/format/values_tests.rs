//! Unit tests for format::values helpers.

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

// ── format_count (full boundary suite in placeholders::tokens_tests) ────

#[test]
fn count_raw_below_1k() {
    assert_eq!(format_count(0), "0");
    assert_eq!(format_count(999), "999");
}

#[test]
fn count_suffixes() {
    assert_eq!(format_count(1_000), "1.0k");
    assert_eq!(format_count(1_000_000), "1.0M");
    assert_eq!(format_count(1_000_000_000), "1.0G");
}
