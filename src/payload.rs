//! Claude Code stdin JSON parser.
//!
//! Parses the JSON document Claude Code pipes to the status-line command
//! on every assistant message.  All fields are `Option<…>`; unknown fields
//! are silently ignored (no `deny_unknown_fields`).

use anyhow::Context as _;
use serde::Deserialize;

/// Rate-limit window (five-hour or seven-day).
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct RateWindow {
    /// 0–100 inclusive, how much of the window has been consumed.
    pub used_percentage: Option<f64>,
    /// Unix epoch seconds at which the window resets.
    pub resets_at: Option<u64>,
}

/// Both rate-limit windows as delivered by Claude Code on stdin.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct RateLimits {
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
}

/// Model metadata block.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Model {
    pub display_name: Option<String>,
    pub id: Option<String>,
}

/// Workspace block.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Workspace {
    pub current_dir: Option<String>,
    pub project_dir: Option<String>,
    pub added_dirs: Option<Vec<String>>,
    pub git_worktree: Option<String>,
}

/// Accumulated cost and timing for the current session.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Cost {
    pub total_cost_usd: Option<f64>,
    pub total_duration_ms: Option<u64>,
    pub total_api_duration_ms: Option<u64>,
    pub total_lines_added: Option<u64>,
    pub total_lines_removed: Option<u64>,
}

/// Per-turn token usage snapshot.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct ContextWindowCurrentUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}

/// Context-window utilisation counters.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct ContextWindow {
    pub total_input_tokens: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub context_window_size: Option<u64>,
    pub used_percentage: Option<f64>,
    pub remaining_percentage: Option<f64>,
    pub current_usage: Option<ContextWindowCurrentUsage>,
}

/// Thinking-effort level (e.g. "high", "medium", "low").
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Effort {
    pub level: Option<String>,
}

/// Whether extended thinking is enabled.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Thinking {
    pub enabled: Option<bool>,
}

/// Active output style (e.g. "verbose", "concise").
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct OutputStyle {
    pub name: Option<String>,
}

/// Vim emulation state.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Vim {
    pub mode: Option<String>,
}

/// Sub-agent identity.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Agent {
    pub name: Option<String>,
}

/// Git-worktree context as reported by Claude Code.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Worktree {
    pub name: Option<String>,
    pub path: Option<String>,
    pub branch: Option<String>,
    pub original_cwd: Option<String>,
    pub original_branch: Option<String>,
}

/// Top-level payload delivered by Claude Code on stdin.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Payload {
    pub model: Option<Model>,
    pub workspace: Option<Workspace>,
    pub transcript_path: Option<String>,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    /// Top-level cwd mirrors `workspace.current_dir`; both are sent by Claude Code.
    pub cwd: Option<String>,
    pub version: Option<String>,
    pub output_style: Option<OutputStyle>,
    pub cost: Option<Cost>,
    pub context_window: Option<ContextWindow>,
    pub exceeds_200k_tokens: Option<bool>,
    pub effort: Option<Effort>,
    pub thinking: Option<Thinking>,
    pub vim: Option<Vim>,
    pub agent: Option<Agent>,
    pub worktree: Option<Worktree>,
    pub rate_limits: Option<RateLimits>,
}

/// Parse a `Payload` from any `std::io::Read` source.
///
/// Returns `Err` on any JSON parse failure.  Never panics.
pub fn parse<R: std::io::Read>(reader: R) -> Result<Payload, anyhow::Error> {
    serde_json::from_reader(reader).context("failed to parse Claude Code stdin JSON")
}

#[cfg(test)]
#[path = "payload_tests.rs"]
mod payload_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(s: &str) -> Result<Payload, anyhow::Error> {
        parse(s.as_bytes())
    }

    #[test]
    fn full_payload_parses() {
        let json = r#"{
            "model": { "display_name": "claude-opus-4-7" },
            "workspace": { "current_dir": "/home/user/project" },
            "transcript_path": "/tmp/transcript.jsonl",
            "session_id": "abc-123",
            "rate_limits": {
                "five_hour": { "used_percentage": 23.5, "resets_at": 1738425600 },
                "seven_day": { "used_percentage": 41.2, "resets_at": 1738857600 }
            }
        }"#;
        let p = parse_str(json).expect("should parse full payload");
        assert_eq!(
            p.model.as_ref().and_then(|m| m.display_name.as_deref()),
            Some("claude-opus-4-7")
        );
        assert_eq!(
            p.workspace.as_ref().and_then(|w| w.current_dir.as_deref()),
            Some("/home/user/project")
        );
        assert_eq!(p.transcript_path.as_deref(), Some("/tmp/transcript.jsonl"));
        assert_eq!(p.session_id.as_deref(), Some("abc-123"));
        let rl = p
            .rate_limits
            .as_ref()
            .expect("rate_limits should be present");
        let fh = rl.five_hour.as_ref().expect("five_hour should be present");
        assert_eq!(fh.used_percentage, Some(23.5));
        assert_eq!(fh.resets_at, Some(1_738_425_600u64));
        let sd = rl.seven_day.as_ref().expect("seven_day should be present");
        assert_eq!(sd.used_percentage, Some(41.2));
        assert_eq!(sd.resets_at, Some(1_738_857_600u64));
    }

    #[test]
    fn missing_rate_limits_parses_to_none() {
        let json = r#"{
            "model": { "display_name": "claude-sonnet-4-6" },
            "session_id": "xyz-789"
        }"#;
        let p = parse_str(json).expect("should parse without rate_limits");
        assert_eq!(p.rate_limits, None);
        assert_eq!(
            p.model.as_ref().and_then(|m| m.display_name.as_deref()),
            Some("claude-sonnet-4-6")
        );
    }

    #[test]
    fn missing_workspace_parses_to_none() {
        let json = r#"{ "session_id": "s1" }"#;
        let p = parse_str(json).expect("should parse without workspace");
        assert_eq!(p.workspace, None);
        assert_eq!(p.session_id.as_deref(), Some("s1"));
    }

    #[test]
    fn malformed_json_returns_err() {
        let result = parse_str("{ not valid json }");
        assert!(result.is_err(), "malformed JSON should return Err");
    }

    #[test]
    fn empty_stdin_returns_err() {
        let result = parse_str("");
        assert!(result.is_err(), "empty input should return Err");
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let json = r#"{ "foo": "bar", "model": { "display_name": "test" }, "baz": 42 }"#;
        let p = parse_str(json).expect("unknown fields should not cause error");
        assert_eq!(
            p.model.as_ref().and_then(|m| m.display_name.as_deref()),
            Some("test")
        );
    }

    #[test]
    fn partial_rate_limits_only_five_hour() {
        let json = r#"{
            "rate_limits": {
                "five_hour": { "used_percentage": 10.0, "resets_at": 9999 }
            }
        }"#;
        let p = parse_str(json).expect("partial rate_limits should parse");
        let rl = p
            .rate_limits
            .as_ref()
            .expect("rate_limits should be present");
        assert!(rl.five_hour.is_some());
        assert_eq!(rl.seven_day, None);
    }

    #[test]
    fn full_payload_with_phase2_fields_parses() {
        let src = include_str!("../tests/fixtures/full-payload.json");
        let p = parse_str(src).expect("full-payload fixture should parse");

        // model
        let m = p.model.as_ref().expect("model");
        assert_eq!(m.id.as_deref(), Some("claude-sonnet-4-6"));
        assert_eq!(m.display_name.as_deref(), Some("claude-sonnet-4-6"));

        // top-level new fields
        assert_eq!(p.cwd.as_deref(), Some("/Users/test/myproject"));
        assert_eq!(p.version.as_deref(), Some("1.2.3"));
        assert_eq!(p.session_name.as_deref(), Some("My Project Session"));
        assert_eq!(p.exceeds_200k_tokens, Some(false));

        // output_style
        let os = p.output_style.as_ref().expect("output_style");
        assert_eq!(os.name.as_deref(), Some("verbose"));

        // cost
        let cost = p.cost.as_ref().expect("cost");
        assert_eq!(cost.total_cost_usd, Some(0.42));
        assert_eq!(cost.total_duration_ms, Some(7_380_000));
        assert_eq!(cost.total_api_duration_ms, Some(6_900_000));
        assert_eq!(cost.total_lines_added, Some(150));
        assert_eq!(cost.total_lines_removed, Some(30));

        // context_window
        let cw = p.context_window.as_ref().expect("context_window");
        assert_eq!(cw.total_input_tokens, Some(12_345));
        assert_eq!(cw.total_output_tokens, Some(6_789));
        assert_eq!(cw.context_window_size, Some(200_000));
        assert_eq!(cw.used_percentage, Some(23.5));
        assert_eq!(cw.remaining_percentage, Some(76.5));
        let cu = cw.current_usage.as_ref().expect("current_usage");
        assert_eq!(cu.input_tokens, Some(4_096));
        assert_eq!(cu.output_tokens, Some(512));
        assert_eq!(cu.cache_creation_input_tokens, Some(1_024));
        assert_eq!(cu.cache_read_input_tokens, Some(2_048));

        // effort / thinking / vim / agent
        assert_eq!(
            p.effort.as_ref().and_then(|e| e.level.as_deref()),
            Some("high")
        );
        assert_eq!(p.thinking.as_ref().and_then(|t| t.enabled), Some(true));
        assert_eq!(
            p.vim.as_ref().and_then(|v| v.mode.as_deref()),
            Some("normal")
        );
        assert_eq!(
            p.agent.as_ref().and_then(|a| a.name.as_deref()),
            Some("code-agent")
        );

        // worktree
        let wt = p.worktree.as_ref().expect("worktree");
        assert_eq!(wt.name.as_deref(), Some("feature-branch"));
        assert_eq!(
            wt.path.as_deref(),
            Some("/Users/test/worktrees/feature-branch")
        );
        assert_eq!(wt.branch.as_deref(), Some("feature/add-placeholders"));
        assert_eq!(wt.original_cwd.as_deref(), Some("/Users/test/myproject"));
        assert_eq!(wt.original_branch.as_deref(), Some("main"));

        // workspace extensions
        let ws = p.workspace.as_ref().expect("workspace");
        assert_eq!(ws.project_dir.as_deref(), Some("/Users/test/myproject"));
        assert_eq!(ws.added_dirs.as_ref().map(|v| v.len()), Some(2));
        assert_eq!(ws.git_worktree.as_deref(), Some("feature-branch"));
    }

    #[test]
    fn partial_phase2_payload_parses() {
        // Most new fields absent — all should be None.
        let json = r#"{ "session_id": "s42", "version": "1.0.0" }"#;
        let p = parse_str(json).expect("partial phase2 payload should parse");
        assert_eq!(p.session_id.as_deref(), Some("s42"));
        assert_eq!(p.version.as_deref(), Some("1.0.0"));
        assert!(p.cwd.is_none());
        assert!(p.session_name.is_none());
        assert!(p.output_style.is_none());
        assert!(p.cost.is_none());
        assert!(p.context_window.is_none());
        assert!(p.exceeds_200k_tokens.is_none());
        assert!(p.effort.is_none());
        assert!(p.thinking.is_none());
        assert!(p.vim.is_none());
        assert!(p.agent.is_none());
        assert!(p.worktree.is_none());
    }

    #[test]
    fn unknown_phase2_fields_are_tolerated() {
        let json = r#"{
            "cost": { "total_cost_usd": 0.1, "future_field": "ignored" },
            "effort": { "level": "low", "extra": 99 },
            "vim": { "mode": "insert", "unknown_key": true }
        }"#;
        let p = parse_str(json).expect("unknown nested fields should be tolerated");
        assert_eq!(p.cost.as_ref().and_then(|c| c.total_cost_usd), Some(0.1));
        assert_eq!(
            p.effort.as_ref().and_then(|e| e.level.as_deref()),
            Some("low")
        );
        assert_eq!(
            p.vim.as_ref().and_then(|v| v.mode.as_deref()),
            Some("insert")
        );
    }

    #[test]
    fn current_usage_null_handled() {
        let json = r#"{
            "context_window": {
                "context_window_size": 200000,
                "current_usage": null
            }
        }"#;
        let p = parse_str(json).expect("null current_usage should parse");
        let cw = p.context_window.as_ref().expect("context_window");
        assert_eq!(cw.context_window_size, Some(200_000));
        assert!(cw.current_usage.is_none());
    }
}
