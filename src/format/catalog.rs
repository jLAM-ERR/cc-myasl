/// All placeholders Phase 1+2 introduced — name + one-line description.
///
/// Used by the TUI placeholder-picker overlay.  Order: by category then alphabetical.
pub const ALL_PLACEHOLDERS: &[(&str, &str)] = &[
    // ── identity ─────────────────────────────────────────────────────────────
    ("model", "Claude model display name (e.g. claude-opus-4-7)"),
    ("model_id", "Claude model ID string"),
    // ── working directory ─────────────────────────────────────────────────────
    ("cwd", "Current working directory (HOME replaced with ~)"),
    ("cwd_basename", "Basename of current working directory"),
    // ── five-hour quota ───────────────────────────────────────────────────────
    ("five_bar", "5-hour quota remaining bar (10 chars)"),
    ("five_bar_long", "5-hour quota remaining bar (20 chars)"),
    ("five_color", "ANSI color reflecting 5-hour quota state"),
    ("five_left", "5-hour quota remaining as integer percent"),
    ("five_reset_clock", "Local time when 5-hour quota resets"),
    (
        "five_reset_in",
        "Time until 5-hour quota resets (e.g. 2h13m)",
    ),
    ("five_state", "Icon reflecting 5-hour quota state"),
    ("five_used", "5-hour quota used as decimal percent"),
    // ── seven-day quota ───────────────────────────────────────────────────────
    ("seven_bar", "7-day quota remaining bar (10 chars)"),
    ("seven_bar_long", "7-day quota remaining bar (20 chars)"),
    ("seven_color", "ANSI color reflecting 7-day quota state"),
    ("seven_left", "7-day quota remaining as integer percent"),
    ("seven_reset_clock", "Local time when 7-day quota resets"),
    (
        "seven_reset_in",
        "Time until 7-day quota resets (e.g. 2h13m)",
    ),
    ("seven_state", "Icon reflecting 7-day quota state"),
    ("seven_used", "7-day quota used as decimal percent"),
    // ── extra usage ───────────────────────────────────────────────────────────
    ("extra_left", "Extra usage remaining as integer percent"),
    ("extra_pct", "Extra usage used as decimal percent"),
    ("extra_used", "Extra usage used as integer percent"),
    // ── combined state ────────────────────────────────────────────────────────
    ("state_icon", "Icon for worst of 5-hour / 7-day quota"),
    // ── session metadata ──────────────────────────────────────────────────────
    ("agent_name", "Name of the active agent"),
    ("effort", "Effort level (low/normal/high)"),
    ("output_style", "Output style (e.g. concise)"),
    ("session_id", "Unique session ID"),
    ("session_name", "Human-readable session name"),
    (
        "thinking_enabled",
        "Prints 'thinking' when extended thinking is on",
    ),
    ("version", "Claude Code version string"),
    (
        "vim_mode",
        "Current Vim mode (INSERT / NORMAL / VISUAL / …)",
    ),
    // ── cost / timing ─────────────────────────────────────────────────────────
    (
        "api_duration",
        "Time spent in API calls this turn (e.g. 1m30s)",
    ),
    (
        "cost_usd",
        "Cumulative cost in USD this session (e.g. 0.42)",
    ),
    ("lines_added", "Lines added this turn"),
    ("lines_changed", "Lines added + removed this turn"),
    ("lines_removed", "Lines removed this turn"),
    ("session_clock", "Total elapsed session time (e.g. 1h05m)"),
    // ── tokens (current turn) ─────────────────────────────────────────────────
    ("tokens_cached_creation", "Cache-creation tokens this turn"),
    ("tokens_cached_read", "Cache-read tokens this turn"),
    (
        "tokens_cached_total",
        "Total cached tokens (creation + read) this turn",
    ),
    ("tokens_input", "Input tokens this turn"),
    ("tokens_output", "Output tokens this turn"),
    ("tokens_total", "All token types summed for this turn"),
    // ── tokens (session totals) ───────────────────────────────────────────────
    ("tokens_input_total", "Total input tokens across session"),
    ("tokens_output_total", "Total output tokens across session"),
    // ── context window ────────────────────────────────────────────────────────
    ("context_bar", "Context window used bar (10 chars)"),
    ("context_bar_long", "Context window used bar (20 chars)"),
    (
        "context_remaining_pct",
        "Context window remaining as decimal percent",
    ),
    ("context_size", "Context window size in tokens"),
    ("context_used_pct", "Context window used as decimal percent"),
    (
        "context_used_pct_int",
        "Context window used as integer percent",
    ),
    (
        "exceeds_200k",
        "Prints '!' when context exceeds 200k tokens",
    ),
    // ── workspace ─────────────────────────────────────────────────────────────
    ("added_dirs_count", "Number of added workspace directories"),
    (
        "project_dir",
        "Project root directory (HOME replaced with ~)",
    ),
    (
        "workspace_git_worktree",
        "Git worktree name detected by Claude Code",
    ),
    // ── worktree (--worktree sessions only) ───────────────────────────────────
    ("worktree_branch", "Branch name for the managed worktree"),
    ("worktree_name", "Name of the managed worktree"),
    (
        "worktree_original_branch",
        "Original branch before worktree switch",
    ),
    (
        "worktree_original_cwd",
        "Original cwd before worktree switch",
    ),
    ("worktree_path", "Filesystem path of the managed worktree"),
    // ── git ───────────────────────────────────────────────────────────────────
    ("git_branch", "Current git branch name"),
    (
        "git_changes",
        "Total changed files (staged + unstaged + untracked)",
    ),
    ("git_root", "Git repo root directory (HOME replaced with ~)"),
    ("git_staged", "Number of staged files"),
    (
        "git_status_clean",
        "Prints 'clean' when working tree has no changes",
    ),
    ("git_unstaged", "Number of unstaged modified files"),
    ("git_untracked", "Number of untracked files"),
    // ── misc ──────────────────────────────────────────────────────────────────
    ("reset", "ANSI reset escape sequence"),
];

/// Case-insensitive substring match on name or description.
pub fn filtered_placeholders(query: &str) -> Vec<&'static (&'static str, &'static str)> {
    let q = query.to_lowercase();
    if q.is_empty() {
        return ALL_PLACEHOLDERS.iter().collect();
    }
    ALL_PLACEHOLDERS
        .iter()
        .filter(|(name, desc)| name.contains(&*q) || desc.to_lowercase().contains(&*q))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{ALL_PLACEHOLDERS, filtered_placeholders};

    #[test]
    fn all_placeholders_includes_known_phase1_names() {
        let names: Vec<&str> = ALL_PLACEHOLDERS.iter().map(|(n, _)| *n).collect();
        for expected in [
            "model",
            "five_left",
            "seven_left",
            "cwd",
            "five_bar",
            "state_icon",
        ] {
            assert!(names.contains(&expected), "missing: {expected}");
        }
    }

    #[test]
    fn all_placeholders_includes_known_phase2_names() {
        let names: Vec<&str> = ALL_PLACEHOLDERS.iter().map(|(n, _)| *n).collect();
        for expected in [
            "git_branch",
            "session_clock",
            "context_used_pct",
            "cost_usd",
        ] {
            assert!(names.contains(&expected), "missing: {expected}");
        }
    }

    #[test]
    fn filtered_placeholders_substring_match() {
        let results = filtered_placeholders("git");
        let names: Vec<&str> = results.iter().map(|(n, _)| *n).collect();
        for expected in [
            "git_branch",
            "git_root",
            "git_changes",
            "git_staged",
            "git_unstaged",
            "git_untracked",
            "git_status_clean",
        ] {
            assert!(names.contains(&expected), "missing: {expected}");
        }
    }

    #[test]
    fn filtered_placeholders_case_insensitive() {
        let lower = filtered_placeholders("git");
        let upper = filtered_placeholders("GIT");
        assert_eq!(lower.len(), upper.len());
        for (l, u) in lower.iter().zip(upper.iter()) {
            assert_eq!(l.0, u.0);
        }
    }

    #[test]
    fn filtered_placeholders_empty_query_returns_all() {
        assert_eq!(filtered_placeholders("").len(), ALL_PLACEHOLDERS.len());
    }

    #[test]
    fn filtered_placeholders_no_match_returns_empty() {
        assert!(filtered_placeholders("zzznomatch").is_empty());
    }

    #[test]
    fn all_placeholders_no_duplicates() {
        let mut sorted = ALL_PLACEHOLDERS.iter().map(|(n, _)| *n).collect::<Vec<_>>();
        sorted.sort_unstable();
        let mut prev = "";
        for n in &sorted {
            assert_ne!(*n, prev, "duplicate: {n}");
            prev = n;
        }
    }
}
