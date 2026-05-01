//! Adversarial deserialization tests for Phase-2 nested structs in payload.rs.
//! Kept in a sibling file to stay under the 500-LOC ceiling of payload.rs.

use super::*;

fn parse_str(s: &str) -> Result<Payload, anyhow::Error> {
    parse(s.as_bytes())
}

// ── Cost ─────────────────────────────────────────────────────────────────────

#[test]
fn cost_absent_is_none() {
    let p = parse_str(r#"{}"#).unwrap();
    assert!(p.cost.is_none());
}

#[test]
fn cost_explicit_null_is_none() {
    let p = parse_str(r#"{"cost": null}"#).unwrap();
    assert!(p.cost.is_none());
}

#[test]
fn cost_empty_object_all_none() {
    let p = parse_str(r#"{"cost": {}}"#).unwrap();
    let c = p.cost.expect("cost Some");
    assert_eq!(c, Cost::default());
    assert!(c.total_cost_usd.is_none());
    assert!(c.total_duration_ms.is_none());
    assert!(c.total_lines_added.is_none());
}

#[test]
fn cost_total_cost_usd_zero() {
    let p = parse_str(r#"{"cost": {"total_cost_usd": 0.0}}"#).unwrap();
    assert_eq!(p.cost.unwrap().total_cost_usd, Some(0.0));
}

#[test]
fn cost_total_cost_usd_null_is_none() {
    let p = parse_str(r#"{"cost": {"total_cost_usd": null}}"#).unwrap();
    assert!(p.cost.unwrap().total_cost_usd.is_none());
}

#[test]
fn cost_total_cost_usd_nan_string_is_err() {
    // Standard JSON does not allow NaN; serde_json must reject it.
    let result = parse_str(r#"{"cost": {"total_cost_usd": NaN}}"#);
    assert!(result.is_err(), "NaN token is not valid JSON");
}

#[test]
fn cost_total_cost_usd_infinity_string_is_err() {
    let result = parse_str(r#"{"cost": {"total_cost_usd": Infinity}}"#);
    assert!(result.is_err(), "Infinity token is not valid JSON");
}

#[test]
fn cost_total_duration_ms_u64_max() {
    // u64::MAX = 18446744073709551615 — fits u64, serde_json parses as u64.
    let json = r#"{"cost": {"total_duration_ms": 18446744073709551615}}"#;
    let p = parse_str(json).unwrap();
    assert_eq!(p.cost.unwrap().total_duration_ms, Some(u64::MAX));
}

#[test]
fn cost_total_duration_ms_i64_max_plus_one() {
    // 2^63 = 9223372036854775808 — out of i64 range but within u64.
    let json = r#"{"cost": {"total_duration_ms": 9223372036854775808}}"#;
    let p = parse_str(json).unwrap();
    assert_eq!(
        p.cost.unwrap().total_duration_ms,
        Some(9_223_372_036_854_775_808u64)
    );
}

#[test]
fn cost_total_lines_added_negative_is_err() {
    // u64 field — negative integer must fail.
    let result = parse_str(r#"{"cost": {"total_lines_added": -1}}"#);
    assert!(result.is_err(), "negative value for u64 field must fail");
}

#[test]
fn cost_total_lines_removed_negative_is_err() {
    let result = parse_str(r#"{"cost": {"total_lines_removed": -5}}"#);
    assert!(result.is_err(), "negative value for u64 field must fail");
}

// ── ContextWindow ─────────────────────────────────────────────────────────────

#[test]
fn context_window_absent_is_none() {
    let p = parse_str(r#"{}"#).unwrap();
    assert!(p.context_window.is_none());
}

#[test]
fn context_window_explicit_null_is_none() {
    let p = parse_str(r#"{"context_window": null}"#).unwrap();
    assert!(p.context_window.is_none());
}

#[test]
fn context_window_empty_object_all_none() {
    let p = parse_str(r#"{"context_window": {}}"#).unwrap();
    let cw = p.context_window.expect("Some");
    assert!(cw.total_input_tokens.is_none());
    assert!(cw.used_percentage.is_none());
    assert!(cw.current_usage.is_none());
}

#[test]
fn context_window_used_percentage_above_100_parses() {
    // Validation is a render-time concern; deserialization must accept it.
    let p = parse_str(r#"{"context_window": {"used_percentage": 150.0}}"#).unwrap();
    assert_eq!(p.context_window.unwrap().used_percentage, Some(150.0));
}

#[test]
fn context_window_used_percentage_negative_parses() {
    let p = parse_str(r#"{"context_window": {"used_percentage": -1.5}}"#).unwrap();
    assert_eq!(p.context_window.unwrap().used_percentage, Some(-1.5));
}

#[test]
fn context_window_current_usage_missing_is_none() {
    let p = parse_str(r#"{"context_window": {"context_window_size": 100000}}"#).unwrap();
    assert!(p.context_window.unwrap().current_usage.is_none());
}

#[test]
fn context_window_current_usage_empty_object_all_none() {
    let p = parse_str(r#"{"context_window": {"current_usage": {}}}"#).unwrap();
    let cu = p
        .context_window
        .unwrap()
        .current_usage
        .expect("current_usage Some");
    assert_eq!(cu, ContextWindowCurrentUsage::default());
}

#[test]
fn context_window_current_usage_negative_token_is_err() {
    let result = parse_str(r#"{"context_window": {"current_usage": {"input_tokens": -1}}}"#);
    assert!(result.is_err());
}

// ── Effort / Thinking / Vim / Agent / OutputStyle ───────────────────────────

#[test]
fn effort_absent_is_none() {
    let p = parse_str(r#"{}"#).unwrap();
    assert!(p.effort.is_none());
}

#[test]
fn effort_empty_string_accepted() {
    let p = parse_str(r#"{"effort": {"level": ""}}"#).unwrap();
    assert_eq!(p.effort.unwrap().level.as_deref(), Some(""));
}

#[test]
fn effort_novel_level_accepted() {
    // Server may add new levels; deserialization must not reject unknown strings.
    let p = parse_str(r#"{"effort": {"level": "xhigh"}}"#).unwrap();
    assert_eq!(p.effort.unwrap().level.as_deref(), Some("xhigh"));
}

#[test]
fn effort_level_null_is_none() {
    let p = parse_str(r#"{"effort": {"level": null}}"#).unwrap();
    assert!(p.effort.unwrap().level.is_none());
}

#[test]
fn thinking_enabled_null_is_none() {
    let p = parse_str(r#"{"thinking": {"enabled": null}}"#).unwrap();
    assert!(p.thinking.unwrap().enabled.is_none());
}

#[test]
fn thinking_absent_is_none() {
    let p = parse_str(r#"{}"#).unwrap();
    assert!(p.thinking.is_none());
}

#[test]
fn thinking_explicit_null_is_none() {
    let p = parse_str(r#"{"thinking": null}"#).unwrap();
    assert!(p.thinking.is_none());
}

#[test]
fn vim_novel_mode_accepted() {
    // Future modes (e.g. "OPERATOR") must not be rejected.
    let p = parse_str(r#"{"vim": {"mode": "OPERATOR"}}"#).unwrap();
    assert_eq!(p.vim.unwrap().mode.as_deref(), Some("OPERATOR"));
}

#[test]
fn vim_mode_empty_string_accepted() {
    let p = parse_str(r#"{"vim": {"mode": ""}}"#).unwrap();
    assert_eq!(p.vim.unwrap().mode.as_deref(), Some(""));
}

#[test]
fn agent_name_empty_string_accepted() {
    let p = parse_str(r#"{"agent": {"name": ""}}"#).unwrap();
    assert_eq!(p.agent.unwrap().name.as_deref(), Some(""));
}

#[test]
fn agent_absent_is_none() {
    let p = parse_str(r#"{}"#).unwrap();
    assert!(p.agent.is_none());
}

#[test]
fn output_style_absent_is_none() {
    let p = parse_str(r#"{}"#).unwrap();
    assert!(p.output_style.is_none());
}

#[test]
fn output_style_explicit_null_is_none() {
    let p = parse_str(r#"{"output_style": null}"#).unwrap();
    assert!(p.output_style.is_none());
}

// ── Worktree ──────────────────────────────────────────────────────────────────

#[test]
fn worktree_absent_is_none() {
    let p = parse_str(r#"{}"#).unwrap();
    assert!(p.worktree.is_none());
}

#[test]
fn worktree_explicit_null_is_none() {
    let p = parse_str(r#"{"worktree": null}"#).unwrap();
    assert!(p.worktree.is_none());
}

#[test]
fn worktree_empty_object_all_none() {
    let p = parse_str(r#"{"worktree": {}}"#).unwrap();
    let wt = p.worktree.expect("Some");
    assert!(wt.name.is_none());
    assert!(wt.path.is_none());
    assert!(wt.branch.is_none());
    assert!(wt.original_cwd.is_none());
    assert!(wt.original_branch.is_none());
}

#[test]
fn worktree_partial_fields_ok() {
    let p = parse_str(r#"{"worktree": {"branch": "feat/x"}}"#).unwrap();
    let wt = p.worktree.expect("Some");
    assert_eq!(wt.branch.as_deref(), Some("feat/x"));
    assert!(wt.name.is_none());
    assert!(wt.original_branch.is_none());
}

#[test]
fn worktree_unicode_path_accepted() {
    let p = parse_str(r#"{"worktree": {"path": "/proj/中文/wt"}}"#).unwrap();
    assert_eq!(
        p.worktree.unwrap().path.as_deref(),
        Some("/proj/\u{4e2d}\u{6587}/wt")
    );
}

// ── Workspace ─────────────────────────────────────────────────────────────────

#[test]
fn workspace_added_dirs_empty_array_is_some_empty_vec() {
    let p = parse_str(r#"{"workspace": {"added_dirs": []}}"#).unwrap();
    let ws = p.workspace.expect("Some");
    assert_eq!(ws.added_dirs, Some(vec![]));
}

#[test]
fn workspace_added_dirs_absent_is_none() {
    let p = parse_str(r#"{"workspace": {}}"#).unwrap();
    assert!(p.workspace.unwrap().added_dirs.is_none());
}

#[test]
fn workspace_added_dirs_null_is_none() {
    let p = parse_str(r#"{"workspace": {"added_dirs": null}}"#).unwrap();
    assert!(p.workspace.unwrap().added_dirs.is_none());
}

// ── Model extensions ──────────────────────────────────────────────────────────

#[test]
fn model_id_present_display_name_absent() {
    let p = parse_str(r#"{"model": {"id": "claude-3-7-sonnet-latest"}}"#).unwrap();
    let m = p.model.expect("Some");
    assert_eq!(m.id.as_deref(), Some("claude-3-7-sonnet-latest"));
    assert!(m.display_name.is_none());
}

#[test]
fn model_display_name_only_no_id_regression() {
    // Existing fixture pattern must still work after Model gained `id`.
    let p = parse_str(r#"{"model": {"display_name": "claude-opus-4-7"}}"#).unwrap();
    let m = p.model.expect("Some");
    assert_eq!(m.display_name.as_deref(), Some("claude-opus-4-7"));
    assert!(m.id.is_none());
}

#[test]
fn model_id_empty_string_accepted() {
    let p = parse_str(r#"{"model": {"id": ""}}"#).unwrap();
    assert_eq!(p.model.unwrap().id.as_deref(), Some(""));
}

// ── Top-level boolean / misc fields ──────────────────────────────────────────

#[test]
fn exceeds_200k_tokens_true_parses() {
    let p = parse_str(r#"{"exceeds_200k_tokens": true}"#).unwrap();
    assert_eq!(p.exceeds_200k_tokens, Some(true));
}

#[test]
fn exceeds_200k_tokens_null_is_none() {
    let p = parse_str(r#"{"exceeds_200k_tokens": null}"#).unwrap();
    assert!(p.exceeds_200k_tokens.is_none());
}

#[test]
fn session_id_unchanged_regression() {
    let p = parse_str(r#"{"session_id": "abc-123"}"#).unwrap();
    assert_eq!(p.session_id.as_deref(), Some("abc-123"));
}

#[test]
fn session_id_unicode_accepted() {
    let p = parse_str(r#"{"session_id": "éàü"}"#).unwrap();
    assert_eq!(p.session_id.as_deref(), Some("\u{00e9}\u{00e0}\u{00fc}"));
}
