//! Appearance settings form — rendered inside the middle pane's list area.
//!
//! Rows (in fixed order):
//!   Powerline mode   [x]/[ ]   on/off
//!   Default fg       [x]/[ ]   color name / none
//!   Default bg       [x]/[ ]   color name / none
//!   Line 1 sep                 " | "     (always shown, ≥ 1 line)
//!   Line 2 sep                 " · "     (only when lines.len() >= 2)
//!   Line 3 sep                 " "       (only when lines.len() >= 3)
//!
//! The selected row uses `Modifier::REVERSED` when `focus == Focus::Middle`.
//! Dispatch (Space / Enter → action) lives in Task 11.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::Paragraph,
};

use crate::tui::app4::{App, Focus};

/// Identifies a row in the appearance settings form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppearanceRow {
    Powerline,
    DefaultFg,
    DefaultBg,
    Separator(usize),
}

/// Returns the ordered list of visible rows for the current app state.
pub fn rows(app: &App) -> Vec<AppearanceRow> {
    let mut out = vec![
        AppearanceRow::Powerline,
        AppearanceRow::DefaultFg,
        AppearanceRow::DefaultBg,
    ];
    for i in 0..app.builder.lines.len() {
        out.push(AppearanceRow::Separator(i));
    }
    out
}

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let visible = rows(app);
    let is_focused = app.focus == Focus::Middle;

    let mut ratatui_lines: Vec<Line<'static>> = Vec::new();

    for (idx, row) in visible.iter().enumerate() {
        let selected = is_focused && app.picker_selected == idx;
        let row_style = if selected {
            Style::default().add_modifier(Modifier::REVERSED)
        } else {
            Style::default()
        };
        let dim_style = if selected {
            row_style
        } else {
            row_style.add_modifier(Modifier::DIM)
        };

        let line = match row {
            AppearanceRow::Powerline => {
                let check = if app.builder.powerline { "[x]" } else { "[ ]" };
                let label = format!("{check} Powerline mode         ");
                let value = if app.builder.powerline { "on" } else { "off" };
                Line::from(vec![
                    Span::styled(label, row_style),
                    Span::styled(value.to_owned(), row_style),
                ])
            }
            AppearanceRow::DefaultFg => {
                let check = if app.builder.default_fg.is_some() {
                    "[x]"
                } else {
                    "[ ]"
                };
                let label = format!("{check} Default foreground     ");
                let (value, val_style) = match app.builder.default_fg {
                    Some(c) => (c.as_str().to_owned(), row_style),
                    None => ("none".to_owned(), dim_style),
                };
                Line::from(vec![
                    Span::styled(label, row_style),
                    Span::styled(value, val_style),
                ])
            }
            AppearanceRow::DefaultBg => {
                let check = if app.builder.default_bg.is_some() {
                    "[x]"
                } else {
                    "[ ]"
                };
                let label = format!("{check} Default background     ");
                let (value, val_style) = match app.builder.default_bg {
                    Some(c) => (c.as_str().to_owned(), row_style),
                    None => ("none".to_owned(), dim_style),
                };
                Line::from(vec![
                    Span::styled(label, row_style),
                    Span::styled(value, val_style),
                ])
            }
            AppearanceRow::Separator(i) => {
                debug_assert!(
                    *i < app.builder.lines.len(),
                    "rows() produced Separator({i}) past lines.len()"
                );
                let sep = app
                    .builder
                    .lines
                    .get(*i)
                    .map(|l| l.separator.as_str())
                    .unwrap_or("");
                let label = format!("    Line {} separator      ", i + 1);
                let value = format!("\"{sep}\"");
                Line::from(vec![
                    Span::styled(label, row_style),
                    Span::styled(value, row_style),
                ])
            }
        };

        ratatui_lines.push(line);
    }

    frame.render_widget(Paragraph::new(ratatui_lines), area);
}

#[cfg(test)]
#[path = "appearance_tests.rs"]
mod tests;
