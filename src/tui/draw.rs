//! Phase 4 draw dispatcher — 3-pane layout with mode overlays.
//!
//! Layout: Top 30% / Middle 55% / Bottom 15%.
//! Overlays rendered on top of the base panes.

use ratatui::{
    Frame,
    layout::{Constraint, Layout},
};

use super::app::{App, Mode};
use super::overlays;
use super::panes::{bottom, middle, top};

pub fn draw(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let panes = Layout::vertical([
        Constraint::Percentage(30),
        Constraint::Percentage(55),
        Constraint::Percentage(15),
    ])
    .split(area);

    top::render(frame, panes[0], app);
    middle::render(frame, panes[1], app);
    bottom::render(frame, panes[2], app);

    // Mode overlays rendered on top.
    match app.mode {
        Mode::PickingFgColor => overlays::color_picker::render(
            frame,
            area,
            app.color_picker_selected,
            overlays::color_picker::PickerMode::Foreground,
        ),
        Mode::PickingBgColor => overlays::color_picker::render(
            frame,
            area,
            app.color_picker_selected,
            overlays::color_picker::PickerMode::Background,
        ),
        Mode::Help => overlays::help::render(frame, area),
        Mode::ConfirmDelete => {
            let n = app
                .builder
                .lines
                .get(app.active_line)
                .map(|l| l.segments.len())
                .unwrap_or(0);
            overlays::confirm::render(
                frame,
                area,
                &overlays::confirm::ConfirmKind::DeleteLine { segments: n },
            );
        }
        Mode::ConfirmQuit => {
            overlays::confirm::render(frame, area, &overlays::confirm::ConfirmKind::QuitDirty);
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use ratatui::backend::TestBackend;

    use crate::config::schema::{Config, Line};
    use crate::tui::app::{App, Mode};

    fn mk_app() -> App {
        App::new(
            Config {
                schema_url: None,
                powerline: false,
                default_fg: None,
                default_bg: None,
                lines: vec![Line {
                    separator: " | ".into(),
                    segments: vec![],
                }],
            },
            PathBuf::from("/tmp/test_draw.json"),
        )
    }

    #[test]
    fn draw_browsing_does_not_panic() {
        let app = mk_app();
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| super::draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_confirm_quit_overlay_does_not_panic() {
        let mut app = mk_app();
        app.mode = Mode::ConfirmQuit;
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| super::draw(f, &app)).unwrap();
    }

    #[test]
    fn draw_help_overlay_does_not_panic() {
        let mut app = mk_app();
        app.mode = Mode::Help;
        let backend = TestBackend::new(80, 24);
        let mut terminal = ratatui::Terminal::new(backend).unwrap();
        terminal.draw(|f| super::draw(f, &app)).unwrap();
    }
}
