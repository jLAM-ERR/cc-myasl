use serde::{Deserialize, Serialize};

pub const MAX_LINES: usize = 3;
pub const MAX_PADDING: u8 = 8;

/// Named ANSI-16 colors accepted by `color` and `bg` segment fields.
pub const NAMED_COLORS: &[&str] = &[
    "red", "green", "yellow", "blue", "magenta", "cyan", "white", "default",
];

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(default)]
pub struct Config {
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema_url: Option<String>,
    pub lines: Vec<Line>,
    /// When true, render segments as Powerline blocks with chevron transitions.
    #[serde(default)]
    pub powerline: bool,
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
    /// Foreground color name (ANSI-16). Must be one of NAMED_COLORS or None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub color: Option<String>,
    /// Background color name (ANSI-16). Must be one of NAMED_COLORS or None.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bg: Option<String>,
}

impl TemplateSegment {
    pub fn new(s: &str) -> TemplateSegment {
        TemplateSegment {
            template: s.to_owned(),
            padding: 0,
            hide_when_absent: false,
            color: None,
            bg: None,
        }
    }

    pub fn with_hide_when_absent(mut self) -> TemplateSegment {
        self.hide_when_absent = true;
        self
    }

    pub fn with_padding(mut self, n: u8) -> TemplateSegment {
        self.padding = n;
        self
    }
}

impl From<TemplateSegment> for Segment {
    fn from(t: TemplateSegment) -> Segment {
        Segment::Template(t)
    }
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
    /// `color` or `bg` field contains a value not in NAMED_COLORS.
    InvalidColor {
        field: &'static str,
        value: String,
    },
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
                        if let Some(c) = &t.color {
                            if !NAMED_COLORS.contains(&c.as_str()) {
                                errors.push(ValidationError {
                                    line_index: Some(li),
                                    segment_index: Some(si),
                                    kind: ValidationErrorKind::InvalidColor {
                                        field: "color",
                                        value: c.clone(),
                                    },
                                });
                            }
                        }
                        if let Some(b) = &t.bg {
                            if !NAMED_COLORS.contains(&b.as_str()) {
                                errors.push(ValidationError {
                                    line_index: Some(li),
                                    segment_index: Some(si),
                                    kind: ValidationErrorKind::InvalidColor {
                                        field: "bg",
                                        value: b.clone(),
                                    },
                                });
                            }
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
#[path = "schema_tests_color.rs"]
mod schema_tests_color;

#[cfg(test)]
mod tests {
    use super::*;

    // --- round-trip serde ---

    #[test]
    fn serde_round_trip_minimal() {
        let mut cfg = Config {
            schema_url: None,
            powerline: false,
            lines: vec![Line {
                separator: " | ".to_owned(),
                segments: vec![Segment::Template(TemplateSegment::new("{five_left}%"))],
            }],
        };
        let json = serde_json::to_string(&cfg).expect("serialize");
        let back: Config = serde_json::from_str(&json).expect("deserialize");
        cfg.validate_and_clamp().expect("minimal must be valid");
        assert_eq!(cfg, back);
    }

    #[test]
    fn serde_round_trip_with_schema_field() {
        let orig = Config {
            schema_url: Some(
                "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json"
                    .to_owned(),
            ),
            powerline: false,
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
            powerline: false,
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

    // --- serde defaults ---

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
