//! Mapping from `Payload` → `RenderCtx` primitives.
//!
//! Pure data transformation only — no I/O, no HTTP, no cache.
//! Hot-path control flow (OAuth fallback, cache decisions) stays in `main.rs`.

use std::path::{Path, PathBuf};

use crate::format::RenderCtx;
use crate::payload::Payload;

/// Build a `RenderCtx` from a parsed `Payload`.
///
/// `now_unix` is the current time, supplied by the caller so tests can pin it.
/// All Phase-1 fields (model, cwd, five_*, seven_*) are populated here;
/// rate_limit fields are left None — the caller fills them after deciding
/// whether to use stdin or the OAuth fallback.
pub fn build_render_ctx(payload: &Payload, now_unix: u64) -> RenderCtx {
    let model = payload.model.as_ref().and_then(|m| m.display_name.clone());
    let model_id = payload.model.as_ref().and_then(|m| m.id.clone());

    // Prefer workspace.current_dir; fall back to top-level cwd.
    let cwd = payload
        .workspace
        .as_ref()
        .and_then(|w| w.current_dir.as_ref())
        .or(payload.cwd.as_ref())
        .map(PathBuf::from);

    let project_dir = payload
        .workspace
        .as_ref()
        .and_then(|w| w.project_dir.as_ref())
        .map(PathBuf::from);

    let added_dirs_count = payload
        .workspace
        .as_ref()
        .and_then(|w| w.added_dirs.as_ref())
        .map(|v| v.len() as u64);

    let workspace_git_worktree = payload
        .workspace
        .as_ref()
        .and_then(|w| w.git_worktree.clone());

    let cost_usd = payload.cost.as_ref().and_then(|c| c.total_cost_usd);
    let total_duration_ms = payload.cost.as_ref().and_then(|c| c.total_duration_ms);
    let api_duration_ms = payload.cost.as_ref().and_then(|c| c.total_api_duration_ms);
    let lines_added = payload.cost.as_ref().and_then(|c| c.total_lines_added);
    let lines_removed = payload.cost.as_ref().and_then(|c| c.total_lines_removed);

    let tokens_input_total = payload
        .context_window
        .as_ref()
        .and_then(|cw| cw.total_input_tokens);
    let tokens_output_total = payload
        .context_window
        .as_ref()
        .and_then(|cw| cw.total_output_tokens);
    let context_size = payload
        .context_window
        .as_ref()
        .and_then(|cw| cw.context_window_size);
    let context_used_pct = payload
        .context_window
        .as_ref()
        .and_then(|cw| cw.used_percentage);
    let context_remaining_pct = payload
        .context_window
        .as_ref()
        .and_then(|cw| cw.remaining_percentage);

    let tokens_input = payload
        .context_window
        .as_ref()
        .and_then(|cw| cw.current_usage.as_ref())
        .and_then(|cu| cu.input_tokens);
    let tokens_output = payload
        .context_window
        .as_ref()
        .and_then(|cw| cw.current_usage.as_ref())
        .and_then(|cu| cu.output_tokens);
    let tokens_cache_creation = payload
        .context_window
        .as_ref()
        .and_then(|cw| cw.current_usage.as_ref())
        .and_then(|cu| cu.cache_creation_input_tokens);
    let tokens_cache_read = payload
        .context_window
        .as_ref()
        .and_then(|cw| cw.current_usage.as_ref())
        .and_then(|cu| cu.cache_read_input_tokens);

    let worktree_name = payload.worktree.as_ref().and_then(|wt| wt.name.clone());
    let worktree_path = payload
        .worktree
        .as_ref()
        .and_then(|wt| wt.path.as_ref())
        .map(PathBuf::from);
    let worktree_branch = payload.worktree.as_ref().and_then(|wt| wt.branch.clone());
    let worktree_original_cwd = payload
        .worktree
        .as_ref()
        .and_then(|wt| wt.original_cwd.as_ref())
        .map(PathBuf::from);
    let worktree_original_branch = payload
        .worktree
        .as_ref()
        .and_then(|wt| wt.original_branch.clone());

    // Rate-limit fields left None — populated by caller after oauth/cache decision.
    RenderCtx {
        now_unix,
        model,
        model_id,
        cwd,
        version: payload.version.clone(),
        session_id: payload.session_id.clone(),
        session_name: payload.session_name.clone(),
        output_style: payload.output_style.as_ref().and_then(|o| o.name.clone()),
        effort_level: payload.effort.as_ref().and_then(|e| e.level.clone()),
        thinking_enabled: payload.thinking.as_ref().and_then(|t| t.enabled),
        vim_mode: payload.vim.as_ref().and_then(|v| v.mode.clone()),
        agent_name: payload.agent.as_ref().and_then(|a| a.name.clone()),
        exceeds_200k: payload.exceeds_200k_tokens,
        cost_usd,
        total_duration_ms,
        api_duration_ms,
        lines_added,
        lines_removed,
        tokens_input_total,
        tokens_output_total,
        tokens_input,
        tokens_output,
        tokens_cache_creation,
        tokens_cache_read,
        context_size,
        context_used_pct,
        context_remaining_pct,
        project_dir,
        added_dirs_count,
        workspace_git_worktree,
        worktree_name,
        worktree_path,
        worktree_branch,
        worktree_original_cwd,
        worktree_original_branch,
        // git fields populated by Task 10
        ..Default::default()
    }
}

/// Populate the git-related fields of `ctx` by discovering the git repo at `cwd`.
///
/// Always pass the current working directory as `cwd`. Never pass `worktree.path` —
/// gix discovery walks parent directories, so `cwd` is correct in both worktree
/// and non-worktree cases.
///
/// Note: a false-positive trigger can occur when a config template contains `{{git_`
/// (escaped braces that render as the literal text `{git_`). This causes a wasted
/// ~5ms gix discovery call. Accepted for Phase 2 — no over-engineering.
pub fn populate_git_ctx(ctx: &mut RenderCtx, cwd: &Path) {
    let Some(repo) = crate::git::discover(cwd) else {
        return;
    };
    ctx.git_branch = repo.branch();
    ctx.git_root = repo.root();
    if let Some(sc) = crate::git::counts(&repo) {
        ctx.git_changes_count = Some(sc.changes);
        ctx.git_staged_count = Some(sc.staged);
        ctx.git_unstaged_count = Some(sc.unstaged);
        ctx.git_untracked_count = Some(sc.untracked);
    }
}

#[cfg(test)]
#[path = "payload_mapping_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "payload_mapping_git_tests.rs"]
mod git_tests;
