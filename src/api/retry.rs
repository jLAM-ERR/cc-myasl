//! RFC 9110 Retry-After header parser — integer seconds ONLY.
//!
//! We deliberately accept only `"\d+"` and ignore HTTP-date format.
//! HTTP-date parsing without `chrono` is non-trivial and we keep
//! the dep set minimal; HTTP-date headers fall back to the caller's
//! default lock duration (300 s).

use std::time::Duration;

/// Parse a `Retry-After` header value as an integer number of seconds.
///
/// Returns `Some(Duration)` if the value is a non-negative integer (after
/// trimming whitespace). Returns `None` for HTTP-date strings, negative
/// integers, non-numeric strings, or empty/whitespace-only input.
///
/// # Deviation from RFC 9110
/// RFC 9110 permits both integer seconds and HTTP-date format for
/// `Retry-After`. This implementation only handles integer seconds.
/// Callers should treat a `None` result as "use the default lock duration
/// (300 s)".
pub fn parse_retry_after(value: &str) -> Option<Duration> {
    let trimmed = value.trim();
    trimmed.parse::<u64>().ok().map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sixty_seconds() {
        assert_eq!(parse_retry_after("60"), Some(Duration::from_secs(60)));
    }

    #[test]
    fn zero_seconds() {
        assert_eq!(parse_retry_after("0"), Some(Duration::from_secs(0)));
    }

    #[test]
    fn three_thousand_six_hundred_seconds() {
        assert_eq!(parse_retry_after("3600"), Some(Duration::from_secs(3600)));
    }

    #[test]
    fn trims_whitespace() {
        assert_eq!(parse_retry_after(" 60 "), Some(Duration::from_secs(60)));
    }

    #[test]
    fn negative_integer_returns_none() {
        assert_eq!(parse_retry_after("-1"), None);
    }

    #[test]
    fn alphabetic_string_returns_none() {
        assert_eq!(parse_retry_after("abc"), None);
    }

    #[test]
    fn empty_string_returns_none() {
        assert_eq!(parse_retry_after(""), None);
    }

    #[test]
    fn whitespace_only_returns_none() {
        assert_eq!(parse_retry_after("   "), None);
    }

    #[test]
    fn http_date_returns_none() {
        assert_eq!(parse_retry_after("Fri, 13 Mar 2026 12:00:00 GMT"), None);
    }

    #[test]
    fn float_returns_none() {
        assert_eq!(parse_retry_after("60.5"), None);
    }
}
