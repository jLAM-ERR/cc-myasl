use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
};

use crate::format::catalog::filtered_placeholders;
use crate::tui::app::App;

/// Centered rect: `pct_w`% wide, `pct_h`% tall, centred in `area`.
pub fn centered_rect(pct_w: u16, pct_h: u16, area: Rect) -> Rect {
    let vertical = Layout::vertical([
        Constraint::Percentage((100 - pct_h) / 2),
        Constraint::Percentage(pct_h),
        Constraint::Percentage((100 - pct_h) / 2),
    ])
    .split(area);

    Layout::horizontal([
        Constraint::Percentage((100 - pct_w) / 2),
        Constraint::Percentage(pct_w),
        Constraint::Percentage((100 - pct_w) / 2),
    ])
    .split(vertical[1])[1]
}

pub fn render_placeholder_picker(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered_rect(60, 70, area);

    // Clear the background so the popup is readable.
    frame.render_widget(Clear, popup);

    let title = format!("Pick a placeholder (filter: {})", app.picker_filter);
    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    // Split inner area: top row for filter hint, rest for list.
    let split = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

    let filter_line = Paragraph::new(format!("> {}", app.picker_filter))
        .alignment(Alignment::Left)
        .style(Style::default().add_modifier(Modifier::BOLD));
    frame.render_widget(filter_line, split[0]);

    let matches = filtered_placeholders(&app.picker_filter);
    let items: Vec<ListItem> = matches
        .iter()
        .map(|(name, desc)| ListItem::new(format!("{{{name}}} — {desc}")))
        .collect();

    let mut state = ListState::default();
    if !matches.is_empty() {
        state.select(Some(app.picker_selected.min(matches.len() - 1)));
    }

    let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, split[1], &mut state);
}
