//! Entry point — orchestrates the render pipeline for cc-myasl.
//! Hard invariants: exit 0 always in render mode; token never on disk or logged.

use std::path::PathBuf;
use std::time::SystemTime;

use cc_myasl::api::{self, FetchOutcome};
use cc_myasl::args::Args;
use cc_myasl::cache::{
    self,
    lock::{Lock, LockError},
    ExtraUsageCache, UsageCache, UsageWindowCache,
};
use cc_myasl::check;
use cc_myasl::creds;
use cc_myasl::debug::Trace;
use cc_myasl::format::{self, RenderCtx};
use cc_myasl::payload;
use cc_myasl::time;

// ── constants ─────────────────────────────────────────────────────────────────

use cc_myasl::api::DEFAULT_OAUTH_BASE_URL;
#[allow(deprecated)]
use cc_myasl::format::DEFAULT_TEMPLATE;

const CACHE_TTL_SECS: u64 = 180;

// ── entry point ───────────────────────────────────────────────────────────────

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = cc_myasl::args::parse(&argv);

    if args.help {
        print_usage();
        std::process::exit(0);
    }
    if args.version {
        println!("cc-myasl {}", env!("CARGO_PKG_VERSION"));
        std::process::exit(0);
    }
    if args.check {
        std::process::exit(check::run());
    }

    run_render(&args);
    std::process::exit(0); // render mode ALWAYS exits 0
}

// ── render orchestrator ───────────────────────────────────────────────────────

fn run_render(args: &Args) {
    let started = SystemTime::now();
    let mut trace = Trace::default();
    let mut ctx = RenderCtx {
        now_unix: time::now_unix(),
        ..Default::default()
    };

    // 1. Parse stdin.
    let payload = match payload::parse(std::io::stdin()) {
        Ok(p) => p,
        Err(e) => {
            trace.error = Some(e.to_string());
            render_and_emit(&mut trace, args, &ctx, started);
            return;
        }
    };

    ctx.model = payload.model.and_then(|m| m.display_name);
    ctx.cwd = payload
        .workspace
        .and_then(|w| w.current_dir.map(PathBuf::from));

    // 2. Hot path: stdin has rate_limits.
    if let Some(rl) = &payload.rate_limits {
        if let Some(fh) = &rl.five_hour {
            ctx.five_used = fh.used_percentage;
            ctx.five_reset_unix = fh.resets_at;
        }
        if let Some(sd) = &rl.seven_day {
            ctx.seven_used = sd.used_percentage;
            ctx.seven_reset_unix = sd.resets_at;
        }
        trace.path = Some("stdin-rate-limits".into());
        render_and_emit(&mut trace, args, &ctx, started);
        return;
    }

    // 3. OAuth fallback path.
    trace.path = Some("oauth-fallback".into());
    let dir = cache::cache_dir();
    let _ = std::fs::create_dir_all(&dir);

    // 3a. Cache hit?
    if let Some(c) = cache::read(&dir) {
        if cache::is_fresh(&c, CACHE_TTL_SECS, ctx.now_unix) {
            apply_cache_to_ctx(&c, &mut ctx);
            trace.cache = Some("hit".into());
            render_and_emit(&mut trace, args, &ctx, started);
            return;
        }
    }
    trace.cache = Some("miss".into());

    // 3b. Lock active?
    let lock_path = dir.join("usage.lock");
    if let Some(lk) = cache::lock::read(&lock_path) {
        if lk.blocked_until > ctx.now_unix {
            if let Some(c) = cache::read_stale(&dir) {
                apply_cache_to_ctx(&c, &mut ctx);
                trace.cache = Some("stale".into());
            }
            render_and_emit(&mut trace, args, &ctx, started);
            return;
        }
    }

    // 3c. Read credentials.
    let token = match creds::read_token() {
        Ok(t) => t,
        Err(e) => {
            trace.error = Some(creds::redact_home(&e.to_string()));
            render_and_emit(&mut trace, args, &ctx, started);
            return;
        }
    };
    trace.token_fp = Some(creds::fingerprint(&token));

    // 3d. Fetch.
    let base_url = std::env::var("STATUSLINE_OAUTH_BASE_URL")
        .unwrap_or_else(|_| DEFAULT_OAUTH_BASE_URL.into());
    let outcome = api::fetch_usage(&token, &base_url);

    // 3e. Dispatch outcome.
    match outcome {
        Ok(FetchOutcome::Ok(resp)) => {
            trace.http = Some(200);
            let nc = build_cache_from_response(&resp, ctx.now_unix);
            let _ = cache::write(&dir, &nc);
            let _ = std::fs::remove_file(&lock_path);
            apply_cache_to_ctx(&nc, &mut ctx);
        }
        Ok(FetchOutcome::AuthFailed) => {
            trace.http = Some(401);
            let _ = cache::lock::write(
                &lock_path,
                &Lock {
                    blocked_until: ctx.now_unix + 3600,
                    error: LockError::AuthFailed,
                },
            );
            if let Some(c) = cache::read_stale(&dir) {
                apply_cache_to_ctx(&c, &mut ctx);
                trace.cache = Some("stale".into());
            }
        }
        Ok(FetchOutcome::RateLimited(d)) => {
            trace.http = Some(429);
            let secs = d.as_secs().max(300);
            let _ = cache::lock::write(
                &lock_path,
                &Lock {
                    blocked_until: ctx.now_unix + secs,
                    error: LockError::RateLimited,
                },
            );
            if let Some(c) = cache::read_stale(&dir) {
                apply_cache_to_ctx(&c, &mut ctx);
                trace.cache = Some("stale".into());
            }
        }
        Ok(FetchOutcome::ServerError) => {
            trace.http = Some(500);
            let secs = cache::backoff::next_lock_seconds(0, LockError::Network);
            let _ = cache::lock::write(
                &lock_path,
                &Lock {
                    blocked_until: ctx.now_unix + secs,
                    error: LockError::Network,
                },
            );
            if let Some(c) = cache::read_stale(&dir) {
                apply_cache_to_ctx(&c, &mut ctx);
                trace.cache = Some("stale".into());
            }
        }
        Ok(FetchOutcome::TimedOut) | Err(_) => {
            let secs = cache::backoff::next_lock_seconds(0, LockError::Network);
            let _ = cache::lock::write(
                &lock_path,
                &Lock {
                    blocked_until: ctx.now_unix + secs,
                    error: LockError::Network,
                },
            );
            if let Some(c) = cache::read_stale(&dir) {
                apply_cache_to_ctx(&c, &mut ctx);
                trace.cache = Some("stale".into());
            }
        }
    }

    render_and_emit(&mut trace, args, &ctx, started);
}

// ── helpers ───────────────────────────────────────────────────────────────────

fn apply_cache_to_ctx(c: &UsageCache, ctx: &mut RenderCtx) {
    if let Some(fh) = &c.five_hour {
        ctx.five_used = fh.utilization;
        ctx.five_reset_unix = fh.resets_at.as_deref().and_then(time::iso_to_unix);
    }
    if let Some(sd) = &c.seven_day {
        ctx.seven_used = sd.utilization;
        ctx.seven_reset_unix = sd.resets_at.as_deref().and_then(time::iso_to_unix);
    }
    if let Some(eu) = &c.extra_usage {
        ctx.extra_enabled = eu.is_enabled;
        ctx.extra_used = eu.used_credits;
        ctx.extra_limit = eu.monthly_limit;
        ctx.extra_pct = eu.utilization;
    }
}

fn build_cache_from_response(r: &api::UsageResponse, now: u64) -> UsageCache {
    UsageCache {
        fetched_at: now,
        five_hour: r.five_hour.as_ref().map(|w| UsageWindowCache {
            utilization: w.utilization,
            resets_at: w.resets_at.clone(),
        }),
        seven_day: r.seven_day.as_ref().map(|w| UsageWindowCache {
            utilization: w.utilization,
            resets_at: w.resets_at.clone(),
        }),
        extra_usage: r.extra_usage.as_ref().map(|e| ExtraUsageCache {
            is_enabled: e.is_enabled,
            monthly_limit: e.monthly_limit,
            used_credits: e.used_credits,
            utilization: e.utilization,
        }),
    }
}

/// Resolve template: `--format` > `STATUSLINE_FORMAT` env > `--template` > built-in default.
#[allow(deprecated)]
fn resolve_template(args: &Args) -> String {
    if let Some(f) = &args.format {
        return f.clone();
    }
    if let Ok(s) = std::env::var("STATUSLINE_FORMAT") {
        if !s.is_empty() {
            return s;
        }
    }
    if let Some(name) = &args.template_name {
        return format::lookup_template(name)
            .map(|s| s.to_string())
            .unwrap_or_else(|| DEFAULT_TEMPLATE.to_string());
    }
    DEFAULT_TEMPLATE.to_string()
}

#[allow(deprecated)]
fn render_and_emit(trace: &mut Trace, args: &Args, ctx: &RenderCtx, started: SystemTime) {
    // Task-7 forward-compat: if --config is set, use the structured renderer.
    // Full Task-8 wiring will replace resolve_template entirely.
    let line = if let Some(path) = &args.config_path {
        match cc_myasl::config::from_file(path) {
            Ok(cfg) => cc_myasl::config::render::render(&cfg, ctx),
            Err(_) => format::render(&resolve_template(args), ctx),
        }
    } else {
        format::render(&resolve_template(args), ctx)
    };
    println!("{}", line);
    trace.took_ms = SystemTime::now()
        .duration_since(started)
        .ok()
        .map(|d| d.as_millis() as u64);
    trace.emit(args.debug);
}

fn print_usage() {
    eprintln!(
        "cc-myasl — Claude Code status line with remaining 5h/7d quota\n\
         \n\
         USAGE: cc-myasl [OPTIONS]\n\
         \n\
         OPTIONS:\n\
           --config <PATH>    Explicit config file (highest precedence)\n\
           --template <NAME>  Named built-in or user template\n\
           --print-config     Print resolved config as JSON and exit\n\
           --debug            Emit a JSON trace line to stderr\n\
           --check            Run setup-verification diagnostic\n\
           -V, --version      Print version and exit\n\
           -h, --help         Print this message and exit\n\
         \n\
         CONFIG PRECEDENCE (highest → lowest):\n\
           --config > --template > STATUSLINE_CONFIG > default file > embedded\n\
           Passing both --config and --template is allowed; --config wins.\n\
           User templates: <config_dir>/cc-myasl/templates/<name>.json\n\
           Built-ins: default, minimal, compact, bars, colored, emoji, emoji_verbose, verbose\n\
         \n\
         ENV:\n\
           STATUSLINE_CONFIG          Config file path (same as --config)\n\
           STATUSLINE_RED, STATUSLINE_YELLOW   Threshold percentages\n\
           STATUSLINE_DEBUG=1         Same as --debug\n\
           STATUSLINE_OAUTH_BASE_URL  Override the OAuth endpoint (testing)"
    );
}

// ── tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use cc_myasl::args;
    use cc_myasl::cache::{ExtraUsageCache, UsageCache, UsageWindowCache};
    use std::sync::Mutex;

    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    // ── apply_cache_to_ctx ────────────────────────────────────────────────────

    #[test]
    fn apply_cache_populates_ctx_correctly() {
        let c = UsageCache {
            fetched_at: 1_700_000_000,
            five_hour: Some(UsageWindowCache {
                utilization: Some(42.0),
                resets_at: Some("2026-04-26T18:00:00Z".to_string()),
            }),
            seven_day: Some(UsageWindowCache {
                utilization: Some(75.0),
                resets_at: Some("2026-04-30T00:00:00Z".to_string()),
            }),
            extra_usage: Some(ExtraUsageCache {
                is_enabled: Some(true),
                monthly_limit: Some(100.0),
                used_credits: Some(37.5),
                utilization: Some(37.5),
            }),
        };
        let mut ctx = RenderCtx {
            now_unix: 0,
            ..Default::default()
        };
        apply_cache_to_ctx(&c, &mut ctx);
        assert_eq!(ctx.five_used, Some(42.0));
        assert!(ctx.five_reset_unix.is_some());
        assert_eq!(ctx.seven_used, Some(75.0));
        assert!(ctx.seven_reset_unix.is_some());
        assert_eq!(ctx.extra_enabled, Some(true));
        assert_eq!(ctx.extra_used, Some(37.5));
        assert_eq!(ctx.extra_limit, Some(100.0));
        assert_eq!(ctx.extra_pct, Some(37.5));
    }

    // ── build_cache_from_response ─────────────────────────────────────────────

    #[test]
    fn build_cache_from_response_round_trip() {
        use cc_myasl::api::response::{ExtraUsage, UsageResponse, UsageWindow};
        let resp = UsageResponse {
            five_hour: Some(UsageWindow {
                utilization: Some(55.0),
                resets_at: Some("2026-04-26T18:00:00Z".to_string()),
            }),
            seven_day: Some(UsageWindow {
                utilization: Some(20.0),
                resets_at: Some("2026-04-30T00:00:00Z".to_string()),
            }),
            extra_usage: Some(ExtraUsage {
                is_enabled: Some(false),
                monthly_limit: Some(50.0),
                used_credits: Some(10.0),
                utilization: Some(20.0),
            }),
        };
        let cache = build_cache_from_response(&resp, 9999);
        assert_eq!(cache.fetched_at, 9999);
        assert_eq!(cache.five_hour.as_ref().unwrap().utilization, Some(55.0));
        assert_eq!(cache.seven_day.as_ref().unwrap().utilization, Some(20.0));
        assert_eq!(cache.extra_usage.as_ref().unwrap().used_credits, Some(10.0));
    }

    // ── DEFAULT_TEMPLATE render ───────────────────────────────────────────────

    #[test]
    fn default_template_render_full_and_empty_ctx() {
        // Full ctx: all optional segments present.
        let now = time::now_unix();
        let ctx = RenderCtx {
            model: Some("claude-opus-4".to_string()),
            five_used: Some(30.0),
            five_reset_unix: Some(now + 3600),
            seven_used: Some(60.0),
            seven_reset_unix: Some(now + 86400),
            now_unix: now,
            ..Default::default()
        };
        let out = format::render(DEFAULT_TEMPLATE, &ctx);
        assert!(out.contains("5h:"), "missing 5h: in {out:?}");
        assert!(out.contains("7d:"), "missing 7d: in {out:?}");
        assert!(out.contains("(resets "), "missing (resets in {out:?}");

        // Empty ctx: optional segments collapse.
        let empty = format::render(
            DEFAULT_TEMPLATE,
            &RenderCtx {
                now_unix: now,
                ..Default::default()
            },
        );
        assert!(!empty.contains("5h:"), "5h: must collapse");
        assert!(!empty.contains("7d:"), "7d: must collapse");
    }

    // ── adversarial / testable render pipeline ────────────────────────────────

    /// Lightweight render helper: parses `stdin_str`, builds ctx, renders template.
    fn render_from_str(stdin_str: &str, args: &Args) -> String {
        let mut ctx = RenderCtx {
            now_unix: time::now_unix(),
            ..Default::default()
        };
        if let Ok(p) = payload::parse(stdin_str.as_bytes()) {
            ctx.model = p.model.and_then(|m| m.display_name);
            ctx.cwd = p.workspace.and_then(|w| w.current_dir.map(PathBuf::from));
            if let Some(rl) = &p.rate_limits {
                if let Some(fh) = &rl.five_hour {
                    ctx.five_used = fh.used_percentage;
                    ctx.five_reset_unix = fh.resets_at;
                }
                if let Some(sd) = &rl.seven_day {
                    ctx.seven_used = sd.used_percentage;
                    ctx.seven_reset_unix = sd.resets_at;
                }
            }
        }
        format!("{}\n", format::render(&resolve_template(args), &ctx))
    }

    #[test]
    fn adversarial_bad_stdin_still_produces_output() {
        let _g = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("STATUSLINE_FORMAT");
        // Malformed JSON — parse fails, output is still non-empty (at least newline).
        let out = render_from_str("{ not valid json !!! }", &args::parse(&[]));
        assert!(!out.is_empty() && out.ends_with('\n'));
        // Empty stdin — same guarantee.
        assert!(!render_from_str("", &args::parse(&[])).is_empty());
    }

    #[test]
    fn adversarial_custom_format_preserved_on_error() {
        let _g = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("STATUSLINE_FORMAT");
        // args.format is no longer set via CLI; construct directly for this test.
        let a = args::Args {
            format: Some("hello world".to_string()),
            ..Default::default()
        };
        assert!(render_from_str("{ bad json }", &a).contains("hello world"));
    }

    #[test]
    fn valid_stdin_with_rate_limits_populates_ctx() {
        let _g = ENV_MUTEX.lock().unwrap();
        std::env::remove_var("STATUSLINE_FORMAT");
        let json = r#"{"model":{"display_name":"claude-opus-4"},"rate_limits":{"five_hour":{"used_percentage":25.0,"resets_at":9999999999},"seven_day":{"used_percentage":50.0,"resets_at":9999999999}}}"#;
        // args.format is no longer set via CLI; construct directly for this test.
        let a = args::Args {
            format: Some("{model} 5h:{five_left}%".to_string()),
            ..Default::default()
        };
        let out = render_from_str(json, &a);
        assert!(out.contains("claude-opus-4"), "model missing: {out:?}");
        assert!(out.contains("75%"), "five_left=75% missing: {out:?}");
    }
}
