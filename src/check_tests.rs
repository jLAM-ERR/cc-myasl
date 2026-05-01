//! Tests for `check.rs`.  Extracted to keep `check.rs` under 500 LOC.

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
        config_ok: true,
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
            config_ok: true,
        },
        CheckReport {
            creds_ok: true,
            network_ok: false,
            cache_ok: true,
            config_ok: true,
        },
        CheckReport {
            creds_ok: true,
            network_ok: true,
            cache_ok: false,
            config_ok: true,
        },
        CheckReport {
            creds_ok: true,
            network_ok: true,
            cache_ok: true,
            config_ok: false,
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

// ── config section ────────────────────────────────────────────────────────

#[test]
fn config_section_embedded_default_passes() {
    let _guard = crate::config::CONFIG_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // No config file, no env var → falls through to embedded default.
    std::env::remove_var("STATUSLINE_CONFIG");
    let mut report = CheckReport::default();
    let mut buf: Vec<u8> = Vec::new();
    check_config_to(&mut report, &mut buf);
    let out = std::str::from_utf8(&buf).unwrap();
    assert!(
        report.config_ok,
        "embedded default should pass config check"
    );
    assert!(
        out.contains("Embedded"),
        "output should mention Embedded source: {out:?}"
    );
    assert!(
        out.contains(SCHEMA_URL),
        "output should contain schema URL: {out:?}"
    );
}

#[test]
fn config_section_valid_config_file_passes() {
    let _guard = crate::config::CONFIG_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().unwrap();
    let config_file = dir.path().join("config.json");
    // Write a minimal valid config.
    std::fs::write(
        &config_file,
        r#"{"lines":[{"separator":"","segments":[{"template":"{model}"}]}]}"#,
    )
    .unwrap();

    // Pin XDG_CONFIG_HOME so the default-file path resolves to our tempdir.
    let prior = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("XDG_CONFIG_HOME", dir.path().to_str().unwrap());
    std::env::remove_var("STATUSLINE_CONFIG");

    let mut report = CheckReport::default();
    let mut buf: Vec<u8> = Vec::new();
    check_config_to(&mut report, &mut buf);

    // Restore.
    match prior {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }

    let out = std::str::from_utf8(&buf).unwrap();
    assert!(report.config_ok, "valid config file should pass: {out}");
    assert!(
        out.contains("DefaultFile") || out.contains("source="),
        "output should mention config source: {out:?}"
    );
    assert!(
        out.contains(SCHEMA_URL),
        "output should contain schema URL: {out:?}"
    );
}

#[test]
fn config_section_malformed_config_falls_back_and_passes() {
    let _guard = crate::config::CONFIG_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let dir = TempDir::new().unwrap();
    let config_file = dir.path().join("config.json");
    std::fs::write(&config_file, b"{ not valid json }").unwrap();

    let prior = std::env::var("XDG_CONFIG_HOME").ok();
    std::env::set_var("XDG_CONFIG_HOME", dir.path().to_str().unwrap());
    std::env::remove_var("STATUSLINE_CONFIG");

    let mut report = CheckReport::default();
    let mut buf: Vec<u8> = Vec::new();
    check_config_to(&mut report, &mut buf);

    match prior {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }

    let out = std::str::from_utf8(&buf).unwrap();
    // Falls back to embedded default → config_ok=true (render mode never fails).
    assert!(
        report.config_ok,
        "malformed config falls back to embedded default → should pass: {out}"
    );
    assert!(
        out.contains("Embedded"),
        "fallback should show Embedded source: {out:?}"
    );
}

#[test]
fn config_section_output_contains_builtin_names() {
    let _guard = crate::config::CONFIG_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("STATUSLINE_CONFIG");
    let mut report = CheckReport::default();
    let mut buf: Vec<u8> = Vec::new();
    check_config_to(&mut report, &mut buf);
    let out = std::str::from_utf8(&buf).unwrap();
    for name in BUILTIN_NAMES {
        assert!(
            out.contains(name),
            "output should list built-in name '{name}': {out:?}"
        );
    }
}

#[test]
fn config_section_output_contains_json_schema_field() {
    let _guard = crate::config::CONFIG_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    std::env::remove_var("STATUSLINE_CONFIG");
    let mut report = CheckReport::default();
    let mut buf: Vec<u8> = Vec::new();
    check_config_to(&mut report, &mut buf);
    let out = std::str::from_utf8(&buf).unwrap();
    assert!(
        out.contains("$schema"),
        "active config JSON should contain $schema field: {out:?}"
    );
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
fn run_inner_config_always_passes() {
    let _guard = ENV_MUTEX.lock().unwrap_or_else(|e| e.into_inner());
    let _cfg_guard = crate::config::CONFIG_MUTEX
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    // Refused port → network fails fast; config check uses resolved default.
    std::env::set_var("STATUSLINE_OAUTH_BASE_URL", "http://127.0.0.1:1");
    std::env::remove_var("STATUSLINE_CONFIG");
    let report = run_inner();
    std::env::remove_var("STATUSLINE_OAUTH_BASE_URL");
    assert!(report.config_ok, "config should always pass");
}
