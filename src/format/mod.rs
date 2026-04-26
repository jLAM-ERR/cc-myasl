//! Format engine for the status-line template language.

pub mod parser;
pub mod placeholders;
pub mod thresholds;
pub mod values;

use parser::Token;
pub use placeholders::RenderCtx;

/// Render `template` against `ctx`, returning the final status-line string.
///
/// - `{name}` placeholders are substituted from `ctx`.
/// - `{? … }` optional blocks are emitted only when every placeholder
///   inside them resolves to a non-empty value; otherwise the whole
///   block is silently suppressed.
/// - Unknown placeholder names produce no output (empty string).
pub fn render(template: &str, ctx: &RenderCtx) -> String {
    let tokens = parser::tokenize(template);
    let mut out = String::new();
    render_tokens(&tokens, ctx, &mut out);
    out
}

fn render_tokens(tokens: &[Token], ctx: &RenderCtx, out: &mut String) {
    for t in tokens {
        match t {
            Token::Text(s) => out.push_str(s),
            Token::Placeholder(name) => {
                if let Some(v) = placeholders::render_placeholder(name, ctx) {
                    out.push_str(&v);
                }
                // unknown placeholder → emit nothing
            }
            Token::Optional(inner) => {
                // Render inner into a scratch buffer; if any placeholder
                // inside it resolves to empty (None or empty string),
                // the whole optional collapses.
                let mut scratch = String::new();
                let all_present = render_optional(inner, ctx, &mut scratch);
                if all_present {
                    out.push_str(&scratch);
                }
            }
        }
    }
}

/// Look up a built-in template by name.
///
/// Returns `Some(&str)` for one of the shipped templates
/// (`default`, `minimal`, `compact`, `bars`, `colored`,
/// `emoji`, `emoji_verbose`, `verbose`).  Returns `None`
/// for any unknown name.  The orchestrator (`main.rs`)
/// resolves the precedence order; this is just the lookup.
pub fn lookup_template(name: &str) -> Option<&'static str> {
    match name {
        "default" => Some(include_str!("../../templates/default.txt")),
        "minimal" => Some(include_str!("../../templates/minimal.txt")),
        "compact" => Some(include_str!("../../templates/compact.txt")),
        "bars" => Some(include_str!("../../templates/bars.txt")),
        "colored" => Some(include_str!("../../templates/colored.txt")),
        "emoji" => Some(include_str!("../../templates/emoji.txt")),
        "emoji_verbose" => Some(include_str!("../../templates/emoji_verbose.txt")),
        "verbose" => Some(include_str!("../../templates/verbose.txt")),
        _ => None,
    }
}

fn render_optional(tokens: &[Token], ctx: &RenderCtx, out: &mut String) -> bool {
    let mut all_present = true;
    for t in tokens {
        match t {
            Token::Text(s) => out.push_str(s),
            Token::Placeholder(name) => match placeholders::render_placeholder(name, ctx) {
                Some(v) if !v.is_empty() => out.push_str(&v),
                _ => all_present = false,
            },
            Token::Optional(inner) => {
                let mut nested = String::new();
                if render_optional(inner, ctx, &mut nested) {
                    out.push_str(&nested);
                }
                // nested optional that collapses does NOT make outer fail
            }
        }
    }
    all_present
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Mutex;

    /// Shared mutex for all tests that set or read threshold env vars.
    static ENV_MUTEX: Mutex<()> = Mutex::new(());

    fn full_ctx() -> RenderCtx {
        RenderCtx {
            model: Some("claude-opus-4".to_owned()),
            cwd: Some(PathBuf::from("/tmp/project")),
            five_used: Some(30.0),
            five_reset_unix: Some(3600),
            seven_used: Some(60.0),
            seven_reset_unix: Some(90000),
            extra_enabled: Some(false),
            extra_used: None,
            extra_limit: None,
            extra_pct: None,
            now_unix: 0,
        }
    }

    // ── happy-path full template ─────────────────────────────────────────────

    #[test]
    fn render_full_template_happy_path() {
        let ctx = full_ctx();
        let tmpl = "{model} · 5h:{five_left}%";
        let out = render(tmpl, &ctx);
        assert_eq!(out, "claude-opus-4 · 5h:70%");
    }

    #[test]
    fn render_plain_text_unchanged() {
        let ctx = RenderCtx::default();
        let out = render("hello world", &ctx);
        assert_eq!(out, "hello world");
    }

    // ── unknown placeholder silently empty ───────────────────────────────────

    #[test]
    fn render_unknown_placeholder_is_empty() {
        let ctx = RenderCtx::default();
        let out = render("prefix {unknown_xyz} suffix", &ctx);
        assert_eq!(out, "prefix  suffix");
    }

    // ── optional collapse ────────────────────────────────────────────────────

    #[test]
    fn optional_collapses_when_placeholder_absent() {
        // five_used is None → {five_left} returns None → optional collapses
        let ctx = RenderCtx::default();
        let out = render("before {? · 5h:{five_left}% } after", &ctx);
        assert_eq!(out, "before  after");
    }

    #[test]
    fn optional_emitted_when_all_present() {
        let ctx = full_ctx();
        let out = render("before {? · 5h:{five_left}% } after", &ctx);
        assert_eq!(out, "before  · 5h:70%  after");
    }

    // ── nested optional independence ─────────────────────────────────────────

    #[test]
    fn nested_optional_inner_collapse_does_not_break_outer() {
        // outer: {model} present; inner: {five_left} absent → inner collapses
        // outer should still emit since its own placeholder resolved.
        // Tokenizer includes surrounding spaces as Text tokens inside optionals.
        let ctx = RenderCtx {
            model: Some("m".to_owned()),
            five_used: None,
            ..Default::default()
        };
        // template: {? {model} {? [{five_left}] } }
        let out = render("{? {model} {? [{five_left}] } }", &ctx);
        // " m  " — leading space (from `{? `), then m, space, empty inner, space
        assert_eq!(out, " m  ");
    }

    #[test]
    fn nested_optional_both_present() {
        let ctx = full_ctx();
        let out = render("{? {model} {? [{five_left}] } }", &ctx);
        assert_eq!(out, " claude-opus-4  [70]  ");
    }

    // ── colour and icon output ───────────────────────────────────────────────

    #[test]
    fn render_five_color_green() {
        // five_used=30 → left=70 → Green
        let ctx = full_ctx();
        let out = render("{five_color}", &ctx);
        assert_eq!(out, "\x1b[32m");
    }

    #[test]
    fn render_five_state_green_emoji() {
        let ctx = full_ctx();
        let out = render("{five_state}", &ctx);
        assert_eq!(out, "🟢");
    }

    #[test]
    fn render_reset_always_present() {
        let ctx = RenderCtx::default();
        let out = render("{reset}", &ctx);
        assert_eq!(out, "\x1b[0m");
    }

    // ── threshold env-var override ───────────────────────────────────────────

    #[test]
    fn render_threshold_override_changes_color() {
        let _guard = ENV_MUTEX.lock().unwrap();

        // five_used=30 → left=70; with red=80, yellow=90 → Red
        std::env::set_var("STATUSLINE_RED", "80");
        std::env::set_var("STATUSLINE_YELLOW", "90");

        let ctx = full_ctx();
        let out = render("{five_color}", &ctx);
        assert_eq!(out, "\x1b[31m"); // Red

        std::env::remove_var("STATUSLINE_RED");
        std::env::remove_var("STATUSLINE_YELLOW");
    }

    // ── lookup_template ──────────────────────────────────────────────────────

    #[test]
    fn lookup_default_returns_some() {
        let s = lookup_template("default").expect("default exists");
        assert!(!s.is_empty());
        assert!(s.contains("{model}"));
    }

    #[test]
    fn lookup_unknown_returns_none() {
        assert!(lookup_template("does-not-exist").is_none());
    }

    #[test]
    fn all_shipped_templates_render_against_full_ctx() {
        let _guard = ENV_MUTEX.lock().unwrap();
        use crate::format::placeholders::RenderCtx;
        use std::path::PathBuf;
        let ctx = RenderCtx {
            model: Some("claude".into()),
            cwd: Some(PathBuf::from("/tmp/proj")),
            five_used: Some(30.0),
            five_reset_unix: Some(3600),
            seven_used: Some(60.0),
            seven_reset_unix: Some(90000),
            extra_enabled: Some(true),
            extra_used: Some(50.0),
            extra_limit: Some(100.0),
            extra_pct: Some(50.0),
            now_unix: 0,
        };
        for name in [
            "default",
            "minimal",
            "compact",
            "bars",
            "colored",
            "emoji",
            "emoji_verbose",
            "verbose",
        ] {
            let tmpl = lookup_template(name).expect(name);
            let out = render(tmpl, &ctx);
            assert!(!out.is_empty(), "{name} rendered empty");
        }
    }

    #[test]
    fn all_shipped_templates_render_against_empty_ctx() {
        let _guard = ENV_MUTEX.lock().unwrap();
        // Empty ctx → optional segments all collapse → output may be very short
        // but must NOT panic.
        use crate::format::placeholders::RenderCtx;
        let ctx = RenderCtx::default();
        for name in [
            "default",
            "minimal",
            "compact",
            "bars",
            "colored",
            "emoji",
            "emoji_verbose",
            "verbose",
        ] {
            let tmpl = lookup_template(name).expect(name);
            let _out = render(tmpl, &ctx);
        }
    }

    #[test]
    fn render_threshold_override_changes_icon() {
        let _guard = ENV_MUTEX.lock().unwrap();

        // five_used=30 → left=70; with yellow=80 → Yellow
        std::env::remove_var("STATUSLINE_RED");
        std::env::set_var("STATUSLINE_YELLOW", "80");

        let ctx = full_ctx();
        let out = render("{five_state}", &ctx);
        assert_eq!(out, "🟡");

        std::env::remove_var("STATUSLINE_YELLOW");
    }
}
