//! Hardcoded built-in template configs — bit-identical translation of
//! `templates/<name>.txt`: `{? ... }` → `hide_when_absent: true` segments.

mod templates;
use templates::*;

use crate::config::schema::Config;

/// Look up a built-in template by name.
///
/// Returns `Some(Config)` for one of the 9 shipped templates;
/// `None` for any unknown name.
pub fn lookup(name: &str) -> Option<Config> {
    match name {
        "default" => Some(default_config()),
        "minimal" => Some(minimal_config()),
        "compact" => Some(compact_config()),
        "bars" => Some(bars_config()),
        "colored" => Some(colored_config()),
        "emoji" => Some(emoji_config()),
        "emoji_verbose" => Some(emoji_verbose_config()),
        "verbose" => Some(verbose_config()),
        "rich" => Some(rich_config()),
        _ => None,
    }
}

#[cfg(test)]
#[path = "builtins/builtins_tests.rs"]
mod builtins_tests;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::schema::{Segment, TemplateSegment};

    const ALL_NAMES: &[&str] = &[
        "default",
        "minimal",
        "compact",
        "bars",
        "colored",
        "emoji",
        "emoji_verbose",
        "verbose",
        "rich",
    ];

    #[test]
    fn lookup_returns_some_for_every_name() {
        for name in ALL_NAMES {
            assert!(lookup(name).is_some(), "lookup({name}) returned None");
        }
    }

    #[test]
    fn lookup_returns_none_for_unknown_name() {
        assert!(lookup("does-not-exist").is_none());
        assert!(lookup("").is_none());
        assert!(lookup("DEFAULT").is_none());
    }

    #[test]
    fn every_config_validates_without_errors() {
        for name in ALL_NAMES {
            let mut cfg = lookup(name).unwrap();
            let result = cfg.validate_and_clamp();
            assert!(
                result.is_ok(),
                "{name} validate_and_clamp returned errors: {result:?}"
            );
        }
    }

    #[test]
    fn every_config_has_at_least_one_segment_on_line_0() {
        for name in ALL_NAMES {
            let cfg = lookup(name).unwrap();
            assert!(!cfg.lines.is_empty(), "{name} has no lines");
            assert!(
                !cfg.lines[0].segments.is_empty(),
                "{name} line 0 has no segments"
            );
        }
    }

    #[test]
    fn every_config_has_empty_separator() {
        for name in ALL_NAMES {
            let cfg = lookup(name).unwrap();
            for (li, line) in cfg.lines.iter().enumerate() {
                assert_eq!(
                    line.separator, "",
                    "{name} line {li} has non-empty separator: {:?}",
                    line.separator
                );
            }
        }
    }

    #[test]
    fn default_template_segment_structure() {
        let cfg = lookup("default").unwrap();
        let segs = &cfg.lines[0].segments;
        assert_eq!(segs.len(), 4);
        assert_seg(segs, 0, "{model}", false);
        assert_seg(segs, 1, " · 5h: {five_left}%", true);
        assert_seg(segs, 2, " · 7d: {seven_left}%", true);
        assert_seg(segs, 3, " (resets {seven_reset_clock})", true);
    }

    #[test]
    fn minimal_template_segment_structure() {
        let cfg = lookup("minimal").unwrap();
        let segs = &cfg.lines[0].segments;
        assert_eq!(segs.len(), 2);
        assert_seg(segs, 0, "{model}", false);
        assert_seg(segs, 1, " {five_left}%/{seven_left}%", true);
    }

    #[test]
    fn compact_template_segment_structure() {
        let cfg = lookup("compact").unwrap();
        let segs = &cfg.lines[0].segments;
        assert_eq!(segs.len(), 2);
        assert_seg(segs, 0, "{model}", false);
        assert_seg(segs, 1, " {five_left}/{seven_left}", true);
    }

    #[test]
    fn bars_template_segment_structure() {
        let cfg = lookup("bars").unwrap();
        let segs = &cfg.lines[0].segments;
        assert_eq!(segs.len(), 3);
        assert_seg(segs, 0, "{model}", false);
        assert_seg(segs, 1, " 5h:{five_bar}", true);
        assert_seg(segs, 2, " 7d:{seven_bar}", true);
    }

    #[test]
    fn colored_template_segment_structure() {
        let cfg = lookup("colored").unwrap();
        let segs = &cfg.lines[0].segments;
        assert_eq!(segs.len(), 3);
        assert_seg(segs, 0, "{model}", false);
        assert_seg(segs, 1, " · 5h: {five_color}{five_left}%{reset}", true);
        assert_seg(segs, 2, " · 7d: {seven_color}{seven_left}%{reset}", true);
    }

    #[test]
    fn emoji_template_segment_structure() {
        let cfg = lookup("emoji").unwrap();
        let segs = &cfg.lines[0].segments;
        assert_eq!(segs.len(), 3);
        assert_seg(segs, 0, "{model}", false);
        assert_seg(segs, 1, " · {five_state} 5h {five_left}%", true);
        assert_seg(segs, 2, " · {seven_state} 7d {seven_left}%", true);
    }

    #[test]
    fn emoji_verbose_template_segment_structure() {
        let cfg = lookup("emoji_verbose").unwrap();
        let segs = &cfg.lines[0].segments;
        assert_eq!(segs.len(), 4);
        assert_seg(segs, 0, "🤖 {model}", false);
        assert_seg(segs, 1, " · {state_icon} {cwd_basename}", true);
        assert_seg(segs, 2, " · ⏳ {five_left}%/{seven_left}%", true);
        assert_seg(segs, 3, " · ⏰ {seven_reset_clock}", true);
    }

    #[test]
    fn verbose_template_segment_structure() {
        let cfg = lookup("verbose").unwrap();
        let segs = &cfg.lines[0].segments;
        assert_eq!(segs.len(), 5);
        assert_seg(segs, 0, "{model}", false);
        assert_seg(segs, 1, " · {cwd_basename}", true);
        assert_seg(
            segs,
            2,
            " · 5h:{five_bar} {five_left}% (in {five_reset_in})",
            true,
        );
        assert_seg(
            segs,
            3,
            " · 7d:{seven_bar} {seven_left}% (in {seven_reset_in})",
            true,
        );
        assert_seg(segs, 4, " · extra:{extra_left}", true);
    }

    fn assert_seg(segs: &[Segment], idx: usize, tmpl: &str, hide: bool) {
        match &segs[idx] {
            Segment::Template(t) => {
                assert_eq!(t.template, tmpl, "seg[{idx}].template mismatch");
                assert_eq!(t.hide_when_absent, hide, "seg[{idx}].hide_when_absent");
                assert_eq!(t.padding, 0, "seg[{idx}].padding should be 0");
            }
            Segment::Flex(_) => panic!("seg[{idx}] is Flex, expected Template"),
        }
    }

    #[test]
    fn template_segment_new_has_defaults() {
        let t = TemplateSegment::new("hello");
        assert_eq!(t.template, "hello");
        assert_eq!(t.padding, 0);
        assert!(!t.hide_when_absent);
    }

    #[test]
    fn with_hide_when_absent_flips_flag_and_chains() {
        let t = TemplateSegment::new("x").with_hide_when_absent();
        assert!(t.hide_when_absent);
        assert_eq!(t.template, "x");
    }

    #[test]
    fn with_padding_sets_value_and_chains() {
        let t = TemplateSegment::new("x").with_padding(3);
        assert_eq!(t.padding, 3);
        assert_eq!(t.template, "x");
        assert!(!t.hide_when_absent);
    }

    #[test]
    fn builder_chain_all_methods() {
        let t = TemplateSegment::new("tmpl")
            .with_hide_when_absent()
            .with_padding(5);
        assert_eq!(t.template, "tmpl");
        assert!(t.hide_when_absent);
        assert_eq!(t.padding, 5);
    }

    #[test]
    fn from_template_segment_produces_segment_template_variant() {
        let ts = TemplateSegment::new("y").with_hide_when_absent();
        let seg: Segment = ts.into();
        match seg {
            Segment::Template(t) => {
                assert_eq!(t.template, "y");
                assert!(t.hide_when_absent);
            }
            Segment::Flex(_) => panic!("expected Segment::Template"),
        }
    }

    // Rendering tests use render_config_manually to simulate config::render collapse semantics.
    #[allow(deprecated)]
    #[test]
    fn default_renders_with_full_ctx() {
        use crate::format;
        let ctx = snapshot_ctx();
        let cfg = lookup("default").unwrap();
        let out = render_config_manually(&cfg, &ctx);
        assert!(
            out.starts_with("claude-opus-4-7 · 5h: 76% · 7d: 59%"),
            "got: {out:?}"
        );
        assert!(out.contains("76%"));
        assert!(out.contains("59%"));
        let _ = format::render("{model}", &ctx);
    }

    #[test]
    fn minimal_renders_with_full_ctx() {
        let ctx = snapshot_ctx();
        let out = render_config_manually(&lookup("minimal").unwrap(), &ctx);
        assert_eq!(out, "claude-opus-4-7 76%/59%");
    }

    #[test]
    fn compact_renders_with_full_ctx() {
        let ctx = snapshot_ctx();
        let out = render_config_manually(&lookup("compact").unwrap(), &ctx);
        assert_eq!(out, "claude-opus-4-7 76/59");
    }

    #[test]
    fn bars_renders_with_full_ctx() {
        let ctx = snapshot_ctx();
        let out = render_config_manually(&lookup("bars").unwrap(), &ctx);
        assert_eq!(out, "claude-opus-4-7 5h:[████████░░] 7d:[██████░░░░]");
    }

    #[test]
    fn colored_renders_with_full_ctx() {
        let ctx = snapshot_ctx();
        let out = render_config_manually(&lookup("colored").unwrap(), &ctx);
        let expected = "claude-opus-4-7 · 5h: \x1b[32m76%\x1b[0m · 7d: \x1b[32m59%\x1b[0m";
        assert_eq!(out, expected);
    }

    #[test]
    fn emoji_renders_with_full_ctx() {
        let ctx = snapshot_ctx();
        let out = render_config_manually(&lookup("emoji").unwrap(), &ctx);
        assert_eq!(out, "claude-opus-4-7 · 🟢 5h 76% · 🟢 7d 59%");
    }

    #[test]
    fn emoji_verbose_renders_non_empty() {
        let ctx = snapshot_ctx();
        let out = render_config_manually(&lookup("emoji_verbose").unwrap(), &ctx);
        assert!(
            out.starts_with("🤖 claude-opus-4-7 · 🟢 proj · ⏳ 76%/59% · ⏰"),
            "got: {out:?}"
        );
    }

    #[test]
    fn verbose_renders_non_empty() {
        let ctx = snapshot_ctx();
        let out = render_config_manually(&lookup("verbose").unwrap(), &ctx);
        assert!(
            out.starts_with("claude-opus-4-7 · proj · 5h:[████████░░] 76% (in"),
            "got: {out:?}"
        );
        assert!(out.contains("7d:[██████░░░░] 59% (in"), "got: {out:?}");
    }

    #[test]
    fn all_templates_render_empty_ctx_without_panic() {
        use crate::format::placeholders::RenderCtx;
        let ctx = RenderCtx::default();
        for name in ALL_NAMES {
            let _ = render_config_manually(&lookup(name).unwrap(), &ctx);
        }
    }

    // Simulates config::render collapse semantics.
    #[allow(deprecated)]
    fn render_config_manually(
        cfg: &Config,
        ctx: &crate::format::placeholders::RenderCtx,
    ) -> String {
        use crate::config::schema::Segment;
        use crate::format;
        let mut line_strs = Vec::new();
        for line in cfg.lines.iter().take(crate::config::schema::MAX_LINES) {
            let mut parts: Vec<String> = Vec::new();
            for seg in &line.segments {
                if let Segment::Template(t) = seg {
                    let rendered = format::render(&t.template, ctx);
                    if !rendered.is_empty() || !t.hide_when_absent {
                        parts.push(rendered);
                    }
                }
            }
            line_strs.push(parts.join(&line.separator));
        }
        line_strs.join("\n")
    }

    // RenderCtx matching tests/fixtures/pro_max_with_rate_limits.json:
    // five_used=24 → five_left=76; seven_used=41 → seven_left=59; cwd basename=proj
    fn snapshot_ctx() -> crate::format::placeholders::RenderCtx {
        use std::path::PathBuf;
        crate::format::placeholders::RenderCtx {
            model: Some("claude-opus-4-7".to_owned()),
            cwd: Some(PathBuf::from("/Users/test/proj")),
            five_used: Some(24.0),
            five_reset_unix: Some(1745700000),
            seven_used: Some(41.0),
            seven_reset_unix: Some(1746000000),
            extra_enabled: Some(false),
            extra_used: None,
            extra_limit: None,
            extra_pct: None,
            now_unix: 0,
            ..Default::default()
        }
    }
}
