use super::*;

fn ctx_empty() -> RenderCtx {
    RenderCtx::default()
}

// ── model_id ──────────────────────────────────────────────────────────────

#[test]
fn model_id_present() {
    let ctx = RenderCtx {
        model_id: Some("claude-opus-4-5".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("model_id", &ctx),
        Some("claude-opus-4-5".to_owned())
    );
}

#[test]
fn model_id_absent() {
    assert_eq!(render_placeholder("model_id", &ctx_empty()), None);
}

// ── version ───────────────────────────────────────────────────────────────

#[test]
fn version_present() {
    let ctx = RenderCtx {
        version: Some("1.2.3".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("version", &ctx),
        Some("1.2.3".to_owned())
    );
}

#[test]
fn version_absent() {
    assert_eq!(render_placeholder("version", &ctx_empty()), None);
}

// ── session_id ────────────────────────────────────────────────────────────

#[test]
fn session_id_present() {
    let ctx = RenderCtx {
        session_id: Some("abc-123".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("session_id", &ctx),
        Some("abc-123".to_owned())
    );
}

#[test]
fn session_id_absent() {
    assert_eq!(render_placeholder("session_id", &ctx_empty()), None);
}

// ── session_name ──────────────────────────────────────────────────────────

#[test]
fn session_name_present() {
    let ctx = RenderCtx {
        session_name: Some("my-session".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("session_name", &ctx),
        Some("my-session".to_owned())
    );
}

#[test]
fn session_name_absent() {
    assert_eq!(render_placeholder("session_name", &ctx_empty()), None);
}

// ── output_style ──────────────────────────────────────────────────────────

#[test]
fn output_style_present() {
    let ctx = RenderCtx {
        output_style: Some("auto".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("output_style", &ctx),
        Some("auto".to_owned())
    );
}

#[test]
fn output_style_absent() {
    assert_eq!(render_placeholder("output_style", &ctx_empty()), None);
}

// ── effort ────────────────────────────────────────────────────────────────

#[test]
fn effort_present() {
    let ctx = RenderCtx {
        effort_level: Some("high".to_owned()),
        ..Default::default()
    };
    assert_eq!(render_placeholder("effort", &ctx), Some("high".to_owned()));
}

#[test]
fn effort_absent() {
    assert_eq!(render_placeholder("effort", &ctx_empty()), None);
}

// ── thinking_enabled ──────────────────────────────────────────────────────

#[test]
fn thinking_enabled_true_returns_thinking() {
    let ctx = RenderCtx {
        thinking_enabled: Some(true),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("thinking_enabled", &ctx),
        Some("thinking".to_owned())
    );
}

#[test]
fn thinking_enabled_false_returns_none() {
    // False collapses the segment rather than rendering "no_thinking".
    let ctx = RenderCtx {
        thinking_enabled: Some(false),
        ..Default::default()
    };
    assert_eq!(render_placeholder("thinking_enabled", &ctx), None);
}

#[test]
fn thinking_enabled_absent() {
    assert_eq!(render_placeholder("thinking_enabled", &ctx_empty()), None);
}

// ── vim_mode ──────────────────────────────────────────────────────────────

#[test]
fn vim_mode_present() {
    let ctx = RenderCtx {
        vim_mode: Some("normal".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("vim_mode", &ctx),
        Some("normal".to_owned())
    );
}

#[test]
fn vim_mode_absent() {
    assert_eq!(render_placeholder("vim_mode", &ctx_empty()), None);
}

// ── agent_name ────────────────────────────────────────────────────────────

#[test]
fn agent_name_present() {
    let ctx = RenderCtx {
        agent_name: Some("subagent".to_owned()),
        ..Default::default()
    };
    assert_eq!(
        render_placeholder("agent_name", &ctx),
        Some("subagent".to_owned())
    );
}

#[test]
fn agent_name_absent() {
    assert_eq!(render_placeholder("agent_name", &ctx_empty()), None);
}
