use super::*;

fn ctx_empty() -> RenderCtx {
    RenderCtx::default()
}

// ── cost_usd ──────────────────────────────────────────────────────────────

#[test]
fn cost_usd_present() {
    let ctx = RenderCtx {
        cost_usd: Some(0.012),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("cost_usd", &ctx),
        Some("0.01".to_owned())
    );
}

#[test]
fn cost_usd_rounds_to_two_decimals() {
    let ctx = RenderCtx {
        cost_usd: Some(1.235),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("cost_usd", &ctx),
        Some("1.24".to_owned())
    );
}

#[test]
fn cost_usd_zero() {
    let ctx = RenderCtx {
        cost_usd: Some(0.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("cost_usd", &ctx),
        Some("0.00".to_owned())
    );
}

#[test]
fn cost_usd_absent() {
    assert_eq!(render_placeholder("cost_usd", &ctx_empty()), None);
}

// ── session_clock ─────────────────────────────────────────────────────────

#[test]
fn session_clock_present_minutes() {
    let ctx = RenderCtx {
        total_duration_ms: Some(3_661_000),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("session_clock", &ctx),
        Some("1h1m".to_owned())
    );
}

#[test]
fn session_clock_present_seconds() {
    let ctx = RenderCtx {
        total_duration_ms: Some(4_500),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("session_clock", &ctx),
        Some("4s".to_owned())
    );
}

#[test]
fn session_clock_absent() {
    assert_eq!(render_placeholder("session_clock", &ctx_empty()), None);
}

// ── api_duration ──────────────────────────────────────────────────────────

#[test]
fn api_duration_present() {
    let ctx = RenderCtx {
        api_duration_ms: Some(60_000),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("api_duration", &ctx),
        Some("1m".to_owned())
    );
}

#[test]
fn api_duration_absent() {
    assert_eq!(render_placeholder("api_duration", &ctx_empty()), None);
}

// ── lines_added ───────────────────────────────────────────────────────────

#[test]
fn lines_added_present() {
    let ctx = RenderCtx {
        lines_added: Some(42),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("lines_added", &ctx),
        Some("42".to_owned())
    );
}

#[test]
fn lines_added_absent() {
    assert_eq!(render_placeholder("lines_added", &ctx_empty()), None);
}

// ── lines_removed ─────────────────────────────────────────────────────────

#[test]
fn lines_removed_present() {
    let ctx = RenderCtx {
        lines_removed: Some(7),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("lines_removed", &ctx),
        Some("7".to_owned())
    );
}

#[test]
fn lines_removed_absent() {
    assert_eq!(render_placeholder("lines_removed", &ctx_empty()), None);
}

// ── lines_changed ─────────────────────────────────────────────────────────

#[test]
fn lines_changed_both_present() {
    let ctx = RenderCtx {
        lines_added: Some(10),
        lines_removed: Some(3),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("lines_changed", &ctx),
        Some("13".to_owned())
    );
}

#[test]
fn lines_changed_only_added_is_absent() {
    let ctx = RenderCtx {
        lines_added: Some(10),
        lines_removed: None,
        ..Default::default()
    };
    assert_eq!(render_placeholder("lines_changed", &ctx), None);
}

#[test]
fn lines_changed_only_removed_is_absent() {
    let ctx = RenderCtx {
        lines_added: None,
        lines_removed: Some(3),
        ..Default::default()
    };
    assert_eq!(render_placeholder("lines_changed", &ctx), None);
}

#[test]
fn lines_changed_both_absent() {
    assert_eq!(render_placeholder("lines_changed", &ctx_empty()), None);
}

#[test]
fn lines_changed_both_zero() {
    let ctx = RenderCtx {
        lines_added: Some(0),
        lines_removed: Some(0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("lines_changed", &ctx),
        Some("0".to_owned())
    );
}
