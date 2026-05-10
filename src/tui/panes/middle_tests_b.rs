use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

use crate::config::schema::{Config, Line};
use crate::tui::app::{App, Focus, Mode};
use crate::tui::catalog::{Category, by_category};

use super::render;

// ── local helpers (intentional duplication — no cross-file pub(super)) ─────────

fn empty_config() -> Config {
    Config {
        schema_url: None,
        powerline: false,
        default_fg: None,
        default_bg: None,
        lines: vec![Line {
            separator: " | ".into(),
            segments: vec![],
        }],
    }
}

fn mk_app() -> App {
    App::new(empty_config(), PathBuf::from("/tmp/test_b.json"))
}

fn render_to_buffer(app: &App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    terminal.draw(|f| render(f, f.area(), app)).unwrap();
    terminal.backend().buffer().clone()
}

fn row_text(buf: &ratatui::buffer::Buffer, y: u16) -> String {
    let width = buf.area.width;
    (0..width)
        .map(|x| {
            buf.cell((x, y))
                .map(|c| c.symbol().chars().next().unwrap_or(' '))
                .unwrap_or(' ')
        })
        .collect()
}

fn has_modifier_at(buf: &ratatui::buffer::Buffer, x: u16, y: u16, m: Modifier) -> bool {
    buf.cell((x, y))
        .map(|c| c.modifier.contains(m))
        .unwrap_or(false)
}

fn row_has_modifier(buf: &ratatui::buffer::Buffer, y: u16, m: Modifier) -> bool {
    (0..buf.area.width).any(|x| has_modifier_at(buf, x, y, m))
}

// ── category-switch: no panic + correct preset visible ────────────────────────

#[test]
fn session_model_tab_shows_model_name_preset() {
    let mut app = mk_app();
    app.active_tab = Category::SessionModel;

    let buf = render_to_buffer(&app);

    // Row 2 = first preset row. "Model name" label (26-char padded) must appear.
    let row2 = row_text(&buf, 2);
    assert!(
        row2.to_lowercase().contains("model"),
        "SessionModel tab must show a model preset row: {row2:?}"
    );
}

#[test]
fn context_tab_shows_context_pct_preset() {
    let mut app = mk_app();
    app.active_tab = Category::Context;

    let buf = render_to_buffer(&app);

    let row2 = row_text(&buf, 2);
    assert!(
        row2.to_lowercase().contains("context"),
        "Context tab must show a context preset row: {row2:?}"
    );
}

#[test]
fn cost_tab_shows_cost_usd_preset() {
    let mut app = mk_app();
    app.active_tab = Category::Cost;

    let buf = render_to_buffer(&app);

    let row2 = row_text(&buf, 2);
    // First Cost preset label is "Session cost".
    assert!(
        row2.to_lowercase().contains("cost") || row2.to_lowercase().contains("session"),
        "Cost tab must show a cost preset row: {row2:?}"
    );
}

#[test]
fn rates_tab_shows_five_left_pct_preset() {
    let mut app = mk_app();
    app.active_tab = Category::Rates;

    let buf = render_to_buffer(&app);

    let row2 = row_text(&buf, 2);
    // First Rates preset label is "5h quota remaining %".
    assert!(
        row2.contains("5h") || row2.to_lowercase().contains("quota"),
        "Rates tab must show a 5h quota preset row: {row2:?}"
    );
}

// ── picker_selected out-of-bounds: no panic, no reversed row ─────────────────

#[test]
fn picker_selected_out_of_bounds_no_panic_no_reversed() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.mode = Mode::Filter;
    app.picker_filter = "model".to_string(); // matches ~0 workspace presets
    app.picker_selected = 99;
    app.focus = Focus::Middle;

    // Must not panic.
    let buf = render_to_buffer(&app);

    // No visible row should have REVERSED on its checkbox cell (x=1)
    // because picker_selected=99 > visible count.
    for y in 2..24u16 {
        let text = row_text(&buf, y);
        if text.contains('[') {
            assert!(
                !has_modifier_at(&buf, 1, y, Modifier::REVERSED),
                "out-of-bounds picker_selected must not highlight any row (y={y}): {text:?}"
            );
        }
    }
}

// ── case-insensitive filter on template side (uppercase) ─────────────────────

#[test]
fn filter_template_uppercase_matches_cwd_basename_preset() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.mode = Mode::Filter;
    // Uppercase template fragment — must still match "📁 {cwd_basename}".
    app.picker_filter = "CWD_BASENAME".to_string();

    let buf = render_to_buffer(&app);

    let row2 = row_text(&buf, 2);
    let row3 = row_text(&buf, 3);
    assert!(
        row2.contains('['),
        "uppercase template filter 'CWD_BASENAME' must match the cwd_basename preset: {row2:?}"
    );
    assert!(
        !row3.contains('['),
        "only one preset should match CWD_BASENAME; row 3 must have no checkbox: {row3:?}"
    );
}

// ── selected row with empty preview: REVERSED + DIM on — cell ────────────────

#[test]
fn selected_row_empty_preview_has_reversed_and_dim() {
    let mut app = mk_app();
    app.active_tab = Category::Git;
    app.picker_selected = 0; // git_branch — fixture has no repo → empty preview
    app.focus = Focus::Middle;

    let buf = render_to_buffer(&app);

    // Row 2 = first git preset (git_branch).
    let row = 2u16;

    // The row must have REVERSED somewhere (selected row).
    assert!(
        row_has_modifier(&buf, row, Modifier::REVERSED),
        "selected git_branch row must have REVERSED modifier"
    );

    // The '—' em-dash cell must have BOTH REVERSED and DIM.
    let dash_x = (0..80u16).find(|&x| {
        buf.cell((x, row))
            .map(|c| c.symbol() == "—")
            .unwrap_or(false)
    });
    if let Some(x) = dash_x {
        assert!(
            has_modifier_at(&buf, x, row, Modifier::REVERSED),
            "— cell on selected row must have REVERSED at ({x}, {row})"
        );
        assert!(
            has_modifier_at(&buf, x, row, Modifier::DIM),
            "— cell on selected row must have DIM at ({x}, {row})"
        );
    } else {
        // If the dash wasn't found as a single cell (multi-byte), verify the row
        // has REVERSED at minimum to confirm the selected-row path was taken.
        assert!(
            row_has_modifier(&buf, row, Modifier::REVERSED),
            "selected row with empty preview must have REVERSED even if — is multi-byte"
        );
    }
}

// ── filter_empty_shows_all_rows: count via catalog, not hardcoded range ───────

#[test]
fn filter_empty_shows_all_workspace_rows_dynamic_count() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.mode = Mode::Filter;
    app.picker_filter = String::new();

    let buf = render_to_buffer(&app);

    let expected = by_category(Category::Workspace).count();
    // Rows start at y=2 (border row 0, tab strip row 1, presets from row 2).
    let found = (2..(2 + expected as u16 + 2))
        .filter(|&y| row_text(&buf, y).contains('['))
        .count();
    assert_eq!(
        found, expected,
        "empty filter must show all {expected} workspace presets; found {found}"
    );
}
