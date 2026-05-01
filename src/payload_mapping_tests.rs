use super::*;
use crate::payload::{
    Agent, ContextWindow, ContextWindowCurrentUsage, Cost, Effort, Model, OutputStyle, Payload,
    RateLimits, RateWindow, Thinking, Vim, Workspace, Worktree,
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
    assert_eq!(ctx.model.as_deref(), Some("claude-sonnet-4-6"));
    assert_eq!(ctx.cwd, Some(PathBuf::from("/Users/test/myproject")));
    assert_eq!(ctx.model_id.as_deref(), Some("claude-sonnet-4-6"));
    assert_eq!(ctx.version.as_deref(), Some("1.2.3"));
    assert_eq!(ctx.session_id.as_deref(), Some("session-abc123"));
    assert_eq!(ctx.session_name.as_deref(), Some("My Project Session"));
    assert_eq!(ctx.output_style.as_deref(), Some("verbose"));
    assert_eq!(ctx.effort_level.as_deref(), Some("high"));
    assert_eq!(ctx.thinking_enabled, Some(true));
    assert_eq!(ctx.vim_mode.as_deref(), Some("normal"));
    assert_eq!(ctx.agent_name.as_deref(), Some("code-agent"));
    assert_eq!(ctx.exceeds_200k, Some(false));
    assert_eq!(ctx.cost_usd, Some(0.42));
    assert_eq!(ctx.total_duration_ms, Some(7_380_000));
    assert_eq!(ctx.api_duration_ms, Some(6_900_000));
    assert_eq!(ctx.lines_added, Some(150));
    assert_eq!(ctx.lines_removed, Some(30));
    assert_eq!(ctx.tokens_input_total, Some(12_345));
    assert_eq!(ctx.tokens_output_total, Some(6_789));
    assert_eq!(ctx.tokens_input, Some(4_096));
    assert_eq!(ctx.tokens_output, Some(512));
    assert_eq!(ctx.tokens_cache_creation, Some(1_024));
    assert_eq!(ctx.tokens_cache_read, Some(2_048));
    assert_eq!(ctx.context_size, Some(200_000));
    assert_eq!(ctx.context_used_pct, Some(23.5));
    assert_eq!(ctx.context_remaining_pct, Some(76.5));
    assert_eq!(
        ctx.project_dir,
        Some(PathBuf::from("/Users/test/myproject"))
    );
    assert_eq!(ctx.added_dirs_count, Some(2));
    assert_eq!(
        ctx.workspace_git_worktree.as_deref(),
        Some("feature-branch")
    );
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
    assert!(ctx.git_branch.is_none());
    assert!(ctx.git_root.is_none());
}

// ── empty payload: all fields None ───────────────────────────────────────

#[test]
fn build_render_ctx_empty_payload() {
    let ctx = build_render_ctx(&Payload::default(), NOW);

    assert_eq!(ctx.now_unix, NOW);
    assert!(ctx.model.is_none() && ctx.model_id.is_none() && ctx.cwd.is_none());
    assert!(ctx.version.is_none() && ctx.session_id.is_none() && ctx.session_name.is_none());
    assert!(ctx.output_style.is_none() && ctx.effort_level.is_none());
    assert!(ctx.thinking_enabled.is_none() && ctx.vim_mode.is_none() && ctx.agent_name.is_none());
    assert!(ctx.exceeds_200k.is_none() && ctx.cost_usd.is_none());
    assert!(ctx.total_duration_ms.is_none() && ctx.api_duration_ms.is_none());
    assert!(ctx.lines_added.is_none() && ctx.lines_removed.is_none());
    assert!(ctx.tokens_input_total.is_none() && ctx.tokens_output_total.is_none());
    assert!(ctx.tokens_input.is_none() && ctx.tokens_output.is_none());
    assert!(ctx.tokens_cache_creation.is_none() && ctx.tokens_cache_read.is_none());
    assert!(ctx.context_size.is_none() && ctx.context_used_pct.is_none());
    assert!(ctx.context_remaining_pct.is_none());
    assert!(ctx.project_dir.is_none() && ctx.added_dirs_count.is_none());
    assert!(ctx.workspace_git_worktree.is_none());
    assert!(ctx.worktree_name.is_none() && ctx.worktree_path.is_none());
    assert!(ctx.worktree_branch.is_none() && ctx.worktree_original_cwd.is_none());
    assert!(ctx.worktree_original_branch.is_none());
}

// ── partial payload ────────────────────────────────────────────────────────

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
    assert!(ctx.model.is_none() && ctx.session_id.is_none());
    assert!(ctx.agent_name.is_none() && ctx.tokens_input.is_none());
}

// ── Phase-1 fields + rate-limit slots left empty ─────────────────────────

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
    assert_eq!(ctx.model.as_deref(), Some("claude-opus-4"));
    assert_eq!(ctx.cwd, Some(PathBuf::from("/home/alice/proj")));
    assert_eq!(ctx.model_id.as_deref(), Some("claude-opus-4-20250514"));
    assert!(ctx.five_used.is_none() && ctx.seven_used.is_none());
    assert!(ctx.five_reset_unix.is_none() && ctx.seven_reset_unix.is_none());
}

// ── cwd priority: workspace.current_dir > top-level cwd > absent ─────────

#[test]
fn build_render_ctx_cwd_priority() {
    // workspace.current_dir wins
    let p = Payload {
        cwd: Some("/top".into()),
        workspace: Some(Workspace {
            current_dir: Some("/ws".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(build_render_ctx(&p, NOW).cwd, Some(PathBuf::from("/ws")));

    // workspace present but current_dir None → fallback to top-level cwd
    let p2 = Payload {
        cwd: Some("/top".into()),
        workspace: Some(Workspace {
            current_dir: None,
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(build_render_ctx(&p2, NOW).cwd, Some(PathBuf::from("/top")));

    // no workspace at all → top-level cwd
    let p3 = Payload {
        cwd: Some("/top".into()),
        workspace: None,
        ..Default::default()
    };
    assert_eq!(build_render_ctx(&p3, NOW).cwd, Some(PathBuf::from("/top")));
}

// ── adversarial: empty-string cwd/version preserved (no filtering) ────────

#[test]
fn build_render_ctx_empty_strings_preserved() {
    let p = Payload {
        cwd: Some(String::new()),
        version: Some(String::new()),
        workspace: None,
        ..Default::default()
    };
    let ctx = build_render_ctx(&p, NOW);
    assert_eq!(ctx.cwd, Some(PathBuf::from("")));
    assert_eq!(ctx.version.as_deref(), Some(""));
}

// ── adversarial: workspace.current_dir="" wins over non-empty cwd ─────────

#[test]
fn build_render_ctx_empty_workspace_dir_wins_over_cwd() {
    let p = Payload {
        cwd: Some("/real/path".into()),
        workspace: Some(Workspace {
            current_dir: Some(String::new()),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(build_render_ctx(&p, NOW).cwd, Some(PathBuf::from("")));
}

// ── adversarial: added_dirs counts (0, 2, 1000) ───────────────────────────

#[test]
fn build_render_ctx_added_dirs_count_empty_vec_is_zero() {
    let p = Payload {
        workspace: Some(Workspace {
            added_dirs: Some(vec![]),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(build_render_ctx(&p, NOW).added_dirs_count, Some(0));
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
    assert!(build_render_ctx(&p, NOW).added_dirs_count.is_none());
}

#[test]
fn build_render_ctx_added_dirs_multiple_and_large() {
    let p2 = Payload {
        workspace: Some(Workspace {
            added_dirs: Some(vec!["a".into(), "b".into()]),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(build_render_ctx(&p2, NOW).added_dirs_count, Some(2));

    let big: Vec<String> = (0..1000).map(|i| format!("d{i}")).collect();
    let p3 = Payload {
        workspace: Some(Workspace {
            added_dirs: Some(big),
            ..Default::default()
        }),
        ..Default::default()
    };
    assert_eq!(build_render_ctx(&p3, NOW).added_dirs_count, Some(1000));
}

// ── context_window: current_usage absent/empty → per-turn tokens None ─────

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
    assert!(ctx.tokens_input.is_none() && ctx.tokens_output.is_none());
    assert!(ctx.tokens_cache_creation.is_none() && ctx.tokens_cache_read.is_none());
}

#[test]
fn build_render_ctx_current_usage_inner_all_none() {
    let p = Payload {
        context_window: Some(ContextWindow {
            current_usage: Some(ContextWindowCurrentUsage::default()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let ctx = build_render_ctx(&p, NOW);
    assert!(ctx.tokens_input.is_none() && ctx.tokens_output.is_none());
    assert!(ctx.tokens_cache_creation.is_none() && ctx.tokens_cache_read.is_none());
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

// ── adversarial: worktree absent → None; relative/nonexistent paths preserved

#[test]
fn build_render_ctx_worktree_paths_preserved_without_fs_check() {
    let p = Payload {
        worktree: Some(Worktree {
            path: Some("relative/wt".into()),
            original_cwd: Some("/nonexistent/path".into()),
            ..Default::default()
        }),
        ..Default::default()
    };
    let ctx = build_render_ctx(&p, NOW);
    assert_eq!(ctx.worktree_path, Some(PathBuf::from("relative/wt")));
    assert_eq!(
        ctx.worktree_original_cwd,
        Some(PathBuf::from("/nonexistent/path"))
    );
}

// ── adversarial: u64::MAX and f64 edge values pass through without change ──

#[test]
fn build_render_ctx_numeric_extremes() {
    let p = Payload {
        cost: Some(Cost {
            total_cost_usd: Some(0.0),
            total_lines_added: Some(u64::MAX),
            total_lines_removed: Some(u64::MAX),
            total_duration_ms: Some(u64::MAX),
            total_api_duration_ms: Some(u64::MAX),
        }),
        context_window: Some(ContextWindow {
            total_input_tokens: Some(u64::MAX),
            context_window_size: Some(u64::MAX),
            used_percentage: Some(1e-300),
            current_usage: Some(ContextWindowCurrentUsage {
                input_tokens: Some(u64::MAX),
                output_tokens: Some(u64::MAX),
                ..Default::default()
            }),
            ..Default::default()
        }),
        ..Default::default()
    };
    let ctx = build_render_ctx(&p, NOW);
    assert_eq!(ctx.cost_usd, Some(0.0));
    assert_eq!(ctx.lines_added, Some(u64::MAX));
    assert_eq!(ctx.lines_removed, Some(u64::MAX));
    assert_eq!(ctx.total_duration_ms, Some(u64::MAX));
    assert_eq!(ctx.api_duration_ms, Some(u64::MAX));
    assert_eq!(ctx.tokens_input_total, Some(u64::MAX));
    assert_eq!(ctx.context_size, Some(u64::MAX));
    assert_eq!(ctx.context_used_pct, Some(1e-300));
    assert_eq!(ctx.tokens_input, Some(u64::MAX));
    assert_eq!(ctx.tokens_output, Some(u64::MAX));
}

// ── adversarial: now_unix boundary values ────────────────────────────────

#[test]
fn build_render_ctx_now_unix_boundaries() {
    assert_eq!(build_render_ctx(&Payload::default(), 0).now_unix, 0);
    assert_eq!(
        build_render_ctx(&Payload::default(), u64::MAX).now_unix,
        u64::MAX
    );
}

// ── adversarial: unicode strings preserved ────────────────────────────────

#[test]
fn build_render_ctx_unicode_preserved() {
    let p = Payload {
        cwd: Some("/Users/тест/проект".into()),
        session_name: Some("名前のあるセッション".into()),
        version: Some("1.0.0-αβγ".into()),
        workspace: None,
        ..Default::default()
    };
    let ctx = build_render_ctx(&p, NOW);
    assert_eq!(ctx.cwd, Some(PathBuf::from("/Users/тест/проект")));
    assert_eq!(ctx.session_name.as_deref(), Some("名前のあるセッション"));
    assert_eq!(ctx.version.as_deref(), Some("1.0.0-αβγ"));
}

// ── adversarial: rate_limits in payload do not leak into RenderCtx ────────

#[test]
fn build_render_ctx_rate_limits_not_mapped() {
    let p = Payload {
        rate_limits: Some(RateLimits {
            five_hour: Some(RateWindow {
                used_percentage: Some(40.0),
                resets_at: Some(9999),
            }),
            seven_day: None,
        }),
        ..Default::default()
    };
    let ctx = build_render_ctx(&p, NOW);
    assert!(ctx.five_used.is_none() && ctx.seven_used.is_none());
    assert!(ctx.five_reset_unix.is_none() && ctx.seven_reset_unix.is_none());
}

// ── adversarial: model field independence ────────────────────────────────

#[test]
fn build_render_ctx_model_field_independence() {
    // display_name None, id Some
    let p1 = Payload {
        model: Some(Model {
            display_name: None,
            id: Some("id-only".into()),
        }),
        ..Default::default()
    };
    let ctx1 = build_render_ctx(&p1, NOW);
    assert!(ctx1.model.is_none());
    assert_eq!(ctx1.model_id.as_deref(), Some("id-only"));

    // display_name Some, id None
    let p2 = Payload {
        model: Some(Model {
            display_name: Some("name-only".into()),
            id: None,
        }),
        ..Default::default()
    };
    let ctx2 = build_render_ctx(&p2, NOW);
    assert_eq!(ctx2.model.as_deref(), Some("name-only"));
    assert!(ctx2.model_id.is_none());
}

// ── adversarial: all wrappers present but inner fields None ───────────────

#[test]
fn build_render_ctx_wrappers_present_inner_none() {
    let p = Payload {
        model: Some(Model::default()),
        output_style: Some(OutputStyle { name: None }),
        effort: Some(Effort { level: None }),
        thinking: Some(Thinking { enabled: None }),
        vim: Some(Vim { mode: None }),
        agent: Some(Agent { name: None }),
        worktree: Some(Worktree::default()),
        cost: Some(Cost::default()),
        context_window: Some(ContextWindow::default()),
        workspace: Some(Workspace::default()),
        ..Default::default()
    };
    let ctx = build_render_ctx(&p, NOW);
    assert!(ctx.model.is_none() && ctx.model_id.is_none());
    assert!(ctx.output_style.is_none() && ctx.effort_level.is_none());
    assert!(ctx.thinking_enabled.is_none() && ctx.vim_mode.is_none() && ctx.agent_name.is_none());
    assert!(ctx.worktree_name.is_none() && ctx.worktree_path.is_none());
    assert!(ctx.cost_usd.is_none() && ctx.lines_added.is_none());
    assert!(ctx.context_size.is_none() && ctx.tokens_input.is_none());
    assert!(ctx.project_dir.is_none() && ctx.added_dirs_count.is_none());
    assert!(ctx.workspace_git_worktree.is_none());
}

// ── adversarial: git fields always None — caller populates in Task 10 ─────

#[test]
fn build_render_ctx_git_fields_always_none() {
    let ctx = build_render_ctx(&full_payload(), NOW);
    assert!(ctx.git_branch.is_none() && ctx.git_root.is_none());
    assert!(ctx.git_changes_count.is_none() && ctx.git_staged_count.is_none());
    assert!(ctx.git_unstaged_count.is_none() && ctx.git_untracked_count.is_none());
}

