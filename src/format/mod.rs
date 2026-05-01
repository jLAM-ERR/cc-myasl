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
#[deprecated(note = "Phase-1 transition only — replaced by config::render in Task 10")]
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

/// Render `template` against `ctx` for use as a single config segment.
///
/// Returns `None` if any top-level placeholder in the template resolves to
/// `None` or empty — the caller (config::render) will then decide whether to
/// hide the segment based on `hide_when_absent`.  Optional blocks (`{? … }`)
/// inside a segment are handled normally: a collapsing inner block renders to
/// empty but does NOT make the outer result `None`.
///
/// Differs from `render`: `render` always returns a `String` (unknown
/// placeholders → empty string); `render_segment` is strict — any top-level
/// placeholder that resolves to absent makes the whole segment `None`.
pub fn render_segment(template: &str, ctx: &RenderCtx) -> Option<String> {
    let tokens = parser::tokenize(template);
    let mut out = String::new();
    for t in &tokens {
        match t {
            Token::Text(s) => out.push_str(s),
            Token::Placeholder(name) => match placeholders::render_placeholder(name, ctx) {
                Some(v) if !v.is_empty() => out.push_str(&v),
                _ => return None,
            },
            Token::Optional(inner) => {
                // Optional blocks collapse internally but do not propagate None upward.
                let mut scratch = String::new();
                if render_optional(inner, ctx, &mut scratch) {
                    out.push_str(&scratch);
                }
            }
        }
    }
    Some(out)
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

/// Shared mutex serializing all tests across `format/*` that mutate the
/// `STATUSLINE_RED` / `STATUSLINE_YELLOW` env vars.  Without this, tests
/// in `format/mod.rs`, `format/thresholds.rs`, and `format/placeholders/`
/// would race against each other on the process-global env table.
#[cfg(test)]
pub(crate) static ENV_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
            ..Default::default()
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

    // ── one-way-import invariant ─────────────────────────────────────────────

    #[test]
    fn format_files_do_not_import_config() {
        // Split the banned pattern so the test source itself never contains it
        // as a contiguous byte sequence (same technique as placeholders::tests).
        let config_import = ["use crate", "::", "config"].concat();
        let format_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/format");
        for entry in walkdir_rs(&format_dir) {
            let content = std::fs::read_to_string(&entry).unwrap_or_default();
            assert!(
                !content.contains(&config_import),
                "format file {:?} must not import crate::config",
                entry
            );
        }
    }

    fn walkdir_rs(dir: &std::path::Path) -> Vec<std::path::PathBuf> {
        let mut out = Vec::new();
        if let Ok(rd) = std::fs::read_dir(dir) {
            for entry in rd.flatten() {
                let p = entry.path();
                if p.is_dir() {
                    out.extend(walkdir_rs(&p));
                } else if p.extension().map_or(false, |e| e == "rs") {
                    out.push(p);
                }
            }
        }
        out
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
