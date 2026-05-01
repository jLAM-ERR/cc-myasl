use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

use crate::config::schema::NAMED_COLORS;
use crate::tui::app::{App, Mode};

use super::placeholder_picker::centered_rect;

/// All entries in the color picker: 8 named colors + "(none)".
/// Index 8 = "(none)" → sets the field to None.
pub const COLOR_PICKER_ENTRIES: usize = 9; // NAMED_COLORS.len() + 1

fn name_to_ratatui(name: &str) -> Color {
    match name {
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        "default" => Color::Reset,
        _ => Color::Reset,
    }
}

pub fn render_color_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(40, 60, area);
    frame.render_widget(Clear, popup);

    let title = match app.mode {
        Mode::PickingFgColor => "Pick foreground color",
        Mode::PickingBgColor => "Pick background color",
        _ => "Pick color",
    };

    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Split off a one-line hint at the top.
    let split = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

    let hint = ratatui::widgets::Paragraph::new("↑/↓ move  Enter select  Esc cancel");
    frame.render_widget(hint, split[0]);

    let items: Vec<ListItem> = NAMED_COLORS
        .iter()
        .map(|&name| ListItem::new(name).style(Style::default().fg(name_to_ratatui(name))))
        .chain(std::iter::once(ListItem::new("(none)")))
        .collect();

    let mut state = ListState::default();
    state.select(Some(
        app.color_picker_selected.min(COLOR_PICKER_ENTRIES - 1),
    ));

    let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, split[1], &mut state);
}
