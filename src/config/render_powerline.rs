//! Powerline rendering: chevron transitions with bg-color flow.
//!
//! Called by `config::render::render` when `config.powerline == true`.
//! Requires a Nerd Font for the chevron glyph U+E0B0 (); see README Task 15.

use crate::config::schema::{Line, Segment};
use crate::format::values::{ansi_bg, ansi_fg};
use crate::format::{self, placeholders::RenderCtx};

/// Powerline right-arrow chevron (U+E0B0). Requires a Nerd Font.
pub const CHEVRON: &str = "\u{E0B0}";

/// ANSI reset — clears all attributes.
const RESET: &str = "\x1b[0m";

/// A resolved item in the Powerline pipeline.
pub enum PlItem {
    /// Rendered template segment.
    Seg {
        text: String,
        fg: Option<String>,
        bg: String,
    },
    /// Flex spacer (bg = "default", width resolved at render time).
    Flex,
}

impl PlItem {
    pub fn bg_name(&self) -> &str {
        match self {
            PlItem::Seg { bg, .. } => bg.as_str(),
            PlItem::Flex => "default",
        }
    }

    /// Bare display text (ANSI-stripped length used for flex calculation).
    pub fn text(&self) -> &str {
        match self {
            PlItem::Seg { text, .. } => text.as_str(),
            PlItem::Flex => "",
        }
    }
}

/// Render one `Line` in Powerline mode.
///
/// Chevron placement (per plan §Powerline rendering algorithm):
/// - No leading chevron before the first segment.
/// - One transition chevron between each pair of adjacent visible items.
/// - One trailing chevron after the last segment (fg=last_bg, bg=default).
/// - Total for N visible items: N chevrons (N-1 between + 1 trailing).
///
/// Flex spacer interaction:
/// - Chevron before flex: (prev_bg → default).
/// - Flex emits spaces with default bg to fill terminal width.
/// - Chevron after flex: (default → next_bg).
pub fn render_powerline_line(line: &Line, ctx: &RenderCtx, term_width: usize) -> String {
    use crate::config::render::{pad, visible_width};

    let items = collect_items(line, ctx, pad);

    if items.is_empty() {
        return String::new();
    }

    // Pre-compute flex fill width (natural text + all chevrons).
    let flex_fill = if items.iter().any(|it| matches!(it, PlItem::Flex)) {
        let text_width: usize = items.iter().map(|it| visible_width(it.text())).sum();
        let chevron_count = items.len(); // N-1 between + 1 trailing
        let chevron_width = chevron_count * CHEVRON.len();
        term_width.saturating_sub(text_width + chevron_width).max(1)
    } else {
        0
    };

    let mut out = String::new();
    let mut prev_bg = "default".to_owned();

    for (i, item) in items.iter().enumerate() {
        let cur_bg = item.bg_name().to_owned();

        if i == 0 {
            // First item: no chevron, just set bg (and fg if set).
            out.push_str(ansi_bg(&cur_bg));
            if let PlItem::Seg { fg: Some(fg), .. } = item {
                out.push_str(ansi_fg(fg));
            }
        } else {
            // Transition chevron: fg=prev_bg, bg=cur_bg.
            out.push_str(RESET);
            out.push_str(ansi_fg(&prev_bg));
            out.push_str(ansi_bg(&cur_bg));
            out.push_str(CHEVRON);
            // Restore text fg for this item.
            if let PlItem::Seg { fg: Some(fg), .. } = item {
                out.push_str(ansi_fg(fg));
            } else {
                out.push_str(ansi_fg("default"));
            }
        }

        match item {
            PlItem::Seg { text, .. } => out.push_str(text),
            PlItem::Flex => out.push_str(&" ".repeat(flex_fill)),
        }

        prev_bg = cur_bg;
    }

    // Trailing chevron: fg=last_bg, bg=default terminal, then full reset.
    out.push_str(RESET);
    out.push_str(ansi_fg(&prev_bg));
    out.push_str(ansi_bg("default"));
    out.push_str(CHEVRON);
    out.push_str(RESET);

    out
}

/// Walk `line.segments`, render and pad template segments, skip hidden ones.
/// `pad_fn` is injected so this stays testable without depending on the render module path.
fn collect_items<F>(line: &Line, ctx: &RenderCtx, pad_fn: F) -> Vec<PlItem>
where
    F: Fn(&str, u8) -> String,
{
    let mut items = Vec::new();
    for seg in &line.segments {
        match seg {
            Segment::Template(t) => {
                let result = format::render_segment(&t.template, ctx);
                let visible = match result {
                    Some(s) if !s.is_empty() => Some(s),
                    _ => None,
                };
                if visible.is_none() && t.hide_when_absent {
                    // Hidden: skip entirely — no chevron slot.
                    continue;
                }
                let text = pad_fn(visible.as_deref().unwrap_or(""), t.padding);
                items.push(PlItem::Seg {
                    text,
                    fg: t.color.clone(),
                    bg: t.bg.clone().unwrap_or_else(|| "default".to_owned()),
                });
            }
            Segment::Flex(_) => {
                items.push(PlItem::Flex);
            }
        }
    }
    items
}

#[cfg(test)]
#[path = "render_powerline_tests.rs"]
mod powerline_tests;
