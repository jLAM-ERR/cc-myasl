use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line as TuiLine, Span},
    widgets::{Block, Borders, Clear, List, ListItem},
};

use super::placeholder_picker::centered_rect;

/// All help entries shown in the overlay, grouped by section.
/// Each entry is either a section header (prefix "## ") or a keybinding.
const HELP_ENTRIES: &[&str] = &[
    "## Browsing",
    "j/k  —  move selection",
    "Tab  —  switch focus / cycle editor fields",
    "Enter  —  edit segment / select",
    "d  —  delete segment",
    "D  —  delete line",
    "n  —  add new line",
    "J/K  —  reorder segment down/up",
    "Ctrl+S  —  save config",
    "q  —  quit (confirm if dirty)",
    "?  —  show this help",
    "## Editing template",
    "Type  —  insert character",
    "Backspace  —  delete character",
    "Enter  —  commit edit",
    "Esc  —  abort edit",
    "p  —  open placeholder picker",
    "## Picker (placeholder / color)",
    "j/k or ↑/↓  —  move selection",
    "Enter  —  select item",
    "Esc  —  cancel",
    "## Global",
    "Shift+P  —  toggle Powerline mode",
    "## Confirm quit",
    "y/Y  —  quit without saving",
    "n/N  —  return to browsing",
    "s/S  —  save then quit",
];

pub fn render_help_overlay(frame: &mut Frame, area: Rect) {
    let popup = centered_rect(70, 85, area);
    frame.render_widget(Clear, popup);

    let block = Block::default().title("Help").borders(Borders::ALL);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let items: Vec<ListItem> = HELP_ENTRIES
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

    let list = List::new(items);
    frame.render_widget(list, inner);
}

#[cfg(test)]
mod tests {
    use super::HELP_ENTRIES;

    #[test]
    fn help_entries_contains_ctrl_s() {
        assert!(
            HELP_ENTRIES.iter().any(|e| e.contains("Ctrl+S")),
            "HELP_ENTRIES must mention Ctrl+S"
        );
    }

    #[test]
    fn help_entries_contains_shift_p() {
        assert!(
            HELP_ENTRIES.iter().any(|e| e.contains("Shift+P")),
            "HELP_ENTRIES must mention Shift+P"
        );
    }

    #[test]
    fn help_entries_contains_question_mark() {
        assert!(
            HELP_ENTRIES.iter().any(|e| e.contains('?')),
            "HELP_ENTRIES must mention ?"
        );
    }

    #[test]
    fn help_entries_contains_browsing_section() {
        assert!(
            HELP_ENTRIES.iter().any(|e| *e == "## Browsing"),
            "HELP_ENTRIES must have a Browsing section"
        );
    }

    #[test]
    fn help_entries_contains_editing_section() {
        assert!(
            HELP_ENTRIES.iter().any(|e| *e == "## Editing template"),
            "HELP_ENTRIES must have an Editing template section"
        );
    }

    #[test]
    fn help_entries_contains_picker_section() {
        assert!(
            HELP_ENTRIES
                .iter()
                .any(|e| e.contains("Picker") || e.contains("picker")),
            "HELP_ENTRIES must have a picker section"
        );
    }

    #[test]
    fn help_entries_contains_confirm_quit_section() {
        assert!(
            HELP_ENTRIES.iter().any(|e| e.contains("Confirm quit")),
            "HELP_ENTRIES must have a Confirm quit section"
        );
    }
}
