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

impl std::str::FromStr for NamedColor {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "red" => Ok(NamedColor::Red),
            "green" => Ok(NamedColor::Green),
            "yellow" => Ok(NamedColor::Yellow),
            "blue" => Ok(NamedColor::Blue),
            "magenta" => Ok(NamedColor::Magenta),
            "cyan" => Ok(NamedColor::Cyan),
            "white" => Ok(NamedColor::White),
            "default" => Ok(NamedColor::Default),
            _ => Err(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every variant round-trips through as_str → parse. Catches drift when
    /// a new variant is added without updating FromStr.
    #[test]
    fn from_str_round_trips_every_variant() {
        let variants = [
            NamedColor::Red,
            NamedColor::Green,
            NamedColor::Yellow,
            NamedColor::Blue,
            NamedColor::Magenta,
            NamedColor::Cyan,
            NamedColor::White,
            NamedColor::Default,
        ];
        for v in variants {
            let s = v.as_str();
            let parsed: NamedColor = s.parse().unwrap_or_else(|_| panic!("{s:?} did not parse"));
            assert_eq!(parsed, v, "round-trip failed for {s:?}");
        }
    }

    #[test]
    fn from_str_rejects_unknown() {
        assert!("".parse::<NamedColor>().is_err());
        assert!("RED".parse::<NamedColor>().is_err());
        assert!("black".parse::<NamedColor>().is_err());
        assert!("Default".parse::<NamedColor>().is_err());
    }
}
