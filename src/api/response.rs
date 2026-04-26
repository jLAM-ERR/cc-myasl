//! OAuth usage endpoint response shape.
//!
//! Note: The OAuth endpoint uses `utilization` (0..=100) and `resets_at` as
//! an ISO-8601 string. This is *different* from the stdin `rate_limits` field,
//! which uses `used_percentage` and a Unix epoch u64. The asymmetry is
//! intentional — do not try to unify them.

use serde::Deserialize;

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct UsageResponse {
    pub five_hour: Option<UsageWindow>,
    pub seven_day: Option<UsageWindow>,
    pub extra_usage: Option<ExtraUsage>,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct UsageWindow {
    pub utilization: Option<f64>,  // 0..=100
    pub resets_at: Option<String>, // ISO-8601 string per docs/research.md
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct ExtraUsage {
    pub is_enabled: Option<bool>,
    pub monthly_limit: Option<f64>,
    pub used_credits: Option<f64>,
    pub utilization: Option<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_response_all_fields() {
        let json = r#"{
            "five_hour": { "utilization": 23.5, "resets_at": "2026-04-26T18:00:00Z" },
            "seven_day": { "utilization": 41.2, "resets_at": "2026-04-30T00:00:00Z" },
            "extra_usage": {
                "is_enabled": true,
                "monthly_limit": 100.0,
                "used_credits": 42.5,
                "utilization": 42.5
            }
        }"#;
        let r: UsageResponse = serde_json::from_str(json).unwrap();
        let five = r.five_hour.unwrap();
        assert_eq!(five.utilization, Some(23.5));
        assert_eq!(five.resets_at.as_deref(), Some("2026-04-26T18:00:00Z"));
        let seven = r.seven_day.unwrap();
        assert_eq!(seven.utilization, Some(41.2));
        assert_eq!(seven.resets_at.as_deref(), Some("2026-04-30T00:00:00Z"));
        let extra = r.extra_usage.unwrap();
        assert_eq!(extra.is_enabled, Some(true));
        assert_eq!(extra.monthly_limit, Some(100.0));
        assert_eq!(extra.used_credits, Some(42.5));
        assert_eq!(extra.utilization, Some(42.5));
    }

    #[test]
    fn partial_response_only_five_hour() {
        let json =
            r#"{ "five_hour": { "utilization": 10.0, "resets_at": "2026-04-26T18:00:00Z" } }"#;
        let r: UsageResponse = serde_json::from_str(json).unwrap();
        assert!(r.five_hour.is_some());
        assert!(r.seven_day.is_none());
        assert!(r.extra_usage.is_none());
        assert_eq!(r.five_hour.unwrap().utilization, Some(10.0));
    }

    #[test]
    fn all_null_fields_parse_to_none() {
        let json = r#"{
            "five_hour": { "utilization": null, "resets_at": null },
            "seven_day": { "utilization": null, "resets_at": null },
            "extra_usage": {
                "is_enabled": null,
                "monthly_limit": null,
                "used_credits": null,
                "utilization": null
            }
        }"#;
        let r: UsageResponse = serde_json::from_str(json).unwrap();
        let five = r.five_hour.unwrap();
        assert_eq!(five.utilization, None);
        assert_eq!(five.resets_at, None);
        let seven = r.seven_day.unwrap();
        assert_eq!(seven.utilization, None);
        assert_eq!(seven.resets_at, None);
        let extra = r.extra_usage.unwrap();
        assert_eq!(extra.is_enabled, None);
        assert_eq!(extra.monthly_limit, None);
        assert_eq!(extra.used_credits, None);
        assert_eq!(extra.utilization, None);
    }

    #[test]
    fn malformed_json_returns_err() {
        let result: Result<UsageResponse, _> = serde_json::from_str("{ not valid json }");
        assert!(result.is_err());
    }

    #[test]
    fn empty_object_all_fields_none() {
        let r: UsageResponse = serde_json::from_str("{}").unwrap();
        assert_eq!(r.five_hour, None);
        assert_eq!(r.seven_day, None);
        assert_eq!(r.extra_usage, None);
    }

    #[test]
    fn unknown_fields_are_ignored() {
        let json = r#"{
            "five_hour": { "utilization": 50.0, "resets_at": "2026-04-26T18:00:00Z", "unknown_field": "ignored" },
            "unknown_top_level": 42
        }"#;
        let r: UsageResponse = serde_json::from_str(json).unwrap();
        let five = r.five_hour.unwrap();
        assert_eq!(five.utilization, Some(50.0));
    }
}
