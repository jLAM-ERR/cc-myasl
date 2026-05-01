use super::*;
use crate::format::values::format_count;

fn ctx_empty() -> RenderCtx {
    RenderCtx::default()
}

// ── format_count boundary tests ───────────────────────────────────────────

#[test]
fn count_boundary_zero() {
    assert_eq!(format_count(0), "0");
}

#[test]
fn count_boundary_one() {
    assert_eq!(format_count(1), "1");
}

#[test]
fn count_boundary_999() {
    assert_eq!(format_count(999), "999");
}

#[test]
fn count_boundary_1000() {
    assert_eq!(format_count(1_000), "1.0k");
}

#[test]
fn count_boundary_1234() {
    assert_eq!(format_count(1_234), "1.2k");
}

#[test]
fn count_boundary_1500() {
    assert_eq!(format_count(1_500), "1.5k");
}

#[test]
fn count_boundary_9999_rounds_up() {
    // 9999 / 1000 = 9.999 → rounds to "10.0k"
    assert_eq!(format_count(9_999), "10.0k");
}

#[test]
fn count_boundary_999_999() {
    // Still in 'k' range; 999.999... → "1000.0k"
    assert_eq!(format_count(999_999), "1000.0k");
}

#[test]
fn count_boundary_1_000_000() {
    assert_eq!(format_count(1_000_000), "1.0M");
}

#[test]
fn count_boundary_1_500_000() {
    assert_eq!(format_count(1_500_000), "1.5M");
}

#[test]
fn count_boundary_999_999_999() {
    // Still in 'M' range; 999.999... → "1000.0M"
    assert_eq!(format_count(999_999_999), "1000.0M");
}

#[test]
fn count_boundary_1_000_000_000() {
    assert_eq!(format_count(1_000_000_000), "1.0G");
}

#[test]
fn count_boundary_u64_max_does_not_panic() {
    // Should not panic; exact value not pinned.
    let _ = format_count(u64::MAX);
}

// ── tokens_input ──────────────────────────────────────────────────────────

#[test]
fn tokens_input_present() {
    let ctx = RenderCtx {
        tokens_input: Some(1_234),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("tokens_input", &ctx),
        Some("1.2k".to_owned())
    );
}

#[test]
fn tokens_input_small() {
    let ctx = RenderCtx {
        tokens_input: Some(500),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("tokens_input", &ctx),
        Some("500".to_owned())
    );
}

#[test]
fn tokens_input_absent() {
    assert_eq!(render_placeholder("tokens_input", &ctx_empty()), None);
}

// ── tokens_output ─────────────────────────────────────────────────────────

#[test]
fn tokens_output_present() {
    let ctx = RenderCtx {
        tokens_output: Some(2_500),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("tokens_output", &ctx),
        Some("2.5k".to_owned())
    );
}

#[test]
fn tokens_output_absent() {
    assert_eq!(render_placeholder("tokens_output", &ctx_empty()), None);
}

// ── tokens_cached_creation ────────────────────────────────────────────────

#[test]
fn tokens_cached_creation_present() {
    let ctx = RenderCtx {
        tokens_cache_creation: Some(1_000_000),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("tokens_cached_creation", &ctx),
        Some("1.0M".to_owned())
    );
}

#[test]
fn tokens_cached_creation_absent() {
    assert_eq!(
        render_placeholder("tokens_cached_creation", &ctx_empty()),
        None
    );
}

// ── tokens_cached_read ────────────────────────────────────────────────────

#[test]
fn tokens_cached_read_present() {
    let ctx = RenderCtx {
        tokens_cache_read: Some(999),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("tokens_cached_read", &ctx),
        Some("999".to_owned())
    );
}

#[test]
fn tokens_cached_read_absent() {
    assert_eq!(render_placeholder("tokens_cached_read", &ctx_empty()), None);
}

// ── tokens_cached_total ───────────────────────────────────────────────────

#[test]
fn tokens_cached_total_both_present() {
    let ctx = RenderCtx {
        tokens_cache_creation: Some(500),
        tokens_cache_read: Some(500),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("tokens_cached_total", &ctx),
        Some("1.0k".to_owned())
    );
}

#[test]
fn tokens_cached_total_creation_absent() {
    let ctx = RenderCtx {
        tokens_cache_creation: None,
        tokens_cache_read: Some(500),
        ..Default::default()
    };
    assert_eq!(render_placeholder("tokens_cached_total", &ctx), None);
}

#[test]
fn tokens_cached_total_read_absent() {
    let ctx = RenderCtx {
        tokens_cache_creation: Some(500),
        tokens_cache_read: None,
        ..Default::default()
    };
    assert_eq!(render_placeholder("tokens_cached_total", &ctx), None);
}

#[test]
fn tokens_cached_total_both_absent() {
    assert_eq!(
        render_placeholder("tokens_cached_total", &ctx_empty()),
        None
    );
}

// ── tokens_total ──────────────────────────────────────────────────────────

#[test]
fn tokens_total_all_present() {
    let ctx = RenderCtx {
        tokens_input: Some(1_000),
        tokens_output: Some(500),
        tokens_cache_creation: Some(200),
        tokens_cache_read: Some(300),
        ..Default::default()
    };
    // 1000 + 500 + 200 + 300 = 2000 → "2.0k"
    assert_eq!(
        render_placeholder("tokens_total", &ctx),
        Some("2.0k".to_owned())
    );
}

#[test]
fn tokens_total_input_absent_returns_none() {
    let ctx = RenderCtx {
        tokens_input: None,
        tokens_output: Some(500),
        tokens_cache_creation: Some(200),
        tokens_cache_read: Some(300),
        ..Default::default()
    };
    assert_eq!(render_placeholder("tokens_total", &ctx), None);
}

#[test]
fn tokens_total_output_absent_returns_none() {
    let ctx = RenderCtx {
        tokens_input: Some(1_000),
        tokens_output: None,
        tokens_cache_creation: Some(200),
        tokens_cache_read: Some(300),
        ..Default::default()
    };
    assert_eq!(render_placeholder("tokens_total", &ctx), None);
}

#[test]
fn tokens_total_cache_creation_absent_returns_none() {
    let ctx = RenderCtx {
        tokens_input: Some(1_000),
        tokens_output: Some(500),
        tokens_cache_creation: None,
        tokens_cache_read: Some(300),
        ..Default::default()
    };
    assert_eq!(render_placeholder("tokens_total", &ctx), None);
}

#[test]
fn tokens_total_cache_read_absent_returns_none() {
    let ctx = RenderCtx {
        tokens_input: Some(1_000),
        tokens_output: Some(500),
        tokens_cache_creation: Some(200),
        tokens_cache_read: None,
        ..Default::default()
    };
    assert_eq!(render_placeholder("tokens_total", &ctx), None);
}

#[test]
fn tokens_total_all_absent() {
    assert_eq!(render_placeholder("tokens_total", &ctx_empty()), None);
}

// ── tokens_input_total ────────────────────────────────────────────────────

#[test]
fn tokens_input_total_present() {
    let ctx = RenderCtx {
        tokens_input_total: Some(150_000),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("tokens_input_total", &ctx),
        Some("150.0k".to_owned())
    );
}

#[test]
fn tokens_input_total_absent() {
    assert_eq!(render_placeholder("tokens_input_total", &ctx_empty()), None);
}

// ── tokens_output_total ───────────────────────────────────────────────────

#[test]
fn tokens_output_total_present() {
    let ctx = RenderCtx {
        tokens_output_total: Some(1_500_000),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("tokens_output_total", &ctx),
        Some("1.5M".to_owned())
    );
}

#[test]
fn tokens_output_total_absent() {
    assert_eq!(
        render_placeholder("tokens_output_total", &ctx_empty()),
        None
    );
}
