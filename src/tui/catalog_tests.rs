use std::collections::HashSet;

use crate::config::render::render;
use crate::config::schema::{Config, Line, Segment, TemplateSegment};
use crate::format::parser::{Token, tokenize};
use crate::format::placeholders::render_placeholder;
use crate::payload::Payload;
use crate::payload_mapping::build_render_ctx;

use super::{Category, PRESETS, by_category, lookup, lookup_by_id};

const NOW: u64 = 1_700_000_000;
const FIXTURE_JSON: &str = include_str!("preview_fixture.json");

fn fixture_ctx() -> crate::format::placeholders::RenderCtx {
    let payload: Payload = serde_json::from_str(FIXTURE_JSON).expect("valid fixture");
    let mut ctx = build_render_ctx(&payload, NOW);
    // Populate rate-limit fields from the fixture's rate_limits block.
    if let Some(rl) = &payload.rate_limits {
        if let Some(fh) = &rl.five_hour {
            ctx.five_used = fh.used_percentage;
            ctx.five_reset_unix = fh.resets_at;
        }
        if let Some(sd) = &rl.seven_day {
            ctx.seven_used = sd.used_percentage;
            ctx.seven_reset_unix = sd.resets_at;
        }
    }
    ctx
}

fn single_segment_config(preset_template: &str) -> Config {
    Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![Segment::Template(TemplateSegment {
                template: preset_template.to_owned(),
                padding: 0,
                hide_when_absent: false,
                color: None,
                bg: None,
            })],
        }],
    }
}

/// Collect all placeholder names from a template string, handling multiple per template.
fn placeholder_names(template: &str) -> Vec<String> {
    fn collect(tokens: &[Token], out: &mut Vec<String>) {
        for tok in tokens {
            match tok {
                Token::Placeholder(name) => out.push(name.clone()),
                Token::Optional(inner) => collect(inner, out),
                Token::Text(_) => {}
            }
        }
    }
    let mut names = Vec::new();
    collect(&tokenize(template), &mut names);
    names
}

/// Every placeholder name known to `render_placeholder`.
fn known_placeholder_names() -> HashSet<&'static str> {
    [
        "model",
        "cwd",
        "cwd_basename",
        "five_used",
        "five_left",
        "five_bar",
        "five_bar_long",
        "five_reset_clock",
        "five_reset_in",
        "five_color",
        "five_state",
        "seven_used",
        "seven_left",
        "seven_bar",
        "seven_bar_long",
        "seven_reset_clock",
        "seven_reset_in",
        "seven_color",
        "seven_state",
        "extra_left",
        "extra_used",
        "extra_pct",
        "state_icon",
        "model_id",
        "version",
        "session_id",
        "session_name",
        "output_style",
        "effort",
        "thinking_enabled",
        "vim_mode",
        "agent_name",
        "cost_usd",
        "session_clock",
        "api_duration",
        "lines_added",
        "lines_removed",
        "lines_changed",
        "tokens_input",
        "tokens_output",
        "tokens_cached_creation",
        "tokens_cached_read",
        "tokens_cached_total",
        "tokens_total",
        "tokens_input_total",
        "tokens_output_total",
        "context_size",
        "context_used_pct",
        "context_remaining_pct",
        "context_used_pct_int",
        "context_bar",
        "context_bar_long",
        "exceeds_200k",
        "project_dir",
        "added_dirs_count",
        "workspace_git_worktree",
        "worktree_name",
        "worktree_path",
        "worktree_branch",
        "worktree_original_cwd",
        "worktree_original_branch",
        "git_branch",
        "git_root",
        "git_changes",
        "git_staged",
        "git_unstaged",
        "git_untracked",
        "git_status_clean",
        "reset",
    ]
    .into_iter()
    .collect()
}

#[test]
fn no_duplicate_ids() {
    let mut seen = HashSet::new();
    for preset in PRESETS {
        assert!(seen.insert(preset.id), "duplicate id: {}", preset.id);
    }
}

#[test]
fn no_duplicate_templates() {
    let mut seen = HashSet::new();
    for preset in PRESETS {
        assert!(
            seen.insert(preset.template),
            "duplicate template in preset '{}': {}",
            preset.id,
            preset.template
        );
    }
}

/// Both lookup tables are injective: distinct inputs map to distinct outputs.
/// Size equality catches collisions that per-entry asserts might miss on panic.
#[test]
fn lookup_tables_are_injective() {
    let id_set: HashSet<&'static str> = PRESETS.iter().map(|p| p.id).collect();
    assert_eq!(
        id_set.len(),
        PRESETS.len(),
        "id table has collisions: {} unique ids for {} presets",
        id_set.len(),
        PRESETS.len()
    );
    let tmpl_set: HashSet<&'static str> = PRESETS.iter().map(|p| p.template).collect();
    assert_eq!(
        tmpl_set.len(),
        PRESETS.len(),
        "template table has collisions: {} unique templates for {} presets",
        tmpl_set.len(),
        PRESETS.len()
    );
    // Every id in the id-set resolves via lookup_by_id.
    for &id in &id_set {
        assert!(
            lookup_by_id(id).is_some(),
            "lookup_by_id({id:?}) returned None"
        );
    }
    // Every template in the tmpl-set resolves via lookup.
    for &tmpl in &tmpl_set {
        assert!(lookup(tmpl).is_some(), "lookup({tmpl:?}) returned None");
    }
}

#[test]
fn tab_counts_match_brainstorm() {
    let expected: &[(Category, usize)] = &[
        (Category::Workspace, 6),
        (Category::Git, 5),
        (Category::SessionModel, 8),
        (Category::Context, 5),
        (Category::Tokens, 6),
        (Category::Cost, 5),
        (Category::Rates, 8),
    ];
    for (cat, count) in expected {
        let actual = by_category(*cat).count();
        assert_eq!(
            actual, *count,
            "category {:?}: expected {} presets, got {}",
            cat, count, actual
        );
    }
}

#[test]
fn lookup_by_template_finds_preset() {
    for preset in PRESETS {
        let found = lookup(preset.template);
        assert!(
            found.is_some(),
            "lookup({:?}) returned None for preset '{}'",
            preset.template,
            preset.id
        );
        assert_eq!(
            found.unwrap().id,
            preset.id,
            "lookup returned wrong preset for template {:?}",
            preset.template
        );
    }
}

#[test]
fn lookup_unknown_template_returns_none() {
    assert!(lookup("completely_unknown_template_xyz").is_none());
}

/// Invariant 12: every placeholder name in every preset template is known;
/// non-hide-when-absent presets must resolve to Some for all their placeholders;
/// hide-when-absent presets must not panic.
#[test]
fn invariant_12_all_presets_render_without_panic() {
    let ctx = fixture_ctx();
    let known = known_placeholder_names();

    // Sanity: at least one multi-placeholder preset exists (git_branch_path has 2).
    let multi = PRESETS
        .iter()
        .filter(|p| placeholder_names(p.template).len() >= 2)
        .count();
    assert!(
        multi >= 1,
        "expected at least one preset with >=2 placeholders, found none"
    );

    for preset in PRESETS {
        let names = placeholder_names(preset.template);

        for name in &names {
            // Every placeholder name must be in the known set — catches typos.
            assert!(
                known.contains(name.as_str()),
                "preset '{}': placeholder '{{{}}}' is not in the known placeholder catalogue",
                preset.id,
                name
            );

            // render_placeholder must not panic regardless of hide_when_absent.
            let result = render_placeholder(name, &ctx);

            // Non-hide-when-absent presets: every constituent placeholder must
            // resolve to Some in the fixture (the fixture is designed to be rich).
            if !preset.hide_when_absent {
                assert!(
                    result.is_some(),
                    "non-hide preset '{}': placeholder '{{{}}}' returned None against fixture",
                    preset.id,
                    name
                );
            }
            // hide_when_absent presets: None is allowed (field may not be set in fixture).
        }

        // For non-hide presets, the fully-rendered segment must also be non-empty.
        if !preset.hide_when_absent {
            let cfg = single_segment_config(preset.template);
            let out = render(&cfg, &ctx);
            assert!(
                !out.is_empty(),
                "non-hide preset '{}' (template={:?}) rendered empty",
                preset.id,
                preset.template
            );
        }
    }
}

#[test]
fn by_category_returns_correct_presets() {
    for cat in Category::ordered() {
        for preset in by_category(*cat) {
            assert_eq!(
                preset.category, *cat,
                "preset '{}' in wrong category",
                preset.id
            );
        }
    }
}

#[test]
fn category_ordered_has_8_entries() {
    assert_eq!(Category::ordered().len(), 8);
}

/// Appearance is settings-only — no presets belong to it.
#[test]
fn appearance_has_no_presets() {
    assert_eq!(
        by_category(Category::Appearance).count(),
        0,
        "Appearance category must have 0 presets (settings-only)"
    );
}
