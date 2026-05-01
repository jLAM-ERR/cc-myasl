/// Unit tests for cc-myasl.schema.json (Task 9).
///
/// Validates that the schema file is well-formed JSON and spot-checks
/// key constraints by direct serde_json::Value traversal.
use crate::config::builtins;

const SCHEMA_SRC: &str = include_str!("../../cc-myasl.schema.json");

fn schema() -> serde_json::Value {
    serde_json::from_str(SCHEMA_SRC).expect("cc-myasl.schema.json must be valid JSON")
}

// ── schema file parses as valid JSON ─────────────────────────────────────

#[test]
fn schema_file_parses_as_valid_json() {
    let v = schema();
    assert!(v.is_object(), "schema root must be a JSON object");
}

// ── spot-check: lines.maxItems == 3 ──────────────────────────────────────

#[test]
fn schema_lines_max_items_is_3() {
    let s = schema();
    let max_items = &s["properties"]["lines"]["maxItems"];
    assert_eq!(
        max_items.as_u64(),
        Some(3),
        "properties.lines.maxItems must be 3, got {max_items}"
    );
}

// ── spot-check: padding.maximum == 8 ─────────────────────────────────────

#[test]
fn schema_padding_maximum_is_8() {
    let s = schema();
    let maximum = &s["definitions"]["TemplateSegment"]["properties"]["padding"]["maximum"];
    assert_eq!(
        maximum.as_u64(),
        Some(8),
        "TemplateSegment.padding.maximum must be 8, got {maximum}"
    );
}

#[test]
fn schema_padding_minimum_is_0() {
    let s = schema();
    let minimum = &s["definitions"]["TemplateSegment"]["properties"]["padding"]["minimum"];
    assert_eq!(
        minimum.as_u64(),
        Some(0),
        "TemplateSegment.padding.minimum must be 0, got {minimum}"
    );
}

// ── spot-check: Segment oneOf has exactly 2 entries ──────────────────────

#[test]
fn schema_segment_one_of_has_exactly_two_entries() {
    let s = schema();
    let one_of = &s["definitions"]["Segment"]["oneOf"];
    let arr = one_of
        .as_array()
        .expect("definitions.Segment.oneOf must be an array");
    assert_eq!(
        arr.len(),
        2,
        "Segment.oneOf must have exactly 2 variants, got {}",
        arr.len()
    );
}

// ── spot-check: flex enum is [true] ──────────────────────────────────────

#[test]
fn schema_flex_enum_is_true_only() {
    let s = schema();
    let enum_val = &s["definitions"]["FlexSegment"]["properties"]["flex"]["enum"];
    let arr = enum_val
        .as_array()
        .expect("FlexSegment.flex.enum must be an array");
    assert_eq!(arr.len(), 1, "flex enum must have exactly one value");
    assert_eq!(arr[0], true, "flex enum must be [true]");
}

// ── spot-check: TemplateSegment requires template field ──────────────────

#[test]
fn schema_template_segment_requires_template() {
    let s = schema();
    let required = &s["definitions"]["TemplateSegment"]["required"];
    let arr = required
        .as_array()
        .expect("TemplateSegment.required must be an array");
    let has_template = arr.iter().any(|v| v.as_str() == Some("template"));
    assert!(
        has_template,
        "TemplateSegment.required must include 'template'"
    );
}

// ── sanity: every built-in serializes with keys that appear in schema.properties ─

#[test]
fn builtins_top_level_keys_are_known_to_schema() {
    let s = schema();
    let schema_props = s["properties"]
        .as_object()
        .expect("schema must have properties object");

    let all_names = [
        "default",
        "minimal",
        "compact",
        "bars",
        "colored",
        "emoji",
        "emoji_verbose",
        "verbose",
    ];
    for name in all_names {
        let cfg = builtins::lookup(name).unwrap_or_else(|| panic!("builtin {name} missing"));
        let json = serde_json::to_value(&cfg).expect("serialize builtin");
        let obj = json.as_object().expect("builtin serializes to object");
        for key in obj.keys() {
            // $schema is optional and listed in schema properties; all other keys must too.
            assert!(
                schema_props.contains_key(key.as_str()),
                "builtin '{name}' has top-level key '{key}' not listed in schema.properties"
            );
        }
    }
}
