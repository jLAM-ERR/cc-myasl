use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
};

use crate::config::schema::MAX_LINES;

use super::super::app::App;

pub fn render_line_list(frame: &mut Frame, area: Rect, app: &App) {
    let line_count = app.config.lines.len();
    let mut items: Vec<ListItem> = (0..line_count)
        .map(|i| ListItem::new(format!("Line {i}")))
        .collect();
    if line_count < MAX_LINES {
        items.push(ListItem::new("+ add line"));
    }

    let focused = app.focus == super::super::app::Focus::LineList;
    let border_style = if focused {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let list = List::new(items)
        .block(
            Block::default()
                .title("Lines")
                .borders(Borders::ALL)
                .border_style(border_style),
        )
        .highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    let mut state = ListState::default();
    state.select(Some(app.selected_line));

    frame.render_stateful_widget(list, area, &mut state);
}
