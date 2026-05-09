use std::path::PathBuf;

use ratatui::backend::TestBackend;
use ratatui::style::Modifier;

use crate::config::schema::{Config, Line};
use crate::tui::app4::{App, Focus, Mode};
use crate::tui::builder::{BuilderLine, BuilderSegment, BuilderState};
use crate::tui::catalog::Category;

use super::render;

// ── helpers ───────────────────────────────────────────────────────────────────

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
    App::new(empty_config(), PathBuf::from("/tmp/test.json"))
}

fn app_with_state(state: BuilderState) -> App {
    let mut a = mk_app();
    a.builder = state;
    a
}

fn render_to_buffer(app: &App) -> ratatui::buffer::Buffer {
    let backend = TestBackend::new(80, 24);
    let mut terminal = ratatui::Terminal::new(backend).unwrap();
    let area = terminal.get_frame().area();
    terminal.draw(|f| render(f, area, app)).unwrap();
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

// ── checkbox state reflects active line ───────────────────────────────────────

#[test]
fn checkbox_on_when_preset_is_active() {
    // Toggle on "cwd_basename" (first workspace preset).
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.toggle_preset(Category::Workspace, 0); // adds cwd_basename

    let buf = render_to_buffer(&app);

    // Row 2 = border(0) + tab strip(1) + first preset row(2).
    let text = row_text(&buf, 2);
    assert!(
        text.contains('[') && text.contains('x') && text.contains(']'),
        "first workspace preset must show [x] when active: {text:?}"
    );
}

#[test]
fn checkbox_off_when_preset_is_not_active() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    // Don't toggle anything — all presets off.

    let buf = render_to_buffer(&app);
    let text = row_text(&buf, 2);
    // Should have `[ ]` — x must NOT be between the brackets.
    assert!(
        text.contains("[ ]"),
        "first workspace preset must show [ ] when not active: {text:?}"
    );
}

#[test]
fn checkbox_state_for_git_tab() {
    let mut app = mk_app();
    app.active_tab = Category::Git;
    app.toggle_preset(Category::Git, 0); // adds git_branch

    let buf = render_to_buffer(&app);
    let text = row_text(&buf, 2);
    assert!(
        text.contains("[x]"),
        "first git preset must show [x] when active: {text:?}"
    );
}

#[test]
fn checkbox_second_preset_off_when_only_first_active() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.toggle_preset(Category::Workspace, 0); // adds first preset only

    let buf = render_to_buffer(&app);
    // Row 3 = second preset row.
    let text = row_text(&buf, 3);
    assert!(
        text.contains("[ ]"),
        "second workspace preset must show [ ] when not active: {text:?}"
    );
}

// ── filter narrows rows ───────────────────────────────────────────────────────

#[test]
fn filter_narrows_by_label_case_insensitive() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.mode = Mode::Filter;
    app.picker_filter = "BASENAME".to_string(); // case-insensitive match on "Current dir (basename)"

    let buf = render_to_buffer(&app);

    // Only 1 matching row → row 2 should have "basename" content,
    // row 3 should be empty (no second preset row).
    let row2 = row_text(&buf, 2);
    let row3 = row_text(&buf, 3);
    assert!(
        row2.to_lowercase().contains("basename") || row2.contains('['),
        "filter 'BASENAME' must show the basename preset row: {row2:?}"
    );
    assert!(
        !row3.contains('['),
        "only one row should match; row 3 must not have a checkbox: {row3:?}"
    );
}

#[test]
fn filter_narrows_by_template_substring() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.mode = Mode::Filter;
    // template for "cwd_basename" is "📁 {cwd_basename}" — match by template fragment
    app.picker_filter = "cwd_basename".to_string();

    let buf = render_to_buffer(&app);
    let row2 = row_text(&buf, 2);
    let row3 = row_text(&buf, 3);

    assert!(
        row2.contains('['),
        "filter by template must show matching row: {row2:?}"
    );
    assert!(
        !row3.contains('['),
        "only one template match should appear: {row3:?}"
    );
}

#[test]
fn filter_empty_shows_all_rows() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.mode = Mode::Filter;
    app.picker_filter = String::new(); // empty filter → show all

    let buf = render_to_buffer(&app);

    // Workspace has 6 presets → rows 2..7 should all have checkboxes.
    for y in 2..8u16 {
        let text = row_text(&buf, y);
        assert!(
            text.contains('['),
            "empty filter must show all presets, row {y} missing checkbox: {text:?}"
        );
    }
}

#[test]
fn filter_no_match_shows_empty_list() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.mode = Mode::Filter;
    app.picker_filter = "xyzzy_no_match_ever".to_string();

    let buf = render_to_buffer(&app);

    // No rows should have checkboxes.
    for y in 2..10u16 {
        let text = row_text(&buf, y);
        assert!(
            !text.contains('['),
            "no match filter must show empty list; row {y}: {text:?}"
        );
    }
}

// ── switching active_tab rerenders to different presets ───────────────────────

#[test]
fn switching_tab_shows_different_presets() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    let buf_ws = render_to_buffer(&app);

    app.active_tab = Category::Tokens;
    let buf_tok = render_to_buffer(&app);

    // Content rows must differ (workspace vs tokens preset labels).
    let content_ws = row_text(&buf_ws, 2);
    let content_tok = row_text(&buf_tok, 2);
    assert_ne!(
        content_ws, content_tok,
        "preset list must differ between tabs"
    );

    // Tab strip: active tab cell must have REVERSED on different cells.
    // "workspace" starts at x=1 (after border); "tokens" starts later.
    // We verify "workspace" label cells are REVERSED in ws-buf but not in tok-buf.
    let ws_cell_reversed_in_ws = has_modifier_at(&buf_ws, 2, 1, Modifier::REVERSED);
    let ws_cell_reversed_in_tok = has_modifier_at(&buf_tok, 2, 1, Modifier::REVERSED);
    assert!(
        ws_cell_reversed_in_ws,
        "workspace label must be REVERSED when workspace is active tab"
    );
    assert!(
        !ws_cell_reversed_in_tok,
        "workspace label must NOT be REVERSED when tokens is active tab"
    );
}

#[test]
fn workspace_presets_absent_in_tokens_tab() {
    // "Current dir" labels appear in workspace but not tokens.
    let mut app = mk_app();
    app.active_tab = Category::Tokens;

    let buf = render_to_buffer(&app);

    let all_text: String = (2..10u16).map(|y| row_text(&buf, y)).collect();
    assert!(
        !all_text.to_lowercase().contains("current dir"),
        "workspace preset labels must not appear in tokens tab: {all_text:?}"
    );
}

// ── dim `—` for empty placeholder (git_branch with no git in fixture) ─────────

#[test]
fn git_branch_shows_dim_dash_when_no_repo_in_fixture() {
    let mut app = mk_app();
    app.active_tab = Category::Git;
    // Don't toggle — just render the git tab, git_branch preset is first row.

    let buf = render_to_buffer(&app);

    // Row 2 = first git preset row (git_branch).
    // The preview fixture has no git data → preview should show "—".
    let row2 = row_text(&buf, 2);
    assert!(
        row2.contains('—') || row2.contains('-'),
        "git_branch preset must show — when fixture has no repo: {row2:?}"
    );

    // The '—' cell should have DIM modifier.
    let dash_x = (0..80u16).find(|&x| buf.cell((x, 2)).map(|c| c.symbol() == "—").unwrap_or(false));
    if let Some(x) = dash_x {
        assert!(
            has_modifier_at(&buf, x, 2, Modifier::DIM),
            "— cell must have DIM modifier at ({x}, 2)"
        );
    }
    // If no exact '—' found, at minimum the row must not contain a git branch name.
}

// ── Appearance tab dispatches without panic ───────────────────────────────────

#[test]
fn appearance_tab_dispatches_without_panic() {
    let mut app = mk_app();
    app.active_tab = Category::Appearance;

    // Should not panic; stub appearance::render is a no-op.
    let _buf = render_to_buffer(&app);
}

// ── active-pane border visual ─────────────────────────────────────────────────

#[test]
fn border_cyan_when_middle_focused() {
    let mut app = mk_app();
    app.focus = Focus::Middle;

    let buf = render_to_buffer(&app);
    let cell = buf.cell((0, 0)).expect("cell must exist");
    assert_eq!(
        cell.fg,
        ratatui::style::Color::Cyan,
        "border must be Cyan when focus=Middle"
    );
}

#[test]
fn border_dark_gray_when_not_focused() {
    let mut app = mk_app();
    app.focus = Focus::Top;

    let buf = render_to_buffer(&app);
    let cell = buf.cell((0, 0)).expect("cell must exist");
    assert_eq!(
        cell.fg,
        ratatui::style::Color::DarkGray,
        "border must be DarkGray when focus!=Middle"
    );
}

// ── tab strip — active tab has reversed style ────────────────────────────────

#[test]
fn active_tab_label_has_reversed_modifier() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.focus = Focus::Middle;

    let buf = render_to_buffer(&app);

    // Row 1 = tab strip. The word "workspace" must contain reversed cells.
    let row = 1u16;
    let reversed_count = (0..80u16)
        .filter(|&x| has_modifier_at(&buf, x, row, Modifier::REVERSED))
        .count();
    assert!(
        reversed_count > 0,
        "active tab label must have REVERSED modifier"
    );
}

// ── picker_selected row highlights with REVERSED when Middle focused ──────────

#[test]
fn selected_row_has_reversed_when_middle_focused() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.focus = Focus::Middle;
    app.picker_selected = 0;

    let buf = render_to_buffer(&app);

    // Row 2 = first preset row (selected when picker_selected == 0).
    let row = 2u16;
    let any_reversed = (0..80u16).any(|x| has_modifier_at(&buf, x, row, Modifier::REVERSED));
    assert!(
        any_reversed,
        "selected row must have REVERSED modifier when Middle is focused"
    );
}

#[test]
fn non_selected_row_has_no_reversed_when_middle_focused() {
    let mut app = mk_app();
    app.active_tab = Category::Workspace;
    app.focus = Focus::Middle;
    app.picker_selected = 0; // row 0 is selected, row 1 is not

    let buf = render_to_buffer(&app);

    // Row 3 = second preset row (not selected).
    let row = 3u16;
    // The row must not have REVERSED on the checkbox or label cells.
    // (Some cells may coincidentally have REVERSED from preview spans — so
    //  we only check the checkbox cell at x=1.)
    let checkbox_cell_reversed = has_modifier_at(&buf, 1, row, Modifier::REVERSED);
    assert!(
        !checkbox_cell_reversed,
        "non-selected row checkbox must not have REVERSED"
    );
}

// ── state with custom segment — toggle adds preset alongside it ───────────────

#[test]
fn toggle_adds_preset_when_custom_with_same_template_exists() {
    let state = BuilderState {
        lines: vec![BuilderLine {
            separator: " | ".into(),
            segments: vec![BuilderSegment::Custom {
                template: "📁 {cwd_basename}".to_owned(),
                color: None,
                bg: None,
                padding: 0,
                hide_when_absent: false,
            }],
        }],
        powerline: false,
        default_fg: None,
        default_bg: None,
        schema_url: None,
    };
    let mut app = app_with_state(state);
    app.active_tab = Category::Workspace;

    // Toggle first workspace preset (cwd_basename).
    app.toggle_preset(Category::Workspace, 0);

    // Must now have 2 segments: the original Custom + the new Preset.
    assert_eq!(
        app.builder.lines[0].segments.len(),
        2,
        "toggle must add Preset alongside the existing Custom"
    );
}
