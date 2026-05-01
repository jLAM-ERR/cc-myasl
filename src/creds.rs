//! Credential reader: macOS Keychain + `~/.claude/.credentials.json` fallback.
//!
//! # Architecture
//!
//! Split into a **pure parser** (testable on every platform) and a
//! **thin Command caller** (macOS-only, `#[cfg(target_os = "macos")]`).
//! This seam lets unit tests exercise all JSON-parsing branches without
//! spawning the `security` binary.

use anyhow::{Context, anyhow};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

// ── internal helper ────────────────────────────────────────────────────────

/// Parse the JSON object `{ "claudeAiOauth": { "accessToken": "…" } }`
/// and return the access token. Used by both public parse functions.
fn parse_oauth_json(s: &str) -> Result<String, anyhow::Error> {
    if s.trim().is_empty() {
        return Err(anyhow!("credential JSON is empty"));
    }
    let v: serde_json::Value = serde_json::from_str(s).context("credential JSON is malformed")?;
    let token = v
        .get("claudeAiOauth")
        .ok_or_else(|| anyhow!("missing field: claudeAiOauth"))?
        .get("accessToken")
        .ok_or_else(|| anyhow!("missing field: claudeAiOauth.accessToken"))?
        .as_str()
        .ok_or_else(|| anyhow!("claudeAiOauth.accessToken is not a string"))?;
    Ok(token.to_owned())
}

// ── public parsers (cross-platform, no I/O) ────────────────────────────────

/// Parse the JSON returned by `security find-generic-password -w` and
/// extract `claudeAiOauth.accessToken`.
///
/// This is a **pure function** — no I/O — so it is testable on every platform.
pub fn parse_keychain_output(stdout: &str) -> Result<String, anyhow::Error> {
    parse_oauth_json(stdout).context("parse_keychain_output failed")
}

/// Parse the contents of `~/.claude/.credentials.json` and extract
/// `claudeAiOauth.accessToken`.
///
/// This is a **pure function** — no I/O — so it is testable on every platform.
pub fn parse_credentials_file(content: &str) -> Result<String, anyhow::Error> {
    parse_oauth_json(content).context("parse_credentials_file failed")
}

// ── thin Command caller (macOS only) ───────────────────────────────────────

/// Invoke `security find-generic-password -s "Claude Code-credentials" -w`
/// and return the parsed access token.
///
/// Uses only `find-generic-password` — the keychain-enumeration subcommand is
/// deliberately excluded (see `docs/security-review.md`, Hard Invariant #4).
///
/// Returns `Err` if the `security` process exits non-zero, produces empty
/// stdout, or the JSON cannot be parsed.
#[cfg(target_os = "macos")]
fn keychain_command_output() -> Result<String, anyhow::Error> {
    let output = std::process::Command::new("security")
        .args([
            "find-generic-password",
            "-s",
            "Claude Code-credentials",
            "-w",
        ])
        .output()
        .context("failed to launch `security` binary")?;

    if !output.status.success() {
        return Err(anyhow!(
            "security exited with status {}",
            output
                .status
                .code()
                .map(|c| c.to_string())
                .unwrap_or_else(|| "unknown".to_owned())
        ));
    }

    let stdout = std::str::from_utf8(&output.stdout)
        .context("security stdout is not valid UTF-8")?
        .trim()
        .to_owned();

    if stdout.is_empty() {
        return Err(anyhow!("security produced empty stdout"));
    }

    parse_keychain_output(&stdout)
}

// ── orchestrator ───────────────────────────────────────────────────────────

/// Read the Claude Code bearer token.
///
/// On **macOS**: tries the Keychain (`Claude Code-credentials` service) first;
/// falls back to `~/.claude/.credentials.json` if the Keychain lookup fails.
///
/// On **Linux / other platforms**: reads `~/.claude/.credentials.json` only.
///
/// Returns `Err` if both sources miss or fail — the caller should degrade
/// gracefully (render without a quota segment) rather than exiting non-zero.
///
/// The token is NEVER logged, even in error messages.
pub fn read_token() -> Result<String, anyhow::Error> {
    #[cfg(target_os = "macos")]
    {
        if let Ok(token) = keychain_command_output() {
            return Ok(token);
        }
    }

    // File fallback — both platforms.
    let home = directories::BaseDirs::new()
        .ok_or_else(|| anyhow!("could not determine home directory"))?
        .home_dir()
        .to_owned();

    let path = home.join(".claude").join(".credentials.json");
    let content = std::fs::read_to_string(&path).with_context(|| {
        format!(
            "credentials file not found at {}",
            redact_home(&path.display().to_string())
        )
    })?;

    parse_credentials_file(&content)
}

// ── home-path redaction helper ─────────────────────────────────────────────

/// Replace occurrences of the user's home-directory path in `s` with `~`.
///
/// Used to strip `$HOME` from error messages and path displays before they
/// reach user-visible output (stdout `--check`, stderr debug trace).
/// If `HOME` is not set or the home prefix is empty, returns the original
/// string unchanged.
pub fn redact_home(s: &str) -> String {
    if let Ok(h) = std::env::var("HOME") {
        if !h.is_empty() {
            return s.replace(&h, "~");
        }
    }
    s.to_owned()
}

// ── fingerprint ────────────────────────────────────────────────────────────

/// Return a 16-character lowercase hex string that identifies a token for
/// rotation detection, without revealing any portion of the token.
///
/// Implementation: takes the **last 8 characters** of `token`, hashes them
/// with `std::collections::hash_map::DefaultHasher` (stdlib SipHash), and
/// formats the resulting `u64` as 16 hex digits.
///
/// # Deviation from plan
///
/// The plan and `docs/research.md` call for SHA-256, but the project's locked
/// crate set excludes `sha2`. We use stdlib SipHash instead — it is **not** a
/// cryptographic primitive, but it is sufficient for the sole purpose here:
/// detecting whether the token has rotated between invocations. The output is
/// an opaque identifier and MUST NOT be used for any security purpose.
///
/// The function NEVER echoes any portion of the input token in its output.
pub fn fingerprint(token: &str) -> String {
    let chars: Vec<char> = token.chars().collect();
    let tail: String = chars
        .iter()
        .rev()
        .take(8)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();

    let mut hasher = DefaultHasher::new();
    tail.hash(&mut hasher);
    let h = hasher.finish();
    format!("{h:016x}")
}

// ── tests ──────────────────────────────────────────────────────────────────

/// Shared mutex serializing every test that reads or mutates the `HOME`
/// env var.  Owned here in `creds` because the file-fallback path is the
/// dominant HOME consumer, but exposed `pub(crate)` so other modules'
/// tests (notably `format::placeholders::tests::cwd_substitutes_home`)
/// can join the same lock and avoid cross-module env-var races.
#[cfg(test)]
pub(crate) static HOME_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    // ── fixture JSON ──────────────────────────────────────────────────────

    const VALID_JSON: &str = r#"{"claudeAiOauth":{"accessToken":"sk-ant-test-bearer-12345"}}"#;
    const MISSING_OAUTH_JSON: &str = r#"{"somethingElse":{}}"#;
    const MISSING_TOKEN_JSON: &str = r#"{"claudeAiOauth":{"notAccessToken":"x"}}"#;
    const MALFORMED_JSON: &str = r#"{ not valid json "#;
    const EMPTY: &str = "";

    const FIXTURE_TOKEN: &str = "sk-ant-test-bearer-12345";

    // ── parse_keychain_output ─────────────────────────────────────────────

    #[test]
    fn keychain_parse_valid() {
        let token = parse_keychain_output(VALID_JSON).unwrap();
        assert_eq!(token, FIXTURE_TOKEN);
    }

    #[test]
    fn keychain_parse_missing_oauth() {
        let err = parse_keychain_output(MISSING_OAUTH_JSON).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("claudeAiOauth"),
            "error should name the missing field"
        );
        assert!(
            !msg.contains(FIXTURE_TOKEN),
            "token must not leak into error"
        );
    }

    #[test]
    fn keychain_parse_missing_access_token() {
        let err = parse_keychain_output(MISSING_TOKEN_JSON).unwrap_err();
        let msg = format!("{err:?}");
        assert!(
            msg.contains("accessToken"),
            "error should name the missing field"
        );
        assert!(
            !msg.contains(FIXTURE_TOKEN),
            "token must not leak into error"
        );
    }

    #[test]
    fn keychain_parse_malformed_json() {
        let err = parse_keychain_output(MALFORMED_JSON).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("malformed"), "should mention malformed JSON");
        assert!(
            !msg.contains(FIXTURE_TOKEN),
            "token must not leak into error"
        );
    }

    #[test]
    fn keychain_parse_empty_string() {
        let err = parse_keychain_output(EMPTY).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("empty"), "should mention empty input");
        assert!(
            !msg.contains(FIXTURE_TOKEN),
            "token must not leak into error"
        );
    }

    // ── parse_credentials_file ────────────────────────────────────────────

    #[test]
    fn creds_file_parse_valid() {
        let token = parse_credentials_file(VALID_JSON).unwrap();
        assert_eq!(token, FIXTURE_TOKEN);
    }

    #[test]
    fn creds_file_parse_missing_oauth() {
        let err = parse_credentials_file(MISSING_OAUTH_JSON).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("claudeAiOauth"));
        assert!(!msg.contains(FIXTURE_TOKEN));
    }

    #[test]
    fn creds_file_parse_missing_access_token() {
        let err = parse_credentials_file(MISSING_TOKEN_JSON).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("accessToken"));
        assert!(!msg.contains(FIXTURE_TOKEN));
    }

    #[test]
    fn creds_file_parse_malformed_json() {
        let err = parse_credentials_file(MALFORMED_JSON).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("malformed"));
        assert!(!msg.contains(FIXTURE_TOKEN));
    }

    #[test]
    fn creds_file_parse_empty_string() {
        let err = parse_credentials_file(EMPTY).unwrap_err();
        let msg = format!("{err:?}");
        assert!(msg.contains("empty"));
        assert!(!msg.contains(FIXTURE_TOKEN));
    }

    // ── file-path tests (tempdir) ─────────────────────────────────────────

    #[test]
    fn file_present_and_valid() {
        let _guard = HOME_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join(".credentials.json"), VALID_JSON).unwrap();

        // Override HOME so `directories::BaseDirs` resolves to our tempdir.
        // SAFETY: single-threaded section serialised by HOME_MUTEX.
        unsafe { std::env::set_var("HOME", dir.path()) };
        let result = read_token();
        unsafe { std::env::remove_var("HOME") };

        assert_eq!(result.unwrap(), FIXTURE_TOKEN);
    }

    #[test]
    fn file_present_and_malformed() {
        let _guard = HOME_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        let claude_dir = dir.path().join(".claude");
        std::fs::create_dir_all(&claude_dir).unwrap();
        std::fs::write(claude_dir.join(".credentials.json"), MALFORMED_JSON).unwrap();

        unsafe { std::env::set_var("HOME", dir.path()) };
        let result = read_token();
        unsafe { std::env::remove_var("HOME") };

        assert!(result.is_err(), "malformed file should yield Err");
        // Token must not appear in the error.
        assert!(!format!("{:?}", result.unwrap_err()).contains(FIXTURE_TOKEN));
    }

    #[test]
    fn file_missing() {
        let _guard = HOME_MUTEX.lock().unwrap();
        let dir = tempdir().unwrap();
        // Don't create .claude dir at all.

        unsafe { std::env::set_var("HOME", dir.path()) };
        let result = read_token();
        unsafe { std::env::remove_var("HOME") };

        assert!(result.is_err(), "missing file should yield Err");
    }

    // ── token-never-in-error test ─────────────────────────────────────────

    /// Verify that no error message leaks the fixture token, regardless of
    /// which parse path produces the error.
    #[test]
    fn token_never_in_error_debug() {
        for input in &[
            MISSING_OAUTH_JSON,
            MISSING_TOKEN_JSON,
            MALFORMED_JSON,
            EMPTY,
        ] {
            for result in [parse_keychain_output(input), parse_credentials_file(input)] {
                if let Err(e) = result {
                    let dbg = format!("{e:?}");
                    assert!(!dbg.contains(FIXTURE_TOKEN), "token leaked in error: {dbg}");
                }
            }
        }
    }

    // ── fingerprint tests ─────────────────────────────────────────────────

    #[test]
    fn fingerprint_length_is_16() {
        let fp = fingerprint(FIXTURE_TOKEN);
        assert_eq!(fp.len(), 16, "fingerprint must be exactly 16 chars");
    }

    #[test]
    fn fingerprint_is_hex() {
        let fp = fingerprint(FIXTURE_TOKEN);
        assert!(
            fp.chars().all(|c| c.is_ascii_hexdigit()),
            "fingerprint must be lowercase hex: {fp}"
        );
    }

    #[test]
    fn fingerprint_is_deterministic() {
        assert_eq!(fingerprint(FIXTURE_TOKEN), fingerprint(FIXTURE_TOKEN));
    }

    #[test]
    fn fingerprint_differs_for_different_tokens() {
        let fp1 = fingerprint("token-alpha-aabbccdd");
        let fp2 = fingerprint("token-beta-xxyyzz00");
        assert_ne!(
            fp1, fp2,
            "distinct tokens should produce distinct fingerprints"
        );
    }

    #[test]
    fn fingerprint_contains_no_input_chars() {
        // The fingerprint must be a pure hex string; ensure none of the
        // token's distinctive characters surface verbatim in the output.
        let token = "sk-ant-super-secret-99";
        let fp = fingerprint(token);
        // Verify it is hex-only (no letters outside a-f, no digits that
        // could reconstruct the token).
        assert!(
            fp.chars().all(|c| matches!(c, '0'..='9' | 'a'..='f')),
            "fingerprint must only contain hex digits: {fp}"
        );
        // Additionally confirm the raw token string is not a substring.
        assert!(
            !fp.contains(token),
            "fingerprint must not contain the raw token"
        );
    }

    // ── fingerprint: short-token edge case (NIT 3) ───────────────────────
    // Opaqueness degrades for tokens < 8 chars; acceptable since Claude Code
    // tokens are ≥ 40 chars in practice. Output is still 16 hex chars.
    #[test]
    fn fingerprint_short_token_does_not_reveal_input() {
        let token = "abc";
        let fp = fingerprint(token);
        assert_eq!(fp.len(), 16, "must be 16 hex chars for short token");
        assert!(
            fp.chars().all(|c| c.is_ascii_hexdigit()),
            "must be hex: {fp}"
        );
        assert!(!fp.contains(token), "must not contain raw token: {fp}");
    }

    // ── redact_home ───────────────────────────────────────────────────────

    #[test]
    fn redact_home_behaviour() {
        let _guard = HOME_MUTEX.lock().unwrap();
        let original = std::env::var("HOME").ok();
        unsafe { std::env::set_var("HOME", "/home/testuser") };
        let in_msg = redact_home("not found at /home/testuser/.claude/.credentials.json");
        let no_match = redact_home("/tmp/something");
        let _restore = original.map(|v| unsafe { std::env::set_var("HOME", v) });
        assert_eq!(in_msg, "not found at ~/.claude/.credentials.json");
        assert_eq!(no_match, "/tmp/something");
    }

    /// read_token error must not contain the expanded HOME path.
    #[test]
    fn read_token_error_does_not_contain_home_path() {
        let _guard = HOME_MUTEX.lock().unwrap();
        use tempfile::tempdir;
        let dir = tempdir().unwrap();
        let fake_home = dir.path().to_str().unwrap().to_owned();
        unsafe { std::env::set_var("HOME", &fake_home) };
        let result = read_token();
        unsafe { std::env::remove_var("HOME") };
        let msg = format!("{:?}", result.expect_err("missing file should yield Err"));
        assert!(!msg.contains(&fake_home), "expanded HOME in error: {msg}");
        assert!(msg.contains("~/.claude"), "tilde path missing: {msg}");
    }

    // ── macOS keychain integration smoke test ─────────────────────────────

    #[cfg(target_os = "macos")]
    #[test]
    fn keychain_command_output_integration() {
        // Only runs when the human explicitly opts in.
        if std::env::var("CLAUDE_STATUSLINE_KEYCHAIN_TEST").as_deref() != Ok("1") {
            return; // skip
        }
        // If the env var IS set, the keychain entry must exist and be parseable.
        let result = keychain_command_output();
        assert!(
            result.is_ok(),
            "keychain lookup failed (is 'Claude Code-credentials' installed?): {:?}",
            result.unwrap_err()
        );
        let token = result.unwrap();
        assert!(!token.is_empty(), "token must be non-empty");
    }
}
