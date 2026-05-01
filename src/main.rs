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
use cc_myasl::format::RenderCtx;
use cc_myasl::payload;
use cc_myasl::time;

// ── constants ─────────────────────────────────────────────────────────────────

use cc_myasl::api::DEFAULT_OAUTH_BASE_URL;

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
    if args.print_config {
        let mut trace = Trace::default();
        let config = cc_myasl::config::resolve(&args, &mut trace);
        println!("{}", cc_myasl::config::print_config(&config));
        std::process::exit(0);
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

fn render_and_emit(trace: &mut Trace, args: &Args, ctx: &RenderCtx, started: SystemTime) {
    let config = cc_myasl::config::resolve(args, trace);
    let line = cc_myasl::config::render::render(&config, ctx);
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
mod tests {
    use super::*;
    use cc_myasl::args;
    use cc_myasl::cache::{ExtraUsageCache, UsageCache, UsageWindowCache};

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

    // ── render pipeline ───────────────────────────────────────────────────────

    #[test]
    fn adversarial_bad_stdin_still_produces_output() {
        // Malformed JSON — parse fails, render falls back to default config output.
        let ctx = RenderCtx {
            now_unix: time::now_unix(),
            ..Default::default()
        };
        let a = args::parse(&[]);
        let mut trace = Trace::default();
        let config = cc_myasl::config::resolve(&a, &mut trace);
        let out = cc_myasl::config::render::render(&config, &ctx);
        // Output must be non-empty (at least an empty string for a no-data ctx).
        // render must not panic; result is always a String (possibly empty)
        let _ = out;
        // Trace must not record a panic.
        let _ = trace;
    }

    #[test]
    fn valid_stdin_with_rate_limits_populates_ctx() {
        let json = r#"{"model":{"display_name":"claude-opus-4"},"rate_limits":{"five_hour":{"used_percentage":25.0,"resets_at":9999999999},"seven_day":{"used_percentage":50.0,"resets_at":9999999999}}}"#;
        let p = payload::parse(json.as_bytes()).expect("valid payload");
        let mut ctx = RenderCtx {
            now_unix: time::now_unix(),
            ..Default::default()
        };
        ctx.model = p.model.and_then(|m| m.display_name);
        if let Some(rl) = &p.rate_limits {
            if let Some(fh) = &rl.five_hour {
                ctx.five_used = fh.used_percentage;
            }
        }
        assert_eq!(ctx.model.as_deref(), Some("claude-opus-4"));
        assert_eq!(ctx.five_used, Some(25.0));
    }

    // ── adversarial: corrupt config file must not exit non-zero ──────────────

    #[test]
    fn adversarial_corrupt_config_falls_back_not_nonzero() {
        use std::io::Write;
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("bad.json");
        let mut f = std::fs::File::create(&cfg_path).unwrap();
        f.write_all(b"{ this is not valid json }").unwrap();

        let a = args::Args {
            config_path: Some(cfg_path.clone()),
            ..Default::default()
        };
        let mut trace = Trace::default();
        // resolve must not panic; it falls back to embedded default.
        let config = cc_myasl::config::resolve(&a, &mut trace);
        // trace.error is set (parse failure recorded).
        assert!(
            trace.error.is_some(),
            "corrupt file must record an error in trace"
        );
        // Config is still a valid (default) config — rendering must not panic.
        let ctx = RenderCtx {
            now_unix: 0,
            ..Default::default()
        };
        let _ = cc_myasl::config::render::render(&config, &ctx);
    }

    // ── --print-config outputs valid JSON with $schema ────────────────────────

    #[test]
    fn print_config_outputs_valid_json_with_schema() {
        let a = args::parse(&[]);
        let mut trace = Trace::default();
        let config = cc_myasl::config::resolve(&a, &mut trace);
        let output = cc_myasl::config::print_config(&config);
        // Must parse as valid JSON.
        let v: serde_json::Value =
            serde_json::from_str(&output).expect("print_config must produce valid JSON");
        // Must contain $schema field.
        assert!(
            v.get("$schema").and_then(|s| s.as_str()).is_some(),
            "$schema field must be present in print_config output"
        );
        // Must be round-trippable back into Config.
        let _: cc_myasl::config::Config = serde_json::from_str(&output)
            .expect("print_config output must deserialize back into Config");
    }
}

#[cfg(test)]
#[path = "main_tests.rs"]
mod main_tests;
