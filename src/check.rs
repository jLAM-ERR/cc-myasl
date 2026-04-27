//! `--check` setup verification command.
//!
//! Prints a human-readable diagnostic to stdout in 4 sections.
//! Exits non-zero if any section fails.  This is the ONE place
//! the binary may exit non-zero (Hard Invariant #3).

use std::io::Write;
use std::path::Path;
use std::time::Instant;

use crate::api::{self, FetchOutcome};
use crate::cache;
use crate::creds;
use crate::format::{self, RenderCtx};

// ── shared constants from sibling modules ─────────────────────────────────────

use crate::api::DEFAULT_OAUTH_BASE_URL;
use crate::format::DEFAULT_TEMPLATE;

// ── report ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct CheckReport {
    pub creds_ok: bool,
    pub network_ok: bool,
    pub cache_ok: bool,
    pub format_ok: bool,
}

impl CheckReport {
    pub fn all_ok(&self) -> bool {
        self.creds_ok && self.network_ok && self.cache_ok && self.format_ok
    }
}

// ── public entry points ───────────────────────────────────────────────────────

/// Run the full diagnostic, printing sections to stdout.
/// Returns 0 if every section passed; 1 otherwise.
pub fn run() -> i32 {
    let report = run_inner();
    if report.all_ok() {
        0
    } else {
        1
    }
}

/// Orchestrate the four section checks and return a `CheckReport`.
/// Prints each section's result to stdout.
pub fn run_inner() -> CheckReport {
    let mut report = CheckReport::default();
    let mut out = std::io::stdout();

    // 1. Credentials
    let cred_result = creds::read_token();
    let token = report_credentials(&mut report, cred_result, &mut out);

    // 2. Network (skip if no creds)
    let base_url = std::env::var("STATUSLINE_OAUTH_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_OAUTH_BASE_URL.to_string());
    check_network(&mut report, token.as_deref(), &base_url);

    // 3. Cache
    let cache_dir = cache::cache_dir();
    check_cache(&mut report, &cache_dir);

    // 4. Format
    check_format(&mut report, DEFAULT_TEMPLATE);

    report
}

// ── section helpers ───────────────────────────────────────────────────────────

/// Report the credentials result; write result to `out`. Returns the token if successful.
///
/// The token read is injected (via `cred_result`) to keep this function testable
/// without touching the HOME env var (which races with creds::tests on macOS Keychain).
///
/// Error messages have the home-directory path tilde-collapsed so that `--check`
/// output shared in bug reports does not reveal the user's `$HOME`.
fn report_credentials(
    report: &mut CheckReport,
    cred_result: Result<String, anyhow::Error>,
    out: &mut dyn Write,
) -> Option<String> {
    match cred_result {
        Ok(token) => {
            let fp = creds::fingerprint(&token);
            let _ = writeln!(out, "Credentials: ✓ found (fingerprint: {fp})");
            report.creds_ok = true;
            Some(token)
        }
        Err(e) => {
            let sanitized = creds::redact_home(&e.to_string());
            let _ = writeln!(out, "Credentials: ✗ {sanitized}");
            let _ = writeln!(
                out,
                "  → ensure you've signed in via Claude Code or that \
                 ~/.claude/.credentials.json exists"
            );
            report.creds_ok = false;
            None
        }
    }
}

/// Check network; print result. Skips if `token` is None.
fn check_network(report: &mut CheckReport, token: Option<&str>, base_url: &str) {
    let Some(token) = token else {
        println!("Network: ✗ skipped (no credentials)");
        report.network_ok = false;
        return;
    };

    let start = Instant::now();
    match api::fetch_usage(token, base_url) {
        Ok(FetchOutcome::Ok(_)) => {
            let ms = start.elapsed().as_millis();
            println!("Network: ✓ HTTP 200 in {ms}ms");
            report.network_ok = true;
        }
        Ok(FetchOutcome::AuthFailed) => {
            println!("Network: ✗ HTTP 401 (auth failed — token may be invalid or expired)");
            report.network_ok = false;
        }
        Ok(FetchOutcome::RateLimited(d)) => {
            println!(
                "Network: ✗ HTTP 429 (rate limited — retry after {}s)",
                d.as_secs()
            );
            report.network_ok = false;
        }
        Ok(FetchOutcome::ServerError) => {
            println!("Network: ✗ server error (5xx)");
            report.network_ok = false;
        }
        Ok(FetchOutcome::TimedOut) => {
            println!("Network: ✗ timed out");
            report.network_ok = false;
        }
        Err(e) => {
            println!("Network: ✗ transport error: {e}");
            report.network_ok = false;
        }
    }
}

/// Collapse the home-directory prefix of `path` to `~` for display.
///
/// Returns the tilde-collapsed string if `$HOME` is set and the path starts
/// with it; otherwise returns the full path string unchanged.
fn display_tilde(path: &Path) -> String {
    creds::redact_home(&path.display().to_string())
}

/// Check cache; print result.
fn check_cache(report: &mut CheckReport, dir: &Path) {
    let path = dir.join("usage.json");
    if !path.exists() {
        println!("Cache: ✓ no cache yet (will be created on first fetch)");
        report.cache_ok = true;
        return;
    }

    match cache::read(dir) {
        Some(c) => {
            let now = crate::time::now_unix();
            let freshness = if cache::is_fresh(&c, 180, now) {
                "fresh"
            } else {
                "stale"
            };
            println!("Cache: ✓ {} ({freshness})", display_tilde(&path));
            report.cache_ok = true;
        }
        None => {
            println!(
                "Cache: ✗ {} exists but could not be parsed (corrupt?)",
                display_tilde(&path)
            );
            report.cache_ok = false;
        }
    }
}

/// Check format; print result.
fn check_format(report: &mut CheckReport, template: &str) {
    let ctx = RenderCtx {
        model: Some("claude-opus-4".to_string()),
        five_used: Some(30.0),
        five_reset_unix: Some(crate::time::now_unix() + 3600),
        seven_used: Some(60.0),
        seven_reset_unix: Some(crate::time::now_unix() + 86400),
        now_unix: crate::time::now_unix(),
        ..Default::default()
    };

    let result = std::panic::catch_unwind(|| format::render(template, &ctx));
    match result {
        Ok(out) if !out.is_empty() => {
            println!("Format: ✓ default template renders");
            report.format_ok = true;
        }
        Ok(_) => {
            println!("Format: ✗ render returned empty string");
            report.format_ok = false;
        }
        Err(_) => {
            println!("Format: ✗ render panicked");
            report.format_ok = false;
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use tempfile::TempDir;

    // Serialise tests that mutate env vars so they don't race.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // ── CheckReport::all_ok ───────────────────────────────────────────────────

    #[test]
    fn all_ok_true_when_all_sections_pass() {
        let r = CheckReport {
            creds_ok: true,
            network_ok: true,
            cache_ok: true,
            format_ok: true,
        };
        assert!(r.all_ok());
    }

    #[test]
    fn all_ok_false_when_any_section_fails() {
        let cases = [
            CheckReport {
                creds_ok: false,
                network_ok: true,
                cache_ok: true,
                format_ok: true,
            },
            CheckReport {
                creds_ok: true,
                network_ok: false,
                cache_ok: true,
                format_ok: true,
            },
            CheckReport {
                creds_ok: true,
                network_ok: true,
                cache_ok: false,
                format_ok: true,
            },
            CheckReport {
                creds_ok: true,
                network_ok: true,
                cache_ok: true,
                format_ok: false,
            },
        ];
        for r in &cases {
            assert!(!r.all_ok(), "expected all_ok=false for {r:?}");
        }
    }

    // ── credentials section (injected — no HOME mutation) ─────────────────────

    #[test]
    fn creds_section_error_result_fails() {
        let mut report = CheckReport::default();
        let mut buf: Vec<u8> = Vec::new();
        let token = report_credentials(
            &mut report,
            Err(anyhow::anyhow!("credentials file not found")),
            &mut buf,
        );
        assert!(!report.creds_ok, "error result should set creds_ok=false");
        assert!(token.is_none(), "no token when creds missing");
    }

    #[test]
    fn creds_section_ok_result_passes() {
        let mut report = CheckReport::default();
        let mut buf: Vec<u8> = Vec::new();
        let token = report_credentials(
            &mut report,
            Ok("sk-ant-test-check-12345".to_string()),
            &mut buf,
        );
        assert!(report.creds_ok, "ok result should set creds_ok=true");
        assert_eq!(
            token.as_deref(),
            Some("sk-ant-test-check-12345"),
            "token must be returned on success"
        );
    }

    /// Fingerprint (not raw token) goes to output; home path tilde-collapsed on error.
    #[test]
    fn creds_section_writer_output_invariants() {
        // Success: raw token must not appear; fingerprint marker must appear.
        let token = "sk-ant-secret-token-9999";
        let mut report = CheckReport::default();
        let mut buf: Vec<u8> = Vec::new();
        report_credentials(&mut report, Ok(token.to_string()), &mut buf);
        let out = std::str::from_utf8(&buf).unwrap();
        assert!(!out.contains(token), "raw token must not appear: {out:?}");
        assert!(
            out.contains("fingerprint"),
            "fingerprint must appear: {out:?}"
        );

        // Error: home path tilde-collapsed.
        let _guard = creds::HOME_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("HOME", "/home/alice");
        let mut report2 = CheckReport::default();
        let mut buf2: Vec<u8> = Vec::new();
        let raw_err = "credentials file not found at /home/alice/.claude/.credentials.json";
        report_credentials(&mut report2, Err(anyhow::anyhow!("{}", raw_err)), &mut buf2);
        std::env::remove_var("HOME");
        let out2 = std::str::from_utf8(&buf2).unwrap();
        assert!(
            !out2.contains("/home/alice"),
            "expanded HOME must not appear: {out2:?}"
        );
        assert!(
            out2.contains("~/.claude"),
            "tilde path must appear: {out2:?}"
        );
    }

    // ── format section ────────────────────────────────────────────────────────

    #[test]
    fn format_section_default_ok_empty_fails() {
        let mut r1 = CheckReport::default();
        check_format(&mut r1, DEFAULT_TEMPLATE);
        assert!(r1.format_ok, "default template should render non-empty");
        let mut r2 = CheckReport::default();
        check_format(&mut r2, "");
        assert!(!r2.format_ok, "empty template should fail");
    }

    #[test]
    fn format_section_stub_ctx_renders_segments() {
        let ctx = RenderCtx {
            model: Some("claude-opus-4".to_string()),
            five_used: Some(30.0),
            five_reset_unix: Some(9_999_999_999),
            seven_used: Some(60.0),
            seven_reset_unix: Some(9_999_999_999),
            now_unix: 0,
            ..Default::default()
        };
        let out = format::render(DEFAULT_TEMPLATE, &ctx);
        assert!(out.contains("5h:"), "should contain 5h: segment");
    }

    // ── cache section ─────────────────────────────────────────────────────────

    #[test]
    fn cache_section_no_file_is_ok() {
        let dir = TempDir::new().unwrap();
        let mut r1 = CheckReport::default();
        check_cache(&mut r1, &dir.path().join("subdir")); // subdir absent
        assert!(r1.cache_ok, "missing dir → ok");
        let mut r2 = CheckReport::default();
        check_cache(&mut r2, dir.path()); // dir exists, no usage.json
        assert!(r2.cache_ok, "missing file → ok");
    }

    #[test]
    fn cache_section_valid_file_ok() {
        let dir = TempDir::new().unwrap();
        let c = cache::UsageCache {
            fetched_at: crate::time::now_unix(),
            ..Default::default()
        };
        cache::write(dir.path(), &c).unwrap();
        let mut report = CheckReport::default();
        check_cache(&mut report, dir.path());
        assert!(report.cache_ok, "valid cache file should be ok");
    }

    #[test]
    fn cache_section_corrupt_file_fails() {
        let dir = TempDir::new().unwrap();
        std::fs::write(dir.path().join("usage.json"), b"{ not json }").unwrap();
        let mut report = CheckReport::default();
        check_cache(&mut report, dir.path());
        assert!(!report.cache_ok, "corrupt cache file should fail");
    }

    // ── network section (mockito) ─────────────────────────────────────────────

    #[test]
    fn network_section_200_ok() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(200)
            .with_header("Content-Type", "application/json")
            .with_body(r#"{"five_hour":{"utilization":0.3},"seven_day":{"utilization":0.6}}"#)
            .create();

        let mut report = CheckReport::default();
        check_network(&mut report, Some("test-token"), &server.url());
        assert!(report.network_ok, "HTTP 200 should set network_ok=true");
    }

    #[test]
    fn network_section_401_fails() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(401)
            .with_body(r#"{"error":"unauthorized"}"#)
            .create();

        let mut report = CheckReport::default();
        check_network(&mut report, Some("bad-token"), &server.url());
        assert!(!report.network_ok, "HTTP 401 should set network_ok=false");
    }

    #[test]
    fn network_section_429_fails() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(429)
            .with_header("Retry-After", "60")
            .with_body(r#"{"error":"rate limited"}"#)
            .create();

        let mut report = CheckReport::default();
        check_network(&mut report, Some("test-token"), &server.url());
        assert!(!report.network_ok, "HTTP 429 should set network_ok=false");
    }

    #[test]
    fn network_section_500_fails() {
        let mut server = mockito::Server::new();
        let _mock = server
            .mock("GET", "/api/oauth/usage")
            .with_status(500)
            .with_body(r#"{"error":"internal server error"}"#)
            .create();

        let mut report = CheckReport::default();
        check_network(&mut report, Some("test-token"), &server.url());
        assert!(!report.network_ok, "HTTP 500 should set network_ok=false");
    }

    #[test]
    fn network_section_timeout_and_no_creds_fail() {
        let mut r1 = CheckReport::default();
        check_network(&mut r1, Some("test-token"), "http://127.0.0.1:1");
        assert!(!r1.network_ok, "connection refused → network_ok=false");
        let mut r2 = CheckReport::default();
        check_network(&mut r2, None, DEFAULT_OAUTH_BASE_URL);
        assert!(!r2.network_ok, "no creds → network_ok=false");
    }

    // ── display_tilde / cache path redaction (CONCERN 3 regression guard) ───────

    #[test]
    fn display_tilde_redaction() {
        let _guard = creds::HOME_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        std::env::set_var("HOME", "/home/alice");
        let home_path = std::path::PathBuf::from("/home/alice/Library/Caches/cc-myasl/usage.json");
        let other_path = std::path::PathBuf::from("/tmp/usage.json");
        let home_result = display_tilde(&home_path);
        let other_result = display_tilde(&other_path);
        std::env::remove_var("HOME");
        assert_eq!(home_result, "~/Library/Caches/cc-myasl/usage.json");
        assert_eq!(other_result, "/tmp/usage.json");
    }

    // ── integration ───────────────────────────────────────────────────────────

    #[test]
    fn run_inner_format_always_passes() {
        let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
        // Refused port → network fails fast; format check uses a stub ctx.
        std::env::set_var("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1");
        let report = run_inner();
        std::env::remove_var("STATUSLINE_OAUTH_BASE_URL");
        assert!(report.format_ok, "format should always pass");
    }
}
