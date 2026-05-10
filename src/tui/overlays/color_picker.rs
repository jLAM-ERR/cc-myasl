use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Layout, Rect},
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
};

use crate::config::named_color::NamedColor;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PickerMode {
    Foreground,
    Background,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickerEvent {
    Pending,
    Commit(NamedColor),
    /// None means the extra "(none)" entry was selected.
    CommitNone,
    Cancel,
}

/// All named colors in display order + a "(none)" sentinel at the end.
const ENTRIES: &[&str] = &[
    "red", "green", "yellow", "blue", "magenta", "cyan", "white", "default", "(none)",
];
pub const ENTRY_COUNT: usize = ENTRIES.len();

fn name_to_ratatui(name: &str) -> Color {
    match name {
        "red" => Color::Red,
        "green" => Color::Green,
        "yellow" => Color::Yellow,
        "blue" => Color::Blue,
        "magenta" => Color::Magenta,
        "cyan" => Color::Cyan,
        "white" => Color::White,
        _ => Color::Reset,
    }
}

/// Render the color picker overlay.  `selected` is 0-indexed into ENTRIES.
pub fn render(frame: &mut Frame, area: Rect, selected: usize, mode: PickerMode) {
    let popup = centered_rect(40, 60, area);
    frame.render_widget(Clear, popup);

    let title = match mode {
        PickerMode::Foreground => "Pick foreground color",
        PickerMode::Background => "Pick background color",
    };

    let block = Block::default().title(title).borders(Borders::ALL);
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let split = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner);

    let hint = ratatui::widgets::Paragraph::new("\u{2191}/\u{2193} move  Enter select  Esc cancel");
    frame.render_widget(hint, split[0]);

    let items: Vec<ListItem> = ENTRIES
        .iter()
        .map(|&name| {
            if name == "(none)" {
                ListItem::new(name)
            } else {
                ListItem::new(name).style(Style::default().fg(name_to_ratatui(name)))
            }
        })
        .collect();

    let mut state = ListState::default();
    state.select(Some(selected.min(ENTRY_COUNT - 1)));

    let list = List::new(items).highlight_style(Style::default().add_modifier(Modifier::REVERSED));
    frame.render_stateful_widget(list, split[1], &mut state);
}

/// Handle a key event for the color picker.
///
/// Up/Down/j/k update `selected` with wraparound.
/// Enter returns Commit or CommitNone.
/// Esc returns Cancel.
/// Other keys return Pending.
pub fn handle(event: KeyEvent, selected: &mut usize) -> PickerEvent {
    match event.code {
        KeyCode::Up | KeyCode::Char('k') => {
            *selected = if *selected == 0 {
                ENTRY_COUNT - 1
            } else {
                *selected - 1
            };
            PickerEvent::Pending
        }
        KeyCode::Down | KeyCode::Char('j') => {
            *selected = (*selected + 1) % ENTRY_COUNT;
            PickerEvent::Pending
        }
        KeyCode::Enter => {
            let name = ENTRIES[(*selected).min(ENTRY_COUNT - 1)];
            if name == "(none)" {
                PickerEvent::CommitNone
            } else {
                let color = name.parse::<NamedColor>().unwrap_or(NamedColor::Default);
                PickerEvent::Commit(color)
            }
        }
        KeyCode::Esc => PickerEvent::Cancel,
        _ => PickerEvent::Pending,
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
