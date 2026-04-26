//! Claude Code stdin JSON parser.
//!
//! Parses the JSON document Claude Code pipes to the status-line command
//! on every assistant message.  All fields are `Option<…>`; unknown fields
//! are silently ignored (no `deny_unknown_fields`).

use anyhow::Context as _;
use serde::Deserialize;

/// Rate-limit window (five-hour or seven-day).
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct RateWindow {
    /// 0–100 inclusive, how much of the window has been consumed.
    pub used_percentage: Option<f64>,
    /// Unix epoch seconds at which the window resets.
    pub resets_at: Option<u64>,
}

/// Both rate-limit windows as delivered by Claude Code on stdin.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct RateLimits {
    pub five_hour: Option<RateWindow>,
    pub seven_day: Option<RateWindow>,
}

/// Model metadata block.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Model {
    pub display_name: Option<String>,
}

/// Workspace block.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Workspace {
    pub current_dir: Option<String>,
}

/// Top-level payload delivered by Claude Code on stdin.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Payload {
    pub model: Option<Model>,
    pub workspace: Option<Workspace>,
    pub transcript_path: Option<String>,
    pub session_id: Option<String>,
    pub rate_limits: Option<RateLimits>,
}

/// Parse a `Payload` from any `std::io::Read` source.
///
/// Returns `Err` on any JSON parse failure.  Never panics.
pub fn parse<R: std::io::Read>(reader: R) -> Result<Payload, anyhow::Error> {
    serde_json::from_reader(reader).context("failed to parse Claude Code stdin JSON")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_str(s: &str) -> Result<Payload, anyhow::Error> {
        parse(s.as_bytes())
    }

    #[test]
    fn full_payload_parses() {
        let json = r#"{
            "model": { "display_name": "claude-opus-4-7" },
            "workspace": { "current_dir": "/home/user/project" },
            "transcript_path": "/tmp/transcript.jsonl",
            "session_id": "abc-123",
            "rate_limits": {
                "five_hour": { "used_percentage": 23.5, "resets_at": 1738425600 },
                "seven_day": { "used_percentage": 41.2, "resets_at": 1738857600 }
            }
        }"#;
        let p = parse_str(json).expect("should parse full payload");
        assert_eq!(
            p.model.as_ref().and_then(|m| m.display_name.as_deref()),
            Some("claude-opus-4-7")
        );
        assert_eq!(
            p.workspace.as_ref().and_then(|w| w.current_dir.as_deref()),
            Some("/home/user/project")
        );
        assert_eq!(p.transcript_path.as_deref(), Some("/tmp/transcript.jsonl"));
        assert_eq!(p.session_id.as_deref(), Some("abc-123"));
        let rl = p
            .rate_limits
            .as_ref()
            .expect("rate_limits should be present");
        let fh = rl.five_hour.as_ref().expect("five_hour should be present");
        assert_eq!(fh.used_percentage, Some(23.5));
        assert_eq!(fh.resets_at, Some(1_738_425_600u64));
        let sd = rl.seven_day.as_ref().expect("seven_day should be present");
        assert_eq!(sd.used_percentage, Some(41.2));
        assert_eq!(sd.resets_at, Some(1_738_857_600u64));
    }

    #[test]
    fn missing_rate_limits_parses_to_none() {
        let json = r#"{
            "model": { "display_name": "claude-sonnet-4-6" },
            "session_id": "xyz-789"
        }"#;
        let p = parse_str(json).expect("should parse without rate_limits");
        assert_eq!(p.rate_limits, None);
        assert_eq!(
            p.model.as_ref().and_then(|m| m.display_name.as_deref()),
            Some("claude-sonnet-4-6")
        );
    }

    #[test]
    fn missing_workspace_parses_to_none() {
        let json = r#"{ "session_id": "s1" }"#;
        let p = parse_str(json).expect("should parse without workspace");
        assert_eq!(p.workspace, None);
        assert_eq!(p.session_id.as_deref(), Some("s1"));
    }

    #[test]
    fn malformed_json_returns_err() {
        let result = parse_str("{ not valid json }");
        assert!(result.is_err(), "malformed JSON should return Err");
    }

    #[test]
    fn empty_stdin_returns_err() {
        let result = parse_str("");
        assert!(result.is_err(), "empty input should return Err");
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let json = r#"{ "foo": "bar", "model": { "display_name": "test" }, "baz": 42 }"#;
        let p = parse_str(json).expect("unknown fields should not cause error");
        assert_eq!(
            p.model.as_ref().and_then(|m| m.display_name.as_deref()),
            Some("test")
        );
    }

    #[test]
    fn partial_rate_limits_only_five_hour() {
        let json = r#"{
            "rate_limits": {
                "five_hour": { "used_percentage": 10.0, "resets_at": 9999 }
            }
        }"#;
        let p = parse_str(json).expect("partial rate_limits should parse");
        let rl = p
            .rate_limits
            .as_ref()
            .expect("rate_limits should be present");
        assert!(rl.five_hour.is_some());
        assert_eq!(rl.seven_day, None);
    }
}
