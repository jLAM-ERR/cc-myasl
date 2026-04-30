use super::*;

pub(super) fn template_seg(tmpl: &str) -> Segment {
    Segment::Template(TemplateSegment {
        template: tmpl.to_owned(),
        padding: 0,
        hide_when_absent: false,
    })
}

pub(super) fn flex_seg() -> Segment {
    Segment::Flex(FlexSegment { flex: true })
}

// ---------------------------------------------------------------------------
// Boundary: padding values
// ---------------------------------------------------------------------------

#[test]
fn padding_zero_is_valid_and_no_warning() {
    let mut cfg = Config {
        schema_url: None,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(TemplateSegment {
                template: "x".to_owned(),
                padding: 0,
                hide_when_absent: false,
            })],
        }],
    };
    let warnings = cfg.validate_and_clamp().expect("padding=0 must not error");
    assert!(warnings.is_empty(), "padding=0 must not warn");
}

#[test]
fn padding_exactly_max_no_warning() {
    assert_eq!(MAX_PADDING, 8, "MAX_PADDING constant must be 8");
    let mut cfg = Config {
        schema_url: None,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(TemplateSegment {
                template: "x".to_owned(),
                padding: 8,
                hide_when_absent: false,
            })],
        }],
    };
    let warnings = cfg
        .validate_and_clamp()
        .expect("padding=8 (exact MAX) must not error");
    assert!(warnings.is_empty());
}

#[test]
fn padding_one_over_max_is_clamped() {
    let mut cfg = Config {
        schema_url: None,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(TemplateSegment {
                template: "x".to_owned(),
                padding: 9,
                hide_when_absent: false,
            })],
        }],
    };
    let warnings = cfg
        .validate_and_clamp()
        .expect("padding=9 clamp must not error");
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0].kind,
        ValidationWarningKind::PaddingClamped { from: 9, to: 8 }
    );
    if let Segment::Template(t) = &cfg.lines[0].segments[0] {
        assert_eq!(t.padding, MAX_PADDING, "struct must be mutated to MAX");
    }
}

#[test]
fn padding_u8_max_255_is_clamped_to_max_padding() {
    let mut cfg = Config {
        schema_url: None,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(TemplateSegment {
                template: "x".to_owned(),
                padding: 255,
                hide_when_absent: false,
            })],
        }],
    };
    let warnings = cfg
        .validate_and_clamp()
        .expect("padding=255 clamp must not error");
    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0].kind,
        ValidationWarningKind::PaddingClamped { from: 255, to: 8 }
    );
    if let Segment::Template(t) = &cfg.lines[0].segments[0] {
        assert_eq!(t.padding, MAX_PADDING, "struct must be mutated to MAX");
    }
}

#[test]
fn padding_warning_carries_correct_line_and_segment_index() {
    let mut cfg = Config {
        schema_url: None,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![
                Segment::Template(TemplateSegment {
                    template: "ok".to_owned(),
                    padding: 1,
                    hide_when_absent: false,
                }),
                Segment::Template(TemplateSegment {
                    template: "bad".to_owned(),
                    padding: 200,
                    hide_when_absent: false,
                }),
            ],
        }],
    };
    let warnings = cfg.validate_and_clamp().expect("must not error");
    assert_eq!(warnings.len(), 1);
    assert_eq!(warnings[0].line_index, 0);
    assert_eq!(warnings[0].segment_index, 1);
}

// ---------------------------------------------------------------------------
// Boundary: empty containers
// ---------------------------------------------------------------------------

#[test]
fn empty_lines_array_is_valid() {
    let mut cfg = Config {
        schema_url: None,
        lines: vec![],
    };
    let warnings = cfg
        .validate_and_clamp()
        .expect("empty lines must not error");
    assert!(warnings.is_empty());
}

#[test]
fn empty_segments_on_a_line_is_valid() {
    let mut cfg = Config {
        schema_url: None,
        lines: vec![Line {
            separator: " | ".to_owned(),
            segments: vec![],
        }],
    };
    let warnings = cfg
        .validate_and_clamp()
        .expect("line with zero segments must not error");
    assert!(warnings.is_empty());
}

// ---------------------------------------------------------------------------
// Boundary: template string content
// ---------------------------------------------------------------------------

#[test]
fn empty_template_string_deserializes_and_is_not_rejected() {
    let json = r#"{"lines":[{"segments":[{"template":""}]}]}"#;
    let cfg: Config = serde_json::from_str(json).expect("empty template string must deserialize");
    if let Segment::Template(t) = &cfg.lines[0].segments[0] {
        assert_eq!(t.template, "");
    } else {
        panic!("expected Template segment");
    }
    let mut cfg = cfg;
    cfg.validate_and_clamp()
        .expect("empty template must not be a validation hard error");
}

#[test]
fn whitespace_only_template_deserializes_and_is_not_rejected() {
    let json = r#"{"lines":[{"segments":[{"template":"   "}]}]}"#;
    let cfg: Config =
        serde_json::from_str(json).expect("whitespace-only template string must deserialize");
    if let Segment::Template(t) = &cfg.lines[0].segments[0] {
        assert_eq!(t.template, "   ");
    } else {
        panic!("expected Template segment");
    }
    let mut cfg = cfg;
    cfg.validate_and_clamp()
        .expect("whitespace-only template must not be a validation hard error");
}

#[test]
fn very_long_template_string_round_trips() {
    let long = "x".repeat(10_000);
    let mut cfg = Config {
        schema_url: None,
        lines: vec![Line {
            separator: "".to_owned(),
            segments: vec![Segment::Template(TemplateSegment {
                template: long.clone(),
                padding: 0,
                hide_when_absent: false,
            })],
        }],
    };
    let json = serde_json::to_string(&cfg).expect("serialize");
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    if let Segment::Template(t) = &back.lines[0].segments[0] {
        assert_eq!(t.template.len(), 10_000);
    }
    cfg.validate_and_clamp()
        .expect("10k-char template must not error");
}

#[test]
fn unicode_multibyte_template_string_round_trips() {
    let unicode = "日本語テスト 🦀 émojis こんにちは".to_owned();
    let orig = Config {
        schema_url: None,
        lines: vec![Line {
            separator: "→".to_owned(),
            segments: vec![Segment::Template(TemplateSegment {
                template: unicode.clone(),
                padding: 0,
                hide_when_absent: false,
            })],
        }],
    };
    let json = serde_json::to_string(&orig).expect("serialize");
    let back: Config = serde_json::from_str(&json).expect("deserialize");
    assert_eq!(orig, back);
    if let Segment::Template(t) = &back.lines[0].segments[0] {
        assert_eq!(t.template, unicode);
    }
}

// ---------------------------------------------------------------------------
// Untagged enum disambiguation
// ---------------------------------------------------------------------------

// serde untagged tries variants in order: Template first, then Flex.
// A JSON object with both "template" and "flex" fields resolves to Template.
#[test]
fn ambiguous_json_with_template_and_flex_resolves_to_template() {
    let json = r#"{"lines":[{"segments":[{"template":"x","flex":true}]}]}"#;
    let cfg: Config = serde_json::from_str(json).expect(
        "object with both template and flex must deserialize (serde untagged picks Template)",
    );
    match &cfg.lines[0].segments[0] {
        Segment::Template(t) => assert_eq!(t.template, "x"),
        Segment::Flex(_) => panic!("expected Template (untagged ordering), got Flex"),
    }
}

#[test]
fn flex_true_alone_deserializes_to_flex_segment() {
    let json = r#"{"lines":[{"segments":[{"flex":true}]}]}"#;
    let cfg: Config =
        serde_json::from_str(json).expect("flex:true must deserialize as FlexSegment");
    match &cfg.lines[0].segments[0] {
        Segment::Flex(f) => assert!(f.flex),
        Segment::Template(_) => panic!("expected Flex segment"),
    }
}

#[test]
fn flex_false_deserializes_to_flex_false_then_validate_rejects() {
    let json = r#"{"lines":[{"segments":[{"flex":false}]}]}"#;
    let cfg: Config = serde_json::from_str(json).expect("flex:false must deserialize");
    match &cfg.lines[0].segments[0] {
        Segment::Flex(f) => assert!(!f.flex, "must be Flex(false)"),
        Segment::Template(_) => panic!("expected Flex segment"),
    }
    let mut cfg = cfg;
    let errors = cfg
        .validate_and_clamp()
        .expect_err("flex:false must fail validation");
    assert!(errors
        .iter()
        .any(|e| e.kind == ValidationErrorKind::FlexFalse));
}

#[test]
fn empty_object_segment_fails_deserialization() {
    let json = r#"{"lines":[{"segments":[{}]}]}"#;
    let result: Result<Config, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "empty segment object must fail deserialization (no template or flex field)"
    );
}

#[test]
fn null_template_field_fails_deserialization() {
    let json = r#"{"lines":[{"segments":[{"template":null}]}]}"#;
    let result: Result<Config, _> = serde_json::from_str(json);
    assert!(
        result.is_err(),
        "template:null must fail deserialization (String cannot be null)"
    );
}
