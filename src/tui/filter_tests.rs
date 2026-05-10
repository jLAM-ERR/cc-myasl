use std::path::PathBuf;

use ratatui::backend::TestBackend;

use crate::config::schema::{Config, Line};
use crate::tui::catalog::Category;

use super::{App, Mode};

// ── helpers ───────────────────────────────────────────────────────────────────

fn fresh_app() -> App {
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
        PathBuf::from("/tmp/test_filter.json"),
    )
}

fn render_bottom(app: &App) -> String {
    use crate::tui::panes::bottom;
    let backend = TestBackend::new(80, 6);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| bottom::render(f, f.area(), app)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let (w, h) = (buf.area.width, buf.area.height);
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            if let Some(c) = buf.cell((x, y)) {
                out.push(c.symbol().chars().next().unwrap_or(' '));
            }
        }
    }
    out
}

fn render_middle(app: &App) -> String {
    use crate::tui::panes::middle;
    let backend = TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| middle::render(f, f.area(), app)).unwrap();
    let buf = terminal.backend().buffer().clone();
    let (w, h) = (buf.area.width, buf.area.height);
    let mut out = String::new();
    for y in 0..h {
        for x in 0..w {
            if let Some(c) = buf.cell((x, y)) {
                out.push(c.symbol().chars().next().unwrap_or(' '));
            }
        }
    }
    out
}

// ── open_filter ───────────────────────────────────────────────────────────────

#[test]
fn open_filter_sets_mode_and_clears() {
    let mut app = fresh_app();
    app.picker_filter = "old".into();
    app.open_filter();
    assert_eq!(app.mode, Mode::Filter);
    assert_eq!(app.picker_filter, "");
}

#[test]
fn open_filter_resets_picker_selected() {
    let mut app = fresh_app();
    app.picker_selected = 5;
    app.open_filter();
    assert_eq!(app.picker_selected, 0);
}

#[test]
fn open_filter_when_already_in_filter_mode_clears_and_stays() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    // Press / again while in Filter mode.
    app.open_filter();
    assert_eq!(app.mode, Mode::Filter);
    assert_eq!(app.picker_filter, "");
}

// ── typing ────────────────────────────────────────────────────────────────────

#[test]
fn typing_appends_to_filter() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    app.filter_type('o');
    assert_eq!(app.picker_filter, "mo");
    assert_eq!(app.mode, Mode::Filter);
}

#[test]
fn typing_resets_picker_selected_each_time() {
    let mut app = fresh_app();
    app.open_filter();
    app.picker_selected = 3;
    app.filter_type('x');
    assert_eq!(app.picker_selected, 0);
}

// ── backspace ─────────────────────────────────────────────────────────────────

#[test]
fn backspace_deletes_last_char() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    app.filter_type('o');
    app.filter_backspace();
    assert_eq!(app.picker_filter, "m");
}

#[test]
fn backspace_on_empty_filter_no_panic() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_backspace();
    assert_eq!(app.picker_filter, "");
}

#[test]
fn backspace_resets_picker_selected() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    app.picker_selected = 2;
    app.filter_backspace();
    assert_eq!(app.picker_selected, 0);
}

// ── cancel_filter (Esc) ───────────────────────────────────────────────────────

#[test]
fn esc_clears_filter_and_returns_browsing() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    app.filter_type('o');
    app.cancel_filter();
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.picker_filter, "");
}

// ── commit_filter (Enter) ─────────────────────────────────────────────────────

#[test]
fn enter_commits_filter_keeps_it_active() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    app.commit_filter();
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.picker_filter, "m"); // retained
}

// ── slash with active committed filter ───────────────────────────────────────

#[test]
fn slash_with_active_filter_clears_and_reopens() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    app.commit_filter();
    // Caller (draw loop) would call open_filter again on `/`.
    app.open_filter();
    assert_eq!(app.mode, Mode::Filter);
    assert_eq!(app.picker_filter, "");
}

// ── tab cycle clears filter ───────────────────────────────────────────────────

#[test]
fn tab_cycle_next_clears_filter() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    app.commit_filter();
    app.tab_cycle_next();
    assert_eq!(app.picker_filter, "");
    assert_eq!(app.picker_selected, 0);
}

#[test]
fn tab_cycle_prev_clears_filter() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    app.commit_filter();
    app.tab_cycle_prev();
    assert_eq!(app.picker_filter, "");
    assert_eq!(app.picker_selected, 0);
}

#[test]
fn filter_survives_focus_change_but_clears_on_tab_change() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    app.commit_filter();
    // Focus change: filter must survive.
    app.focus_cycle_forward();
    assert_eq!(app.picker_filter, "m", "filter must survive focus change");
    // Tab change: filter must clear.
    app.tab_cycle_next();
    assert_eq!(
        app.picker_filter, "",
        "filter must clear on category change"
    );
}

// ── middle pane narrows rows after committed filter ───────────────────────────

#[test]
fn typing_into_filter_narrows_rows_live() {
    let mut app = fresh_app();
    app.active_tab = Category::Workspace;
    app.focus = crate::tui::app4::Focus::Middle;
    // No filter: all workspace rows visible.
    let unfiltered = render_middle(&app);
    let unfiltered_count = unfiltered.matches("[ ]").count() + unfiltered.matches("[x]").count();

    app.open_filter();
    // Use a filter that matches only one workspace preset by label ("cwd").
    app.filter_type('c');
    app.filter_type('w');
    app.filter_type('d');
    let filtered = render_middle(&app);
    let filtered_count = filtered.matches("[ ]").count() + filtered.matches("[x]").count();

    assert!(
        filtered_count < unfiltered_count,
        "filter 'cwd' must narrow rows: before={unfiltered_count} after={filtered_count}"
    );
    assert!(
        filtered_count >= 1,
        "filter 'cwd' must keep at least one row"
    );
}

#[test]
fn committed_filter_keeps_rows_narrowed() {
    let mut app = fresh_app();
    app.active_tab = Category::Workspace;
    app.focus = crate::tui::app4::Focus::Middle;

    // Unfiltered count.
    let unfiltered = render_middle(&app);
    let unfiltered_count = unfiltered.matches("[ ]").count() + unfiltered.matches("[x]").count();

    // Open filter, type, commit.
    app.open_filter();
    app.filter_type('c');
    app.filter_type('w');
    app.filter_type('d');
    app.commit_filter(); // returns to Browsing; filter kept.

    assert_eq!(app.mode, Mode::Browsing);
    let filtered = render_middle(&app);
    let filtered_count = filtered.matches("[ ]").count() + filtered.matches("[x]").count();
    assert!(
        filtered_count < unfiltered_count,
        "committed filter must keep rows narrowed"
    );
}

// ── bottom pane shows filter hint after commit ────────────────────────────────

#[test]
fn bottom_pane_shows_filter_hint_on_committed_filter() {
    let mut app = fresh_app();
    app.focus = crate::tui::app4::Focus::Middle;
    app.open_filter();
    app.filter_type('m');
    app.filter_type('o');
    app.commit_filter();

    let output = render_bottom(&app);
    assert!(
        output.contains("filter: mo"),
        "bottom pane must show 'filter: mo' when committed filter is active; output={output:?}"
    );
    assert!(
        output.contains("/:clear"),
        "bottom pane must show '/:clear' hint; output={output:?}"
    );
}

#[test]
fn bottom_pane_no_filter_hint_when_filter_is_empty() {
    let mut app = fresh_app();
    app.focus = crate::tui::app4::Focus::Middle;
    // No filter active.
    let output = render_bottom(&app);
    assert!(
        !output.contains("filter:"),
        "bottom pane must not show filter hint when no filter active; output={output:?}"
    );
}

#[test]
fn bottom_pane_no_filter_hint_in_filter_mode() {
    // During active typing (Filter mode) the hint is not shown — just the edit keymap.
    let mut app = fresh_app();
    app.focus = crate::tui::app4::Focus::Middle;
    app.open_filter();
    app.filter_type('m');
    let output = render_bottom(&app);
    // In Filter mode the keymap shows "[edit] type to change  Enter:commit  Esc:cancel".
    assert!(
        output.contains("Esc"),
        "Filter mode must show Esc:cancel; output={output:?}"
    );
    // The committed-filter hint must not appear yet.
    assert!(
        !output.contains("/:clear"),
        "Filter mode must not show /:clear yet; output={output:?}"
    );
}

// ── tab cycle resets mode ─────────────────────────────────────────────────────

#[test]
fn tab_cycle_resets_mode_from_filter_to_browsing() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('m');
    assert_eq!(app.mode, Mode::Filter);
    app.tab_cycle_next();
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.picker_filter, "");
}

#[test]
fn tab_cycle_prev_resets_mode_from_filter_to_browsing() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('x');
    assert_eq!(app.mode, Mode::Filter);
    app.tab_cycle_prev();
    assert_eq!(app.mode, Mode::Browsing);
    assert_eq!(app.picker_filter, "");
}

// ── filter hint hidden on Appearance tab ──────────────────────────────────────

#[test]
fn filter_hint_hidden_on_appearance_tab() {
    let mut app = fresh_app();
    app.active_tab = Category::Appearance;
    app.open_filter();
    app.filter_type('t');
    app.filter_type('e');
    app.commit_filter();
    app.focus = crate::tui::app4::Focus::Middle;
    let output = render_bottom(&app);
    assert!(
        !output.contains("filter:"),
        "filter hint must not appear on Appearance tab; output={output:?}"
    );
}

// ── clear_filter ──────────────────────────────────────────────────────────────

#[test]
fn clear_filter_empties_filter_stays_browsing() {
    let mut app = fresh_app();
    app.open_filter();
    app.filter_type('x');
    app.commit_filter();
    app.clear_filter();
    assert_eq!(app.picker_filter, "");
    assert_eq!(app.picker_selected, 0);
    assert_eq!(app.mode, Mode::Browsing);
}
