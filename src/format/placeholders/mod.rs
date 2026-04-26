//! Render context + placeholder catalogue.
//!
//! `RenderCtx` deliberately contains ONLY primitive Option types and
//! stdlib types.  This module must never import from `crate::api` or
//! `crate::cache`.  The api→ctx and cache→ctx mappings live in `main.rs`.

use std::path::PathBuf;

use crate::format::thresholds::{classify, pick_color, pick_icon};
use crate::format::values::{bar, clock_local, countdown, percent_decimal, percent_int};

/// All data the renderer needs — primitives and stdlib types only.
///
/// Constructed fresh per invocation by `main.rs` from whichever data
/// source was used (stdin `rate_limits`, API response, or cache).
#[derive(Debug, Default)]
pub struct RenderCtx {
    pub model: Option<String>,
    pub cwd: Option<PathBuf>,
    pub five_used: Option<f64>,       // 0..=100
    pub five_reset_unix: Option<u64>, // unix seconds
    pub seven_used: Option<f64>,
    pub seven_reset_unix: Option<u64>,
    pub extra_enabled: Option<bool>,
    pub extra_used: Option<f64>,
    pub extra_limit: Option<f64>,
    pub extra_pct: Option<f64>,
    pub now_unix: u64,
}

/// Render a single placeholder `name` against `ctx`.
///
/// Returns `None` when the placeholder is unknown or the required
/// context fields are absent.  The renderer collapses any optional
/// segment that contains a `None`-returning placeholder.
pub fn render_placeholder(name: &str, ctx: &RenderCtx) -> Option<String> {
    match name {
        "model" => ctx.model.clone(),

        "cwd" => {
            let path = ctx.cwd.as_ref()?;
            let s = path.to_str()?;
            if s.is_empty() {
                return None;
            }
            let home = std::env::var("HOME").unwrap_or_default();
            if !home.is_empty() && s.starts_with(&home) {
                Some(format!("~{}", &s[home.len()..]))
            } else {
                Some(s.to_owned())
            }
        }

        "cwd_basename" => {
            let path = ctx.cwd.as_ref()?;
            let name = path.file_name()?.to_str()?;
            if name.is_empty() {
                None
            } else {
                Some(name.to_owned())
            }
        }

        // ── five-hour placeholders ────────────────────────────────────────────
        "five_used" => Some(percent_decimal(ctx.five_used?)),
        "five_left" => Some(percent_int(100.0 - ctx.five_used?)),
        "five_bar" => Some(bar(100.0 - ctx.five_used?, 10)),
        "five_bar_long" => Some(bar(100.0 - ctx.five_used?, 20)),
        "five_reset_clock" => Some(clock_local(ctx.five_reset_unix?)),
        "five_reset_in" => Some(countdown(ctx.five_reset_unix?, ctx.now_unix)),
        "five_color" => Some(pick_color(classify(Some(100.0 - ctx.five_used?))).to_owned()),
        "five_state" => Some(pick_icon(classify(Some(100.0 - ctx.five_used?))).to_owned()),

        // ── seven-day placeholders ────────────────────────────────────────────
        "seven_used" => Some(percent_decimal(ctx.seven_used?)),
        "seven_left" => Some(percent_int(100.0 - ctx.seven_used?)),
        "seven_bar" => Some(bar(100.0 - ctx.seven_used?, 10)),
        "seven_bar_long" => Some(bar(100.0 - ctx.seven_used?, 20)),
        "seven_reset_clock" => Some(clock_local(ctx.seven_reset_unix?)),
        "seven_reset_in" => Some(countdown(ctx.seven_reset_unix?, ctx.now_unix)),
        "seven_color" => Some(pick_color(classify(Some(100.0 - ctx.seven_used?))).to_owned()),
        "seven_state" => Some(pick_icon(classify(Some(100.0 - ctx.seven_used?))).to_owned()),

        // ── extra-usage placeholders ──────────────────────────────────────────
        "extra_left" => {
            if ctx.extra_enabled != Some(true) {
                return None;
            }
            Some(percent_int(ctx.extra_limit? - ctx.extra_used?))
        }

        "extra_used" => {
            if ctx.extra_enabled != Some(true) {
                return None;
            }
            Some(percent_int(ctx.extra_used?))
        }

        "extra_pct" => {
            if ctx.extra_enabled != Some(true) {
                return None;
            }
            Some(percent_decimal(ctx.extra_pct?))
        }

        // ── combined state ────────────────────────────────────────────────────
        "state_icon" => {
            let min_left = match (ctx.five_used, ctx.seven_used) {
                (Some(f), Some(s)) => Some(f64::min(100.0 - f, 100.0 - s)),
                (Some(f), None) => Some(100.0 - f),
                (None, Some(s)) => Some(100.0 - s),
                (None, None) => None,
            };
            Some(pick_icon(classify(min_left)).to_owned())
        }

        // ── ANSI reset ────────────────────────────────────────────────────────
        "reset" => Some("\x1b[0m".to_owned()),

        // Unknown placeholder → None (renderer will collapse optional blocks)
        _ => None,
    }
}

#[cfg(test)]
#[path = "tests.rs"]
mod tests;
