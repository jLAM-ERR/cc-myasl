# Sensitive-data leak audit — 2026-04-27

Auditor: Sonnet sub-agent (orchestrated by Opus 4.7).
Scope: `src/**/*.rs`, focus on `cc-myasl`'s leak surface.

## Summary

Zero critical findings. Two concerns were identified: (1) the user's home-directory path is embedded in credentials-error messages that flow into both the `--check` stdout and the `STATUSLINE_DEBUG=1` stderr trace, and (2) the full cache-file path (which also contains `$HOME`) is printed verbatim in `--check` stdout. Neither of these leaks the bearer OAuth token. Three nits cover test-coverage gaps and minor hardening opportunities. Overall verdict: **clean on the highest-priority invariants (bearer token), minor on the secondary path-leakage concern**.

---

## Critical findings

None.

---

## Concerns

### CONCERN 1 — Home-directory path in `--check` stdout via credentials error

**File and line range:** `src/check.rs:90`, `src/creds.rs:125`

**What leaks where:**
When `~/.claude/.credentials.json` is absent or unreadable, `creds::read_token()` returns an `anyhow::Error` whose `Display` message is:

```
credentials file not found at /Users/alice/.claude/.credentials.json
```

That full path (which embeds `$HOME`) is echoed verbatim to **stdout** at:

```rust
// check.rs:90
println!("Credentials: ✗ {e}");
```

Users frequently share `--check` output in bug reports and Slack threads.

**Reproduction:**
Run `cc-myasl --check` on a machine where the credentials file does not exist. The printed line will contain the fully-expanded home path.

**Concrete fix:**
Either redact the path in the error message (e.g., replace `path.display()` with `"~/.claude/.credentials.json"` literal), or strip the home prefix before printing in `report_credentials`:

```rust
// In check.rs::report_credentials, error branch:
let msg = e.to_string();
let sanitized = if let Ok(h) = std::env::var("HOME") {
    msg.replace(&h, "~")
} else {
    msg
};
println!("Credentials: ✗ {sanitized}");
```

---

### CONCERN 2 — Home-directory path in `STATUSLINE_DEBUG=1` stderr trace via credentials error

**File and line range:** `src/main.rs:123`, `src/creds.rs:125`

**What leaks where:**
In `run_render`, when `creds::read_token()` fails, the error is placed into `trace.error`:

```rust
// main.rs:123
trace.error = Some(e.to_string());
```

When `STATUSLINE_DEBUG=1` or `--debug` is active, this string is serialized to the JSON trace on **stderr**. The string includes the fully-expanded credential file path (`/Users/alice/.claude/.credentials.json`). This is the same underlying message as Concern 1 but surfacing via the debug channel rather than `--check`.

**Reproduction:**
```bash
STATUSLINE_DEBUG=1 echo '{}' | cc-myasl
# (on a machine without credentials)
# stderr will contain: {"error":"[CacheRead] ... /Users/alice/.claude/.credentials.json ..."}
```

**Concrete fix:**
Same redaction approach as Concern 1. Alternatively, apply the sanitization in `error::Error::Display` for `CredsRead` by stripping home-path prefixes before populating the `String` payload.

---

### CONCERN 3 — Full cache path with `$HOME` in `--check` stdout

**File and line range:** `src/check.rs:159`, `src/check.rs:163-166`

**What leaks where:**
`check_cache` prints the fully-expanded path of the on-disk cache file:

```rust
// check.rs:159
println!("Cache: ✓ {} ({freshness})", path.display());
// check.rs:163-166
println!(
    "Cache: ✗ {} exists but could not be parsed (corrupt?)",
    path.display()
);
```

On macOS this resolves to `~/Library/Caches/cc-myasl/usage.json`, which contains the home directory. Low risk in isolation, but consistent with the project's goal of not echoing env-var-derived paths in user-facing output.

**Reproduction:**
`cc-myasl --check` on any machine with an existing cache file.

**Concrete fix:**
Replace `path.display()` with a home-tilde-collapsed variant:

```rust
fn display_tilde(path: &Path) -> String {
    let s = path.display().to_string();
    if let Ok(h) = std::env::var("HOME") {
        if s.starts_with(&h) {
            return format!("~{}", &s[h.len()..]);
        }
    }
    s
}
```

---

## Nits

### NIT 1 — `debug.rs` redaction test doesn't use a realistic error message

**File and line range:** `src/debug.rs:184-230`

**What:** The `bearer_token_never_in_json_output` test constructs a `Trace` with `error: Some("oh no".into())`. It never tests the realistic production scenario where `trace.error` is set to `e.to_string()` from a `creds::read_token()` failure — which would include a home path. The test therefore can't catch Concern 2.

**Fix:** Add a parameterized assertion that a trace with `error` set to a realistic creds-error string (including a fake home path) still doesn't contain the bearer token (already true) and ideally also doesn't contain the raw home path (would catch Concern 2 if redaction were added).

---

### NIT 2 — `check.rs::creds_section_token_not_printed` doesn't actually capture stdout

**File and line range:** `src/check.rs:283-298`

**What:** The test comment acknowledges it cannot capture stdout without a subprocess. It only verifies that `creds::fingerprint` is hex-only — it does not verify that `report_credentials` doesn't call `println!("{token}")` somewhere. A future refactor adding a raw-token print would go undetected.

**Fix:** Refactor `report_credentials` to write to a `&mut dyn Write` instead of calling `println!` directly, then test it with a captured buffer (same pattern as `debug::Trace::emit_to`). This makes the no-raw-token invariant machine-checkable.

---

### NIT 3 — `fingerprint` uses only last-8-chars input; doc says "non-cryptographic, fine for rotation detection" but the claim needs a test for short tokens

**File and line range:** `src/creds.rs:148-163`

**What:** `fingerprint` takes the last 8 characters of the token. For a token shorter than 8 characters, `tail` is the entire token and the resulting fingerprint could be brute-forced (trivially for very short tokens). No test covers the short-token edge case.

**Fix:** Add a test `fingerprint_short_token_does_not_reveal_input` asserting that for a 3-character token, the fingerprint is still 16 hex chars and doesn't contain the token as a substring. Also document in the function comment that tokens shorter than 8 chars degrade the opaqueness of the fingerprint (acceptable given only Claude Code-format tokens are expected in practice).

---

## Defenses verified working

- **Bearer token never written to disk.** `UsageCache` has no token field (compile-time guarantee). Verified by `cache::tests::no_forbidden_substrings_in_serialized_cache` which checks for `"token"`, `"bearer"`, `"secret"`, `"auth"`, `"access"` substrings in the serialized JSON (case-insensitive). Also verified by `tests/golden.rs` (test 8 — fixture hygiene).

- **Bearer token never appears in error chain messages.** `creds::tests::token_never_in_error_debug` iterates over all parse-error paths and confirms the fixture token `"sk-ant-test-bearer-12345"` never appears in `format!("{e:?}")`.

- **Bearer token never appears in the debug trace JSON.** `debug::tests::bearer_token_never_in_json_output` constructs a `Trace` with a mock fingerprint and verifies that `"sk-ant"`, `"bearer-12345"`, and `"test-bearer"` are absent from the emitted JSON.

- **Only fingerprint goes into `Trace.token_fp`.** `main.rs:128` calls `creds::fingerprint(&token)` before storing in `trace.token_fp`. Code review confirms no other path stores the raw token in `Trace`.

- **`--check` Credentials section prints fingerprint, not token.** `check.rs:84-85` calls `creds::fingerprint(&token)` and prints only `fp`. Unit test `creds_section_ok_result_passes` verifies the token is returned from `report_credentials` (for forwarding to `check_network`) but no test captures the `println!` output to verify `fp != token`. See NIT 2.

- **No `security dump-keychain` invocation.** `creds.rs::keychain_command_output` uses only `find-generic-password -s "Claude Code-credentials" -w`. Enforced by `scripts/check-invariants.sh` grep.

- **`format/*.rs` decoupled from `api` and `cache`.** Enforced by `format::placeholders::tests::format_module_does_not_depend_on_api_or_cache`, which string-scans source files for `use crate::api` and `use crate::cache`.

- **`main` never exits non-zero in render mode.** Every error branch in `run_render` calls `render_and_emit` and returns, followed by `std::process::exit(0)`. Verified by `main::tests::adversarial_bad_stdin_still_produces_output` and golden test 7.

- **HTTP response body not echoed in errors.** `api::mod.rs:65-69` reads the body only to JSON-parse it; on parse failure the body is discarded and `FetchOutcome::ServerError` is returned (no body string in the error chain). Non-200 status responses (`Err(ureq::Error::Status(...))`) drop the response body entirely — only the status code is inspected.

- **`STATUSLINE_OAUTH_BASE_URL` value not logged.** `main.rs:131-132` reads the env var into a local `base_url` variable. `base_url` is passed to `api::fetch_usage` and used only to construct the URL for the HTTP call. It is not written to `trace` or any output channel.

- **Atomic writes prevent corrupt cache files.** `cache::atomic_helper::write_atomic` uses a per-call unique tmp filename (`path.tmp.<pid>.<counter>`) followed by `rename(2)`. Verified by `concurrent_writes_last_writer_wins_no_tmp_left` and `concurrent_read_never_observes_partial_write` tests.

---

## Test coverage gaps

- **No test verifies that `trace.error` (rendered into debug JSON) does not contain the user's home-directory path** when credentials are missing. The `debug.rs` redaction test uses an artificial error message ("oh no") that is not representative of a real creds failure.

- **No test captures the actual stdout of `--check`** to verify that the raw bearer token never appears in any printed line. `creds_section_token_not_printed` only checks the fingerprint function in isolation, not the integrated `println!` in `report_credentials`.

- **No test for `fingerprint` with a token shorter than 8 characters.** The function's opaqueness degrades for short tokens; this is acceptable for the target token format but isn't explicitly documented or tested.

- **No test verifies that `--check` stdout does not contain the fully-expanded home path** for the credential-error and cache-path messages. The golden test suite (`tests/golden.rs`) pins `HOME` to a temp dir but doesn't assert the absence of real home paths in `--check` output.

- **`CLAUDE_STATUSLINE_KEYCHAIN_TEST=1` smoke test** (`creds::tests::keychain_command_output_integration`) is deliberately opt-in and never runs in CI. There is no CI-level assertion that the macOS Keychain path reads the correct service name and doesn't fall back to `dump-keychain`.

---

## Out of scope (deliberately not audited)

- Supply-chain (Cargo crate code for `ureq`, `serde_json`, `directories`, `anyhow`, `mockito`).
- GitHub Actions secrets and workflow runner environment.
- `README.md`, `CHANGELOG.md`, `templates/*.txt`, and `docs/*.md` content (no code paths).
- `scripts/install.sh` (shell script, not Rust source).
- The undocumented Anthropic OAuth endpoint's server-side behaviour.
- macOS Keychain security model and Gatekeeper code-signing.
