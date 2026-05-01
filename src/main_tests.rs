// Adversarial / boundary tests for main.rs wiring:
// --print-config edge cases, render-mode exit-0 invariant, build/apply helpers.

use super::*;
use cc_myasl::args;
use cc_myasl::debug::ConfigSource;
use std::sync::Mutex;

/// Serializes tests that mutate STATUSLINE_CONFIG or XDG_CONFIG_HOME.
static ENV_LOCK: Mutex<()> = Mutex::new(());

// ── --print-config with bad explicit config path ───────────────────────────

#[test]
fn print_config_bad_config_path_still_produces_valid_json() {
    let dir = tempfile::tempdir().unwrap();
    let bad = dir.path().join("nonexistent_config_xyz.json");
    let a = args::Args {
        config_path: Some(bad),
        print_config: true,
        ..Default::default()
    };
    let mut trace = Trace::default();
    let config = cc_myasl::config::resolve(&a, &mut trace);
    let output = cc_myasl::config::print_config(&config);
    let v: serde_json::Value =
        serde_json::from_str(&output).expect("must be valid JSON even with bad path");
    assert!(v.get("$schema").is_some(), "$schema must be present");
    // CliPath must NOT be recorded when the file was missing.
    assert_ne!(
        trace.config_source,
        Some(ConfigSource::CliPath),
        "CliPath must not be set when config file is missing"
    );
}

// ── --print-config with unknown --template falls back, records error ───────

#[test]
fn print_config_unknown_template_falls_back_records_error() {
    let _g = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let prior_cfg = std::env::var("STATUSLINE_CONFIG").ok();
    let prior_xdg = std::env::var("XDG_CONFIG_HOME").ok();
    let iso_dir = tempfile::tempdir().unwrap();
    std::env::remove_var("STATUSLINE_CONFIG");
    // Pin XDG_CONFIG_HOME to an empty tempdir — no default config.json there.
    std::env::set_var("XDG_CONFIG_HOME", iso_dir.path());

    let a = args::Args {
        template_name: Some("totally_nonexistent_xyz".to_owned()),
        print_config: true,
        ..Default::default()
    };
    let mut trace = Trace::default();
    let config = cc_myasl::config::resolve(&a, &mut trace);
    let output = cc_myasl::config::print_config(&config);

    match prior_cfg {
        Some(v) => std::env::set_var("STATUSLINE_CONFIG", v),
        None => std::env::remove_var("STATUSLINE_CONFIG"),
    }
    match prior_xdg {
        Some(v) => std::env::set_var("XDG_CONFIG_HOME", v),
        None => std::env::remove_var("XDG_CONFIG_HOME"),
    }

    let v: serde_json::Value =
        serde_json::from_str(&output).expect("must be valid JSON even with bad template");
    assert!(v.get("$schema").is_some(), "$schema must be present");
    assert!(
        trace.error.is_some(),
        "unknown template must record error in trace"
    );
    assert!(
        trace
            .error
            .as_deref()
            .unwrap_or("")
            .contains("totally_nonexistent_xyz"),
        "trace.error must mention the unknown name: {:?}",
        trace.error
    );
    assert_eq!(
        trace.config_source,
        Some(ConfigSource::Embedded),
        "must fall back to Embedded when template unknown and no other config"
    );
}

// ── --print-config output is deterministic across repeated calls ───────────

#[test]
fn print_config_output_is_idempotent_across_resolve_calls() {
    let a = args::Args {
        print_config: true,
        ..Default::default()
    };
    let mut t1 = Trace::default();
    let out1 = cc_myasl::config::print_config(&cc_myasl::config::resolve(&a, &mut t1));
    let mut t2 = Trace::default();
    let out2 = cc_myasl::config::print_config(&cc_myasl::config::resolve(&a, &mut t2));
    assert_eq!(
        out1, out2,
        "print_config must be deterministic across calls"
    );
}

// ── build_cache_from_response: all-None windows leaves ctx untouched ──────

#[test]
fn build_cache_all_none_windows_leaves_ctx_empty() {
    use cc_myasl::api::response::UsageResponse;
    let resp = UsageResponse {
        five_hour: None,
        seven_day: None,
        extra_usage: None,
    };
    let c = build_cache_from_response(&resp, 12345);
    assert_eq!(c.fetched_at, 12345);
    assert!(c.five_hour.is_none());
    assert!(c.seven_day.is_none());
    assert!(c.extra_usage.is_none());

    let mut ctx = RenderCtx {
        now_unix: 0,
        ..Default::default()
    };
    apply_cache_to_ctx(&c, &mut ctx);
    assert!(ctx.five_used.is_none(), "five_used must remain None");
    assert!(
        ctx.five_reset_unix.is_none(),
        "five_reset_unix must remain None"
    );
    assert!(ctx.seven_used.is_none(), "seven_used must remain None");
    assert!(
        ctx.extra_enabled.is_none(),
        "extra_enabled must remain None"
    );
}

// ── apply_cache_to_ctx: None resets_at string → None reset_unix ───────────

#[test]
fn apply_cache_none_resets_at_leaves_reset_unix_none() {
    use cc_myasl::cache::UsageWindowCache;
    let c = UsageCache {
        fetched_at: 0,
        five_hour: Some(UsageWindowCache {
            utilization: Some(10.0),
            resets_at: None,
        }),
        seven_day: None,
        extra_usage: None,
    };
    let mut ctx = RenderCtx {
        now_unix: 0,
        ..Default::default()
    };
    apply_cache_to_ctx(&c, &mut ctx);
    assert_eq!(ctx.five_used, Some(10.0));
    assert!(
        ctx.five_reset_unix.is_none(),
        "None resets_at must yield None reset_unix"
    );
}

// ── --print-config --debug: trace is populated and config JSON is valid ───

#[test]
fn print_config_debug_emits_trace_to_stderr() {
    // Verify the --print-config branch populates the Trace (so emit is meaningful)
    // and that print_config still produces valid JSON on stdout.
    let a = args::Args {
        print_config: true,
        debug: true,
        ..Default::default()
    };
    let mut trace = Trace::default();
    let config = cc_myasl::config::resolve(&a, &mut trace);
    // config_source must be set — this is what the trace serialises for diagnostics.
    assert!(
        trace.config_source.is_some(),
        "config_source must be populated by resolve for the debug trace"
    );
    // Calling emit(true) must not panic (actual stderr capture is in debug::tests).
    // We call emit with force=true here just to exercise the code path.
    trace.emit(true);
    // Stdout output must remain valid JSON regardless of debug flag.
    let output = cc_myasl::config::print_config(&config);
    let v: serde_json::Value =
        serde_json::from_str(&output).expect("print_config must remain valid JSON");
    assert!(
        v.get("$schema").is_some(),
        "$schema must be present in print_config output"
    );
}

// ── render-mode invariant: all-None ctx renders without panic ─────────────

#[test]
fn render_and_emit_all_none_ctx_does_not_panic() {
    let ctx = RenderCtx {
        now_unix: 0,
        ..Default::default()
    };
    let a = args::parse(&[]);
    let mut trace = Trace::default();
    let config = cc_myasl::config::resolve(&a, &mut trace);
    let _ = cc_myasl::config::render::render(&config, &ctx);
}

// ── config_uses_git ───────────────────────────────────────────────────────

fn make_config_with_template(template: &str) -> cc_myasl::config::Config {
    use cc_myasl::config::{Config, Line, Segment, TemplateSegment};
    Config {
        schema_url: None,
        lines: vec![Line {
            separator: " ".to_owned(),
            segments: vec![Segment::Template(TemplateSegment::new(template))],
        }],
    }
}

#[test]
fn config_uses_git_returns_true_with_git_placeholder() {
    let cfg = make_config_with_template("{git_branch}");
    assert!(config_uses_git(&cfg));
}

#[test]
fn config_uses_git_returns_true_for_any_git_placeholder() {
    for tmpl in &[
        "{git_root}",
        "{git_changes}",
        "{git_staged}",
        "{git_unstaged}",
        "{git_untracked}",
        "{git_status_clean}",
    ] {
        let cfg = make_config_with_template(tmpl);
        assert!(config_uses_git(&cfg), "should return true for {tmpl}");
    }
}

#[test]
fn config_uses_git_returns_false_without_git_placeholder() {
    let cfg = make_config_with_template("{model} {five_used}");
    assert!(!config_uses_git(&cfg));
}

#[test]
fn config_uses_git_empty_config_returns_false() {
    use cc_myasl::config::Config;
    let cfg = Config {
        schema_url: None,
        lines: vec![],
    };
    assert!(!config_uses_git(&cfg));
}
