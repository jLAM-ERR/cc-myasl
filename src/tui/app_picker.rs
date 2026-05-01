use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::format::catalog::filtered_placeholders;

use super::app::{App, Mode};

#[cfg(test)]
#[path = "picker_tests.rs"]
mod tests;

/// Handle a key event while in `Mode::PickingPlaceholder`.
pub fn handle_picking_placeholder(app: &mut App, event: KeyEvent) {
    match (event.code, event.modifiers) {
        (KeyCode::Esc, _) => {
            app.mode = Mode::EditingTemplate;
        }
        (KeyCode::Enter, _) => {
            let matches = filtered_placeholders(&app.picker_filter);
            if !matches.is_empty() {
                let idx = app.picker_selected.min(matches.len() - 1);
                let name = matches[idx].0;
                let insertion = format!("{{{name}}}");
                if let Some(buf) = app.editing_buf.as_mut() {
                    buf.push_str(&insertion);
                }
            }
            app.mode = Mode::EditingTemplate;
        }
        (KeyCode::Up, _) | (KeyCode::Char('k'), KeyModifiers::NONE) => {
            if app.picker_selected > 0 {
                app.picker_selected -= 1;
            }
        }
        (KeyCode::Down, _) | (KeyCode::Char('j'), KeyModifiers::NONE) => {
            let matches = filtered_placeholders(&app.picker_filter);
            let max = matches.len().saturating_sub(1);
            if app.picker_selected < max {
                app.picker_selected += 1;
            }
        }
        (KeyCode::Backspace, _) => {
            app.picker_filter.pop();
            let matches = filtered_placeholders(&app.picker_filter);
            let max = matches.len().saturating_sub(1);
            if app.picker_selected > max {
                app.picker_selected = max;
            }
        }
        (KeyCode::Char(c), KeyModifiers::NONE) | (KeyCode::Char(c), KeyModifiers::SHIFT) => {
            app.picker_filter.push(c);
            app.picker_selected = 0;
        }
        _ => {}
    }
}
