pub mod app;
pub mod app_color;
pub mod app_editor;
pub mod app_picker;
pub mod app_save;
pub mod draw;
pub mod preview;
pub mod save;
pub mod widgets;

use std::io;
use std::path::PathBuf;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::config::schema::Config;

use app::App;
use draw::draw;

/// Restores the terminal to its normal state on drop (handles panics too).
struct TerminalGuard;

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
    }
}

pub fn run(config: Config, output_path: PathBuf) -> io::Result<()> {
    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;
    // Guard restores terminal even if the loop panics or returns early.
    let _guard = TerminalGuard;

    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(config, output_path);

    loop {
        terminal.draw(|f| draw(f, &app))?;
        if let Event::Key(key) = event::read()? {
            app.handle(key);
        }
        if app.should_quit {
            break;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    /// Walk `src/tui/` and assert no `.rs` file imports forbidden crates.
    /// Split strings so this source itself never contains the banned patterns.
    #[test]
    fn tui_module_does_not_depend_on_api_cache_or_git() {
        let forbidden: &[&str] = &[
            &["use crate", "::", "api"].concat(),
            &["use crate", "::", "cache"].concat(),
            &["use crate", "::", "git"].concat(),
        ];

        fn walk(dir: &Path, forbidden: &[&str]) {
            let entries = fs::read_dir(dir).expect("read_dir failed");
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    walk(&path, forbidden);
                } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
                    let src = fs::read_to_string(&path).unwrap_or_default();
                    for pat in forbidden {
                        assert!(
                            !src.contains(*pat),
                            "{} has forbidden import: {}",
                            path.display(),
                            pat
                        );
                    }
                }
            }
        }

        walk(Path::new("src/tui"), forbidden);
    }
}
