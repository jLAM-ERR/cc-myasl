use super::*;

// ---------------------------------------------------------------------------
// color / bg serde round-trips
// ---------------------------------------------------------------------------

#[test]
fn serde_round_trip_segment_with_color_and_bg() {
    let mut seg = TemplateSegment::new("{model}");
    seg.color = Some("cyan".to_owned());
    seg.bg = Some("blue".to_owned());
    let orig = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(seg)],
        }],
    };
    let json = serde_json::to_string(&orig).expect("serialize");
    assert!(
        json.contains("\"color\":\"cyan\""),
        "color must appear in JSON"
    );
    assert!(json.contains("\"bg\":\"blue\""), "bg must appear in JSON");
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(orig, back);
}

#[test]
fn serde_round_trip_segment_without_color_or_bg() {
    let orig = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
        }],
    };
    let json = serde_json::to_string(&orig).expect("serialize");
    assert!(
        !json.contains("\"color\""),
        "absent color must be omitted: {json}"
    );
    assert!(
        !json.contains("\"bg\""),
        "absent bg must be omitted: {json}"
    );
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(orig, back);
}

// Phase 1+2 configs without color/bg fields still deserialize correctly.
#[test]
fn back_compat_phase1_json_no_color_fields() {
    let json = r#"{"lines":[{"separator":" · ","segments":[{"template":"{model}","padding":1,"hide_when_absent":true}]}]}"#;
    let cfg: Config = serde_json::from_str(json).expect("phase1 config must deserialize");
    if let Segment::Template(t) = &cfg.lines[0].segments[0] {
        assert!(t.color.is_none(), "color must default to None");
        assert!(t.bg.is_none(), "bg must default to None");
    } else {
        panic!("expected Template segment");
    }
    assert!(!cfg.powerline, "powerline must default to false");
}

// ---------------------------------------------------------------------------
// color / bg validation
// ---------------------------------------------------------------------------

#[test]
fn validate_rejects_invalid_color_purple() {
    let mut seg = TemplateSegment::new("x");
    seg.color = Some("purple".to_owned());
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(seg)],
        }],
    };
    let errors = cfg
        .validate_and_clamp()
        .expect_err("purple must fail validation");
    let has_invalid_color = errors.iter().any(|e| {
        matches!(
            &e.kind,
            ValidationErrorKind::InvalidColor { field: "color", value }
            if value == "purple"
        )
    });
    assert!(
        has_invalid_color,
        "expected InvalidColor {{field:color, value:purple}}: {errors:?}"
    );
}

#[test]
fn validate_rejects_empty_string_color() {
    let mut seg = TemplateSegment::new("x");
    seg.color = Some("".to_owned());
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(seg)],
        }],
    };
    let errors = cfg
        .validate_and_clamp()
        .expect_err("empty string color must fail validation");
    assert!(
        errors
            .iter()
            .any(|e| matches!(&e.kind, ValidationErrorKind::InvalidColor { .. })),
        "expected InvalidColor error: {errors:?}"
    );
}

#[test]
fn validate_accepts_all_named_colors_for_fg() {
    for name in NAMED_COLORS {
        let mut seg = TemplateSegment::new("x");
        seg.color = Some((*name).to_owned());
        let mut cfg = Config {
            schema_url: None,
            powerline: false,
            lines: vec![Line {
                separator: "".to_owned(),
                segments: vec![Segment::Template(seg)],
            }],
        };
        let result = cfg.validate_and_clamp();
        assert!(
            result.is_ok(),
            "color '{name}' must be valid, got: {result:?}"
        );
    }
}

#[test]
fn validate_accepts_all_named_colors_for_bg() {
    for name in NAMED_COLORS {
        let mut seg = TemplateSegment::new("x");
        seg.bg = Some((*name).to_owned());
        let mut cfg = Config {
            schema_url: None,
            powerline: false,
            lines: vec![Line {
                separator: "".to_owned(),
                segments: vec![Segment::Template(seg)],
            }],
        };
        let result = cfg.validate_and_clamp();
        assert!(result.is_ok(), "bg '{name}' must be valid, got: {result:?}");
    }
}

#[test]
fn validate_accepts_none_color_and_bg() {
    let seg = TemplateSegment::new("x"); // color=None, bg=None
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(seg)],
        }],
    };
    let result = cfg.validate_and_clamp();
    assert!(result.is_ok(), "None color/bg must be valid: {result:?}");
}

#[test]
fn validate_rejects_invalid_bg_value() {
    let mut seg = TemplateSegment::new("x");
    seg.bg = Some("orange".to_owned());
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(seg)],
        }],
    };
    let errors = cfg
        .validate_and_clamp()
        .expect_err("orange bg must fail validation");
    let has_invalid_bg = errors.iter().any(|e| {
        matches!(
            &e.kind,
            ValidationErrorKind::InvalidColor { field: "bg", value }
            if value == "orange"
        )
    });
    assert!(
        has_invalid_bg,
        "expected InvalidColor {{field:bg, value:orange}}: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// powerline serde + validation
// ---------------------------------------------------------------------------

#[test]
fn serde_round_trip_config_with_powerline_true() {
    let orig = Config {
        schema_url: None,
        powerline: true,
        lines: vec![],
    };
    let json = serde_json::to_string(&orig).expect("serialize");
    assert!(
        json.contains("\"powerline\":true"),
        "powerline must appear: {json}"
    );
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(orig, back);
    assert!(back.powerline);
}

#[test]
fn serde_round_trip_config_with_powerline_false() {
    let orig = Config {
        schema_url: None,
        powerline: false,
        lines: vec![],
    };
    let json = serde_json::to_string(&orig).expect("serialize");
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(orig, back);
    assert!(!back.powerline);
}

#[test]
fn validate_accepts_powerline_true() {
    let mut cfg = Config {
        schema_url: None,
        powerline: true,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
        }],
    };
    let result = cfg.validate_and_clamp();
    assert!(result.is_ok(), "powerline:true must be valid: {result:?}");
}

#[test]
fn validate_accepts_powerline_false() {
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(TemplateSegment::new("{model}"))],
        }],
    };
    let result = cfg.validate_and_clamp();
    assert!(result.is_ok(), "powerline:false must be valid: {result:?}");
}

#[test]
fn powerline_defaults_to_false_on_deserialize() {
    let json = r#"{"lines":[]}"#;
    let cfg: Config = serde_json::from_str(json).expect("deserialize");
    assert!(!cfg.powerline, "powerline must default to false");
}
