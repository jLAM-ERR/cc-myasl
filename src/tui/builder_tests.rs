use crate::config::builtins;
use crate::config::named_color::NamedColor;
use crate::config::schema::{Config, FlexSegment, Line, Segment, TemplateSegment};

use super::{BuilderSegment, from_config, to_config};

// ── round-trip every builtin ─────────────────────────────────────────────────

#[test]
fn every_builtin_round_trips_via_builder() {
    for &name in builtins::ALL_NAMES {
        let orig = builtins::lookup(name).expect("builtin must exist");
        let orig_json = serde_json::to_string(&orig).expect("serialize original");
        let builder = from_config(&orig);
        let rebuilt = to_config(&builder);
        let rebuilt_json = serde_json::to_string(&rebuilt).expect("serialize rebuilt");
        let orig_val: serde_json::Value = serde_json::from_str(&orig_json).expect("parse original");
        let rebuilt_val: serde_json::Value =
            serde_json::from_str(&rebuilt_json).expect("parse rebuilt");
        assert_eq!(
            orig_val, rebuilt_val,
            "{name}: round-trip produced different JSON"
        );
    }
}

// ── custom template survives round-trip ──────────────────────────────────────

#[test]
fn custom_template_becomes_custom_segment() {
    let cfg = Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![Segment::Template(TemplateSegment::new("${cost_usd}"))],
        }],
    };
    let builder = from_config(&cfg);
    assert_eq!(builder.lines.len(), 1);
    match &builder.lines[0].segments[0] {
        BuilderSegment::Custom { template, .. } => {
            assert_eq!(template, "${cost_usd}");
        }
        other => panic!("expected Custom, got {other:?}"),
    }
    let rebuilt = to_config(&builder);
    let orig_val: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&cfg).unwrap()).unwrap();
    let rebuilt_val: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&rebuilt).unwrap()).unwrap();
    assert_eq!(orig_val, rebuilt_val, "custom round-trip must be exact");
}

// ── preset color override preserved ──────────────────────────────────────────

#[test]
fn preset_color_override_survives_round_trip() {
    // Use a catalog template with a user-specified color override.
    // Catalog has "\u{1f916} {model}" → id "model_name".
    let mut seg = TemplateSegment::new("\u{1f916} {model}");
    seg.color = Some("red".to_owned());
    let cfg = Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![Segment::Template(seg)],
        }],
    };
    let builder = from_config(&cfg);
    match &builder.lines[0].segments[0] {
        BuilderSegment::Preset { id, color, .. } => {
            assert_eq!(*id, "model_name");
            assert_eq!(
                *color,
                Some(NamedColor::Red),
                "color override must be preserved"
            );
        }
        other => panic!("expected Preset, got {other:?}"),
    }
    let rebuilt = to_config(&builder);
    if let Segment::Template(t) = &rebuilt.lines[0].segments[0] {
        assert_eq!(
            t.color.as_deref(),
            Some("red"),
            "color must survive to_config"
        );
    } else {
        panic!("expected Template segment");
    }
}

// ── default_fg / default_bg round-trip ──────────────────────────────────────

#[test]
fn default_fg_survives_round_trip() {
    let cfg = Config {
        schema_url: None,
        powerline: false,
        default_fg: Some(NamedColor::Cyan),
        default_bg: None,
        lines: vec![],
    };
    let builder = from_config(&cfg);
    assert_eq!(builder.default_fg, Some(NamedColor::Cyan));
    assert_eq!(builder.default_bg, None);
    let rebuilt = to_config(&builder);
    assert_eq!(rebuilt.default_fg, Some(NamedColor::Cyan));
    assert_eq!(rebuilt.default_bg, None);
}

#[test]
fn default_fg_none_omitted_from_json() {
    let cfg = Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![],
    };
    let json = serde_json::to_string(&cfg).expect("serialize");
    assert!(
        !json.contains("default_fg"),
        "None default_fg must be omitted: {json}"
    );
    assert!(
        !json.contains("default_bg"),
        "None default_bg must be omitted: {json}"
    );
}

#[test]
fn default_bg_survives_round_trip() {
    let cfg = Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: Some(NamedColor::Blue),
        lines: vec![],
    };
    let builder = from_config(&cfg);
    let rebuilt = to_config(&builder);
    assert_eq!(rebuilt.default_bg, Some(NamedColor::Blue));
    assert_eq!(rebuilt.default_fg, None);
}

// ── legacy config (no new fields) deserializes cleanly ──────────────────────

#[test]
fn legacy_config_without_new_fields_deserializes() {
    let json = r#"{"lines":[{"separator":"","segments":[{"template":"{model}","padding":0,"hide_when_absent":false}]}]}"#;
    let cfg: Config = serde_json::from_str(json).expect("legacy config must deserialize");
    assert_eq!(
        cfg.default_fg, None,
        "default_fg must be None for legacy config"
    );
    assert_eq!(
        cfg.default_bg, None,
        "default_bg must be None for legacy config"
    );
    // Re-serializing must not add the new keys.
    let out = serde_json::to_string(&cfg).expect("serialize");
    assert!(
        !out.contains("default_fg"),
        "None fields must be omitted: {out}"
    );
    assert!(
        !out.contains("default_bg"),
        "None fields must be omitted: {out}"
    );
}

// ── empty config round-trip ──────────────────────────────────────────────────

#[test]
fn empty_config_round_trips() {
    let cfg = Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![],
    };
    let builder = from_config(&cfg);
    assert!(builder.lines.is_empty());
    assert!(!builder.powerline);
    assert_eq!(builder.default_fg, None);
    assert_eq!(builder.default_bg, None);
    let rebuilt = to_config(&builder);
    assert_eq!(cfg, rebuilt);
}

// ── FlexSpacer preserved ─────────────────────────────────────────────────────

#[test]
fn flex_spacer_preserved_through_round_trip() {
    let cfg = Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![Line {
            separator: String::new(),
            segments: vec![
                Segment::Template(TemplateSegment::new("{model}")),
                Segment::Flex(FlexSegment { flex: true }),
                Segment::Template(TemplateSegment::new("{five_left}%")),
            ],
        }],
    };
    let builder = from_config(&cfg);
    assert!(
        matches!(builder.lines[0].segments[1], BuilderSegment::FlexSpacer),
        "middle segment must be FlexSpacer"
    );
    let rebuilt = to_config(&builder);
    match &rebuilt.lines[0].segments[1] {
        Segment::Flex(f) => assert!(f.flex, "flex must be true"),
        _ => panic!("expected Flex segment at index 1"),
    }
}

// ── BuilderState fields ───────────────────────────────────────────────────────

#[test]
fn from_config_copies_powerline_and_schema_url() {
    let cfg = Config {
        schema_url: Some("https://example.com/schema.json".to_owned()),
        powerline: true,
        default_fg: None,
        default_bg: None,
        lines: vec![],
    };
    let builder = from_config(&cfg);
    assert!(builder.powerline);
    assert_eq!(
        builder.schema_url.as_deref(),
        Some("https://example.com/schema.json")
    );
    let rebuilt = to_config(&builder);
    assert!(rebuilt.powerline);
    assert_eq!(
        rebuilt.schema_url.as_deref(),
        Some("https://example.com/schema.json")
    );
}

// ── catalog Preset lookup by id ───────────────────────────────────────────────

#[test]
fn lookup_by_id_matches_lookup_by_template() {
    use crate::tui::catalog;
    for preset in catalog::PRESETS {
        let by_id = catalog::lookup_by_id(preset.id).expect("id must resolve");
        let by_tmpl = catalog::lookup(preset.template).expect("template must resolve");
        assert_eq!(by_id.id, by_tmpl.id, "id and template lookup must agree");
    }
}
