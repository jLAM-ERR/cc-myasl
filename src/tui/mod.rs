pub mod ansi;
pub mod app;
pub mod app4;
pub mod app4_handle;
pub mod app_color;
pub mod app_editor;
pub mod app_picker;
pub mod app_save;
pub mod builder;
pub mod catalog;
pub mod draw;
pub mod draw4;
pub mod overlays;
pub mod panes;
pub mod preview;
pub mod save;
pub mod widgets;

use std::io;
use std::io::IsTerminal;
use std::path::PathBuf;

use crossterm::{
    event::{self, Event},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};

use crate::config::schema::Config;
use crate::error::Error;

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

/// Phase 3 entry point — kept intact for `--configure` while Phase 4 develops.
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

/// Phase 4 entry point — 3-pane builder TUI.
///
/// Exposed for integration testing via `run4_with_app` below.  Not yet wired
/// to `--configure` in main.rs — Task 12 swaps the call site once Phase 3
/// is removed.  `--configure` continues to call `tui::run` (Phase 3) during
/// Phase 4 development.
pub fn run4(config: Config, output_path: PathBuf) -> Result<(), Error> {
    if !io::stdout().is_terminal() {
        return Err(Error::NotATty);
    }
    let app = app4::App::new(config, output_path);
    run4_with_app(app)
}

/// Headless-testable Phase 4 event loop. Callers may construct any `App` state.
pub fn run4_with_app(mut app: app4::App) -> Result<(), Error> {
    use crossterm::event::{DisableBracketedPaste, EnableBracketedPaste, KeyEventKind};
    use crossterm::terminal::{EnterAlternateScreen as Enter, LeaveAlternateScreen as Leave};

    crossterm::terminal::enable_raw_mode().map_err(Error::Io)?;
    let mut stdout = io::stdout();
    crossterm::execute!(stdout, Enter, EnableBracketedPaste).map_err(Error::Io)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend).map_err(Error::Io)?;

    let result = (|| -> Result<(), Error> {
        loop {
            terminal.draw(|f| draw4::draw(f, &app)).map_err(Error::Io)?;
            if app.should_quit {
                break;
            }
            if let crossterm::event::Event::Key(key) =
                crossterm::event::read().map_err(Error::Io)?
            {
                if key.kind == KeyEventKind::Press {
                    app.handle(key);
                }
            }
            // Saving is synchronous — handle here so the loop sees updated state.
            if app.mode == app4::Mode::Saving {
                process_save_if_needed(&mut app);
            }
        }
        Ok(())
    })();

    crossterm::execute!(terminal.backend_mut(), DisableBracketedPaste, Leave,)
        .map_err(Error::Io)?;
    crossterm::terminal::disable_raw_mode().map_err(Error::Io)?;

    result
}

/// Execute the save block once.  Sets status, clears dirty on success only.
/// Called from `run4_with_app` and exposed for integration tests.
pub(crate) fn process_save_if_needed(app: &mut app4::App) {
    app.set_status_ok("saving\u{2026}".into());
    let cfg = builder::to_config(&app.builder);
    match overlays::save::save(&app.output_path, &cfg) {
        Ok(p) => {
            app.set_status_ok(format!("saved \u{2192} {}", p.display()));
            app.dirty = false;
        }
        Err(e) => {
            app.set_status_err(format!("save failed: {e}"));
            // dirty stays true so user can retry
        }
    }
    app.mode = app4::Mode::Browsing;
}

#[cfg(test)]
#[path = "integration_tests.rs"]
mod integration;

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
