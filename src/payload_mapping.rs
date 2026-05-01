//! Mapping from `Payload` → `RenderCtx` primitives.
//!
//! Pure data transformation only — no I/O, no HTTP, no cache.
//! Hot-path control flow (OAuth fallback, cache decisions) stays in `main.rs`.

use std::path::PathBuf;

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::payload::{
        ContextWindow, ContextWindowCurrentUsage, Cost, Model, Payload, Vim, Workspace,
    };

    const NOW: u64 = 1_746_000_000;

    fn full_payload() -> Payload {
        let src = include_str!("../tests/fixtures/full-payload.json");
        crate::payload::parse(src.as_bytes()).expect("full-payload fixture")
    }

    // ── happy path: full fixture ──────────────────────────────────────────────

    #[test]
    fn build_render_ctx_full_payload() {
        let p = full_payload();
        let ctx = build_render_ctx(&p, NOW);

        assert_eq!(ctx.now_unix, NOW);

        // Phase-1 fields
        assert_eq!(ctx.model.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(ctx.cwd, Some(PathBuf::from("/Users/test/myproject")));

        // model_id
        assert_eq!(ctx.model_id.as_deref(), Some("claude-sonnet-4-6"));

        // session metadata
        assert_eq!(ctx.version.as_deref(), Some("1.2.3"));
        assert_eq!(ctx.session_id.as_deref(), Some("session-abc123"));
        assert_eq!(ctx.session_name.as_deref(), Some("My Project Session"));
        assert_eq!(ctx.output_style.as_deref(), Some("verbose"));
        assert_eq!(ctx.effort_level.as_deref(), Some("high"));
        assert_eq!(ctx.thinking_enabled, Some(true));
        assert_eq!(ctx.vim_mode.as_deref(), Some("normal"));
        assert_eq!(ctx.agent_name.as_deref(), Some("code-agent"));
        assert_eq!(ctx.exceeds_200k, Some(false));

        // cost / session clock
        assert_eq!(ctx.cost_usd, Some(0.42));
        assert_eq!(ctx.total_duration_ms, Some(7_380_000));
        assert_eq!(ctx.api_duration_ms, Some(6_900_000));
        assert_eq!(ctx.lines_added, Some(150));
        assert_eq!(ctx.lines_removed, Some(30));

        // token totals
        assert_eq!(ctx.tokens_input_total, Some(12_345));
        assert_eq!(ctx.tokens_output_total, Some(6_789));

        // current-turn tokens
        assert_eq!(ctx.tokens_input, Some(4_096));
        assert_eq!(ctx.tokens_output, Some(512));
        assert_eq!(ctx.tokens_cache_creation, Some(1_024));
        assert_eq!(ctx.tokens_cache_read, Some(2_048));

        // context window
        assert_eq!(ctx.context_size, Some(200_000));
        assert_eq!(ctx.context_used_pct, Some(23.5));
        assert_eq!(ctx.context_remaining_pct, Some(76.5));

        // workspace
        assert_eq!(
            ctx.project_dir,
            Some(PathBuf::from("/Users/test/myproject"))
        );
        assert_eq!(ctx.added_dirs_count, Some(2));
        assert_eq!(
            ctx.workspace_git_worktree.as_deref(),
            Some("feature-branch")
        );

        // worktree
        assert_eq!(ctx.worktree_name.as_deref(), Some("feature-branch"));
        assert_eq!(
            ctx.worktree_path,
            Some(PathBuf::from("/Users/test/worktrees/feature-branch"))
        );
        assert_eq!(
            ctx.worktree_branch.as_deref(),
            Some("feature/add-placeholders")
        );
        assert_eq!(
            ctx.worktree_original_cwd,
            Some(PathBuf::from("/Users/test/myproject"))
        );
        assert_eq!(ctx.worktree_original_branch.as_deref(), Some("main"));

        // git fields remain None (Task 10)
        assert!(ctx.git_branch.is_none());
        assert!(ctx.git_root.is_none());
    }

    // ── empty payload: all new fields None ───────────────────────────────────

    #[test]
    fn build_render_ctx_empty_payload() {
        let p = Payload::default();
        let ctx = build_render_ctx(&p, NOW);

        assert_eq!(ctx.now_unix, NOW);
        assert!(ctx.model.is_none());
        assert!(ctx.model_id.is_none());
        assert!(ctx.cwd.is_none());
        assert!(ctx.version.is_none());
        assert!(ctx.session_id.is_none());
        assert!(ctx.session_name.is_none());
        assert!(ctx.output_style.is_none());
        assert!(ctx.effort_level.is_none());
        assert!(ctx.thinking_enabled.is_none());
        assert!(ctx.vim_mode.is_none());
        assert!(ctx.agent_name.is_none());
        assert!(ctx.exceeds_200k.is_none());
        assert!(ctx.cost_usd.is_none());
        assert!(ctx.total_duration_ms.is_none());
        assert!(ctx.api_duration_ms.is_none());
        assert!(ctx.lines_added.is_none());
        assert!(ctx.lines_removed.is_none());
        assert!(ctx.tokens_input_total.is_none());
        assert!(ctx.tokens_output_total.is_none());
        assert!(ctx.tokens_input.is_none());
        assert!(ctx.tokens_output.is_none());
        assert!(ctx.tokens_cache_creation.is_none());
        assert!(ctx.tokens_cache_read.is_none());
        assert!(ctx.context_size.is_none());
        assert!(ctx.context_used_pct.is_none());
        assert!(ctx.context_remaining_pct.is_none());
        assert!(ctx.project_dir.is_none());
        assert!(ctx.added_dirs_count.is_none());
        assert!(ctx.workspace_git_worktree.is_none());
        assert!(ctx.worktree_name.is_none());
        assert!(ctx.worktree_path.is_none());
        assert!(ctx.worktree_branch.is_none());
        assert!(ctx.worktree_original_cwd.is_none());
        assert!(ctx.worktree_original_branch.is_none());
    }

    // ── partial payload: only some fields populated ───────────────────────────

    #[test]
    fn build_render_ctx_partial_payload() {
        let p = Payload {
            version: Some("2.0.0".into()),
            vim: Some(Vim {
                mode: Some("insert".into()),
            }),
            cost: Some(Cost {
                total_cost_usd: Some(1.5),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = build_render_ctx(&p, NOW);

        assert_eq!(ctx.version.as_deref(), Some("2.0.0"));
        assert_eq!(ctx.vim_mode.as_deref(), Some("insert"));
        assert_eq!(ctx.cost_usd, Some(1.5));

        // unset fields remain None
        assert!(ctx.model.is_none());
        assert!(ctx.session_id.is_none());
        assert!(ctx.agent_name.is_none());
        assert!(ctx.tokens_input.is_none());
    }

    // ── preserves Phase-1 fields ──────────────────────────────────────────────

    #[test]
    fn build_render_ctx_preserves_phase1_fields() {
        let p = Payload {
            model: Some(Model {
                display_name: Some("claude-opus-4".into()),
                id: Some("claude-opus-4-20250514".into()),
            }),
            workspace: Some(Workspace {
                current_dir: Some("/home/alice/proj".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = build_render_ctx(&p, NOW);

        // Phase-1 fields
        assert_eq!(ctx.model.as_deref(), Some("claude-opus-4"));
        assert_eq!(ctx.cwd, Some(PathBuf::from("/home/alice/proj")));

        // Phase-2 model_id
        assert_eq!(ctx.model_id.as_deref(), Some("claude-opus-4-20250514"));

        // Rate-limit fields stay None (set by caller)
        assert!(ctx.five_used.is_none());
        assert!(ctx.seven_used.is_none());
        assert!(ctx.five_reset_unix.is_none());
        assert!(ctx.seven_reset_unix.is_none());
    }

    // ── cwd fallback: top-level cwd used when workspace.current_dir absent ───

    #[test]
    fn build_render_ctx_cwd_fallback() {
        let p = Payload {
            cwd: Some("/top/level/cwd".into()),
            workspace: Some(Workspace {
                current_dir: None,
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = build_render_ctx(&p, NOW);
        assert_eq!(ctx.cwd, Some(PathBuf::from("/top/level/cwd")));
    }

    // ── workspace.current_dir wins over top-level cwd ─────────────────────────

    #[test]
    fn build_render_ctx_workspace_current_dir_wins() {
        let p = Payload {
            cwd: Some("/top/level/cwd".into()),
            workspace: Some(Workspace {
                current_dir: Some("/workspace/dir".into()),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = build_render_ctx(&p, NOW);
        assert_eq!(ctx.cwd, Some(PathBuf::from("/workspace/dir")));
    }

    // ── added_dirs_count: empty vec → 0, absent → None ───────────────────────

    #[test]
    fn build_render_ctx_added_dirs_count_empty_vec_is_zero() {
        let p = Payload {
            workspace: Some(Workspace {
                added_dirs: Some(vec![]),
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = build_render_ctx(&p, NOW);
        assert_eq!(ctx.added_dirs_count, Some(0));
    }

    #[test]
    fn build_render_ctx_added_dirs_count_absent_is_none() {
        let p = Payload {
            workspace: Some(Workspace {
                added_dirs: None,
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = build_render_ctx(&p, NOW);
        assert!(ctx.added_dirs_count.is_none());
    }

    // ── context_window: current_usage absent → token fields None ─────────────

    #[test]
    fn build_render_ctx_current_usage_absent_token_fields_none() {
        let p = Payload {
            context_window: Some(ContextWindow {
                context_window_size: Some(100_000),
                current_usage: None,
                ..Default::default()
            }),
            ..Default::default()
        };
        let ctx = build_render_ctx(&p, NOW);
        assert_eq!(ctx.context_size, Some(100_000));
        assert!(ctx.tokens_input.is_none());
        assert!(ctx.tokens_output.is_none());
        assert!(ctx.tokens_cache_creation.is_none());
        assert!(ctx.tokens_cache_read.is_none());
    }

    // ── worktree absent → all worktree fields None ────────────────────────────

    #[test]
    fn build_render_ctx_worktree_absent_all_none() {
        let p = Payload {
            worktree: None,
            ..Default::default()
        };
        let ctx = build_render_ctx(&p, NOW);
        assert!(ctx.worktree_name.is_none());
        assert!(ctx.worktree_path.is_none());
        assert!(ctx.worktree_branch.is_none());
        assert!(ctx.worktree_original_cwd.is_none());
        assert!(ctx.worktree_original_branch.is_none());
    }

    // ── full context_window with current_usage ────────────────────────────────

    #[test]
    fn build_render_ctx_full_context_window() {
        let p = Payload {
            context_window: Some(ContextWindow {
                total_input_tokens: Some(5_000),
                total_output_tokens: Some(3_000),
                context_window_size: Some(200_000),
                used_percentage: Some(50.0),
                remaining_percentage: Some(50.0),
                current_usage: Some(ContextWindowCurrentUsage {
                    input_tokens: Some(100),
                    output_tokens: Some(200),
                    cache_creation_input_tokens: Some(300),
                    cache_read_input_tokens: Some(400),
                }),
            }),
            ..Default::default()
        };
        let ctx = build_render_ctx(&p, NOW);
        assert_eq!(ctx.tokens_input_total, Some(5_000));
        assert_eq!(ctx.tokens_output_total, Some(3_000));
        assert_eq!(ctx.context_size, Some(200_000));
        assert_eq!(ctx.context_used_pct, Some(50.0));
        assert_eq!(ctx.context_remaining_pct, Some(50.0));
        assert_eq!(ctx.tokens_input, Some(100));
        assert_eq!(ctx.tokens_output, Some(200));
        assert_eq!(ctx.tokens_cache_creation, Some(300));
        assert_eq!(ctx.tokens_cache_read, Some(400));
    }
}
