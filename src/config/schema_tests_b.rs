use super::schema_tests::{flex_seg, template_seg};
use super::*;

// ---------------------------------------------------------------------------
// Multiple errors collected (not short-circuit)
// ---------------------------------------------------------------------------

#[test]
fn multiple_errors_collected_too_many_lines_and_multi_flex() {
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![
            Line {
                separator: "".to_owned(),
                segments: vec![flex_seg(), flex_seg()],
            },
            Line {
                separator: "".to_owned(),
                segments: vec![template_seg("b")],
            },
            Line {
                separator: "".to_owned(),
                segments: vec![template_seg("c")],
            },
            Line {
                separator: "".to_owned(),
                segments: vec![template_seg("d")],
            },
        ],
    };
    let errors = cfg.validate_and_clamp().expect_err("must have errors");
    let has_too_many = errors
        .iter()
        .any(|e| e.kind == ValidationErrorKind::TooManyLines);
    let has_multi_flex = errors
        .iter()
        .any(|e| e.kind == ValidationErrorKind::MultipleFlex);
    assert!(has_too_many, "expected TooManyLines in errors: {errors:?}");
    assert!(
        has_multi_flex,
        "expected MultipleFlex in errors: {errors:?}"
    );
}

#[test]
fn multiple_flex_false_errors_all_collected() {
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![
                Segment::Flex(FlexSegment { flex: false }),
                Segment::Flex(FlexSegment { flex: false }),
            ],
        }],
    };
    let errors = cfg.validate_and_clamp().expect_err("must have errors");
    let flex_false_count = errors
        .iter()
        .filter(|e| e.kind == ValidationErrorKind::FlexFalse)
        .count();
    assert!(
        flex_false_count >= 2,
        "both FlexFalse errors must be collected, got {flex_false_count}: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// Padding clamp happens even when there are hard errors
// ---------------------------------------------------------------------------

#[test]
fn padding_clamped_even_when_hard_errors_present() {
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![
                Segment::Template(TemplateSegment::new("y").with_padding(100)),
                Segment::Flex(FlexSegment { flex: false }),
            ],
        }],
    };
    let _errors = cfg.validate_and_clamp().expect_err("FlexFalse must error");
    if let Segment::Template(t) = &cfg.lines[0].segments[0] {
        assert_eq!(
            t.padding, MAX_PADDING,
            "padding must be clamped even when validate returns Err"
        );
    } else {
        panic!("expected Template segment");
    }
}

// ---------------------------------------------------------------------------
// $schema field: order independence and preservation
// ---------------------------------------------------------------------------

#[test]
fn schema_field_first_in_json_still_deserializes() {
    let json = r#"{"$schema":"https://example.com/s.json","lines":[]}"#;
    let cfg: Config = serde_json::from_str(json).expect("$schema first must deserialize");
    assert_eq!(
        cfg.schema_url.as_deref(),
        Some("https://example.com/s.json")
    );
}

#[test]
fn schema_field_last_in_json_still_deserializes() {
    let json = r#"{"lines":[],"$schema":"https://example.com/s.json"}"#;
    let cfg: Config = serde_json::from_str(json).expect("$schema last must deserialize");
    assert_eq!(
        cfg.schema_url.as_deref(),
        Some("https://example.com/s.json")
    );
}

#[test]
fn schema_url_is_preserved_through_serialize_deserialize() {
    let url = "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/schema.json";
    let orig = Config {
        schema_url: Some(url.to_owned()),
        powerline: false,
        lines: vec![],
    };
    let json = serde_json::to_string(&orig).expect("serialize");
    assert!(
        json.contains(r#""$schema""#),
        "key must be $schema in JSON: {json}"
    );
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(back.schema_url.as_deref(), Some(url));
}

// ---------------------------------------------------------------------------
// Default derivation: Config::default()
// ---------------------------------------------------------------------------

#[test]
fn config_default_has_lines_and_no_schema_url() {
    // Config::default() returns the built-in "default" template, which has ≥ 1 line.
    let cfg = Config::default();
    assert!(
        !cfg.lines.is_empty(),
        "default config must have at least one line"
    );
    assert!(cfg.schema_url.is_none());
}

#[test]
fn config_default_is_valid() {
    let mut cfg = Config::default();
    let warnings = cfg
        .validate_and_clamp()
        .expect("default Config must be valid");
    assert!(warnings.is_empty());
}

// ---------------------------------------------------------------------------
// MAX_LINES boundary
// ---------------------------------------------------------------------------

#[test]
fn validate_one_over_max_lines_produces_exactly_one_too_many_lines_error() {
    let make_line = || Line {
        separator: "".to_owned(),
        segments: vec![],
    };
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: (0..MAX_LINES + 1).map(|_| make_line()).collect(),
    };
    let errors = cfg.validate_and_clamp().expect_err("must error");
    let count = errors
        .iter()
        .filter(|e| e.kind == ValidationErrorKind::TooManyLines)
        .count();
    assert_eq!(
        count, 1,
        "exactly one TooManyLines error expected: {errors:?}"
    );
}

// ---------------------------------------------------------------------------
// FlexSegment with flex:true on its own is valid
// ---------------------------------------------------------------------------

#[test]
fn flex_true_alone_on_line_is_valid() {
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![flex_seg()],
        }],
    };
    let warnings = cfg
        .validate_and_clamp()
        .expect("single flex:true must be valid");
    assert!(warnings.is_empty());
}

// ---------------------------------------------------------------------------
// Multiple lines: flex uniqueness is per-line, not global
// ---------------------------------------------------------------------------

#[test]
fn flex_on_different_lines_is_valid() {
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![
            Line {
                separator: "".to_owned(),
                segments: vec![flex_seg(), template_seg("a")],
            },
            Line {
                separator: "".to_owned(),
                segments: vec![flex_seg(), template_seg("b")],
            },
        ],
    };
    let warnings = cfg
        .validate_and_clamp()
        .expect("one flex per line across two lines must be valid");
    assert!(warnings.is_empty());
}

// ---------------------------------------------------------------------------
// Padding clamp warning includes correct from/to values
// ---------------------------------------------------------------------------

#[test]
fn padding_clamp_warning_reports_original_value_not_clamped_value() {
    let original_padding: u8 = 42;
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(
                TemplateSegment::new("z").with_padding(original_padding),
            )],
        }],
    };
    let warnings = cfg.validate_and_clamp().expect("must not hard-error");
    assert_eq!(warnings.len(), 1);
    match warnings[0].kind {
        ValidationWarningKind::PaddingClamped { from, to } => {
            assert_eq!(from, original_padding, "from must be the original padding");
            assert_eq!(to, MAX_PADDING, "to must be MAX_PADDING");
        }
    }
}

// ---------------------------------------------------------------------------
// Idempotency: calling validate_and_clamp twice
// ---------------------------------------------------------------------------

#[test]
fn validate_and_clamp_is_idempotent() {
    let mut cfg = Config {
        schema_url: None,
        powerline: false,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(
                TemplateSegment::new("x").with_padding(50),
            )],
        }],
    };
    let w1 = cfg.validate_and_clamp().expect("first call must not error");
    assert_eq!(w1.len(), 1, "first call must warn about clamp");
    let w2 = cfg
        .validate_and_clamp()
        .expect("second call must not error");
    assert!(
        w2.is_empty(),
        "second call must produce no warnings (already clamped)"
    );
}
