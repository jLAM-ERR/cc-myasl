use serde::{Deserialize, Serialize};

pub const MAX_LINES: usize = 3;
pub const MAX_PADDING: u8 = 8;

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Config {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema_url: Option<String>,
    pub lines: Vec<Line>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct Line {
    #[serde(default)]
    pub separator: String,
    #[serde(default)]
    pub segments: Vec<Segment>,
}

/// Variant order matters: `Template` is tried first by `#[serde(untagged)]`.
/// JSON with both `template` and `flex` keys silently resolves to `Template`.
/// JSON with only `flex` resolves to `Flex`. Reordering would flip behaviour.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Segment {
    Template(TemplateSegment),
    Flex(FlexSegment),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TemplateSegment {
    pub template: String,
    #[serde(default)]
    pub padding: u8,
    #[serde(default)]
    pub hide_when_absent: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct FlexSegment {
    pub flex: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationError {
    /// `None` for config-level errors (e.g. TooManyLines); `Some(idx)` for line-scoped errors.
    pub line_index: Option<usize>,
    pub segment_index: Option<usize>,
    pub kind: ValidationErrorKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationErrorKind {
    TooManyLines,
    MultipleFlex,
    /// FlexSegment with `flex: false` — semantically invalid.
    FlexFalse,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValidationWarning {
    pub line_index: usize,
    pub segment_index: usize,
    pub kind: ValidationWarningKind,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ValidationWarningKind {
    PaddingClamped { from: u8, to: u8 },
}

impl Config {
    /// Validates the config and clamps out-of-range padding values.
    ///
    /// Returns `Ok(warnings)` when there are no hard errors.
    /// Returns `Err(errors)` when hard constraints are violated.
    /// Mutates `self` to apply padding clamps regardless of errors.
    pub fn validate_and_clamp(&mut self) -> Result<Vec<ValidationWarning>, Vec<ValidationError>> {
        let mut errors: Vec<ValidationError> = Vec::new();
        let mut warnings: Vec<ValidationWarning> = Vec::new();

        if self.lines.len() > MAX_LINES {
            errors.push(ValidationError {
                line_index: None,
                segment_index: None,
                kind: ValidationErrorKind::TooManyLines,
            });
        }

        for (li, line) in self.lines.iter_mut().enumerate() {
            let mut flex_count = 0usize;
            for (si, segment) in line.segments.iter_mut().enumerate() {
                match segment {
                    Segment::Template(t) => {
                        if t.padding > MAX_PADDING {
                            let from = t.padding;
                            t.padding = MAX_PADDING;
                            warnings.push(ValidationWarning {
                                line_index: li,
                                segment_index: si,
                                kind: ValidationWarningKind::PaddingClamped {
                                    from,
                                    to: MAX_PADDING,
                                },
                            });
                        }
                    }
                    Segment::Flex(f) => {
                        if !f.flex {
                            errors.push(ValidationError {
                                line_index: Some(li),
                                segment_index: Some(si),
                                kind: ValidationErrorKind::FlexFalse,
                            });
                        }
                        flex_count += 1;
                        if flex_count > 1 {
                            errors.push(ValidationError {
                                line_index: Some(li),
                                segment_index: Some(si),
                                kind: ValidationErrorKind::MultipleFlex,
                            });
                        }
                    }
                }
            }
        }

        if errors.is_empty() {
            Ok(warnings)
        } else {
            Err(errors)
        }
    }
}

#[cfg(test)]
#[path = "schema_tests.rs"]
mod schema_tests;

#[cfg(test)]
#[path = "schema_tests_b.rs"]
mod schema_tests_b;

#[cfg(test)]
mod tests {
    use super::*;

    fn template_seg(tmpl: &str) -> Segment {
        Segment::Template(TemplateSegment {
            template: tmpl.to_owned(),
            padding: 0,
            hide_when_absent: false,
        })
    }

    fn flex_seg() -> Segment {
        Segment::Flex(FlexSegment { flex: true })
    }

    fn minimal_config() -> Config {
        Config {
            schema_url: None,
            lines: vec![Line {
                separator: " | ".to_owned(),
                segments: vec![template_seg("{five_left}%")],
            }],
        }
    }

    fn full_config() -> Config {
        Config {
            schema_url: Some("https://example.com/schema.json".to_owned()),
            lines: vec![
                Line {
                    separator: " · ".to_owned(),
                    segments: vec![
                        template_seg("{five_left}%"),
                        flex_seg(),
                        Segment::Template(TemplateSegment {
                            template: "{model}".to_owned(),
                            padding: 2,
                            hide_when_absent: true,
                        }),
                    ],
                },
                Line {
                    separator: "".to_owned(),
                    segments: vec![template_seg("{seven_left}%")],
                },
            ],
        }
    }

    // --- round-trip serde ---

    #[test]
    fn serde_round_trip_minimal() {
        let orig = minimal_config();
        let json = serde_json::to_string(&orig).expect("serialize");
        let back: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(orig, back);
    }

    #[test]
    fn serde_round_trip_full() {
        let orig = full_config();
        let json = serde_json::to_string(&orig).expect("serialize");
        let back: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(orig, back);
    }

    #[test]
    fn serde_round_trip_with_schema_field() {
        let orig = Config {
            schema_url: Some(
                "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json"
                    .to_owned(),
            ),
            lines: vec![],
        };
        let json = serde_json::to_string(&orig).expect("serialize");
        assert!(
            json.contains("$schema"),
            "serialized JSON must contain $schema key"
        );
        let back: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(orig, back);
    }

    #[test]
    fn serde_round_trip_without_schema_field() {
        let orig = Config {
            schema_url: None,
            lines: vec![],
        };
        let json = serde_json::to_string(&orig).expect("serialize");
        assert!(!json.contains("$schema"), "None schema_url must be omitted");
        let back: Config = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(orig, back);
    }

    #[test]
    fn deserialize_schema_url_from_dollar_schema_key() {
        let json = r#"{"$schema":"https://example.com/s.json","lines":[]}"#;
        let cfg: Config = serde_json::from_str(json).expect("deserialize with $schema key");
        assert_eq!(
            cfg.schema_url.as_deref(),
            Some("https://example.com/s.json")
        );
    }

    // --- unknown-field tolerance ---

    #[test]
    fn unknown_top_level_field_is_ignored() {
        let json = r#"{"lines":[],"extra_field":"should be ignored","version":2}"#;
        let cfg: Config =
            serde_json::from_str(json).expect("unknown top-level field must not error");
        assert_eq!(cfg.lines.len(), 0);
    }

    #[test]
    fn unknown_field_in_line_is_ignored() {
        let json = r#"{"lines":[{"separator":" | ","segments":[],"unknown_line_field":true}]}"#;
        let cfg: Config = serde_json::from_str(json).expect("unknown line field must not error");
        assert_eq!(cfg.lines.len(), 1);
        assert_eq!(cfg.lines[0].separator, " | ");
    }

    #[test]
    fn unknown_field_in_template_segment_is_ignored() {
        let json = r#"{"lines":[{"separator":"","segments":[{"template":"x","padding":0,"hide_when_absent":false,"unknown_seg_field":"yes"}]}]}"#;
        let cfg: Config = serde_json::from_str(json).expect("unknown segment field must not error");
        assert_eq!(cfg.lines[0].segments.len(), 1);
        if let Segment::Template(t) = &cfg.lines[0].segments[0] {
            assert_eq!(t.template, "x");
        } else {
            panic!("expected Template segment");
        }
    }

    // --- validation rejections ---

    #[test]
    fn validate_rejects_too_many_lines() {
        let mut cfg = Config {
            schema_url: None,
            lines: vec![
                Line {
                    separator: "".to_owned(),
                    segments: vec![template_seg("a")],
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
        let result = cfg.validate_and_clamp();
        let errors = result.expect_err("4 lines should produce errors");
        assert!(
            errors
                .iter()
                .any(|e| e.kind == ValidationErrorKind::TooManyLines),
            "expected TooManyLines error, got: {errors:?}"
        );
    }

    #[test]
    fn validate_rejects_two_flex_on_one_line() {
        let mut cfg = Config {
            schema_url: None,
            lines: vec![Line {
                separator: "".to_owned(),
                segments: vec![flex_seg(), template_seg("x"), flex_seg()],
            }],
        };
        let result = cfg.validate_and_clamp();
        let errors = result.expect_err("two flex segments should produce errors");
        assert!(
            errors
                .iter()
                .any(|e| e.kind == ValidationErrorKind::MultipleFlex),
            "expected MultipleFlex error, got: {errors:?}"
        );
    }

    #[test]
    fn validate_rejects_flex_false() {
        let mut cfg = Config {
            schema_url: None,
            lines: vec![Line {
                separator: "".to_owned(),
                segments: vec![Segment::Flex(FlexSegment { flex: false })],
            }],
        };
        let result = cfg.validate_and_clamp();
        let errors = result.expect_err("flex:false should produce errors");
        assert!(
            errors
                .iter()
                .any(|e| e.kind == ValidationErrorKind::FlexFalse),
            "expected FlexFalse error, got: {errors:?}"
        );
    }

    // --- padding clamp ---

    #[test]
    fn validate_clamps_padding_over_max() {
        let mut cfg = Config {
            schema_url: None,
            lines: vec![Line {
                separator: "".to_owned(),
                segments: vec![Segment::Template(TemplateSegment {
                    template: "x".to_owned(),
                    padding: 99,
                    hide_when_absent: false,
                })],
            }],
        };
        let warnings = cfg
            .validate_and_clamp()
            .expect("padding clamp should not error");
        assert_eq!(warnings.len(), 1);
        assert_eq!(
            warnings[0].kind,
            ValidationWarningKind::PaddingClamped {
                from: 99,
                to: MAX_PADDING
            }
        );
        // self was mutated
        if let Segment::Template(t) = &cfg.lines[0].segments[0] {
            assert_eq!(t.padding, MAX_PADDING);
        } else {
            panic!("expected Template segment");
        }
    }

    #[test]
    fn validate_padding_at_max_produces_no_warning() {
        let mut cfg = Config {
            schema_url: None,
            lines: vec![Line {
                separator: "".to_owned(),
                segments: vec![Segment::Template(TemplateSegment {
                    template: "x".to_owned(),
                    padding: MAX_PADDING,
                    hide_when_absent: false,
                })],
            }],
        };
        let warnings = cfg
            .validate_and_clamp()
            .expect("max padding should not error");
        assert!(warnings.is_empty(), "padding at MAX_PADDING must not warn");
    }

    #[test]
    fn validate_valid_config_returns_no_errors_no_warnings() {
        let mut cfg = minimal_config();
        let warnings = cfg
            .validate_and_clamp()
            .expect("valid config must not error");
        assert!(warnings.is_empty());
    }

    #[test]
    fn validate_full_config_is_valid() {
        let mut cfg = full_config();
        let warnings = cfg
            .validate_and_clamp()
            .expect("full config must not error");
        // padding=2 is within MAX_PADDING, no warnings expected
        assert!(warnings.is_empty());
    }

    #[test]
    fn validate_exactly_max_lines_is_valid() {
        let mut cfg = Config {
            schema_url: None,
            lines: vec![
                Line {
                    separator: "".to_owned(),
                    segments: vec![template_seg("a")],
                },
                Line {
                    separator: "".to_owned(),
                    segments: vec![template_seg("b")],
                },
                Line {
                    separator: "".to_owned(),
                    segments: vec![template_seg("c")],
                },
            ],
        };
        let result = cfg.validate_and_clamp();
        assert!(result.is_ok(), "exactly MAX_LINES lines must not error");
    }

    #[test]
    fn serde_defaults_separator_to_empty_string() {
        let json = r#"{"lines":[{"segments":[{"template":"x"}]}]}"#;
        let cfg: Config = serde_json::from_str(json).expect("deserialize without separator");
        assert_eq!(cfg.lines[0].separator, "");
    }

    #[test]
    fn serde_defaults_padding_and_hide_when_absent() {
        let json = r#"{"lines":[{"segments":[{"template":"x"}]}]}"#;
        let cfg: Config = serde_json::from_str(json).expect("deserialize");
        if let Segment::Template(t) = &cfg.lines[0].segments[0] {
            assert_eq!(t.padding, 0);
            assert!(!t.hide_when_absent);
        } else {
            panic!("expected Template segment");
        }
    }
}
