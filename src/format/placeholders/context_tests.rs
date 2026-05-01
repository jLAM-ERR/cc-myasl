use super::*;

fn ctx_empty() -> RenderCtx {
    RenderCtx::default()
}

// ── context_size ──────────────────────────────────────────────────────────

#[test]
fn context_size_present() {
    let ctx = RenderCtx {
        context_size: Some(200_000),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_size", &ctx),
        Some("200000".to_owned())
    );
}

#[test]
fn context_size_zero() {
    let ctx = RenderCtx {
        context_size: Some(0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_size", &ctx),
        Some("0".to_owned())
    );
}

#[test]
fn context_size_absent() {
    assert_eq!(render_placeholder("context_size", &ctx_empty()), None);
}

// ── context_used_pct ──────────────────────────────────────────────────────

#[test]
fn context_used_pct_present() {
    let ctx = RenderCtx {
        context_used_pct: Some(8.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_used_pct", &ctx),
        Some("8.0".to_owned())
    );
}

#[test]
fn context_used_pct_one_decimal() {
    let ctx = RenderCtx {
        context_used_pct: Some(42.567),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_used_pct", &ctx),
        Some("42.6".to_owned())
    );
}

#[test]
fn context_used_pct_absent() {
    assert_eq!(render_placeholder("context_used_pct", &ctx_empty()), None);
}

// ── context_remaining_pct ─────────────────────────────────────────────────

#[test]
fn context_remaining_pct_present() {
    let ctx = RenderCtx {
        context_remaining_pct: Some(92.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_remaining_pct", &ctx),
        Some("92.0".to_owned())
    );
}

#[test]
fn context_remaining_pct_one_decimal() {
    let ctx = RenderCtx {
        context_remaining_pct: Some(57.433),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_remaining_pct", &ctx),
        Some("57.4".to_owned())
    );
}

#[test]
fn context_remaining_pct_absent() {
    assert_eq!(
        render_placeholder("context_remaining_pct", &ctx_empty()),
        None
    );
}

// ── context_used_pct_int ──────────────────────────────────────────────────

#[test]
fn context_used_pct_int_floors_down() {
    let ctx = RenderCtx {
        context_used_pct: Some(8.7),
        ..Default::default()
    };
    // floor(8.7) == 8
    assert_eq!(
        render_placeholder("context_used_pct_int", &ctx),
        Some("8".to_owned())
    );
}

#[test]
fn context_used_pct_int_exact() {
    let ctx = RenderCtx {
        context_used_pct: Some(50.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_used_pct_int", &ctx),
        Some("50".to_owned())
    );
}

#[test]
fn context_used_pct_int_zero() {
    let ctx = RenderCtx {
        context_used_pct: Some(0.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_used_pct_int", &ctx),
        Some("0".to_owned())
    );
}

#[test]
fn context_used_pct_int_absent() {
    assert_eq!(
        render_placeholder("context_used_pct_int", &ctx_empty()),
        None
    );
}

// ── context_bar (10-char) ─────────────────────────────────────────────────

#[test]
fn context_bar_zero_percent() {
    let ctx = RenderCtx {
        context_used_pct: Some(0.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_bar", &ctx),
        Some("[░░░░░░░░░░]".to_owned())
    );
}

#[test]
fn context_bar_fifty_percent() {
    let ctx = RenderCtx {
        context_used_pct: Some(50.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_bar", &ctx),
        Some("[█████░░░░░]".to_owned())
    );
}

#[test]
fn context_bar_hundred_percent() {
    let ctx = RenderCtx {
        context_used_pct: Some(100.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_bar", &ctx),
        Some("[██████████]".to_owned())
    );
}

#[test]
fn context_bar_absent() {
    assert_eq!(render_placeholder("context_bar", &ctx_empty()), None);
}

// ── context_bar_long (20-char) ────────────────────────────────────────────

#[test]
fn context_bar_long_zero_percent() {
    let ctx = RenderCtx {
        context_used_pct: Some(0.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_bar_long", &ctx),
        Some("[░░░░░░░░░░░░░░░░░░░░]".to_owned())
    );
}

#[test]
fn context_bar_long_fifty_percent() {
    let ctx = RenderCtx {
        context_used_pct: Some(50.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_bar_long", &ctx),
        Some("[██████████░░░░░░░░░░]".to_owned())
    );
}

#[test]
fn context_bar_long_hundred_percent() {
    let ctx = RenderCtx {
        context_used_pct: Some(100.0),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("context_bar_long", &ctx),
        Some("[████████████████████]".to_owned())
    );
}

#[test]
fn context_bar_long_absent() {
    assert_eq!(render_placeholder("context_bar_long", &ctx_empty()), None);
}

// ── exceeds_200k ──────────────────────────────────────────────────────────

#[test]
fn exceeds_200k_true_returns_bang() {
    let ctx = RenderCtx {
        exceeds_200k: Some(true),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("exceeds_200k", &ctx),
        Some("!".to_owned())
    );
}

#[test]
fn exceeds_200k_false_returns_none() {
    let ctx = RenderCtx {
        exceeds_200k: Some(false),
        ..Default::default()
    };
    assert_eq!(render_placeholder("exceeds_200k", &ctx), None);
}

#[test]
fn exceeds_200k_absent_returns_none() {
    assert_eq!(render_placeholder("exceeds_200k", &ctx_empty()), None);
}
