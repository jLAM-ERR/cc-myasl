use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::config::schema::{MAX_PADDING, Segment};

use super::app::{App, EditorField, Mode};

/// Handle a key event while in `Mode::EditingTemplate`.
pub fn handle_editing_template(app: &mut App, event: KeyEvent) {
    if event.code == KeyCode::Char('p') && event.modifiers == KeyModifiers::CONTROL {
        app.picker_filter = String::new();
        app.picker_selected = 0;
        app.mode = Mode::PickingPlaceholder;
        return;
    }

    match event.code {
        KeyCode::Char(c) => {
            if let Some(buf) = app.editing_buf.as_mut() {
                buf.push(c);
            }
        }
        KeyCode::Backspace => {
            if let Some(buf) = app.editing_buf.as_mut() {
                buf.pop();
            }
        }
        KeyCode::Enter => {
            if let (Some(si), Some(buf)) = (app.selected_segment, app.editing_buf.take()) {
                if let Some(line) = app.config.lines.get_mut(app.selected_line) {
                    if let Some(Segment::Template(t)) = line.segments.get_mut(si) {
                        t.template = buf;
                        app.dirty = true;
                    }
                }
            }
            app.editing_buf = None;
            app.mode = Mode::Browsing;
        }
        KeyCode::Esc => {
            app.editing_buf = None;
            app.mode = Mode::Browsing;
        }
        _ => {}
    }
}

pub fn editor_padding_increment(app: &mut App) {
    if app.editor_field != EditorField::Padding {
        return;
    }
    if let Some(si) = app.selected_segment {
        if let Some(line) = app.config.lines.get_mut(app.selected_line) {
            if let Some(Segment::Template(t)) = line.segments.get_mut(si) {
                if t.padding < MAX_PADDING {
                    t.padding += 1;
                    app.dirty = true;
                }
            }
        }
    }
}

pub fn editor_padding_decrement(app: &mut App) {
    if app.editor_field != EditorField::Padding {
        return;
    }
    if let Some(si) = app.selected_segment {
        if let Some(line) = app.config.lines.get_mut(app.selected_line) {
            if let Some(Segment::Template(t)) = line.segments.get_mut(si) {
                if t.padding > 0 {
                    t.padding -= 1;
                    app.dirty = true;
                }
            }
        }
    }
}

pub fn editor_toggle_hide(app: &mut App) {
    if let Some(si) = app.selected_segment {
        if let Some(line) = app.config.lines.get_mut(app.selected_line) {
            if let Some(Segment::Template(t)) = line.segments.get_mut(si) {
                t.hide_when_absent = !t.hide_when_absent;
                app.dirty = true;
            }
        }
    }
}
