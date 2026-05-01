use crossterm::event::{KeyCode, KeyEvent};

use crate::config::schema::{NAMED_COLORS, Segment};
use crate::tui::widgets::color_picker::COLOR_PICKER_ENTRIES;

use super::app::{App, Mode};

#[cfg(test)]
#[path = "color_picker_tests.rs"]
mod tests;

/// Handle a key event while in `Mode::PickingFgColor` or `Mode::PickingBgColor`.
pub fn handle_picking_color(app: &mut App, event: KeyEvent) {
    let picking_fg = app.mode == Mode::PickingFgColor;

    match event.code {
        KeyCode::Esc => {
            app.mode = Mode::Browsing;
        }
        KeyCode::Enter => {
            let sel = app.color_picker_selected.min(COLOR_PICKER_ENTRIES - 1);
            // Index 0..NAMED_COLORS.len()-1 maps to a named color; last entry = "(none)".
            let chosen: Option<String> = if sel < NAMED_COLORS.len() {
                Some(NAMED_COLORS[sel].to_owned())
            } else {
                None
            };
            if let Some(si) = app.selected_segment {
                if let Some(line) = app.config.lines.get_mut(app.selected_line) {
                    if let Some(Segment::Template(t)) = line.segments.get_mut(si) {
                        if picking_fg {
                            t.color = chosen;
                        } else {
                            t.bg = chosen;
                        }
                        app.dirty = true;
                    }
                }
            }
            app.mode = Mode::Browsing;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.color_picker_selected > 0 {
                app.color_picker_selected -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if app.color_picker_selected + 1 < COLOR_PICKER_ENTRIES {
                app.color_picker_selected += 1;
            }
        }
        _ => {}
    }
}
