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
use crate::config;
use crate::creds;
use crate::format::RenderCtx;

use crate::api::DEFAULT_OAUTH_BASE_URL;

const SCHEMA_URL: &str =
    "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json";

const BUILTIN_NAMES: &[&str] = &[
    "default",
    "minimal",
    "compact",
    "bars",
    "colored",
    "emoji",
    "emoji_verbose",
    "verbose",
];

// ── report ────────────────────────────────────────────────────────────────────

#[derive(Debug, Default)]
pub struct CheckReport {
    pub creds_ok: bool,
    pub network_ok: bool,
    pub cache_ok: bool,
    pub config_ok: bool,
}

impl CheckReport {
    pub fn all_ok(&self) -> bool {
        self.creds_ok && self.network_ok && self.cache_ok && self.config_ok
    }
}

// ── public entry points ───────────────────────────────────────────────────────

/// Run the full diagnostic, printing sections to stdout.
/// Returns 0 if every section passed; 1 otherwise.
pub fn run() -> i32 {
    let report = run_inner();
    if report.all_ok() { 0 } else { 1 }
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

    // 4. Config
    check_config(&mut report);

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

/// Check config; print result.
pub(crate) fn check_config(report: &mut CheckReport) {
    check_config_to(report, &mut std::io::stdout());
}

/// Testable core of `check_config` — writes to `out`.
pub(crate) fn check_config_to(report: &mut CheckReport, out: &mut dyn Write) {
    // Resolve config and capture source.
    let mut trace = crate::debug::Trace::default();
    let args = crate::args::Args::default();
    let cfg = config::resolve(&args, &mut trace);

    let source_label = trace
        .config_source
        .as_ref()
        .map(|s| s.as_str())
        .unwrap_or("unknown");

    // Print config source.
    let _ = writeln!(out, "Config: ✓ source={source_label}");

    // Print active config as pretty JSON.
    let json = config::print_config(&cfg);
    let _ = writeln!(out, "Config: active config:");
    for line in json.lines() {
        let _ = writeln!(out, "  {line}");
    }

    // List built-in names.
    let _ = writeln!(
        out,
        "Config: built-in templates: {}",
        BUILTIN_NAMES.join(", ")
    );

    // Print schema URL.
    let _ = writeln!(out, "Config: schema URL: {SCHEMA_URL}");

    // Print user templates dir.
    match config::user_templates_dir() {
        Some(dir) => {
            let display = display_tilde(&dir);
            if dir.exists() {
                // List files present.
                let entries: Vec<String> = std::fs::read_dir(&dir)
                    .ok()
                    .into_iter()
                    .flatten()
                    .flatten()
                    .filter_map(|e| e.file_name().into_string().ok())
                    .collect();
                if entries.is_empty() {
                    let _ = writeln!(out, "Config: user templates dir: {display} (empty)");
                } else {
                    let _ = writeln!(
                        out,
                        "Config: user templates dir: {display} ({})",
                        entries.join(", ")
                    );
                }
            } else {
                let _ = writeln!(
                    out,
                    "Config: user templates dir: {display} (does not exist)"
                );
            }
        }
        None => {
            let _ = writeln!(out, "Config: user templates dir: (cannot determine)");
        }
    }

    // Verify the resolved config renders without panic.
    let ctx = RenderCtx {
        model: Some("claude-opus-4".to_string()),
        five_used: Some(30.0),
        five_reset_unix: Some(crate::time::now_unix() + 3600),
        seven_used: Some(60.0),
        seven_reset_unix: Some(crate::time::now_unix() + 86400),
        now_unix: crate::time::now_unix(),
        ..Default::default()
    };
    let render_result = std::panic::catch_unwind(|| config::render::render(&cfg, &ctx));
    match render_result {
        Ok(s) if !s.is_empty() => {
            let _ = writeln!(out, "Config: ✓ renders non-empty output");
            report.config_ok = true;
        }
        Ok(_) => {
            let _ = writeln!(out, "Config: ✗ renders empty output");
            report.config_ok = false;
        }
        Err(_) => {
            let _ = writeln!(out, "Config: ✗ render panicked");
            report.config_ok = false;
        }
    }
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "check_tests.rs"]
mod tests;
