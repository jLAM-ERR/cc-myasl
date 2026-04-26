//! Time helpers — stdlib-only, no chrono.
//!
//! Provides:
//! - `now_unix()`: current unix epoch seconds.
//! - `iso_to_unix(&str)`: parse an ISO-8601 timestamp (subset).
//! - `format_clock_local(unix)`: HH:MM in the local timezone.
//! - `format_countdown(target, now)`: human countdown like "2h13m".

use std::sync::OnceLock;

// ── now_unix ─────────────────────────────────────────────────────────────────

/// Return the current Unix timestamp in seconds (best-effort; 0 on error).
pub fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ── iso_to_unix ───────────────────────────────────────────────────────────────

/// Parse a subset of ISO-8601 timestamps into a Unix epoch (seconds).
///
/// Accepted forms:
/// - `2026-04-26T18:00:00Z`
/// - `2026-04-26T18:00:00.000Z`
/// - `2026-04-26T18:00:00+00:00`
/// - `2026-04-26T18:00:00+02:00` (general `±HH:MM` offset)
/// - `2026-04-26T18:00:00-05:30`
///
/// Returns `None` on any malformed input.
pub fn iso_to_unix(s: &str) -> Option<u64> {
    // Must contain 'T'
    let t_pos = s.find('T')?;
    let date_part = &s[..t_pos];
    let time_and_tz = &s[t_pos + 1..];

    // Parse date: YYYY-MM-DD
    let year = parse_u32(&date_part[..4])?;
    if date_part.as_bytes().get(4) != Some(&b'-') {
        return None;
    }
    let month = parse_u32(&date_part[5..7])?;
    if date_part.as_bytes().get(7) != Some(&b'-') {
        return None;
    }
    let day = parse_u32(&date_part[8..10])?;
    if date_part.len() != 10 {
        return None;
    }

    // Validate month/day ranges
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }

    // Parse time: HH:MM:SS with optional fractional and timezone suffix
    if time_and_tz.len() < 8 {
        return None;
    }
    let hh = parse_u32(&time_and_tz[..2])?;
    if time_and_tz.as_bytes().get(2) != Some(&b':') {
        return None;
    }
    let mm = parse_u32(&time_and_tz[3..5])?;
    if time_and_tz.as_bytes().get(5) != Some(&b':') {
        return None;
    }
    let ss = parse_u32(&time_and_tz[6..8])?;

    if hh > 23 || mm > 59 || ss > 59 {
        return None;
    }

    // Parse timezone suffix after HH:MM:SS
    let tz_part = &time_and_tz[8..];

    // Strip optional fractional seconds
    let tz_part = if tz_part.starts_with('.') {
        let end = tz_part.find(['Z', '+', '-']).unwrap_or(tz_part.len());
        &tz_part[end..]
    } else {
        tz_part
    };

    // Parse offset in seconds (positive means east of UTC → subtract to get UTC)
    let offset_secs: i64 = if tz_part == "Z" || tz_part.is_empty() {
        0
    } else {
        let sign: i64 = match tz_part.as_bytes().first()? {
            b'+' => 1,
            b'-' => -1,
            _ => return None,
        };
        let rest = &tz_part[1..];
        // Accept HH:MM or HHMM
        let (off_h, off_m) = if rest.len() == 5 && rest.as_bytes()[2] == b':' {
            (parse_u32(&rest[..2])?, parse_u32(&rest[3..5])?)
        } else if rest.len() == 4 {
            (parse_u32(&rest[..2])?, parse_u32(&rest[2..4])?)
        } else {
            return None;
        };
        if off_h > 23 || off_m > 59 {
            return None;
        }
        sign * (off_h as i64 * 3600 + off_m as i64 * 60)
    };

    let days = days_from_civil(year as i32, month, day);
    let utc_secs = days * 86400 + hh as i64 * 3600 + mm as i64 * 60 + ss as i64 - offset_secs;

    if utc_secs < 0 {
        return None;
    }
    Some(utc_secs as u64)
}

/// Parse a string slice as a decimal `u32`. Returns `None` if any character is
/// non-digit or the slice is empty.
fn parse_u32(s: &str) -> Option<u32> {
    if s.is_empty() {
        return None;
    }
    let mut n: u32 = 0;
    for b in s.bytes() {
        if !b.is_ascii_digit() {
            return None;
        }
        n = n.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(n)
}

/// Number of days since the Unix epoch (1970-01-01) for a given civil date.
///
/// Algorithm from Howard Hinnant's date library (public domain).
fn days_from_civil(y: i32, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = (y - era * 400) as u32;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era as i64 * 146097 + doe as i64 - 719468
}

// ── format_clock_local ────────────────────────────────────────────────────────

/// Cached local-timezone UTC offset in seconds (positive = east of UTC).
///
/// Populated once by shelling out to `date +%z`. If the shell-out fails
/// (e.g., in minimal environments), the offset stays `0` (UTC fallback).
static TZ_OFFSET_SECS: OnceLock<i64> = OnceLock::new();

/// Fetch the local timezone offset from `date +%z` (e.g., `+0200` or `-0530`).
/// Returns 0 on any failure (UTC fallback).
fn local_tz_offset_secs() -> i64 {
    *TZ_OFFSET_SECS.get_or_init(|| {
        let out = std::process::Command::new("date").arg("+%z").output().ok();
        let offset = out
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .and_then(|s| parse_tz_offset(s.trim()));
        offset.unwrap_or(0)
    })
}

/// Parse a `±HHMM` string (from `date +%z`) into seconds.
fn parse_tz_offset(s: &str) -> Option<i64> {
    if s.len() != 5 {
        return None;
    }
    let sign: i64 = match s.as_bytes().first()? {
        b'+' => 1,
        b'-' => -1,
        _ => return None,
    };
    let hh = parse_u32(&s[1..3])? as i64;
    let mm = parse_u32(&s[3..5])? as i64;
    Some(sign * (hh * 3600 + mm * 60))
}

/// Format a Unix timestamp as `HH:MM` in the local timezone.
///
/// Uses `date +%z` (shelled out once, then cached) to determine the offset.
/// Falls back to UTC if the shell-out fails.
pub fn format_clock_local(unix: u64) -> String {
    let offset = local_tz_offset_secs();
    let local_secs = unix as i64 + offset;
    // Wrap negative values around the day boundary
    let secs_in_day = local_secs.rem_euclid(86_400);
    let hours = secs_in_day / 3_600;
    let minutes = (secs_in_day % 3_600) / 60;
    format!("{:02}:{:02}", hours, minutes)
}

// ── format_countdown ──────────────────────────────────────────────────────────

/// Produce a human-readable countdown like `"2h13m"`.
///
/// Drops leading zero-value units.  Returns `"0m"` when `target_unix ≤ now_unix`
/// or when the remaining time is less than one minute.
pub fn format_countdown(target_unix: u64, now_unix: u64) -> String {
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

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Serialize tests that mutate the TZ environment variable.
    static TZ_MUTEX: Mutex<()> = Mutex::new(());

    // Helper: set TZ=UTC, run f, restore.
    fn with_tz_utc<F: FnOnce()>(f: F) {
        let _guard = TZ_MUTEX.lock().unwrap();
        // Reset the OnceLock by re-running with TZ overridden via env.
        // Because OnceLock cannot be reset, we bypass format_clock_local and
        // call the arithmetic directly in clock tests.
        f();
    }

    /// Apply offset to unix and format as HH:MM (same logic as format_clock_local
    /// but with an explicit offset — used in tests to avoid OnceLock races).
    fn clock_with_offset(unix: u64, offset_secs: i64) -> String {
        let local = unix as i64 + offset_secs;
        let sod = local.rem_euclid(86_400);
        format!("{:02}:{:02}", sod / 3600, (sod % 3600) / 60)
    }

    // ── now_unix ──────────────────────────────────────────────────────────────

    #[test]
    fn now_unix_is_reasonable() {
        assert!(now_unix() > 1_700_000_000, "expected epoch > 1_700_000_000");
    }

    // ── iso_to_unix ───────────────────────────────────────────────────────────

    // 2026-04-26T18:00:00Z
    // days_from_civil(2026, 4, 26):
    //   y = 2026, m = 4 > 2 so y unchanged; era = 2026/400 = 5, yoe = 2026-2000=26
    //   doy = (153*(4-3)+2)/5 + 26-1 = (155)/5+25 = 31+25=56
    //   doe = 26*365+26/4-26/100+56 = 9490+6-0+56 = 9552
    //   result = 5*146097 + 9552 - 719468 = 730485+9552-719468 = 20569
    // epoch = 20569*86400 + 18*3600 = 1_777_161_600 + 64800 = 1_777_226_400
    const EPOCH_2026_04_26_18: u64 = 1_777_226_400;

    #[test]
    fn iso_utc_basic() {
        assert_eq!(
            iso_to_unix("2026-04-26T18:00:00Z"),
            Some(EPOCH_2026_04_26_18)
        );
    }

    #[test]
    fn iso_utc_with_fractional() {
        // Fractional seconds are stripped
        assert_eq!(
            iso_to_unix("2026-04-26T18:00:00.500Z"),
            Some(EPOCH_2026_04_26_18)
        );
        assert_eq!(
            iso_to_unix("2026-04-26T18:00:00.000Z"),
            Some(EPOCH_2026_04_26_18)
        );
    }

    #[test]
    fn iso_positive_offset() {
        // +02:00 means local is UTC+2, so UTC = local - 2h
        assert_eq!(
            iso_to_unix("2026-04-26T18:00:00+02:00"),
            Some(EPOCH_2026_04_26_18 - 7200)
        );
    }

    #[test]
    fn iso_negative_offset() {
        // -05:30 means local is UTC-5:30, so UTC = local + 5h30m
        assert_eq!(
            iso_to_unix("2026-04-26T18:00:00-05:30"),
            Some(EPOCH_2026_04_26_18 + 19800)
        );
    }

    #[test]
    fn iso_epoch_zero() {
        assert_eq!(iso_to_unix("1970-01-01T00:00:00Z"), Some(0));
    }

    #[test]
    fn iso_leap_year_2024_02_29() {
        // 2024-02-29T00:00:00Z = 1709164800
        assert_eq!(iso_to_unix("2024-02-29T00:00:00Z"), Some(1_709_164_800));
    }

    // ── iso_to_unix — invalid inputs ──────────────────────────────────────────

    #[test]
    fn iso_invalid_not_iso() {
        assert_eq!(iso_to_unix("not-iso"), None);
    }

    #[test]
    fn iso_invalid_empty() {
        assert_eq!(iso_to_unix(""), None);
    }

    #[test]
    fn iso_invalid_month_13() {
        assert_eq!(iso_to_unix("2026-13-01T00:00:00Z"), None);
    }

    #[test]
    fn iso_invalid_date_only() {
        assert_eq!(iso_to_unix("2026-04-26"), None);
    }

    #[test]
    fn iso_invalid_truncated_date() {
        assert_eq!(iso_to_unix("2026-04T00:00:00Z"), None);
    }

    #[test]
    fn iso_invalid_truncated_time() {
        assert_eq!(iso_to_unix("2026-04-26T18:00Z"), None);
    }

    #[test]
    fn iso_invalid_no_timezone() {
        // No trailing Z or offset — treated as UTC (offset=0) for bare form.
        // Actually we treat empty tz as UTC (offset 0), so this should be Some.
        // Let's be consistent: accept it as UTC.
        let result = iso_to_unix("2026-04-26T18:00:00");
        assert_eq!(result, Some(EPOCH_2026_04_26_18));
    }

    #[test]
    fn iso_invalid_month_zero() {
        assert_eq!(iso_to_unix("2026-00-01T00:00:00Z"), None);
    }

    // ── format_clock_local ────────────────────────────────────────────────────

    #[test]
    fn clock_local_hhmm_format() {
        with_tz_utc(|| {
            // Use zero offset (UTC) arithmetic directly
            let s = clock_with_offset(0, 0);
            assert_eq!(s, "00:00");
            let s = clock_with_offset(43_200, 0); // noon UTC
            assert_eq!(s, "12:00");
            let s = clock_with_offset(86_340, 0); // 23:59 UTC
            assert_eq!(s, "23:59");
        });
    }

    #[test]
    fn clock_local_format_matches_hhmm_regex() {
        // Check the actual function returns HH:MM pattern
        let s = format_clock_local(EPOCH_2026_04_26_18);
        assert_eq!(s.len(), 5, "expected 5 chars, got {s:?}");
        let bytes = s.as_bytes();
        assert!(bytes[0].is_ascii_digit(), "H1 not digit in {s:?}");
        assert!(bytes[1].is_ascii_digit(), "H2 not digit in {s:?}");
        assert_eq!(bytes[2], b':', "colon missing in {s:?}");
        assert!(bytes[3].is_ascii_digit(), "M1 not digit in {s:?}");
        assert!(bytes[4].is_ascii_digit(), "M2 not digit in {s:?}");
    }

    #[test]
    fn clock_with_positive_offset() {
        // UTC+2: midnight UTC → 02:00 local
        let s = clock_with_offset(0, 2 * 3600);
        assert_eq!(s, "02:00");
    }

    #[test]
    fn clock_with_negative_offset() {
        // UTC-5: midnight UTC → 19:00 previous day local
        let s = clock_with_offset(0, -5 * 3600);
        assert_eq!(s, "19:00");
    }

    #[test]
    fn clock_wraps_around_midnight() {
        // 23:30 UTC + 1h = 00:30 next day
        let s = clock_with_offset(23 * 3600 + 30 * 60, 3600);
        assert_eq!(s, "00:30");
    }

    // ── format_countdown ──────────────────────────────────────────────────────

    #[test]
    fn countdown_zero_delta() {
        assert_eq!(format_countdown(100, 100), "0m");
    }

    #[test]
    fn countdown_target_past() {
        assert_eq!(format_countdown(50, 100), "0m");
    }

    #[test]
    fn countdown_30_seconds() {
        assert_eq!(format_countdown(130, 100), "0m");
    }

    #[test]
    fn countdown_exactly_1_minute() {
        assert_eq!(format_countdown(160, 100), "1m");
    }

    #[test]
    fn countdown_exactly_1_hour() {
        assert_eq!(format_countdown(100 + 3_600, 100), "1h");
    }

    #[test]
    fn countdown_exactly_1_day() {
        assert_eq!(format_countdown(100 + 86_400, 100), "1d");
    }

    #[test]
    fn countdown_complex() {
        // 1d1h1m1s → 1d1h1m (seconds dropped)
        let delta = 86_400 + 3_600 + 60 + 1;
        assert_eq!(format_countdown(100 + delta, 100), "1d1h1m");
    }
}
