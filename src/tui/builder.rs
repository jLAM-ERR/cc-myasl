//! Builder-state representation of a Config — preset-aware, roundtrip-safe.
//!
//! `from_config` walks a Config, resolving each template to a catalog Preset
//! on hit or a Custom segment on miss.  `to_config` projects back to a
//! serializable Config.  FlexSpacer (`{"flex":true}`) is preserved verbatim.

use crate::config::named_color::NamedColor;
use crate::config::schema::{Config, FlexSegment, Line, Segment, TemplateSegment};
use crate::tui::catalog;

/// A segment in builder representation.
#[derive(Debug, Clone, PartialEq)]
pub enum BuilderSegment {
    /// A catalog preset, identified by its stable `id`.
    Preset {
        id: &'static str,
        color: Option<NamedColor>,
        bg: Option<NamedColor>,
    },
    /// A hand-edited template string not found in the catalog.
    Custom {
        template: String,
        color: Option<NamedColor>,
        bg: Option<NamedColor>,
        padding: u8,
        hide_when_absent: bool,
    },
    /// A flex spacer (`{"flex":true}`), preserved verbatim.
    FlexSpacer,
}

/// A line in builder representation.
#[derive(Debug, Clone, PartialEq)]
pub struct BuilderLine {
    pub separator: String,
    pub segments: Vec<BuilderSegment>,
}

/// Top-level builder state, mirroring Config but preset-aware.
#[derive(Debug, Clone, PartialEq)]
pub struct BuilderState {
    pub lines: Vec<BuilderLine>,
    pub powerline: bool,
    pub default_fg: Option<NamedColor>,
    pub default_bg: Option<NamedColor>,
    pub schema_url: Option<String>,
}

/// Convert a `NamedColor` to its lowercase string for use in `TemplateSegment`.
fn named_color_to_str(c: NamedColor) -> String {
    c.as_str().to_owned()
}

/// Parse a `TemplateSegment` color string to `NamedColor`, silently dropping unknown values.
fn str_to_named_color(s: &str) -> Option<NamedColor> {
    match s {
        "red" => Some(NamedColor::Red),
        "green" => Some(NamedColor::Green),
        "yellow" => Some(NamedColor::Yellow),
        "blue" => Some(NamedColor::Blue),
        "magenta" => Some(NamedColor::Magenta),
        "cyan" => Some(NamedColor::Cyan),
        "white" => Some(NamedColor::White),
        "default" => Some(NamedColor::Default),
        _ => None,
    }
}

/// Build a `BuilderState` from a `Config`.
///
/// Each `Segment::Template` is resolved against the catalog:
/// - Hit → `BuilderSegment::Preset` (preserving user color/bg overrides).
/// - Miss → `BuilderSegment::Custom`.
/// - `Segment::Flex` → `BuilderSegment::FlexSpacer`.
pub fn from_config(c: &Config) -> BuilderState {
    let lines = c
        .lines
        .iter()
        .map(|line| {
            let segments = line
                .segments
                .iter()
                .map(|seg| match seg {
                    Segment::Flex(_) => BuilderSegment::FlexSpacer,
                    Segment::Template(t) => {
                        let color = t.color.as_deref().and_then(str_to_named_color);
                        let bg = t.bg.as_deref().and_then(str_to_named_color);
                        if let Some(preset) = catalog::lookup(&t.template) {
                            BuilderSegment::Preset {
                                id: preset.id,
                                color,
                                bg,
                            }
                        } else {
                            BuilderSegment::Custom {
                                template: t.template.clone(),
                                color,
                                bg,
                                padding: t.padding,
                                hide_when_absent: t.hide_when_absent,
                            }
                        }
                    }
                })
                .collect();
            BuilderLine {
                separator: line.separator.clone(),
                segments,
            }
        })
        .collect();

    BuilderState {
        lines,
        powerline: c.powerline,
        default_fg: c.default_fg,
        default_bg: c.default_bg,
        schema_url: c.schema_url.clone(),
    }
}

/// Project a `BuilderState` back to a `Config`.
///
/// `Preset` → `Segment::Template` using catalog template + preserved color/bg.
/// `Custom` → `Segment::Template` directly.
/// `FlexSpacer` → `Segment::Flex`.
pub fn to_config(b: &BuilderState) -> Config {
    let lines = b
        .lines
        .iter()
        .map(|line| {
            let segments = line
                .segments
                .iter()
                .map(|seg| match seg {
                    BuilderSegment::FlexSpacer => Segment::Flex(FlexSegment { flex: true }),
                    BuilderSegment::Custom {
                        template,
                        color,
                        bg,
                        padding,
                        hide_when_absent,
                    } => Segment::Template(TemplateSegment {
                        template: template.clone(),
                        padding: *padding,
                        hide_when_absent: *hide_when_absent,
                        color: color.map(named_color_to_str),
                        bg: bg.map(named_color_to_str),
                    }),
                    BuilderSegment::Preset { id, color, bg } => {
                        let preset =
                            catalog::lookup_by_id(id).expect("Preset id must exist in catalog");
                        Segment::Template(TemplateSegment {
                            template: preset.template.to_owned(),
                            padding: 0,
                            hide_when_absent: preset.hide_when_absent,
                            color: color.map(named_color_to_str),
                            bg: bg.map(named_color_to_str),
                        })
                    }
                })
                .collect();
            Line {
                separator: line.separator.clone(),
                segments,
            }
        })
        .collect();

    Config {
        schema_url: b.schema_url.clone(),
        lines,
        powerline: b.powerline,
        default_fg: b.default_fg,
        default_bg: b.default_bg,
    }
}

#[cfg(test)]
#[path = "builder_tests.rs"]
mod tests;
