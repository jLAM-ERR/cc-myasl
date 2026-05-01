use ratatui::{
    Frame,
    layout::Rect,
    style::{Modifier, Style},
    text::{Line as TuiLine, Span},
    widgets::Paragraph,
};

use crate::tui::app::{App, Mode};

/// Render a single-row status bar into `area`.
/// `now_unix` is the current unix timestamp (seconds); passed in so tests can inject it.
pub fn render_status_bar(frame: &mut Frame, area: Rect, app: &App, now_unix: u64) {
    let mut parts: Vec<Span> = Vec::new();

    // Mode label.
    let mode_label = match app.mode {
        Mode::Browsing => "[Browsing]",
        Mode::EditingTemplate => "[EditingTemplate]",
        Mode::PickingPlaceholder => "[PickingPlaceholder]",
        Mode::PickingFgColor => "[PickingFgColor]",
        Mode::PickingBgColor => "[PickingBgColor]",
        Mode::Saving => "[Saving]",
        Mode::Help => "[Help]",
        Mode::ConfirmQuit => "[ConfirmQuit]",
    };
    parts.push(Span::styled(
        mode_label,
        Style::default().add_modifier(Modifier::BOLD),
    ));

    // Dirty indicator.
    if app.dirty {
        parts.push(Span::raw(" ●"));
    }

    // Transient status message (shown for up to 3 seconds).
    if let Some((ref msg, ts)) = app.status_message {
        if now_unix.saturating_sub(ts) < 3 {
            parts.push(Span::raw("  "));
            parts.push(Span::raw(msg.clone()));
        }
    }

    // Validation error (in Saving mode): show first error abbreviated.
    if app.mode == Mode::Saving {
        if let Some(err) = app.last_save_errors.first() {
            let brief = format!("  Error: {:?}", err);
            let brief = if brief.len() > 60 {
                format!("{}…", &brief[..59])
            } else {
                brief
            };
            parts.push(Span::raw(brief));
        }
    }

    // Help hint (Browsing mode only).
    if app.mode == Mode::Browsing {
        parts.push(Span::raw("  ? for help"));
    }

    let line = TuiLine::from(parts);
    let para = Paragraph::new(line);
    frame.render_widget(para, area);
}

/// Pure-logic helper: returns true if the status message should be visible.
pub fn status_message_visible(ts: u64, now_unix: u64) -> bool {
    now_unix.saturating_sub(ts) < 3
}

#[cfg(test)]
mod tests {
    use super::status_message_visible;

    #[test]
    fn status_message_visible_within_3_seconds() {
        // message set at t=100, now=102 → 2 s elapsed → visible
        assert!(status_message_visible(100, 102));
    }

    #[test]
    fn status_message_visible_at_boundary() {
        // exactly 2 s elapsed
        assert!(status_message_visible(100, 102));
    }

    #[test]
    fn status_message_hidden_after_3_seconds() {
        // message set at t=100, now=104 → 4 s elapsed → hidden
        assert!(!status_message_visible(100, 104));
    }

    #[test]
    fn status_message_hidden_exactly_at_3_seconds() {
        // exactly 3 s elapsed → NOT visible (< 3 is false)
        assert!(!status_message_visible(100, 103));
    }

    #[test]
    fn status_message_visible_at_same_second() {
        // ts == now → 0 elapsed → visible
        assert!(status_message_visible(100, 100));
    }
}
