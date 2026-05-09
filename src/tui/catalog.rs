//! 43-entry preset catalog for the TUI segment picker.

use std::collections::HashMap;
use std::sync::OnceLock;

use crate::config::named_color::NamedColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Category {
    Workspace,
    Git,
    SessionModel,
    Context,
    Tokens,
    Cost,
    Rates,
    Appearance,
}

impl Category {
    pub fn ordered() -> &'static [Category] {
        &[
            Category::Workspace,
            Category::Git,
            Category::SessionModel,
            Category::Context,
            Category::Tokens,
            Category::Cost,
            Category::Rates,
            Category::Appearance,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Preset {
    pub id: &'static str,
    pub category: Category,
    pub label: &'static str,
    pub template: &'static str,
    pub hide_when_absent: bool,
    pub default_color: Option<NamedColor>,
    pub default_bg: Option<NamedColor>,
}

pub const PRESETS: &[Preset] = &[
    // ── workspace (6) ────────────────────────────────────────────────────────
    Preset {
        id: "cwd_basename",
        category: Category::Workspace,
        label: "Current dir (basename)",
        template: "\u{1f4c1} {cwd_basename}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "cwd_full",
        category: Category::Workspace,
        label: "Current dir (full path)",
        template: "\u{1f4c1} {cwd}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "project_dir",
        category: Category::Workspace,
        label: "Project dir",
        template: "\u{1f4c2} {project_dir}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "worktree_name",
        category: Category::Workspace,
        label: "Worktree name",
        template: "\u{1f332} {worktree_name}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "worktree_path",
        category: Category::Workspace,
        label: "Worktree path",
        template: "\u{1f332} {worktree_path}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "added_dirs_count",
        category: Category::Workspace,
        label: "Added dirs count",
        template: "+{added_dirs_count} dirs",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    // ── git (5, all hide) ────────────────────────────────────────────────────
    Preset {
        id: "git_branch",
        category: Category::Git,
        label: "Git branch",
        template: "\u{1f33f} {git_branch}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "git_branch_path",
        category: Category::Git,
        label: "Git branch + root",
        template: "\u{1f33f} {git_branch} ({git_root})",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "git_changes_count",
        category: Category::Git,
        label: "Git changes count",
        template: "\u{00b1}{git_changes}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "git_status_detailed",
        category: Category::Git,
        label: "Git status detailed",
        template: "+{git_staged} ~{git_unstaged} ?{git_untracked}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "git_clean_indicator",
        category: Category::Git,
        label: "Git clean indicator",
        template: "{git_status_clean}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    // ── session/model (8) ─────────────────────────────────────────────────────
    Preset {
        id: "model_name",
        category: Category::SessionModel,
        label: "Model name",
        template: "\u{1f916} {model}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "model_full",
        category: Category::SessionModel,
        label: "Model + version",
        template: "\u{1f916} {model} ({version})",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "model_id",
        category: Category::SessionModel,
        label: "Model ID",
        template: "{model_id}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "effort",
        category: Category::SessionModel,
        label: "Effort level",
        template: "\u{26a1} {effort}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "output_style",
        category: Category::SessionModel,
        label: "Output style",
        template: "\u{2728} {output_style}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "thinking",
        category: Category::SessionModel,
        label: "Thinking indicator",
        template: "\u{1f4ad} {thinking_enabled}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "vim_mode",
        category: Category::SessionModel,
        label: "Vim mode",
        template: "\u{2328} {vim_mode}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "agent_name",
        category: Category::SessionModel,
        label: "Agent name",
        template: "\u{1f3ad} {agent_name}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    // ── context (5) ──────────────────────────────────────────────────────────
    Preset {
        id: "context_pct_int",
        category: Category::Context,
        label: "Context % (integer)",
        template: "\u{1f9e0} {context_used_pct_int}%",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "context_pct_decimal",
        category: Category::Context,
        label: "Context % (decimal)",
        template: "\u{1f9e0} {context_used_pct}%",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "context_bar",
        category: Category::Context,
        label: "Context bar (short)",
        template: "\u{1f9e0} {context_bar} {context_used_pct_int}%",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "context_bar_long",
        category: Category::Context,
        label: "Context bar (long)",
        template: "\u{1f9e0} {context_bar_long} {context_used_pct_int}%",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "context_size",
        category: Category::Context,
        label: "Context size",
        template: "\u{1f9e0} {context_size}/200k",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    // ── tokens (6) ───────────────────────────────────────────────────────────
    Preset {
        id: "tokens_total_turn",
        category: Category::Tokens,
        label: "Tokens this turn (total)",
        template: "\u{2191}\u{2193} {tokens_total}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "tokens_in_out",
        category: Category::Tokens,
        label: "Tokens in/out",
        template: "\u{2191}{tokens_input}/\u{2193}{tokens_output}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "tokens_cached_read",
        category: Category::Tokens,
        label: "Cached read tokens",
        template: "\u{1f4cb} {tokens_cached_read}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "tokens_cached_creation",
        category: Category::Tokens,
        label: "Cache creation tokens",
        template: "\u{1f4dd} {tokens_cached_creation}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "tokens_cached_total",
        category: Category::Tokens,
        label: "Cached tokens (total)",
        template: "\u{1f4cb} {tokens_cached_total}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "tokens_session_totals",
        category: Category::Tokens,
        label: "Session token totals",
        template: "\u{03a3} \u{2191}{tokens_input_total}/\u{2193}{tokens_output_total}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    // ── cost (5) ─────────────────────────────────────────────────────────────
    Preset {
        id: "cost_usd",
        category: Category::Cost,
        label: "Session cost",
        template: "\u{1f4b0} ${cost_usd}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "session_clock",
        category: Category::Cost,
        label: "Session duration",
        template: "\u{23f1} {session_clock}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "api_duration",
        category: Category::Cost,
        label: "API duration",
        template: "\u{26a1} {api_duration}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "lines_added",
        category: Category::Cost,
        label: "Lines added",
        template: "+{lines_added}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "lines_changed",
        category: Category::Cost,
        label: "Lines added/removed",
        template: "+{lines_added}/-{lines_removed}",
        hide_when_absent: true,
        default_color: None,
        default_bg: None,
    },
    // ── rates (8; seven_reset_in dropped — 7d resets are days away,
    //           countdown is rarely actionable) ─────────────────────────────
    Preset {
        id: "five_left_pct",
        category: Category::Rates,
        label: "5h quota remaining %",
        template: "5h:{five_color}{five_left}%{reset}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "five_bar",
        category: Category::Rates,
        label: "5h quota bar (short)",
        template: "5h:{five_color}{five_bar} {five_left}%{reset}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "five_bar_long",
        category: Category::Rates,
        label: "5h quota bar (long)",
        template: "5h:{five_color}{five_bar_long} {five_left}%{reset}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "five_reset_in",
        category: Category::Rates,
        label: "5h resets in",
        template: "5h in {five_reset_in}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "five_reset_clock",
        category: Category::Rates,
        label: "5h reset clock",
        template: "5h \u{23f0} {five_reset_clock}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "seven_left_pct",
        category: Category::Rates,
        label: "7d quota remaining %",
        template: "7d:{seven_color}{seven_left}%{reset}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "seven_bar",
        category: Category::Rates,
        label: "7d quota bar (short)",
        template: "7d:{seven_color}{seven_bar} {seven_left}%{reset}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
    Preset {
        id: "state_icon",
        category: Category::Rates,
        label: "State icon",
        template: "{state_icon}",
        hide_when_absent: false,
        default_color: None,
        default_bg: None,
    },
];

static LOOKUP: OnceLock<HashMap<&'static str, &'static Preset>> = OnceLock::new();
static LOOKUP_BY_ID: OnceLock<HashMap<&'static str, &'static Preset>> = OnceLock::new();

pub fn lookup(template: &str) -> Option<&'static Preset> {
    let map = LOOKUP.get_or_init(|| PRESETS.iter().map(|p| (p.template, p)).collect());
    map.get(template).copied()
}

pub fn lookup_by_id(id: &str) -> Option<&'static Preset> {
    let map = LOOKUP_BY_ID.get_or_init(|| PRESETS.iter().map(|p| (p.id, p)).collect());
    map.get(id).copied()
}

pub fn by_category(c: Category) -> impl Iterator<Item = &'static Preset> {
    PRESETS.iter().filter(move |p| p.category == c)
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
