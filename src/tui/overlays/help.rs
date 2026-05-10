use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Constraint, Rect},
    style::{Modifier, Style},
    text::{Line as TuiLine, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
};

use crate::tui::catalog::{Category, by_category};

const KEYMAP: &[&str] = &[
    "## Pane navigation",
    "Tab          — Top → Middle → Bottom → Top",
    "Shift+Tab    — reverse cycle",
    "## Top pane (Focus::Top, Browsing)",
    "←/→          — move cursor left/right",
    "</> (Shift)  — reorder segment left/right",
    "x            — delete segment at cursor",
    "c            — pick foreground color",
    "b            — pick background color",
    "↑/↓          — move active line up/down",
    "Enter (Gutter)  — add new line (virtual row)",
    "## Top pane (Focus::Top, ConfirmDelete)",
    "y/Y          — confirm delete line",
    "n/N / Esc    — cancel",
    "## Middle pane (Focus::Middle, Browsing)",
    "Space        — toggle preset on/off",
    "/            — enter Filter mode",
    "[            — previous category tab",
    "]            — next category tab",
    "## Middle pane (Focus::Middle, Filter)",
    "Type         — append to filter",
    "Backspace    — delete last char",
    "Enter        — commit filter",
    "Esc          — clear filter + return to Browsing",
    "## Editing separator",
    "Type         — insert character",
    "Backspace    — delete character",
    "Enter        — commit",
    "Esc          — cancel",
    "## Modes",
    "?            — toggle Help overlay",
    "Ctrl+S       — save config",
    "q            — quit (ConfirmQuit if dirty)",
    "Shift+P      — toggle Powerline mode",
    "## Color picker",
    "↑/↓ or k/j  — move selection (wraps)",
    "Enter        — commit color",
    "Esc          — cancel",
    "## Confirm quit",
    "y/Y          — quit without saving",
    "n/N / Esc    — return to Browsing",
];

/// Render a full-screen help overlay.
pub fn render(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(70, 80, area);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(" Help — ? to close ")
        .borders(Borders::ALL);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Split: keymap on top half, preset catalog on bottom half.
    let halves =
        ratatui::layout::Layout::vertical([Constraint::Percentage(55), Constraint::Percentage(45)])
            .split(inner);

    // ── keymap ────────────────────────────────────────────────────────────────
    let keymap_items: Vec<ListItem> = KEYMAP
        .iter()
        .map(|&entry| {
            if let Some(title) = entry.strip_prefix("## ") {
                ListItem::new(TuiLine::from(Span::styled(
                    title,
                    Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                )))
            } else {
                ListItem::new(entry)
            }
        })
        .collect();
    frame.render_widget(List::new(keymap_items), halves[0]);

    // ── preset catalog by category ────────────────────────────────────────────
    let mut catalog_items: Vec<ListItem> = Vec::new();
    for cat in Category::ordered() {
        let heading = cat_name(cat).to_string();
        catalog_items.push(ListItem::new(TuiLine::from(Span::styled(
            heading,
            Style::default().add_modifier(Modifier::BOLD),
        ))));
        for p in by_category(*cat) {
            catalog_items.push(ListItem::new(format!("  {} — {}", p.id, p.label)));
        }
    }
    frame.render_widget(List::new(catalog_items), halves[1]);
}

fn cat_name(c: &Category) -> &'static str {
    match c {
        Category::Workspace => "Workspace",
        Category::Git => "Git",
        Category::SessionModel => "Session/Model",
        Category::Context => "Context",
        Category::Tokens => "Tokens",
        Category::Cost => "Cost",
        Category::Rates => "Rates",
        Category::Appearance => "Appearance",
    }
}

fn centered_rect(pct_w: u16, pct_h: u16, area: Rect) -> Rect {
    let vertical = ratatui::layout::Layout::vertical([
        Constraint::Percentage((100 - pct_h) / 2),
        Constraint::Percentage(pct_h),
        Constraint::Percentage((100 - pct_h) / 2),
    ])
    .split(area);
    ratatui::layout::Layout::horizontal([
        Constraint::Percentage((100 - pct_w) / 2),
        Constraint::Percentage(pct_w),
        Constraint::Percentage((100 - pct_w) / 2),
    ])
    .split(vertical[1])[1]
}

/// Returns true if the overlay should be dismissed.
///
/// `?` toggles (dismisses), any other key also dismisses.
pub fn handle(_event: KeyEvent) -> bool {
    true
}
