use std::collections::HashSet;

use crate::config::render::render;
use crate::config::schema::{Config, Line, Segment, TemplateSegment};
use crate::format::parser::{Token, tokenize};
use crate::format::placeholders::render_placeholder;
use crate::payload::Payload;
use crate::payload_mapping::build_render_ctx;

use super::{Category, PRESETS, by_category, lookup};

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

/// Collect all placeholder names from a template string.
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

/// Invariant 12: every placeholder in every preset template renders without panic;
/// non-hide presets produce non-empty output.
#[test]
fn invariant_12_all_presets_render_without_panic() {
    let ctx = fixture_ctx();

    for preset in PRESETS {
        // Each placeholder name in the template must not panic.
        for name in placeholder_names(preset.template) {
            // render_placeholder returning None is fine — it just means
            // the value is absent in the fixture; the call must not panic.
            let _ = render_placeholder(&name, &ctx);
        }

        // For non-hide presets, the rendered segment must be non-empty.
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
