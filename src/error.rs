//! Single error enum covering everything the binary can encounter.
//!
//! All variants are recoverable in render mode — `main.rs` will
//! degrade rather than crash.  `--check` is the only path that
//! surfaces an error to the user.

use std::fmt;

/// Unified error type for the cc-myasl binary.
///
/// Each variant carries a plain `String` message — no nested error
/// chains — so that it is easy to redact sensitive content before
/// surfacing an error to the user or a debug trace.
#[derive(Debug)]
pub enum Error {
    /// Failure to parse the JSON document on stdin.
    StdinParse(String),
    /// Failure to read or parse credentials from Keychain or the
    /// credentials file.
    CredsRead(String),
    /// Network-level transport error (DNS failure, connection refused,
    /// timeout) when calling the OAuth usage endpoint.
    ApiTransport(String),
    /// The API returned HTTP 401 — the token is invalid or expired.
    ApiAuth(String),
    /// The API returned HTTP 429 — the caller is rate-limited.
    ApiRateLimited(String),
    /// Failure to read the on-disk usage cache.
    CacheRead(String),
    /// Failure to write the on-disk usage cache or lock file.
    CacheWrite(String),
    /// Failure during template rendering.
    FormatRender(String),
    /// Failure to read or parse a config file.
    ConfigParse(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::StdinParse(msg) => write!(f, "[StdinParse] {msg}"),
            Error::CredsRead(msg) => write!(f, "[CredsRead] {msg}"),
            Error::ApiTransport(msg) => write!(f, "[ApiTransport] {msg}"),
            Error::ApiAuth(msg) => write!(f, "[ApiAuth] {msg}"),
            Error::ApiRateLimited(msg) => write!(f, "[ApiRateLimited] {msg}"),
            Error::CacheRead(msg) => write!(f, "[CacheRead] {msg}"),
            Error::CacheWrite(msg) => write!(f, "[CacheWrite] {msg}"),
            Error::FormatRender(msg) => write!(f, "[FormatRender] {msg}"),
            Error::ConfigParse(msg) => write!(f, "[ConfigParse] {msg}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    /// Maps an `std::io::Error` to `CacheRead` by default.
    ///
    /// This is an intentionally broad mapping — callers that know the
    /// I/O was a *write* should construct `Error::CacheWrite` directly
    /// rather than relying on this `From` impl.
    fn from(e: std::io::Error) -> Self {
        Error::CacheRead(e.to_string())
    }
}

impl From<serde_json::Error> for Error {
    /// JSON parse errors are always treated as stdin parse failures.
    fn from(e: serde_json::Error) -> Self {
        Error::StdinParse(e.to_string())
    }
}

impl From<anyhow::Error> for Error {
    /// Generic boundary conversion — wraps the `anyhow` message in
    /// `CacheRead` since the most common anyhow source in this codebase
    /// is I/O from the credentials or cache layer.  Callers that need
    /// a more specific variant should construct it directly.
    fn from(e: anyhow::Error) -> Self {
        Error::CacheRead(e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::ErrorKind;

    #[test]
    fn display_stdin_parse() {
        let e = Error::StdinParse("bad json".into());
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(s.contains("StdinParse"), "expected variant tag: {s}");
        assert!(s.contains("bad json"), "expected message: {s}");
    }

    #[test]
    fn display_creds_read() {
        let e = Error::CredsRead("no keychain".into());
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(s.contains("CredsRead"), "expected variant tag: {s}");
    }

    #[test]
    fn display_api_transport() {
        let e = Error::ApiTransport("connection refused".into());
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(s.contains("ApiTransport"), "expected variant tag: {s}");
    }

    #[test]
    fn display_api_auth() {
        let e = Error::ApiAuth("401 Unauthorized".into());
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(s.contains("ApiAuth"), "expected variant tag: {s}");
    }

    #[test]
    fn display_api_rate_limited() {
        let e = Error::ApiRateLimited("retry after 300s".into());
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(s.contains("ApiRateLimited"), "expected variant tag: {s}");
    }

    #[test]
    fn display_cache_read() {
        let e = Error::CacheRead("file not found".into());
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(s.contains("CacheRead"), "expected variant tag: {s}");
    }

    #[test]
    fn display_cache_write() {
        let e = Error::CacheWrite("permission denied".into());
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(s.contains("CacheWrite"), "expected variant tag: {s}");
    }

    #[test]
    fn display_format_render() {
        let e = Error::FormatRender("unknown placeholder".into());
        let s = e.to_string();
        assert!(!s.is_empty());
        assert!(s.contains("FormatRender"), "expected variant tag: {s}");
    }

    #[test]
    fn from_io_error_produces_cache_read() {
        let io_err = std::io::Error::from(ErrorKind::NotFound);
        let e = Error::from(io_err);
        assert!(
            matches!(e, Error::CacheRead(_)),
            "io::Error should map to CacheRead, got: {e}"
        );
        let s = e.to_string();
        assert!(!s.is_empty(), "display must be non-empty");
    }

    #[test]
    fn from_serde_json_error_produces_stdin_parse() {
        let json_err = serde_json::from_str::<u32>("not-json").unwrap_err();
        let e = Error::from(json_err);
        assert!(
            matches!(e, Error::StdinParse(_)),
            "serde_json::Error should map to StdinParse, got: {e}"
        );
        let s = e.to_string();
        assert!(!s.is_empty(), "display must be non-empty");
    }

    #[test]
    fn from_anyhow_error_display_contains_message() {
        let anyhow_err = anyhow::anyhow!("boom");
        let e = Error::from(anyhow_err);
        let s = e.to_string();
        assert!(
            s.contains("boom"),
            "Display of anyhow-derived Error must contain original message: {s}"
        );
        assert!(!s.is_empty(), "display must be non-empty");
    }

    #[test]
    fn all_variant_displays_are_non_empty() {
        let variants: &[Error] = &[
            Error::StdinParse("x".into()),
            Error::CredsRead("x".into()),
            Error::ApiTransport("x".into()),
            Error::ApiAuth("x".into()),
            Error::ApiRateLimited("x".into()),
            Error::CacheRead("x".into()),
            Error::CacheWrite("x".into()),
            Error::FormatRender("x".into()),
        ];
        for v in variants {
            let s = v.to_string();
            assert!(!s.is_empty(), "Display must be non-empty for {v:?}");
        }
    }
}
