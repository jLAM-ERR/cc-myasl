/// Named ANSI-16 color usable as a segment or config-level fg/bg.
///
/// Matches the string values in `schema::NAMED_COLORS`.  Used by the
/// catalog and (Task 3) as `Option<NamedColor>` on `Config`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NamedColor {
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
    Default,
}

impl NamedColor {
    /// Convert to the string accepted by `TemplateSegment::color`/`bg`.
    pub fn as_str(self) -> &'static str {
        match self {
            NamedColor::Red => "red",
            NamedColor::Green => "green",
            NamedColor::Yellow => "yellow",
            NamedColor::Blue => "blue",
            NamedColor::Magenta => "magenta",
            NamedColor::Cyan => "cyan",
            NamedColor::White => "white",
            NamedColor::Default => "default",
        }
    }
}
