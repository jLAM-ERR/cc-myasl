//! End-to-end integration tests.
//!
//! Each test spawns the release binary, pipes a fixture stdin,
//! optionally configures a `mockito` server for the OAuth endpoint
//! (via STATUSLINE_OAUTH_BASE_URL), and asserts structural
//! properties of stdout.

use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};

// ── helpers ───────────────────────────────────────────────────────────────────

fn fixture(name: &str) -> String {
    let path = format!("tests/fixtures/{}.json", name);
    fs::read_to_string(&path).unwrap_or_else(|_| panic!("fixture not found: {}", path))
}

fn bin() -> Command {
    Command::cargo_bin("claude-statusline").expect("binary must build")
}

/// Return the cache dir the binary will use given `home`.
/// On macOS: `<home>/Library/Caches/ai.claude-statusline.claude-statusline`
/// On Linux: `<home>/.cache/ai.claude-statusline.claude-statusline`
fn cache_dir_for_home(home: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Caches")
            .join("ai.claude-statusline.claude-statusline")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".cache")
            .join("ai.claude-statusline.claude-statusline")
    }
}

/// Write a minimal credentials file so the binary can read a token.
fn write_creds(home: &Path, token: &str) {
    let claude_dir = home.join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let creds_json = format!(r#"{{"claudeAiOauth":{{"accessToken":"{}"}}}}"#, token);
    fs::write(claude_dir.join(".credentials.json"), creds_json).unwrap();
}

// ── test 1: pro_max hot path — byte-exact snapshot canary ────────────────────

/// Byte-exact snapshot test on the hot path (rate_limits in stdin).
/// Expected to break on intentional formatter changes and is updated by hand.
/// Port 1 is not-listening; if any HTTP call escaped the binary would time out
/// in ~5 s — the test completing in < 1 s IS the proof that no call was made.
#[test]
fn pro_max_hot_path_renders_quota() {
    let started = std::time::Instant::now();

    bin()
        .args(["--format", "{model} · 5h: {five_left}% · 7d: {seven_left}%"])
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(fixture("pro_max_with_rate_limits"))
        .assert()
        .success()
        .stdout("claude-opus-4-7 · 5h: 76% · 7d: 59%\n");

    assert!(
        started.elapsed().as_millis() < 1000,
        "hot path took > 1 s — HTTP call may have escaped"
    );
}

// ── test 2: default template structural regex ─────────────────────────────────

/// Default template output: structural regex, TZ-independent.
#[test]
fn pro_max_hot_path_default_template_structural() {
    bin()
        .env("STATUSLINE_OAUTH_BASE_URL", "http://invalid-url-no-connect")
        .env("TZ", "UTC")
        .write_stdin(fixture("pro_max_with_rate_limits"))
        .assert()
        .success()
        .stdout(
            predicates::str::is_match(
                r"^claude-opus-4-7 · 5h: 76% · 7d: 59% \(resets \d{2}:\d{2}\)\n$",
            )
            .unwrap(),
        );
}

// ── test 3: api_key path — OAuth 200 renders quota ───────────────────────────

#[test]
fn api_key_oauth_200_renders_quota() {
    let home = tempfile::tempdir().unwrap();
    write_creds(home.path(), "fixture-test-token");

    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/api/oauth/usage")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{"five_hour":{"utilization":25.0,"resets_at":"2026-04-26T18:00:00Z"},
               "seven_day":{"utilization":40.0,"resets_at":"2026-05-03T00:00:00Z"}}"#,
        )
        .create();

    bin()
        .args(["--format", "{five_left}/{seven_left}"])
        .env("STATUSLINE_OAUTH_BASE_URL", server.url())
        .env("HOME", home.path())
        // On Linux, `directories::ProjectDirs` honours `$XDG_CACHE_HOME`
        // BEFORE falling back to `$HOME/.cache`. CI runners export
        // XDG_CACHE_HOME (e.g. `/home/runner/.cache`) which would route the
        // binary's cache writes outside our tempdir even when HOME is set.
        // Pin XDG_CACHE_HOME to inside the tempdir so both code paths agree.
        // (No-op on macOS where ProjectDirs uses Library/Caches regardless.)
        .env("XDG_CACHE_HOME", home.path().join(".cache"))
        .write_stdin(fixture("api_key_no_rate_limits"))
        .assert()
        .success()
        .stdout("75/60\n");

    mock.assert();
}

// ── test 4: api_key path — OAuth 401 drops quota segment ─────────────────────

#[test]
fn api_key_oauth_401_drops_quota_segment() {
    let home = tempfile::tempdir().unwrap();
    write_creds(home.path(), "fixture-test-token");

    let mut server = mockito::Server::new();
    server
        .mock("GET", "/api/oauth/usage")
        .with_status(401)
        .create();

    bin()
        .args(["--format", "{model}{? · 5h:{five_left}%}"])
        .env("STATUSLINE_OAUTH_BASE_URL", server.url())
        .env("HOME", home.path())
        // On Linux, `directories::ProjectDirs` honours `$XDG_CACHE_HOME`
        // BEFORE falling back to `$HOME/.cache`. CI runners export
        // XDG_CACHE_HOME (e.g. `/home/runner/.cache`) which would route the
        // binary's cache writes outside our tempdir even when HOME is set.
        // Pin XDG_CACHE_HOME to inside the tempdir so both code paths agree.
        // (No-op on macOS where ProjectDirs uses Library/Caches regardless.)
        .env("XDG_CACHE_HOME", home.path().join(".cache"))
        .write_stdin(fixture("api_key_no_rate_limits"))
        .assert()
        .success()
        .stdout("claude-sonnet-4-6\n");
}

// ── test 5: api_key path — OAuth 429 writes lock file ────────────────────────

#[test]
fn api_key_oauth_429_writes_lock() {
    let home = tempfile::tempdir().unwrap();
    write_creds(home.path(), "fixture-test-token");
    let cache_dir = cache_dir_for_home(home.path());
    fs::create_dir_all(&cache_dir).unwrap();

    let mut server = mockito::Server::new();
    server
        .mock("GET", "/api/oauth/usage")
        .with_status(429)
        .with_header("Retry-After", "600")
        .create();

    let now_before = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    bin()
        .args(["--format", "{model}"])
        .env("STATUSLINE_OAUTH_BASE_URL", server.url())
        .env("HOME", home.path())
        // On Linux, `directories::ProjectDirs` honours `$XDG_CACHE_HOME`
        // BEFORE falling back to `$HOME/.cache`. CI runners export
        // XDG_CACHE_HOME (e.g. `/home/runner/.cache`) which would route the
        // binary's cache writes outside our tempdir even when HOME is set.
        // Pin XDG_CACHE_HOME to inside the tempdir so both code paths agree.
        // (No-op on macOS where ProjectDirs uses Library/Caches regardless.)
        .env("XDG_CACHE_HOME", home.path().join(".cache"))
        .write_stdin(fixture("api_key_no_rate_limits"))
        .assert()
        .success()
        .stdout("claude-sonnet-4-6\n");

    // Verify lock file exists and blocked_until > now + 590.
    let lock_path = cache_dir.join("usage.lock");
    assert!(lock_path.exists(), "lock file should have been written");

    let lock_json = fs::read_to_string(&lock_path).unwrap();
    let lock: serde_json::Value = serde_json::from_str(&lock_json).unwrap();
    let blocked_until = lock["blocked_until"]
        .as_u64()
        .expect("blocked_until must be u64");
    assert!(
        blocked_until > now_before + 590,
        "blocked_until ({}) should be > now + 590 ({})",
        blocked_until,
        now_before + 590
    );
}

// ── test 6: extra_usage renders when enabled ──────────────────────────────────

#[test]
fn extra_usage_renders_when_enabled() {
    let home = tempfile::tempdir().unwrap();
    write_creds(home.path(), "fixture-test-token");

    let mut server = mockito::Server::new();
    server
        .mock("GET", "/api/oauth/usage")
        .with_status(200)
        .with_header("content-type", "application/json")
        .with_body(
            r#"{
               "five_hour":{"utilization":100.0,"resets_at":"2026-04-26T18:00:00Z"},
               "seven_day":{"utilization":100.0,"resets_at":"2026-05-03T00:00:00Z"},
               "extra_usage":{"is_enabled":true,"used_credits":25.0,"monthly_limit":100.0,"utilization":25.0}
            }"#,
        )
        .create();

    bin()
        .args(["--format", "{model}{? extra:{extra_left}}"])
        .env("STATUSLINE_OAUTH_BASE_URL", server.url())
        .env("HOME", home.path())
        // On Linux, `directories::ProjectDirs` honours `$XDG_CACHE_HOME`
        // BEFORE falling back to `$HOME/.cache`. CI runners export
        // XDG_CACHE_HOME (e.g. `/home/runner/.cache`) which would route the
        // binary's cache writes outside our tempdir even when HOME is set.
        // Pin XDG_CACHE_HOME to inside the tempdir so both code paths agree.
        // (No-op on macOS where ProjectDirs uses Library/Caches regardless.)
        .env("XDG_CACHE_HOME", home.path().join(".cache"))
        .write_stdin(fixture("extra_usage_enabled"))
        .assert()
        .success()
        .stdout(predicates::str::is_match(r"^claude-opus-4-7 extra:75\n$").unwrap());
}

// ── test 7: malformed payload — graceful degrade ──────────────────────────────

/// Malformed stdin types (display_name is a number, workspace is null, etc.)
/// The parser should fail gracefully; main must still exit 0 with non-empty output.
#[test]
fn malformed_payload_degrades_to_exit_zero() {
    bin()
        .args(["--format", "{model}"])
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(fixture("malformed_field"))
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// ── test 8: fixture hygiene — no real bearer tokens ──────────────────────────

/// Returns the longest run of consecutive alphanumeric chars in `s`.
fn longest_alnum_run(s: &str) -> usize {
    let mut max = 0usize;
    let mut cur = 0usize;
    for c in s.chars() {
        if c.is_ascii_alphanumeric() {
            cur += 1;
            if cur > max {
                max = cur;
            }
        } else {
            cur = 0;
        }
    }
    max
}

/// Asserts that no fixture file contains a string of 30+ alphanumerics
/// (heuristic for "looks like a real bearer token"). Protects against
/// accidentally checking in leaked credentials.
#[test]
fn fixture_hygiene_no_real_bearer_tokens() {
    let fixture_names = [
        "pro_max_with_rate_limits",
        "api_key_no_rate_limits",
        "extra_usage_enabled",
        "malformed_field",
    ];

    for name in &fixture_names {
        let content = fixture(name);
        let run = longest_alnum_run(&content);
        assert!(
            run < 30,
            "fixture '{}' contains a {}-char alnum run — possible leaked token",
            name,
            run
        );
    }
}
