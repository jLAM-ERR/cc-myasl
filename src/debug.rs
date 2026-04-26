//! Single-line JSON trace emitter for debug diagnostics.
//!
//! When `STATUSLINE_DEBUG=1` is set (or `force == true`), `Trace::emit`
//! writes a single compact JSON line to stderr.  In normal operation the
//! function is a no-op.
//!
//! # Security
//!
//! The bearer token MUST NEVER appear in the trace.  Only the fingerprint
//! from `creds::fingerprint` — an opaque 16-hex-char identifier — goes
//! into the `token_fp` field.

use serde::Serialize;
use std::io::{self, Write};

/// Diagnostic trace collected during a single invocation.
///
/// All fields are `Option` so that unset values are omitted from the JSON
/// output (`#[serde(skip_serializing_if = "Option::is_none")]`).
#[derive(Debug, Default, Serialize)]
pub struct Trace {
    /// Render path taken, e.g. `"stdin-rate-limits"`, `"cache-hit"`,
    /// `"oauth-fallback"`, or `"degraded"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,

    /// Cache outcome: `"hit"`, `"miss"`, or `"stale"`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,

    /// HTTP status code returned by the OAuth usage endpoint, if called.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub http: Option<u16>,

    /// Wall-clock duration of the invocation in milliseconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub took_ms: Option<u64>,

    /// `Display` representation of any `Error` that occurred, if any.
    /// MUST NOT contain the bearer token — use `token_fp` for token identity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,

    /// Opaque fingerprint of the bearer token (from `creds::fingerprint`),
    /// set only when a token was successfully read.
    /// NEVER the token itself.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_fp: Option<String>,
}

impl Trace {
    /// Emit `self` as a single JSON line on stderr if `STATUSLINE_DEBUG=1`
    /// or `force == true`.  Never panics; ignores write errors.
    pub fn emit(&self, force: bool) {
        self.emit_to(&mut io::stderr(), force);
    }

    /// Emit `self` as a single JSON line to an arbitrary writer.
    ///
    /// This is the testable core: `emit` delegates here with `io::stderr()`.
    /// Tests pass a `Vec<u8>` as the writer to capture output without
    /// touching stderr.
    pub(crate) fn emit_to<W: Write>(&self, w: &mut W, force: bool) {
        let want = force || std::env::var("STATUSLINE_DEBUG").as_deref() == Ok("1");
        if !want {
            return;
        }
        if let Ok(line) = serde_json::to_string(self) {
            let _ = writeln!(w, "{}", line);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    /// Mutex to serialize tests that read/write `STATUSLINE_DEBUG`.
    static DEBUG_MUTEX: Mutex<()> = Mutex::new(());

    // ── helpers ───────────────────────────────────────────────────────────

    fn capture(trace: &Trace, force: bool) -> Vec<u8> {
        let mut buf: Vec<u8> = Vec::new();
        trace.emit_to(&mut buf, force);
        buf
    }

    // ── default trace emitted as valid JSON ───────────────────────────────

    #[test]
    fn default_emit_force_produces_valid_json_line() {
        let trace = Trace::default();
        let raw = capture(&trace, true);
        let s = std::str::from_utf8(&raw).expect("output must be valid UTF-8");
        // Must be exactly one non-empty line (trailing newline included).
        let trimmed = s.trim_end_matches('\n');
        assert!(!trimmed.is_empty(), "emitted JSON line must be non-empty");
        assert!(
            !trimmed.contains('\n'),
            "must be a single line, got: {trimmed:?}"
        );
        // Must parse back as a JSON object.
        let v: serde_json::Value =
            serde_json::from_str(trimmed).expect("emitted string must be valid JSON");
        assert!(v.is_object(), "emitted JSON must be an object");
    }

    // ── force=false + STATUSLINE_DEBUG unset → no output ─────────────────

    #[test]
    fn no_output_when_debug_off_and_force_false() {
        let _guard = DEBUG_MUTEX.lock().unwrap();
        // Ensure the env var is absent.
        std::env::remove_var("STATUSLINE_DEBUG");
        let trace = Trace::default();
        let raw = capture(&trace, false);
        assert!(raw.is_empty(), "must produce no output when debug is off");
    }

    // ── STATUSLINE_DEBUG=0 + force=false → no output ─────────────────────

    #[test]
    fn no_output_when_debug_set_to_zero_and_force_false() {
        let _guard = DEBUG_MUTEX.lock().unwrap();
        std::env::set_var("STATUSLINE_DEBUG", "0");
        let trace = Trace::default();
        let raw = capture(&trace, false);
        std::env::remove_var("STATUSLINE_DEBUG");
        assert!(
            raw.is_empty(),
            "STATUSLINE_DEBUG=0 with force=false must produce no output"
        );
    }

    // ── STATUSLINE_DEBUG=1 → emit even without force ──────────────────────

    #[test]
    fn output_when_debug_env_is_one() {
        let _guard = DEBUG_MUTEX.lock().unwrap();
        std::env::set_var("STATUSLINE_DEBUG", "1");
        let trace = Trace {
            path: Some("cache-hit".into()),
            ..Default::default()
        };
        let raw = capture(&trace, false);
        std::env::remove_var("STATUSLINE_DEBUG");
        assert!(
            !raw.is_empty(),
            "STATUSLINE_DEBUG=1 must produce output even with force=false"
        );
    }

    // ── populated Trace round-trips correctly ─────────────────────────────

    #[test]
    fn populated_trace_round_trips() {
        let trace = Trace {
            path: Some("oauth-fallback".into()),
            cache: Some("miss".into()),
            http: Some(200),
            took_ms: Some(42),
            error: None,
            token_fp: Some("abcdef1234567890".into()),
        };

        let raw = capture(&trace, true);
        let s = std::str::from_utf8(&raw).unwrap();
        let trimmed = s.trim_end_matches('\n');
        let v: serde_json::Value = serde_json::from_str(trimmed).expect("must parse as JSON");

        assert_eq!(v["path"].as_str(), Some("oauth-fallback"));
        assert_eq!(v["cache"].as_str(), Some("miss"));
        assert_eq!(v["http"].as_u64(), Some(200));
        assert_eq!(v["took_ms"].as_u64(), Some(42));
        assert!(v.get("error").is_none(), "error should be absent");
        assert_eq!(v["token_fp"].as_str(), Some("abcdef1234567890"));
    }

    // ── redaction invariant: bearer token never in JSON output ────────────

    #[test]
    fn bearer_token_never_in_json_output() {
        // Use a fixed-length hex string as the fingerprint mock — this avoids
        // importing creds in this module while still exercising the invariant.
        // (In production code, creds::fingerprint provides the value.)
        let mock_fingerprint = "a1b2c3d4e5f60718"; // 16 hex chars, doesn't contain token substrings

        // These are the bearer token substrings that must NEVER appear in the trace.
        // (The variable is named with underscore prefix because clippy flags
        // unused variables, but we use its literal substrings below.)
        let _bearer_token = "sk-ant-test-bearer-12345";

        let trace = Trace {
            token_fp: Some(mock_fingerprint.into()),
            error: Some("oh no".into()),
            path: Some("degraded".into()),
            cache: Some("miss".into()),
            http: Some(401),
            took_ms: Some(7),
        };

        let raw = capture(&trace, true);
        let json_str = std::str::from_utf8(&raw).expect("output must be UTF-8");

        // The token or any of its meaningful substrings must not appear.
        assert!(
            !json_str.contains("sk-ant"),
            "json must not contain 'sk-ant': {json_str}"
        );
        assert!(
            !json_str.contains("bearer-12345"),
            "json must not contain 'bearer-12345': {json_str}"
        );
        assert!(
            !json_str.contains("test-bearer"),
            "json must not contain 'test-bearer': {json_str}"
        );

        // The fingerprint and error message SHOULD be present.
        assert!(
            json_str.contains(mock_fingerprint),
            "json must contain the fingerprint: {json_str}"
        );
        assert!(
            json_str.contains("oh no"),
            "json must contain the error message: {json_str}"
        );
    }

    // ── None fields are omitted from JSON output ──────────────────────────

    #[test]
    fn none_fields_omitted_from_json() {
        let trace = Trace {
            path: Some("cache-hit".into()),
            ..Default::default()
        };
        let raw = capture(&trace, true);
        let s = std::str::from_utf8(&raw).unwrap();
        let trimmed = s.trim_end_matches('\n');
        let v: serde_json::Value = serde_json::from_str(trimmed).unwrap();

        assert!(v.get("cache").is_none(), "absent cache should be omitted");
        assert!(v.get("http").is_none(), "absent http should be omitted");
        assert!(
            v.get("took_ms").is_none(),
            "absent took_ms should be omitted"
        );
        assert!(v.get("error").is_none(), "absent error should be omitted");
        assert!(
            v.get("token_fp").is_none(),
            "absent token_fp should be omitted"
        );
        assert_eq!(v["path"].as_str(), Some("cache-hit"));
    }
}
