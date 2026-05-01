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
    Command::cargo_bin("cc-myasl").expect("binary must build")
}

/// Write a single-segment config JSON to a tempfile and return the path.
/// The template string may contain `{? … }` optional blocks.
fn write_config_for_template(dir: &tempfile::TempDir, template: &str) -> PathBuf {
    let path = dir.path().join("test_config.json");
    let escaped = template.replace('\\', "\\\\").replace('"', "\\\"");
    let json =
        format!(r#"{{"lines":[{{"separator":"","segments":[{{"template":"{escaped}"}}]}}]}}"#);
    fs::write(&path, json).unwrap();
    path
}

/// Return the cache dir the binary will use given `home`.
fn cache_dir_for_home(home: &Path) -> PathBuf {
    #[cfg(target_os = "macos")]
    {
        home.join("Library")
            .join("Caches")
            .join("ai.cc-myasl.cc-myasl")
    }
    #[cfg(not(target_os = "macos"))]
    {
        home.join(".cache").join("cc-myasl")
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
/// Port 1 is not-listening; completing < 1 s proves no HTTP call was made.
#[test]
fn pro_max_hot_path_renders_quota() {
    let started = std::time::Instant::now();
    let tmpdir = tempfile::tempdir().unwrap();
    let cfg = write_config_for_template(&tmpdir, "{model} · 5h: {five_left}% · 7d: {seven_left}%");

    bin()
        .arg("--config")
        .arg(&cfg)
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

    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write_config_for_template(&cfg_dir, "{five_left}/{seven_left}");

    bin()
        .arg("--config")
        .arg(&cfg)
        .env("STATUSLINE_OAUTH_BASE_URL", server.url())
        .env("HOME", home.path())
        // Pin XDG_CACHE_HOME so Linux CI doesn't route writes outside tempdir.
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

    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write_config_for_template(&cfg_dir, "{model}{? · 5h:{five_left}%}");

    bin()
        .arg("--config")
        .arg(&cfg)
        .env("STATUSLINE_OAUTH_BASE_URL", server.url())
        .env("HOME", home.path())
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

    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write_config_for_template(&cfg_dir, "{model}");

    bin()
        .arg("--config")
        .arg(&cfg)
        .env("STATUSLINE_OAUTH_BASE_URL", server.url())
        .env("HOME", home.path())
        .env("XDG_CACHE_HOME", home.path().join(".cache"))
        .write_stdin(fixture("api_key_no_rate_limits"))
        .assert()
        .success()
        .stdout("claude-sonnet-4-6\n");

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

    let cfg_dir = tempfile::tempdir().unwrap();
    let cfg = write_config_for_template(&cfg_dir, "{model}{? extra:{extra_left}}");

    bin()
        .arg("--config")
        .arg(&cfg)
        .env("STATUSLINE_OAUTH_BASE_URL", server.url())
        .env("HOME", home.path())
        .env("XDG_CACHE_HOME", home.path().join(".cache"))
        .write_stdin(fixture("extra_usage_enabled"))
        .assert()
        .success()
        .stdout(predicates::str::is_match(r"^claude-opus-4-7 extra:75\n$").unwrap());
}

// ── test 7: malformed payload — graceful degrade ──────────────────────────────

/// Malformed stdin: display_name is a number, workspace is null, rate_limits wrong type.
/// Parser must fail gracefully; main must still exit 0 with non-empty output.
#[test]
fn malformed_payload_degrades_to_exit_zero() {
    bin()
        .arg("--template")
        .arg("default")
        .env("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1")
        .write_stdin(fixture("malformed_field"))
        .assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

// ── test 8: fixture hygiene — no real bearer tokens ──────────────────────────

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

/// No fixture file contains a 30+ alphanumeric run (heuristic for real bearer tokens).
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
