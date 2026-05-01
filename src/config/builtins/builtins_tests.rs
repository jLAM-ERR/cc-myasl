use super::*;
use crate::config::schema::{MAX_PADDING, Segment, TemplateSegment};

// ---------------------------------------------------------------------------
// lookup: boundary and edge-case names
// ---------------------------------------------------------------------------

/// Hyphen form "emoji-verbose" must NOT resolve — only the underscore form is valid.
#[test]
fn lookup_hyphen_form_returns_none() {
    assert!(
        lookup("emoji-verbose").is_none(),
        "hyphen form must not resolve; only underscore form is canonical"
    );
}

/// Leading/trailing whitespace is not stripped — must return None.
#[test]
fn lookup_name_with_whitespace_returns_none() {
    assert!(lookup(" default ").is_none());
    assert!(lookup("default ").is_none());
    assert!(lookup(" default").is_none());
    assert!(lookup("\tdefault").is_none());
}

/// Mixed-case must not match (lookup is case-sensitive).
#[test]
fn lookup_mixed_case_returns_none() {
    assert!(lookup("Default").is_none());
    assert!(lookup("MINIMAL").is_none());
    assert!(lookup("Compact").is_none());
    assert!(lookup("Emoji_Verbose").is_none());
}

/// Underscore-only partial (no match).
#[test]
fn lookup_partial_name_returns_none() {
    assert!(lookup("emoji_").is_none());
    assert!(lookup("_verbose").is_none());
    assert!(lookup("emoji_v").is_none());
}

// ---------------------------------------------------------------------------
// validate_and_clamp: built-ins must produce zero warnings (not just no errors)
// ---------------------------------------------------------------------------

/// Every built-in must validate without any warnings.  If any built-in used
/// padding > MAX_PADDING the old test (every_config_validates_without_errors)
/// would still pass — this test catches that.
#[test]
fn every_builtin_validates_with_no_warnings() {
    const ALL_NAMES: &[&str] = &[
        "default",
        "minimal",
        "compact",
        "bars",
        "colored",
        "emoji",
        "emoji_verbose",
        "verbose",
    ];
    for name in ALL_NAMES {
        let mut cfg = lookup(name).unwrap();
        let warnings = cfg.validate_and_clamp().unwrap_or_else(|e| {
            panic!("{name} validate_and_clamp returned errors: {e:?}");
        });
        assert!(
            warnings.is_empty(),
            "{name} validate_and_clamp returned warnings (padding > {MAX_PADDING}?): {warnings:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Builder: order independence and boundary values
// ---------------------------------------------------------------------------

/// with_padding first, then with_hide_when_absent — must be order-independent.
#[test]
fn builder_padding_then_hide_order_independent() {
    let t1 = TemplateSegment::new("x")
        .with_padding(3)
        .with_hide_when_absent();
    let t2 = TemplateSegment::new("x")
        .with_hide_when_absent()
        .with_padding(3);
    assert_eq!(t1.template, t2.template);
    assert_eq!(t1.padding, t2.padding);
    assert_eq!(t1.hide_when_absent, t2.hide_when_absent);
}

/// with_padding(0) is valid and produces no mutation of hide_when_absent.
#[test]
fn with_padding_zero_boundary() {
    let t = TemplateSegment::new("y").with_padding(0);
    assert_eq!(t.padding, 0);
    assert!(!t.hide_when_absent);
}

/// with_padding(MAX_PADDING) sets value exactly at the allowed maximum —
/// validate_and_clamp must produce no warnings.
#[test]
fn with_padding_at_max_boundary_no_warning() {
    let seg: Segment = TemplateSegment::new("z").with_padding(MAX_PADDING).into();
    let mut cfg = crate::config::schema::Config {
        schema_url: None,
        powerline: false,
        lines: vec![crate::config::schema::Line {
            separator: String::new(),
            segments: vec![seg],
        }],
    };
    let warnings = cfg
        .validate_and_clamp()
        .expect("padding=MAX_PADDING must not hard-error");
    assert!(
        warnings.is_empty(),
        "padding at MAX must produce no clamp warning"
    );
}

/// with_padding(255) stores 255 — validate_and_clamp clamps to MAX_PADDING and warns.
/// The builder itself must not clamp; clamping is the validator's job.
#[test]
fn with_padding_u8_max_builder_does_not_clamp() {
    let t = TemplateSegment::new("z").with_padding(255);
    assert_eq!(
        t.padding, 255,
        "builder must store the raw value; validator clamps"
    );
}

// ---------------------------------------------------------------------------
// From<TemplateSegment> for Segment: full field preservation
// ---------------------------------------------------------------------------

/// From must preserve padding as well as template and hide_when_absent.
#[test]
fn from_template_segment_preserves_padding() {
    let ts = TemplateSegment::new("abc")
        .with_padding(MAX_PADDING)
        .with_hide_when_absent();
    let seg: Segment = ts.into();
    match seg {
        Segment::Template(t) => {
            assert_eq!(t.template, "abc");
            assert_eq!(t.padding, MAX_PADDING);
            assert!(t.hide_when_absent);
        }
        Segment::Flex(_) => panic!("expected Segment::Template"),
    }
}

/// From with all defaults (padding=0, hide_when_absent=false) round-trips cleanly.
#[test]
fn from_template_segment_preserves_defaults() {
    let ts = TemplateSegment::new("q");
    let seg: Segment = ts.into();
    match seg {
        Segment::Template(t) => {
            assert_eq!(t.template, "q");
            assert_eq!(t.padding, 0);
            assert!(!t.hide_when_absent);
        }
        Segment::Flex(_) => panic!("expected Segment::Template"),
    }
}

// ---------------------------------------------------------------------------
// Segment variant-order pinning via JSON deserialization
// ---------------------------------------------------------------------------

/// JSON with only "template" key must deserialize to Segment::Template.
/// If a future refactor reorders the untagged variants, this breaks.
#[test]
fn segment_json_with_template_key_is_template_variant() {
    let json = r#"{"template":"{model}","padding":0,"hide_when_absent":false}"#;
    let seg: Segment = serde_json::from_str(json).expect("deserialize template segment");
    match seg {
        Segment::Template(t) => assert_eq!(t.template, "{model}"),
        Segment::Flex(_) => panic!("expected Template variant, got Flex — variant order changed?"),
    }
}

/// JSON with only "flex":true must deserialize to Segment::Flex.
/// Verifies the variant order is still [Template, Flex].
#[test]
fn segment_json_with_flex_key_is_flex_variant() {
    let json = r#"{"flex":true}"#;
    let seg: Segment = serde_json::from_str(json).expect("deserialize flex segment");
    match seg {
        Segment::Flex(f) => assert!(f.flex),
        Segment::Template(_) => panic!("expected Flex variant — variant order changed?"),
    }
}

// ---------------------------------------------------------------------------
// Empty-ctx render output assertions
// ---------------------------------------------------------------------------

/// With empty ctx every built-in must produce output that:
/// 1. Does not start or end with a separator literal (`·`, `⏰`, `⏳`, etc.).
/// 2. Does not contain a lone `%` or `/` (would indicate a half-rendered slot).
#[test]
fn all_templates_empty_ctx_no_dangling_separator() {
    const ALL_NAMES: &[&str] = &[
        "default",
        "minimal",
        "compact",
        "bars",
        "colored",
        "emoji",
        "emoji_verbose",
        "verbose",
    ];
    use crate::config::render;
    use crate::format::placeholders::RenderCtx;
    let ctx = RenderCtx::default();
    for name in ALL_NAMES {
        let out = render::render(&lookup(name).unwrap(), &ctx);
        assert!(
            !out.starts_with(" · "),
            "{name} output starts with dangling separator: {out:?}"
        );
        assert!(
            !out.ends_with(" · "),
            "{name} output ends with dangling separator: {out:?}"
        );
        assert!(
            !out.contains('%'),
            "{name} output contains lone '%' with empty ctx: {out:?}"
        );
        assert!(
            !out.ends_with('/'),
            "{name} output ends with '/' with empty ctx: {out:?}"
        );
    }
}

/// "default" with empty ctx must emit exactly "" — only {model} segment is
/// non-optional and it resolves to None when model is None.
#[test]
fn default_template_empty_ctx_emits_empty_string() {
    use crate::config::render;
    use crate::format::placeholders::RenderCtx;
    let ctx = RenderCtx::default();
    let out = render::render(&lookup("default").unwrap(), &ctx);
    assert_eq!(out, "", "default with no ctx must emit empty, got: {out:?}");
}

/// "emoji_verbose" with empty ctx must not emit bare emoji separators.
#[test]
fn emoji_verbose_empty_ctx_emits_only_model_or_empty() {
    use crate::config::render;
    use crate::format::placeholders::RenderCtx;
    let ctx = RenderCtx::default();
    let out = render::render(&lookup("emoji_verbose").unwrap(), &ctx);
    // First segment is "🤖 {model}" — non-optional; with model=None, render_segment
    // returns None and hide_when_absent=false → emits "" (empty string, leading text dropped).
    // All other segments are optional and must collapse.
    assert!(
        !out.contains("⏰"),
        "seven_reset_clock absent → ⏰ block must collapse: {out:?}"
    );
    assert!(
        !out.contains("⏳"),
        "quota absent → ⏳ block must collapse: {out:?}"
    );
}

// ---------------------------------------------------------------------------
// TemplateSegment::new with empty string
// ---------------------------------------------------------------------------

/// new("") is valid — empty template string is allowed by the type.
#[test]
fn template_segment_new_empty_string() {
    let t = TemplateSegment::new("");
    assert_eq!(t.template, "");
    assert_eq!(t.padding, 0);
    assert!(!t.hide_when_absent);
    // Converting to Segment and back via serde must round-trip.
    let seg: Segment = t.into();
    let json = serde_json::to_string(&seg).expect("serialize");
    let back: Segment = serde_json::from_str(&json).expect("deserialize");
    match back {
        Segment::Template(t) => assert_eq!(t.template, ""),
        Segment::Flex(_) => panic!("expected Template"),
    }
}

// ---------------------------------------------------------------------------
// lookup returns fresh Config each call (not shared mutable state)
// ---------------------------------------------------------------------------

/// Two calls to lookup("default") must return independent values —
/// mutating one must not affect the other.
#[test]
fn lookup_returns_independent_copies() {
    let mut cfg1 = lookup("default").unwrap();
    let cfg2 = lookup("default").unwrap();
    // Mutate cfg1's first segment template.
    if let Segment::Template(t) = &mut cfg1.lines[0].segments[0] {
        t.template = "MUTATED".to_owned();
    }
    // cfg2 must be unaffected.
    match &cfg2.lines[0].segments[0] {
        Segment::Template(t) => assert_ne!(
            t.template, "MUTATED",
            "lookup must return independent instances"
        ),
        Segment::Flex(_) => panic!("expected Template"),
    }
}

// ---------------------------------------------------------------------------
// rich built-in template (Phase-2 showcase)
// ---------------------------------------------------------------------------

#[test]
fn rich_validates_without_errors() {
    let mut cfg = lookup("rich").unwrap();
    let result = cfg.validate_and_clamp();
    assert!(
        result.is_ok(),
        "rich validate_and_clamp returned errors: {result:?}"
    );
}

#[test]
fn rich_renders_non_empty_against_full_payload() {
    use crate::config::render;
    let src = include_str!("../../../tests/fixtures/full-payload.json");
    let payload = crate::payload::parse(src.as_bytes()).expect("full-payload fixture");
    let ctx = crate::payload_mapping::build_render_ctx(&payload, 0);
    let out = render::render(&lookup("rich").unwrap(), &ctx);
    assert!(
        !out.is_empty(),
        "rich must render non-empty output against full payload"
    );
    assert!(
        out.contains("claude-sonnet-4-6"),
        "rich line 0 must contain model: {out:?}"
    );
}

#[test]
fn rich_collapses_against_empty_ctx() {
    use crate::config::render;
    use crate::format::placeholders::RenderCtx;
    let ctx = RenderCtx::default();
    let out = render::render(&lookup("rich").unwrap(), &ctx);
    // model is None → non-optional segment renders to "" → output is empty or multiline with empties
    assert!(!out.contains("ctx:"), "context bar must collapse: {out:?}");
    assert!(!out.contains("⎇"), "git_branch must collapse: {out:?}");
    assert!(!out.contains("tok"), "tokens must collapse: {out:?}");
}
