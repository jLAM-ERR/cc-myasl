//! Bottom pane — cursor-aware keymap.
//!
//! Renders a context-sensitive keymap based on (focus, mode, cursor).
//! When the cursor is on a Custom segment, shows a one-line hint above
//! the keymap.  When powerline is active and focus is Top, shows a
//! one-line powerline-preview note.
//!
//! Truncation: when width < total formatted length, drops pairs from
//! the front (lowest priority) while always preserving `q:quit` and
//! `Ctrl+S:save` (priority 255).

use std::collections::HashSet;

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Paragraph},
};

use crate::tui::app4::{App, Cursor, Focus, Mode};
use crate::tui::builder::BuilderSegment;
use crate::tui::catalog::Category;

// ── priority helpers ──────────────────────────────────────────────────────────

/// Returns a priority value (higher = keep longer during truncation).
/// Only priority 255 is unconditionally required; lower values are optional.
fn priority(key: &str) -> u8 {
    match key {
        "q" | "Ctrl+S" => 255,
        "?" => 200,
        _ => 1,
    }
}

// ── keymap pair computation ───────────────────────────────────────────────────

fn keymap_pairs(app: &App) -> Vec<(&'static str, &'static str)> {
    match (app.focus, app.mode, &app.cursor) {
        // Editing modes — same keymap regardless of focus/cursor.
        (_, Mode::Filter, _) | (_, Mode::EditingSeparator, _) => {
            vec![
                ("[edit]", "type to change"),
                ("Enter", "commit"),
                ("Esc", "cancel"),
            ]
        }
        (_, Mode::PickingFgColor, _) | (_, Mode::PickingBgColor, _) => {
            vec![("j/k", "move"), ("Enter", "pick"), ("Esc", "cancel")]
        }
        // Confirm / overlay modes.
        (_, Mode::ConfirmDelete, _) => {
            vec![("y", "confirm"), ("n", "cancel")]
        }
        (_, Mode::ConfirmQuit, _) => {
            vec![("y", "quit"), ("n", "cancel"), ("Esc", "cancel")]
        }
        (_, Mode::Help, _) => {
            vec![("Esc", "close"), ("?", "close"), ("any", "close")]
        }
        (_, Mode::Saving, _) => {
            vec![("Esc", "cancel")]
        }
        // Top pane — Segment cursor.
        (Focus::Top, Mode::Browsing, Cursor::Segment(_)) => vec![
            ("←/→", "cursor"),
            ("</>", "reorder"),
            ("x", "delete"),
            ("c", "fg"),
            ("b", "bg"),
            ("↑/↓", "line"),
            ("Tab", "middle"),
            ("q", "quit"),
            ("?", "help"),
        ],
        // Top pane — Gutter cursor.
        (Focus::Top, Mode::Browsing, Cursor::Gutter) => vec![
            ("↑/↓", "line"),
            ("s", "separator"),
            ("J/K", "move-line"),
            ("y", "duplicate"),
            ("x", "delete-line"),
            ("Tab", "middle"),
            ("q", "quit"),
            ("?", "help"),
        ],
        // Top pane — VirtualNewLine cursor.
        (Focus::Top, Mode::Browsing, Cursor::VirtualNewLine) => vec![
            ("Enter", "add-line"),
            ("↑", "back"),
            ("Tab", "middle"),
            ("q", "quit"),
            ("?", "help"),
        ],
        // Middle pane — Appearance tab.
        (Focus::Middle, Mode::Browsing, _) if app.active_tab == Category::Appearance => vec![
            ("Space", "toggle"),
            ("Enter", "edit"),
            ("[/]", "tab"),
            ("j/k", "row"),
            ("Tab", "top"),
            ("Ctrl+S", "save"),
            ("q", "quit"),
        ],
        // Middle pane — preset rows.
        (Focus::Middle, Mode::Browsing, _) => vec![
            ("Space", "toggle"),
            ("/", "filter"),
            ("[/]", "tab"),
            ("j/k", "row"),
            ("Tab", "top"),
            ("Ctrl+S", "save"),
            ("q", "quit"),
            ("?", "help"),
        ],
        // Fallback (Bottom focus, etc.).
        _ => vec![("q", "quit"), ("?", "help")],
    }
}

// ── format a pair as spans ────────────────────────────────────────────────────

fn pair_width(key: &str, action: &str) -> usize {
    // "KEY:action  " — key + ":" + action + 2 spaces separator.
    // Use char count for multi-byte keys like ←/→.
    key.chars().count() + 1 + action.chars().count() + 2
}

fn append_pair(spans: &mut Vec<Span<'static>>, key: &'static str, action: &'static str) {
    spans.push(Span::styled(
        key.to_owned(),
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD),
    ));
    spans.push(Span::raw(":".to_owned()));
    spans.push(Span::raw(action.to_owned()));
    spans.push(Span::raw("  ".to_owned()));
}

// ── truncation ────────────────────────────────────────────────────────────────

fn truncate_pairs(
    pairs: Vec<(&'static str, &'static str)>,
    width: usize,
) -> Vec<(&'static str, &'static str)> {
    let total: usize = pairs.iter().map(|(k, a)| pair_width(k, a)).sum();
    if total <= width {
        return pairs;
    }

    // Separate required pairs (priority 255) from optional ones.
    let mut required: Vec<(&'static str, &'static str)> = Vec::new();
    let mut optional: Vec<(&'static str, &'static str)> = Vec::new();

    for &(k, a) in &pairs {
        if priority(k) == 255 {
            required.push((k, a));
        } else {
            optional.push((k, a));
        }
    }

    // Add optional pairs from the end (highest semantic priority) until full.
    let req_width: usize = required.iter().map(|(k, a)| pair_width(k, a)).sum();
    let mut budget = width.saturating_sub(req_width);
    let mut kept: HashSet<(&str, &str)> = HashSet::new();
    for &(k, a) in optional.iter().rev() {
        let w = pair_width(k, a);
        if w <= budget {
            kept.insert((k, a));
            budget -= w;
        }
    }

    // Reconstruct in original order.
    let required_set: HashSet<(&str, &str)> = required.iter().copied().collect();
    pairs
        .into_iter()
        .filter(|&(k, a)| kept.contains(&(k, a)) || required_set.contains(&(k, a)))
        .collect()
}

// ── public render entry point ─────────────────────────────────────────────────

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let (border_color, title_text) = if app.focus == Focus::Bottom {
        (Color::Cyan, "▶ Keys ")
    } else {
        (Color::DarkGray, "  Keys ")
    };
    let title_style = if app.focus == Focus::Bottom {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    let block = Block::bordered()
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(title_text, title_style));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    if inner.height == 0 {
        return;
    }

    let width = inner.width as usize;
    let inner_height = inner.height as usize;
    let mut ratatui_lines: Vec<Line<'static>> = Vec::new();

    // ── hints (only when there is room above the mandatory keymap row) ────────
    if app.focus == Focus::Top && inner_height > 1 {
        let mut hint_count = 0usize;

        if let Cursor::Segment(seg_idx) = app.cursor {
            if let Some(BuilderSegment::Custom { template, .. }) = app
                .builder
                .lines
                .get(app.active_line)
                .and_then(|l| l.segments.get(seg_idx))
            {
                let hint = format!(
                    "custom: `{}` — toggle disabled (edit JSON to change)",
                    template
                );
                ratatui_lines.push(Line::from(vec![Span::styled(
                    hint,
                    Style::default().add_modifier(Modifier::DIM),
                )]));
                hint_count += 1;
            }
        }

        // Powerline hint — only if it fits alongside any prior hint + keymap.
        if app.builder.powerline && hint_count + 1 < inner_height {
            ratatui_lines.push(Line::from(vec![Span::styled(
                "(powerline preview shows plain — actual render uses chevrons)".to_owned(),
                Style::default()
                    .fg(Color::DarkGray)
                    .add_modifier(Modifier::DIM),
            )]));
        }
    }

    // ── keymap line ──────────────────────────────────────────────────────────
    let pairs = keymap_pairs(app);
    let pairs = truncate_pairs(pairs, width);

    let mut spans: Vec<Span<'static>> = Vec::new();
    for (key, action) in pairs {
        append_pair(&mut spans, key, action);
    }
    ratatui_lines.push(Line::from(spans));

    frame.render_widget(Paragraph::new(ratatui_lines), inner);
}

#[cfg(test)]
#[path = "bottom_tests.rs"]
mod tests;
