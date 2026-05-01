//! Config structs for every built-in template.

use crate::config::schema::{Config, Line, TemplateSegment};

pub(super) fn line(segments: Vec<TemplateSegment>) -> Line {
    Line {
        separator: String::new(),
        segments: segments.into_iter().map(Into::into).collect(),
    }
}

pub(super) fn s(tmpl: &str) -> TemplateSegment {
    TemplateSegment::new(tmpl)
}

pub(super) fn opt(tmpl: &str) -> TemplateSegment {
    TemplateSegment::new(tmpl).with_hide_when_absent()
}

// default.txt: {model}{? · 5h: {five_left}%}{? · 7d: {seven_left}%}{? (resets {seven_reset_clock})}
pub(super) fn default_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![line(vec![
            s("{model}"),
            opt(" · 5h: {five_left}%"),
            opt(" · 7d: {seven_left}%"),
            opt(" (resets {seven_reset_clock})"),
        ])],
    }
}

// minimal.txt: {model}{? {five_left}%/{seven_left}%}
pub(super) fn minimal_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![line(vec![s("{model}"), opt(" {five_left}%/{seven_left}%")])],
    }
}

// compact.txt: {model}{? {five_left}/{seven_left}}
pub(super) fn compact_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![line(vec![s("{model}"), opt(" {five_left}/{seven_left}")])],
    }
}

// bars.txt: {model}{? 5h:{five_bar}}{? 7d:{seven_bar}}
pub(super) fn bars_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![line(vec![
            s("{model}"),
            opt(" 5h:{five_bar}"),
            opt(" 7d:{seven_bar}"),
        ])],
    }
}

// colored.txt: {model}{? · 5h: {five_color}{five_left}%{reset}}{? · 7d: {seven_color}{seven_left}%{reset}}
pub(super) fn colored_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![line(vec![
            s("{model}"),
            opt(" · 5h: {five_color}{five_left}%{reset}"),
            opt(" · 7d: {seven_color}{seven_left}%{reset}"),
        ])],
    }
}

// emoji.txt: {model}{? · {five_state} 5h {five_left}%}{? · {seven_state} 7d {seven_left}%}
pub(super) fn emoji_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![line(vec![
            s("{model}"),
            opt(" · {five_state} 5h {five_left}%"),
            opt(" · {seven_state} 7d {seven_left}%"),
        ])],
    }
}

// emoji_verbose.txt: 🤖 {model}{? · {state_icon} {cwd_basename}}{? · ⏳ {five_left}%/{seven_left}%}{? · ⏰ {seven_reset_clock}}
pub(super) fn emoji_verbose_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![line(vec![
            s("🤖 {model}"),
            opt(" · {state_icon} {cwd_basename}"),
            opt(" · ⏳ {five_left}%/{seven_left}%"),
            opt(" · ⏰ {seven_reset_clock}"),
        ])],
    }
}

// verbose.txt: {model}{? · {cwd_basename}}{? · 5h:{five_bar} {five_left}% (in {five_reset_in})}{? · 7d:{seven_bar} {seven_left}% (in {seven_reset_in})}{? · extra:{extra_left}}
pub(super) fn verbose_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![line(vec![
            s("{model}"),
            opt(" · {cwd_basename}"),
            opt(" · 5h:{five_bar} {five_left}% (in {five_reset_in})"),
            opt(" · 7d:{seven_bar} {seven_left}% (in {seven_reset_in})"),
            opt(" · extra:{extra_left}"),
        ])],
    }
}

// rich: Phase-2 showcase — model · vim · context bar / git branch + changes + cwd / cost · clock · tokens
pub(super) fn rich_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        lines: vec![
            line(vec![
                s("{model}"),
                opt(" · {vim_mode}"),
                opt(" · ctx:{context_bar} {context_used_pct_int}%"),
            ]),
            line(vec![
                opt("⎇ {git_branch}"),
                opt(" ({git_changes})"),
                opt(" · {cwd}"),
            ]),
            line(vec![
                opt("{cost_usd}$"),
                opt(" · {session_clock}"),
                opt(" · {tokens_total}tok"),
            ]),
        ],
    }
}
