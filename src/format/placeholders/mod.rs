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
    // ── existing Phase-1 fields ───────────────────────────────────────────
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

    // ── Phase-2: Claude Code session / metadata ───────────────────────────
    pub model_id: Option<String>,
    pub version: Option<String>,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub output_style: Option<String>,
    pub effort_level: Option<String>,
    pub thinking_enabled: Option<bool>,
    pub vim_mode: Option<String>,
    pub agent_name: Option<String>,

    // ── Phase-2: cost / session clock ─────────────────────────────────────
    pub cost_usd: Option<f64>,
    pub total_duration_ms: Option<u64>,
    pub api_duration_ms: Option<u64>,
    pub lines_added: Option<u64>,
    pub lines_removed: Option<u64>,

    // ── Phase-2: token counters (session totals) ──────────────────────────
    pub tokens_input_total: Option<u64>,
    pub tokens_output_total: Option<u64>,

    // ── Phase-2: token counters (current turn) ────────────────────────────
    pub tokens_input: Option<u64>,
    pub tokens_output: Option<u64>,
    pub tokens_cache_creation: Option<u64>,
    pub tokens_cache_read: Option<u64>,

    // ── Phase-2: context window ───────────────────────────────────────────
    pub context_size: Option<u64>,
    pub context_used_pct: Option<f64>,
    pub context_remaining_pct: Option<f64>,
    pub exceeds_200k: Option<bool>,

    // ── Phase-2: workspace ────────────────────────────────────────────────
    pub project_dir: Option<PathBuf>,
    pub added_dirs_count: Option<u64>,
    pub workspace_git_worktree: Option<String>,

    // ── Phase-2: worktree ─────────────────────────────────────────────────
    pub worktree_name: Option<String>,
    pub worktree_path: Option<PathBuf>,
    pub worktree_branch: Option<String>,
    pub worktree_original_cwd: Option<PathBuf>,
    pub worktree_original_branch: Option<String>,

    // ── Phase-2: git (populated by git module in Task 10) ─────────────────
    pub git_branch: Option<String>,
    pub git_root: Option<PathBuf>,
    pub git_changes_count: Option<u64>,
    pub git_staged_count: Option<u64>,
    pub git_unstaged_count: Option<u64>,
    pub git_untracked_count: Option<u64>,
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

#[cfg(test)]
mod phase2_struct_tests {
    use super::RenderCtx;

    /// Sanity check: all 38 Phase-2 Option fields default to None.
    #[test]
    fn render_ctx_default_all_phase2_option_fields_are_none() {
        let c = RenderCtx::default();
        assert!(c.model_id.is_none() && c.version.is_none() && c.session_id.is_none());
        assert!(c.session_name.is_none() && c.output_style.is_none() && c.effort_level.is_none());
        assert!(c.thinking_enabled.is_none() && c.vim_mode.is_none() && c.agent_name.is_none());
        assert!(c.cost_usd.is_none() && c.total_duration_ms.is_none());
        assert!(
            c.api_duration_ms.is_none() && c.lines_added.is_none() && c.lines_removed.is_none()
        );
        assert!(c.tokens_input_total.is_none() && c.tokens_output_total.is_none());
        assert!(c.tokens_input.is_none() && c.tokens_output.is_none());
        assert!(c.tokens_cache_creation.is_none() && c.tokens_cache_read.is_none());
        assert!(c.context_size.is_none() && c.context_used_pct.is_none());
        assert!(c.context_remaining_pct.is_none() && c.exceeds_200k.is_none());
        assert!(c.project_dir.is_none() && c.added_dirs_count.is_none());
        assert!(c.workspace_git_worktree.is_none() && c.worktree_name.is_none());
        assert!(c.worktree_path.is_none() && c.worktree_branch.is_none());
        assert!(c.worktree_original_cwd.is_none() && c.worktree_original_branch.is_none());
        assert!(c.git_branch.is_none() && c.git_root.is_none());
        assert!(c.git_changes_count.is_none() && c.git_staged_count.is_none());
        assert!(c.git_unstaged_count.is_none() && c.git_untracked_count.is_none());
    }
}
