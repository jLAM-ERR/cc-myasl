use super::*;
use std::path::PathBuf;

fn ctx_full() -> RenderCtx {
    RenderCtx {
        model: Some("claude-3-5-sonnet".to_owned()),
        cwd: Some(PathBuf::from("/home/user/projects/myapp")),
        five_used: Some(30.0),
        five_reset_unix: Some(3600), // 01:00 UTC
        seven_used: Some(60.0),
        seven_reset_unix: Some(86400 + 3600 * 2), // 02:00 UTC next day
        extra_enabled: Some(true),
        extra_used: Some(25.0),
        extra_limit: Some(100.0),
        extra_pct: Some(25.0),
        now_unix: 0,
        ..Default::default()
    }
}

fn ctx_empty() -> RenderCtx {
    RenderCtx::default()
}

// ── decoupling invariant ─────────────────────────────────────────────────

/// Walk `src/format/` recursively and assert no `.rs` file contains
/// forbidden imports.  New siblings under `src/format/placeholders/`
/// are covered automatically without editing this test.
#[test]
fn format_module_does_not_depend_on_api_cache_or_git() {
    use std::fs;
    use std::path::Path;

    // Split strings so this test source itself never contains the
    // literal banned patterns as a contiguous byte sequence.
    let forbidden: &[&str] = &[
        &["use crate", "::", "api"].concat(),
        &["use crate", "::", "cache"].concat(),
        &["use crate", "::", "git"].concat(),
        &["use crate", "::", "config"].concat(),
    ];

    fn walk(dir: &Path, forbidden: &[&str]) {
        let entries = fs::read_dir(dir).expect("read_dir failed");
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(&path, forbidden);
            } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                let src = fs::read_to_string(&path).unwrap_or_default();
                for pat in forbidden {
                    assert!(
                        !src.contains(*pat),
                        "{} has forbidden import: {}",
                        path.display(),
                        pat
                    );
                }
            }
        }
    }

    walk(Path::new("src/format"), forbidden);
}

// ── model ────────────────────────────────────────────────────────────────

#[test]
fn model_present() {
    let ctx = ctx_full();
    assert_eq!(
        render_placeholder("model", &ctx),
        Some("claude-3-5-sonnet".to_owned())
    );
}

#[test]
fn model_absent() {
    assert_eq!(render_placeholder("model", &ctx_empty()), None);
}

// ── cwd ──────────────────────────────────────────────────────────────────

#[test]
fn cwd_present_no_home_substitution() {
    let ctx = RenderCtx {
        cwd: Some(PathBuf::from("/tmp/project")),
        ..Default::default()
    };
    // HOME may or may not match /tmp; just check it returns Some non-empty.
    let result = render_placeholder("cwd", &ctx);
    assert!(result.is_some());
    assert!(!result.unwrap().is_empty());
}

#[test]
fn cwd_substitutes_home() {
    // Self-contained: set HOME to a known value, render, restore.  Joins
    // `creds::HOME_MUTEX` so we don't race siblings that also mutate HOME.
    // (creds tests `remove_var("HOME")` after their work, which would leave
    // an inherited-HOME read in this test seeing nothing.)
    let _guard = crate::creds::HOME_MUTEX.lock().unwrap();
    let saved = std::env::var("HOME").ok();
    unsafe { std::env::set_var("HOME", "/tmp/test-home") };
    let ctx = RenderCtx {
        cwd: Some(PathBuf::from("/tmp/test-home/projects/foo")),
        ..Default::default()
    };
    let result = render_placeholder("cwd", &ctx).unwrap();
    // Restore HOME before asserting so a panic doesn't leak state.
    match saved {
        Some(v) => unsafe { std::env::set_var("HOME", v) },
        None => unsafe { std::env::remove_var("HOME") },
    }
    assert_eq!(result, "~/projects/foo");
}

#[test]
fn cwd_absent() {
    assert_eq!(render_placeholder("cwd", &ctx_empty()), None);
}

// ── cwd_basename ─────────────────────────────────────────────────────────

#[test]
fn cwd_basename_present() {
    let ctx = ctx_full();
    assert_eq!(
        render_placeholder("cwd_basename", &ctx),
        Some("myapp".to_owned())
    );
}

#[test]
fn cwd_basename_absent() {
    assert_eq!(render_placeholder("cwd_basename", &ctx_empty()), None);
}

// ── five_used ────────────────────────────────────────────────────────────

#[test]
fn five_used_present() {
    let ctx = ctx_full();
    assert_eq!(
        render_placeholder("five_used", &ctx),
        Some("30.0".to_owned())
    );
}

#[test]
fn five_used_absent() {
    assert_eq!(render_placeholder("five_used", &ctx_empty()), None);
}

// ── five_left ────────────────────────────────────────────────────────────

#[test]
fn five_left_present() {
    let ctx = ctx_full(); // five_used=30.0 → left=70.0 → "70"
    assert_eq!(render_placeholder("five_left", &ctx), Some("70".to_owned()));
}

#[test]
fn five_left_absent() {
    assert_eq!(render_placeholder("five_left", &ctx_empty()), None);
}

// ── five_bar ─────────────────────────────────────────────────────────────

#[test]
fn five_bar_present() {
    // left=70% of width 10 → 7 filled + 3 empty
    let ctx = ctx_full();
    let result = render_placeholder("five_bar", &ctx).unwrap();
    assert!(result.starts_with('[') && result.ends_with(']'));
    // Block chars are 3 bytes each; total = 2 + 10*3 = 32 bytes
    assert_eq!(result.len(), 32);
}

#[test]
fn five_bar_absent() {
    assert_eq!(render_placeholder("five_bar", &ctx_empty()), None);
}

// ── five_bar_long ────────────────────────────────────────────────────────

#[test]
fn five_bar_long_present() {
    let ctx = ctx_full();
    let result = render_placeholder("five_bar_long", &ctx).unwrap();
    assert!(result.starts_with('[') && result.ends_with(']'));
}

#[test]
fn five_bar_long_absent() {
    assert_eq!(render_placeholder("five_bar_long", &ctx_empty()), None);
}

// ── five_reset_clock ─────────────────────────────────────────────────────

#[test]
fn five_reset_clock_present() {
    let ctx = ctx_full(); // five_reset_unix=3600 → "01:00" UTC
    assert_eq!(
        render_placeholder("five_reset_clock", &ctx),
        Some("01:00".to_owned())
    );
}

#[test]
fn five_reset_clock_absent() {
    assert_eq!(render_placeholder("five_reset_clock", &ctx_empty()), None);
}

// ── five_reset_in ────────────────────────────────────────────────────────

#[test]
fn five_reset_in_present() {
    let ctx = RenderCtx {
        five_reset_unix: Some(3700),
        now_unix: 100,
        ..Default::default()
    };
    // 3700 - 100 = 3600 s = 1h
    assert_eq!(
        render_placeholder("five_reset_in", &ctx),
        Some("1h".to_owned())
    );
}

#[test]
fn five_reset_in_absent() {
    assert_eq!(render_placeholder("five_reset_in", &ctx_empty()), None);
}

// ── five_color ───────────────────────────────────────────────────────────

#[test]
fn five_color_present_green() {
    // five_used=30.0 → left=70.0 → Green
    let ctx = ctx_full();
    assert_eq!(
        render_placeholder("five_color", &ctx),
        Some("\x1b[32m".to_owned())
    );
}

#[test]
fn five_color_absent() {
    assert_eq!(render_placeholder("five_color", &ctx_empty()), None);
}

// ── five_state ───────────────────────────────────────────────────────────

#[test]
fn five_state_present() {
    let ctx = ctx_full(); // left=70% → Green
    assert_eq!(
        render_placeholder("five_state", &ctx),
        Some("🟢".to_owned())
    );
}

#[test]
fn five_state_red() {
    let ctx = RenderCtx {
        five_used: Some(90.0), // left=10% → Red
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("five_state", &ctx),
        Some("🔴".to_owned())
    );
}

#[test]
fn five_state_absent() {
    assert_eq!(render_placeholder("five_state", &ctx_empty()), None);
}

// ── seven_used ───────────────────────────────────────────────────────────

#[test]
fn seven_used_present() {
    let ctx = ctx_full(); // seven_used=60.0
    assert_eq!(
        render_placeholder("seven_used", &ctx),
        Some("60.0".to_owned())
    );
}

#[test]
fn seven_used_absent() {
    assert_eq!(render_placeholder("seven_used", &ctx_empty()), None);
}

// ── seven_left ───────────────────────────────────────────────────────────

#[test]
fn seven_left_present() {
    let ctx = ctx_full(); // seven_used=60 → left=40
    assert_eq!(
        render_placeholder("seven_left", &ctx),
        Some("40".to_owned())
    );
}

#[test]
fn seven_left_absent() {
    assert_eq!(render_placeholder("seven_left", &ctx_empty()), None);
}

// ── seven_reset_clock ────────────────────────────────────────────────────

#[test]
fn seven_reset_clock_present() {
    let ctx = ctx_full(); // seven_reset_unix = 86400+7200 = 93600 → 02:00 UTC
    assert_eq!(
        render_placeholder("seven_reset_clock", &ctx),
        Some("02:00".to_owned())
    );
}

#[test]
fn seven_reset_clock_absent() {
    assert_eq!(render_placeholder("seven_reset_clock", &ctx_empty()), None);
}

// ── seven_reset_in ───────────────────────────────────────────────────────

#[test]
fn seven_reset_in_present() {
    let ctx = RenderCtx {
        seven_reset_unix: Some(3700),
        now_unix: 100,
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("seven_reset_in", &ctx),
        Some("1h".to_owned())
    );
}

#[test]
fn seven_reset_in_absent() {
    assert_eq!(render_placeholder("seven_reset_in", &ctx_empty()), None);
}

// ── seven_color ──────────────────────────────────────────────────────────

#[test]
fn seven_color_yellow() {
    // seven_used=60.0 → left=40.0 → Yellow (20 <= 40 < 50)
    let ctx = ctx_full();
    assert_eq!(
        render_placeholder("seven_color", &ctx),
        Some("\x1b[33m".to_owned())
    );
}

#[test]
fn seven_color_absent() {
    assert_eq!(render_placeholder("seven_color", &ctx_empty()), None);
}

// ── seven_state ──────────────────────────────────────────────────────────

#[test]
fn seven_state_yellow() {
    let ctx = ctx_full(); // left=40% → Yellow
    assert_eq!(
        render_placeholder("seven_state", &ctx),
        Some("🟡".to_owned())
    );
}

#[test]
fn seven_state_absent() {
    assert_eq!(render_placeholder("seven_state", &ctx_empty()), None);
}

// ── extra placeholders ───────────────────────────────────────────────────

#[test]
fn extra_left_present() {
    let ctx = ctx_full(); // limit=100, used=25 → left=75
    assert_eq!(
        render_placeholder("extra_left", &ctx),
        Some("75".to_owned())
    );
}

#[test]
fn extra_left_disabled() {
    let ctx = RenderCtx {
        extra_enabled: Some(false),
        extra_used: Some(25.0),
        extra_limit: Some(100.0),
        ..Default::default()
    };
    assert_eq!(render_placeholder("extra_left", &ctx), None);
}

#[test]
fn extra_left_absent() {
    assert_eq!(render_placeholder("extra_left", &ctx_empty()), None);
}

#[test]
fn extra_used_present() {
    let ctx = ctx_full(); // extra_used=25.0
    assert_eq!(
        render_placeholder("extra_used", &ctx),
        Some("25".to_owned())
    );
}

#[test]
fn extra_used_disabled() {
    let ctx = RenderCtx {
        extra_enabled: Some(false),
        extra_used: Some(25.0),
        ..Default::default()
    };
    assert_eq!(render_placeholder("extra_used", &ctx), None);
}

#[test]
fn extra_pct_present() {
    let ctx = ctx_full(); // extra_pct=25.0
    assert_eq!(
        render_placeholder("extra_pct", &ctx),
        Some("25.0".to_owned())
    );
}

#[test]
fn extra_pct_disabled() {
    let ctx = RenderCtx {
        extra_enabled: Some(false),
        extra_pct: Some(25.0),
        ..Default::default()
    };
    assert_eq!(render_placeholder("extra_pct", &ctx), None);
}

// ── state_icon ───────────────────────────────────────────────────────────

#[test]
fn state_icon_uses_minimum_left() {
    // five_used=30 → left=70 (Green), seven_used=60 → left=40 (Yellow)
    // min = 40 → Yellow
    let ctx = ctx_full();
    assert_eq!(
        render_placeholder("state_icon", &ctx),
        Some("🟡".to_owned())
    );
}

#[test]
fn state_icon_both_absent_is_unknown() {
    assert_eq!(
        render_placeholder("state_icon", &ctx_empty()),
        Some("⚪".to_owned())
    );
}

#[test]
fn state_icon_only_five_present() {
    let ctx = RenderCtx {
        five_used: Some(90.0), // left=10 → Red
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("state_icon", &ctx),
        Some("🔴".to_owned())
    );
}

// ── reset ────────────────────────────────────────────────────────────────

#[test]
fn reset_always_returns_ansi_reset() {
    assert_eq!(
        render_placeholder("reset", &ctx_empty()),
        Some("\x1b[0m".to_owned())
    );
}

// ── unknown placeholder ──────────────────────────────────────────────────

#[test]
fn unknown_placeholder_returns_none() {
    assert_eq!(render_placeholder("nonexistent_ph", &ctx_empty()), None);
    assert_eq!(render_placeholder("", &ctx_empty()), None);
}
