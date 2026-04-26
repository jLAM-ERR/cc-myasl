//! HTTP client for the Anthropic OAuth `usage` endpoint.
//!
//! `response` defines the wire-format types.
//! `retry` parses the `Retry-After` header (integer seconds only).
//! `fetch_usage` performs the actual HTTP request.

pub mod response;
pub mod retry;

pub use response::{ExtraUsage, UsageResponse, UsageWindow};

use std::time::Duration;

/// Default OAuth `usage` endpoint host.  Tests override via the
/// `STATUSLINE_OAUTH_BASE_URL` env var (read in `main.rs` and `check.rs`).
pub const DEFAULT_OAUTH_BASE_URL: &str = "https://api.anthropic.com";

/// Outcome of a `fetch_usage` call.
pub enum FetchOutcome {
    /// 200 OK with a successfully parsed body.
    Ok(UsageResponse),
    /// 429 Too Many Requests; caller should wait at least this long.
    RateLimited(Duration),
    /// 401 Unauthorized — token is invalid or expired.
    AuthFailed,
    /// 5xx or unparseable 200 body.
    ServerError,
    /// Transport-level failure (timeout, DNS, connection refused).
    TimedOut,
}

/// Fetch quota usage from the Anthropic OAuth usage endpoint.
///
/// `base_url` is the scheme+host portion, e.g. `"https://api.anthropic.com"`.
/// Tests pass a `mockito` server URL here so no real network calls are made.
///
/// TLS is handled automatically by ureq when built with the `rustls` feature
/// (enabled via `default-features = false, features = ["rustls"]` in Cargo.toml).
/// Plain `http://` URLs (used in tests) bypass TLS entirely.
///
/// # Token safety
/// The bearer token is never logged, included in error chains, or written
/// to disk. If you need to identify the token for diagnostics, use
/// `crate::creds::fingerprint`.
pub fn fetch_usage(token: &str, base_url: &str) -> Result<FetchOutcome, anyhow::Error> {
    // ureq 2.x with `rustls` feature: TLS is automatic for https:// URLs.
    // No manual TLS config is needed or available in this version.
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(5))
        .build();

    let url = format!("{base_url}/api/oauth/usage");
    let user_agent = format!("cc-myasl/{}", env!("CARGO_PKG_VERSION"));

    let result = agent
        .get(&url)
        .set("Authorization", &format!("Bearer {token}"))
        .set("anthropic-beta", "oauth-2025-04-20")
        .set("User-Agent", &user_agent)
        .call();

    match result {
        Ok(resp) => {
            // Status 200
            let body = resp.into_string()?;
            match serde_json::from_str::<UsageResponse>(&body) {
                Ok(usage) => Ok(FetchOutcome::Ok(usage)),
                Err(_) => Ok(FetchOutcome::ServerError),
            }
        }
        Err(ureq::Error::Status(code, resp)) => match code {
            401 => Ok(FetchOutcome::AuthFailed),
            429 => {
                let duration = resp
                    .header("Retry-After")
                    .and_then(retry::parse_retry_after)
                    .unwrap_or(Duration::from_secs(300));
                Ok(FetchOutcome::RateLimited(duration))
            }
            500..=599 => Ok(FetchOutcome::ServerError),
            _ => Ok(FetchOutcome::ServerError),
        },
        Err(ureq::Error::Transport(_)) => Ok(FetchOutcome::TimedOut),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn valid_body() -> &'static str {
        r#"{
            "five_hour": { "utilization": 42.0, "resets_at": "2026-04-26T18:00:00Z" },
            "seven_day": { "utilization": 15.0, "resets_at": "2026-04-30T00:00:00Z" }
        }"#
    }

    #[test]
    fn status_200_valid_body_returns_ok() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(valid_body())
            .create();

        let outcome = fetch_usage("test-token", &server.url()).unwrap();
        match outcome {
            FetchOutcome::Ok(usage) => {
                let five = usage.five_hour.expect("five_hour should be present");
                assert_eq!(five.utilization, Some(42.0));
            }
            _ => panic!("expected FetchOutcome::Ok"),
        }
    }

    #[test]
    fn status_200_empty_body_returns_server_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body("")
            .create();

        let outcome = fetch_usage("test-token", &server.url()).unwrap();
        assert!(
            matches!(outcome, FetchOutcome::ServerError),
            "expected FetchOutcome::ServerError"
        );
    }

    #[test]
    fn status_401_returns_auth_failed() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(401)
            .with_body(r#"{"error": "unauthorized"}"#)
            .create();

        let outcome = fetch_usage("bad-token", &server.url()).unwrap();
        assert!(
            matches!(outcome, FetchOutcome::AuthFailed),
            "expected FetchOutcome::AuthFailed"
        );
    }

    #[test]
    fn status_429_with_retry_after_integer_uses_that_duration() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(429)
            .with_header("Retry-After", "60")
            .with_body(r#"{"error": "rate limited"}"#)
            .create();

        let outcome = fetch_usage("test-token", &server.url()).unwrap();
        match outcome {
            FetchOutcome::RateLimited(d) => assert_eq!(d, Duration::from_secs(60)),
            _ => panic!("expected FetchOutcome::RateLimited(60s)"),
        }
    }

    #[test]
    fn status_429_with_http_date_retry_after_uses_default_300s() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(429)
            .with_header("Retry-After", "Fri, 13 Mar 2026 12:00:00 GMT")
            .with_body(r#"{"error": "rate limited"}"#)
            .create();

        let outcome = fetch_usage("test-token", &server.url()).unwrap();
        match outcome {
            FetchOutcome::RateLimited(d) => assert_eq!(d, Duration::from_secs(300)),
            _ => panic!("expected FetchOutcome::RateLimited(300s default)"),
        }
    }

    #[test]
    fn status_429_with_no_retry_after_uses_default_300s() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(429)
            .with_body(r#"{"error": "rate limited"}"#)
            .create();

        let outcome = fetch_usage("test-token", &server.url()).unwrap();
        match outcome {
            FetchOutcome::RateLimited(d) => assert_eq!(d, Duration::from_secs(300)),
            _ => panic!("expected FetchOutcome::RateLimited(300s default)"),
        }
    }

    #[test]
    fn status_500_returns_server_error() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(500)
            .with_body(r#"{"error": "internal server error"}"#)
            .create();

        let outcome = fetch_usage("test-token", &server.url()).unwrap();
        assert!(
            matches!(outcome, FetchOutcome::ServerError),
            "expected FetchOutcome::ServerError"
        );
    }

    #[test]
    fn connect_refused_returns_timed_out() {
        // Port 1 is reserved and should refuse connections immediately.
        let outcome = fetch_usage("test-token", "http://127.0.0.1:1").unwrap();
        assert!(
            matches!(outcome, FetchOutcome::TimedOut),
            "expected FetchOutcome::TimedOut for refused connection"
        );
    }

    /// Manual-only test: exercises the rustls TLS path against example.com.
    /// Marked `#[ignore]` so it doesn't run in CI.
    #[test]
    #[ignore]
    fn rustls_path_does_not_panic_on_real_https() {
        // We expect a non-200 (the endpoint doesn't exist on example.com),
        // but the important thing is that rustls is wired and we don't panic.
        let outcome = fetch_usage("test-token", "https://example.com");
        assert!(
            outcome.is_ok(),
            "fetch_usage should return Ok(FetchOutcome::*), not Err"
        );
    }
}
