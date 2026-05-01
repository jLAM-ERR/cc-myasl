use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::app::{App, Mode};
use super::save::{self, SaveError};

/// Handle a key event while in `Mode::ConfirmQuit`.
pub fn handle_confirm_quit(app: &mut App, event: KeyEvent) {
    match event.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.should_quit = true;
        }
        KeyCode::Char('n') | KeyCode::Char('N') => {
            app.mode = Mode::Browsing;
        }
        KeyCode::Char('s') | KeyCode::Char('S') => {
            let path = app.output_path.clone();
            match save::save(&app.config, &path) {
                Ok(()) => {
                    app.dirty = false;
                    app.should_quit = true;
                }
                Err(SaveError::Validation(errs)) => {
                    app.last_save_errors = errs;
                    app.mode = Mode::Saving;
                }
                Err(e) => {
                    app.status_message = Some((e.to_string(), now_unix()));
                    app.mode = Mode::Browsing;
                }
            }
        }
        _ => {}
    }
}

/// Handle `Ctrl+S` pressed in `Mode::Browsing`.
pub fn handle_ctrl_s(app: &mut App) {
    let path = app.output_path.clone();
    match save::save(&app.config, &path) {
        Ok(()) => {
            app.dirty = false;
            let msg = format!("Saved to {}", path.display());
            app.status_message = Some((msg, now_unix()));
        }
        Err(SaveError::Validation(errs)) => {
            app.last_save_errors = errs;
            app.mode = Mode::Saving;
        }
        Err(e) => {
            app.status_message = Some((e.to_string(), now_unix()));
        }
    }
}

fn now_unix() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Returns true if the status message is within the 3-second display window.
pub fn status_message_active(ts: u64) -> bool {
    now_unix().saturating_sub(ts) < 3
}

#[cfg(test)]
#[path = "app_save_tests.rs"]
mod tests;

/// Shim so the key modifier constant is available in tests without importing crossterm.
#[allow(dead_code)]
pub(crate) fn ctrl_key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}
