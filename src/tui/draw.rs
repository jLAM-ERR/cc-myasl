use ratatui::{
    Frame,
    layout::{Constraint, Layout},
    text::{Line as TuiLine, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};

use super::app::{App, Mode};
use super::preview::{load_fixture, render_preview};
use super::widgets::color_picker::render_color_picker;
use super::widgets::help::render_help_overlay;
use super::widgets::line_list::render_line_list;
use super::widgets::placeholder_picker::render_placeholder_picker;
use super::widgets::segment_editor::render_segment_editor;
use super::widgets::segment_list::render_segment_list;
use super::widgets::status::render_status_bar;

pub fn draw(frame: &mut Frame, app: &App) {
    let full = frame.area();

    // Reserve the last row for the status bar.
    let outer = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(full);
    let main_area = outer[0];
    let status_area = outer[1];

    let areas = Layout::vertical([
        Constraint::Percentage(60),
        Constraint::Percentage(30),
        Constraint::Percentage(10),
    ])
    .split(main_area);

    // Top: LineList (30%) + SegmentList (70%).
    let top_split = Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)])
        .split(areas[0]);

    render_line_list(frame, top_split[0], app);
    render_segment_list(frame, top_split[1], app);

    render_segment_editor(frame, areas[1], app);

    // Bottom pane: live preview + dirty indicator in title.
    let dirty_indicator = if app.dirty { " ●" } else { "" };
    let title = format!("Preview{dirty_indicator}");

    let fixture = load_fixture();
    let preview_text = render_preview(&app.config, &fixture, 0);

    // Strip ANSI escapes so ratatui renders plain text without garbled bytes.
    let plain = strip_ansi(&preview_text);

    let lines: Vec<TuiLine> = plain
        .split('\n')
        .map(|l| TuiLine::from(Span::raw(l.to_owned())))
        .collect();

    let para = Paragraph::new(lines)
        .block(Block::new().title(title).borders(Borders::ALL))
        .wrap(Wrap { trim: false });

    frame.render_widget(para, areas[2]);

    // Status bar (always rendered).
    let now = crate::time::now_unix();
    render_status_bar(frame, status_area, app, now);

    // Overlay: placeholder picker rendered on top of the base layout.
    if app.mode == Mode::PickingPlaceholder {
        render_placeholder_picker(frame, full, app);
    }

    // Overlay: color picker (fg or bg).
    if app.mode == Mode::PickingFgColor || app.mode == Mode::PickingBgColor {
        render_color_picker(frame, full, app);
    }

    // Overlay: help.
    if app.mode == Mode::Help {
        render_help_overlay(frame, full);
    }
}

/// Remove ANSI CSI escape sequences (e.g. `\x1b[31m`) from a string.
fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            if chars.peek() == Some(&'[') {
                chars.next(); // consume '['
                // consume until a letter (command byte)
                for nc in chars.by_ref() {
                    if nc.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            // other ESC sequences: skip the ESC only
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::strip_ansi;

    #[test]
    fn strip_ansi_removes_color_codes() {
        let s = "\x1b[31mhello\x1b[0m world";
        assert_eq!(strip_ansi(s), "hello world");
    }

    #[test]
    fn strip_ansi_plain_text_unchanged() {
        let s = "no escapes here";
        assert_eq!(strip_ansi(s), s);
    }
}
