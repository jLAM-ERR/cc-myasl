# Phase 4 — Builder-style TUI rewrite

## Overview

Replace the Phase 3 4-pane template-text editor (~3046 LOC across 24
files in `src/tui/`) with a 3-pane preset-driven builder.  Users no
longer hand-edit `{placeholder}` strings; instead they tick checkboxes
in a curated catalog of pre-formatted segments, see live ANSI-rendered
output (icons, colors, numbers) in the top pane, and discover all
legal keys in a cursor-aware keymap at the bottom.

This rewrite consolidates the brainstorm session of 2026-05-10:
preset segment as the unit of choice (icons + label + value), top pane
shows live preview with cursor-driven reorder/delete, middle pane is
tab-grouped (workspace, git, session/model, context, tokens, cost,
rates, Appearance), bottom pane rewrites its keymap per (focus, mode,
cursor) tuple.  Custom (hand-edited) templates passthrough — rendered
dim, reorder/delete/recolor still work, only the middle-pane *toggle*
gesture is disabled for them.

Schema gains two new **optional** top-level fields (`default_fg`,
`default_bg` — both `Option<NamedColor>`) to support the Appearance
tab's default-color settings.  The change is additive and
backward-compatible: existing configs without these fields parse
fine (defaults to `None`).  All other fields (`lines`, `powerline`,
`schema_url`) are unchanged.  No schema-version bump.

## Context (from discovery)

**Files involved:**

- Replace entirely: `src/tui/*.rs` and `src/tui/widgets/*.rs` (24 files,
  3046 LOC) — full delete-and-rewrite.
- Reuse with minor adapt: `src/tui/preview_fixture.json` — TUI-only
  stable Payload fixture (must NOT move into production paths).
- Modify: `src/config/schema.rs` — add `default_fg: Option<NamedColor>`
  and `default_bg: Option<NamedColor>` to `Config` with
  `#[serde(default, skip_serializing_if = "Option::is_none")]`
  (additive, backward-compatible).
- Untouched: `src/config/{builtins,render}.rs` — built-in templates
  still load via existing `config::resolve`; render path doesn't
  consume the new defaults yet (Phase 4 reads them only inside the
  TUI; render-time application is a future task).
- Untouched: `src/format/placeholders/mod.rs` — every preset references
  an existing placeholder name; a new invariant (12) enforces this at
  test time.
- Modify: `src/main.rs` `--configure` dispatch — call site is the
  same; the new `tui::run` has the same signature.
- Modify: `scripts/check-invariants.sh` — add invariant 12 grep.
- Modify: `README.md` — rewrite `--configure` section + new
  asciicast/screenshot.
- Modify: `CLAUDE.md` — update module tree + invariant list.

**Not modified in this phase:** `Cargo.toml` — the version bump
and the `v*` tag push are deliberately deferred to a separate
user-gated release-prep commit on `main` after this branch merges.

**Patterns reused from Phase 3:**

- `config::render::render(&Config, &RenderCtx) -> String` powers the
  live preview (already pure, ANSI-bearing).
- Atomic save (tmp + rename + fsync parent) — pattern in
  `tui/save.rs` lifted into the new `overlays/save.rs`.
- Color picker — Phase 3's named ANSI 16 picker (16 colors + reset)
  is lifted with minimal adaptation into `overlays/color_picker.rs`.
- TTY detection (`std::io::IsTerminal`) — `--configure` exits non-zero
  on non-TTY; behavior preserved.

**Dependencies:** ratatui 0.29 + crossterm 0.28 unchanged.  No new
top-level deps.  ANSI-to-Spans parser is hand-rolled (~50 LOC) — no
`ansi-to-tui` crate added.

## Development Approach

- **Testing approach: Regular** (code + tests per task; tests run
  before next task starts).  Matches Phase 1/2/3 convention and
  CLAUDE.md mandate of `unit tests required for every code change`.
- Complete each task fully before moving to the next.
- Make small, focused changes.
- **CRITICAL: every task MUST include new/updated tests** — listed as
  separate checklist items, not bundled with implementation.
- **CRITICAL: all tests must pass before starting next task**.
- **CRITICAL: update this plan file when scope changes during
  implementation** (➕ for new tasks, ⚠️ for blockers).
- Run `cargo test` and (where applicable) `bash scripts/check-loc.sh`
  + `bash scripts/check-invariants.sh` after each change.
- Maintain backward compatibility — schema additive (new optional
  `default_fg`/`default_bg`), existing user configs round-trip.
- **MUST commit each completed task as a separate git commit** —
  one task = one logical commit, with a message describing the
  task's outcome (e.g. `feat(tui): add ANSI-to-Spans parser
  (Task 1)`).  Do not batch multiple tasks into one commit; do not
  defer commits to the end of the phase.  This keeps the history
  bisectable and lets reviewers track progress task-by-task.

## Testing Strategy

- **Unit tests**: required per task (see Development Approach).
- **Integration tests**: `tests/golden_phase4.rs` (new) drives
  `App::handle(KeyEvent)` synthetically and asserts saved Config JSON
  matches expected for several scripted sessions.  No PTY emulation —
  pure event-loop driving.
- **Round-trip property test**: every shipped builtin template loads,
  builds, saves byte-identical (modulo serde key ordering).
- **ANSI parser tests**: fg/bg color codes, dim, reverse, reset,
  plain text, malformed escape — happy path and error cases.
- **Hard invariants** (CI gates):
  - Invariant 11 (`tui/*.rs` no `crate::api`/`cache`/`git`) preserved.
  - **New invariant 12**: every preset's template renders against
    the fixture without panic; reverse-lookup table is collision-free.
  - LOC ≤ 500 per file (`scripts/check-loc.sh`).
  - Forbidden strings (`security dump-keychain`, `@latest`, etc.).
- **Cross-test mutexes**: 5 mutexes (HOME_MUTEX, ENV_MUTEX,
  CONFIG_MUTEX, COLS_MUTEX, GIT_ENV_MUTEX) unchanged.

## Progress Tracking

- mark completed items with `[x]` immediately when done.
- add newly discovered tasks with ➕ prefix.
- document issues/blockers with ⚠️ prefix.
- update plan if implementation deviates from original scope.
- keep plan in sync with actual work done.

## What Goes Where

- **Implementation Steps** (`[ ]`): code, tests, docs reachable inside
  this repo.
- **Post-Completion** (no checkboxes): release-tag pushing,
  asciicast recording, social announcement — external steps.

## Implementation Steps

### Task 1: ANSI-to-Spans parser

**Files:**
- Create: `src/tui/ansi.rs`
- Create: `src/tui/ansi_tests.rs`

- [x] create `src/tui/ansi.rs` exporting `pub fn ansi_to_lines(s: &str)
      -> Vec<ratatui::text::Line<'static>>` where every `Span` holds
      `Cow::Owned(String)` content (no borrows of the input — input
      can be discarded after the call).
- [x] hand-roll a CSI parser that walks `\x1b[...m` sequences and
      maps numeric codes to ratatui `Style` (fg colors 30-37, 90-97;
      bg 40-47, 100-107; reset 0; dim 2; reverse 7; bold 1).
- [x] **No** `StyledText` helper type — callers mutate
      `Span::style` directly via `&mut Vec<Line<'static>>` after
      receipt.  `Span<'static>` carries `Style` as an owned field;
      mutation is lifetime-safe.  Document this contract in a
      module-level rustdoc.
- [x] write tests for fg color (`\x1b[31m`), bg color (`\x1b[44m`),
      reset, dim, reverse, bold, plain text, multi-line input,
      malformed escape (no `[` after ESC), unterminated CSI.
- [x] write test asserting `Span::style` can be mutated post-parse
      and the result renders correctly (smoke against a `Frame`).
- [x] run `cargo test ansi::` — must pass before Task 2.

### Task 2: Preset catalog

**Files:**
- Create: `src/tui/catalog.rs`
- Create: `src/tui/catalog_tests.rs`

- [x] define `pub enum Category { Workspace, Git, SessionModel,
      Context, Tokens, Cost, Rates, Appearance }` with `pub fn
      ordered() -> &'static [Category]` returning the 8 in tab order
      (workspace → git → session_model → context → tokens → cost →
      rates → appearance).
- [x] define `pub struct Preset { id: &'static str, category:
      Category, label: &'static str, template: &'static str,
      hide_when_absent: bool, default_color: Option<NamedColor>,
      default_bg: Option<NamedColor> }`.
- [x] define `pub const PRESETS: &[Preset]` with the 43 entries from
      the brainstorm: workspace 6, git 5, session_model 8, context 5,
      tokens 6, cost 5, rates 8.
- [x] add `pub fn lookup(template: &str) -> Option<&'static Preset>`
      backed by a `OnceLock<HashMap<&'static str, &'static Preset>>`.
- [x] add `pub fn by_category(c: Category) -> impl Iterator<Item =
      &'static Preset>`.
- [x] write test asserting no duplicate `id`s across the catalog.
- [x] write test asserting no duplicate `template`s (lookup table
      collision-free).
- [x] write test (invariant 12) — for every preset, call
      `format::placeholders::render_placeholder(name, &fixture_ctx)`
      for every placeholder name appearing in the template; assert
      no panic and (for non-`hide_when_absent` presets) that
      `config::render::render(&single_segment_config, &fixture_ctx)`
      returns non-empty.
- [x] write test asserting catalog tab counts match brainstorm: 6, 5,
      8, 5, 6, 5, 8.
- [x] run `cargo test catalog::` — must pass before Task 3.

### Task 3: Builder state + Config conversion + schema additive fields

**Files:**
- Modify: `src/config/schema.rs`
- Create: `src/tui/builder.rs`
- Create: `src/tui/builder_tests.rs`

- [x] add to `Config` two new fields:
      `default_fg: Option<NamedColor>` and `default_bg:
      Option<NamedColor>`, both `#[serde(default,
      skip_serializing_if = "Option::is_none")]`.  Backward-compat:
      old configs without these fields parse cleanly to `None`.
- [x] define `pub enum BuilderSegment { Preset { id: &'static str,
      color, bg }, Custom { template, color, bg, padding,
      hide_when_absent } }`.
- [x] define `pub struct BuilderLine { separator: String, segments:
      Vec<BuilderSegment> }` and `pub struct BuilderState { lines:
      Vec<BuilderLine>, powerline: bool, default_fg:
      Option<NamedColor>, default_bg: Option<NamedColor>, schema_url:
      Option<String> }`.
- [x] implement `pub fn from_config(c: &Config) -> BuilderState` —
      walks each `Segment::Template`, calls `catalog::lookup(template)`,
      maps to `Preset` on hit or `Custom` on miss.  Preserves user's
      color/bg overrides (ignores preset defaults if user customized).
      Copies `c.default_fg`/`default_bg` into `BuilderState`.
- [x] implement `pub fn to_config(b: &BuilderState) -> Config` —
      project `Preset` back via catalog template + preserved color/bg;
      project `Custom` 1:1; copy `BuilderState.default_fg`/`default_bg`
      back into `Config`.
- [x] write test: round-trip every entry in `config::builtins::all()`
      — `to_config(from_config(c))` is **structurally identical** via
      `serde_json::from_str::<serde_json::Value>(&serialized)`
      equality (NOT byte-identity — the schema uses struct-only
      types so byte-identity holds today, but `Value` equality is
      the contract we test against to insulate from future
      `HashMap`-bearing changes).
- [x] write test: a custom template (`${cost_usd}` with `$` prefix)
      becomes `BuilderSegment::Custom`, survives round-trip exactly.
- [x] write test: a preset segment with a non-default color override
      preserves that color across `to_config(from_config(...))`.
- [x] write test: `default_fg`/`default_bg` round-trip both as
      `None` (omitted from JSON) and `Some(NamedColor::Cyan)`
      (serialized as a key).
- [x] write test: a config WITHOUT the new fields (legacy JSON)
      deserializes with `default_fg=None`, `default_bg=None` —
      serializing back omits the keys entirely (no clutter for
      users who don't use them).
- [x] write test: empty config → empty BuilderState → empty config.
- [x] run `cargo test builder:: && cargo test config::schema::`
      — must pass before Task 4.

### Task 4: App state machine

**Files:**
- Create: `src/tui/app.rs`
- Create: `src/tui/app_tests.rs`

- [x] define `pub enum Focus { Top, Middle, Bottom }`.
- [x] define `pub enum Mode { Browsing, Filter, EditingSeparator,
      PickingFgColor, PickingBgColor, Saving, Help, ConfirmDelete,
      ConfirmQuit }`.
- [x] define `pub enum Cursor { Gutter, Segment(usize), VirtualNewLine }`
      and `pub struct App { builder: BuilderState, output_path:
      PathBuf, active_line: usize, cursor: Cursor, focus: Focus,
      mode: Mode, active_tab: Category, picker_filter: String,
      picker_selected: usize, color_picker_selected: usize, dirty:
      bool, status_message: Option<(String, u64)>, last_save_errors:
      Vec<ValidationError>, should_quit: bool }`.
- [x] implement `pub fn new(config: Config, output_path: PathBuf) ->
      App` — initial focus = Top, cursor = Gutter on line 0, active
      tab = Workspace.
- [x] implement cursor walks: `cursor_left`, `cursor_right`,
      `cursor_up_line`, `cursor_down_line` (cycles through real lines
      + virtual `+ new line` row when `lines.len() < MAX_LINES`).
- [x] implement focus cycle: `Tab` Top→Middle→Bottom→Top;
      `Shift+Tab` reverse.
- [x] implement tab cycle: `[` previous Category, `]` next Category;
      wraps Workspace ↔ Appearance.
- [x] implement line ops: `add_line()` on virtual row Enter,
      `delete_line()` (with ConfirmDelete prompt if segments ≥ 1,
      floor 1), `move_line_up/down()`, `duplicate_line()`,
      `edit_separator()` (enters EditingSeparator mode).
- [x] implement segment ops: `delete_segment()`, `reorder_left/right()`,
      `toggle_preset(category, preset_index)` — adds preset to active
      line if not present, removes if present (matches by `id`).
- [x] write tests for every state-machine transition: focus cycle,
      tab cycle, cursor walks (including virtual row hide when 3
      lines), add/delete/reorder/duplicate line, toggle preset on
      empty/full line, custom-segment-protection (toggle preset
      when matching custom exists adds preset as new segment, doesn't
      replace custom).
- [x] write tests for ConfirmDelete flow and ConfirmQuit-on-dirty.
- [x] write test: pressing `x` on Gutter when `lines.len() == 1`
      sets `status_message = "cannot remove last line"` and does
      not mutate state.
- [x] run `cargo test app4::` — must pass before Task 5.

### Task 5: Top pane rendering

**Files:**
- Create: `src/tui/panes/mod.rs`
- Create: `src/tui/panes/top.rs`
- Create: `src/tui/panes/top_tests.rs`

- [x] in `panes/mod.rs` declare `pub mod top; pub mod middle;
      pub mod bottom; pub mod appearance;`.
- [x] in `panes/top.rs` implement `pub fn render(frame: &mut Frame,
      area: Rect, app: &App)` — lays out N real lines + virtual
      `+ new line` row when applicable.
- [x] for each line, render **per-segment** to avoid post-hoc span
      identification: call `config::render::render(&one_segment_config,
      &fixture_ctx)` for each `BuilderSegment` in turn, run each
      through `ansi::ansi_to_lines`, and concatenate the resulting
      `Span`s with separator spans between them.  Per-segment
      rendering lets us apply `Modifier` per segment cleanly.
- [x] apply `Modifier::DIM` (mutating `Span::style` in place) to
      every span produced by a `BuilderSegment::Custom`.
- [x] apply `Modifier::REVERSED` (mutating `Span::style` in place)
      to every span produced by the cursor segment (when
      `Focus::Top` and `Cursor::Segment(i)`).
- [x] render `>` in gutter column for active line; blank otherwise.
- [x] render virtual `+ new line` row when cursor reachable; reverse
      when cursor on it.
- [x] honor active-pane visual: `border_style(Color::Cyan)` + bold
      `▶ Preview ` title when `focus == Top`; `Color::DarkGray` +
      `  Preview ` otherwise.
- [x] write test: with `Focus::Top, Cursor::Segment(1)` the second
      segment of the active line has `Modifier::REVERSED`.
- [x] write test: a `BuilderSegment::Custom` segment renders with
      `Modifier::DIM`.
- [x] write test: virtual `+ new line` row visible iff
      `lines.len() < 3`.
- [x] write test: gutter `>` only on the active line.
- [x] write test: active-pane border color toggles when `focus`
      changes.
- [x] run `cargo test panes::top::` — must pass before Task 6.

### Task 6: Middle pane — tabs + preset checkboxes

**Files:**
- Create: `src/tui/panes/middle.rs`
- Create: `src/tui/panes/middle_tests.rs`

- [x] implement `pub fn render(frame: &mut Frame, area: Rect, app:
      &App)` — top row: tab strip with all 8 categories, active one
      reverse-video; below: checkbox list for `app.active_tab`.
- [x] for each preset row: `[x]` if active line contains a
      `BuilderSegment::Preset { id, .. }` matching the row, `[ ]`
      otherwise; label column; live-rendered preview column.
- [x] live preview column: render the preset's template against
      fixture, run through `ansi_to_lines`, take first line of
      spans.  Empty placeholder → dim `—`.
- [x] when `mode == Filter`, narrow visible rows by case-insensitive
      substring match against label OR template.
- [x] honor active-pane visual style identical to top pane.
- [x] when `app.active_tab == Category::Appearance`, dispatch to
      `panes::appearance::render` instead.
- [x] write test: checkbox state correctly reflects active line for
      each tab.
- [x] write test: filter narrows rows correctly (case-insensitive,
      label or template hits both work).
- [x] write test: switching `active_tab` rerenders to a different
      preset list.
- [x] write test: a preset with an empty placeholder value (e.g.
      `git_branch` when fixture has no repo) shows dim `—`.
- [x] run `cargo test panes::middle::` — must pass before Task 7.

### Task 7: Middle pane — Appearance settings

**Files:**
- Create: `src/tui/panes/appearance.rs`
- Create: `src/tui/panes/appearance_tests.rs`

- [x] implement `pub fn render(frame: &mut Frame, area: Rect, app:
      &App)` — settings form:
      `[ ] Powerline mode      (off | on)`
      `[ ] Default fg color    (none | named)`
      `[ ] Default bg color    (none | named)`
      `Line 1 separator        " | "`
      `Line 2 separator        " · "`
      `Line 3 separator        " "`  (only shown when line exists).
- [x] `Space` on Powerline row toggles `app.builder.powerline`,
      sets `dirty=true`.
- [x] `Space` on Default fg/bg row → opens corresponding color
      picker overlay; on commit, writes to `app.builder.default_fg`
      / `default_bg` (new fields, see Task 3 — add if missing).
- [x] `Enter` on a separator row → opens EditingSeparator popover
      pre-filled with current value.
- [x] write test: powerline toggle flips and dirties state.
- [x] write test: separator edit commit updates the right line's
      separator.
- [x] write test: separator edit cancel (Esc) leaves state unchanged.
- [x] write test: when `lines.len() == 2`, only Line 1 / Line 2 rows
      visible.
- [x] run `cargo test panes::appearance::` — must pass before Task 8.

### Task 8: Bottom pane — cursor-aware keymap

**Files:**
- Create: `src/tui/panes/bottom.rs`
- Create: `src/tui/panes/bottom_tests.rs`

- [x] implement `pub fn render(frame: &mut Frame, area: Rect, app:
      &App)` — produces a keymap based on tuple `(focus, mode,
      cursor-position)`.
- [x] key→action mapping table per state:
  - Top + Browsing + Cursor::Segment: `←/→:cursor  </>:reorder  x:delete  c:fg  b:bg  ↑/↓:line  Tab:middle  q:quit  ?:help`
  - Top + Browsing + Cursor::Gutter: `↑/↓:line  s:separator  J/K:move-line  y:duplicate  x:delete-line  Tab:middle  q:quit  ?:help`
  - Top + Browsing + Cursor::VirtualNewLine: `Enter:add-line  ↑:back  Tab:middle  q:quit  ?:help`
  - Middle + Browsing (preset row): `Space:toggle  /:filter  [/]:tab  j/k:row  Tab:top  Ctrl+S:save  q:quit  ?:help`
  - Middle + Browsing (Appearance tab): `Space:toggle  Enter:edit  [/]:tab  j/k:row  Tab:top  Ctrl+S:save  q:quit`
  - Mode::Filter / EditingSeparator / etc.: `[edit] type to change  Enter:commit  Esc:cancel`
- [x] when cursor on a `BuilderSegment::Custom` segment, prepend a
      one-line note: ``custom: `{template}` — toggle disabled``.
- [x] truncate lowest-priority pairs first when width < 60 cols;
      `q:quit` and `Ctrl+S:save` always visible.
- [x] write test: keymap content matches expected for each (focus,
      mode, cursor) tuple.
- [x] write test: custom-segment hint appears when cursor on Custom.
- [x] write test: truncation at narrow widths preserves required
      pairs.
- [x] run `cargo test panes::bottom::` — must pass before Task 9.

### Task 9: Overlays — color picker, help, save, confirm

**Files:**
- Create: `src/tui/overlays/mod.rs`
- Create: `src/tui/overlays/color_picker.rs`
- Create: `src/tui/overlays/help.rs`
- Create: `src/tui/overlays/save.rs`
- Create: `src/tui/overlays/confirm.rs`
- Create: `src/tui/overlays/tests.rs`

- [x] in `overlays/mod.rs` declare submodules.
- [x] `color_picker.rs` — lift Phase 3's `tui/widgets/color_picker.rs`
      with minimal API change: `pub fn render(frame, area, selected:
      usize, mode: PickerMode)` and `pub fn handle(event: KeyEvent,
      selected: &mut usize) -> Option<NamedColor>` (returns Some on
      Enter commit, None on movement/Esc).
- [x] `help.rs` — full-screen overlay listing every key in every
      mode + the preset catalog grouped by category.  `?` toggles;
      any other key dismisses.
- [x] `save.rs` — atomic write (`<path>.tmp` → fsync → rename →
      fsync parent).  Backup rule: if `<path>` exists AND
      `<path>.bak` does NOT exist, copy `<path>` → `<path>.bak`
      before writing the tmp file (preserves the user's first
      pre-TUI version forever; subsequent saves do not overwrite
      `.bak`).  Returns `Result<PathBuf, io::Error>`.  Sets
      `app.status_message = Some(("saved → /path".into(), now + 2))`
      on success; `("save failed: <io>".into(), now + 5)` on failure.
- [x] `confirm.rs` — modal dialogs for `delete line with N segments?
      y/n` and `unsaved changes — quit anyway? y/n`.
- [x] write tests for color_picker selection wraparound.
- [x] write tests for save: writes to tmp first, atomically renames,
      creates `.bak` only when `.bak` does not already exist
      (subsequent saves leave `.bak` untouched, preserving the
      original pre-TUI snapshot).
- [x] write tests for save failure path (read-only directory) sets
      error status_message.
- [x] write tests for confirm dialogs: y commits, n/Esc cancels.
- [x] run `cargo test overlays::` — must pass before Task 10.

### Task 10: Filter mode wiring

**Files:**
- Modify: `src/tui/app.rs`
- Modify: `src/tui/panes/middle.rs`
- Create: `src/tui/filter_tests.rs`

- [x] `/` in Middle pane sets `mode = Filter` and clears
      `app.picker_filter`.
- [x] in Filter mode: typing chars appends to `picker_filter`,
      Backspace deletes, Esc clears + returns to Browsing,
      Enter commits filter and returns to Browsing keeping the
      filter active.
- [x] middle pane renders only matching rows during Filter mode
      and after committed filter.
- [x] `/` again with active filter clears it.
- [x] write test: typing into filter narrows rows live.
- [x] write test: Esc clears filter and returns to Browsing.
- [x] write test: Enter commits filter; rows stay narrowed; bottom
      pane shows `filter: <text>  /:clear`.
- [x] write test: filter survives tab switch but resets on category
      change (decision: clear on `[`/`]`).
- [x] run `cargo test filter::` — must pass before Task 11.

### Task 11: Draw dispatcher + entry/teardown

**Files:**
- Create: `src/tui/draw.rs`
- Create: `src/tui/mod.rs`
- Modify: `src/main.rs`
- Create: `src/tui/integration_tests.rs`

- [x] `draw.rs` — `pub fn draw(frame: &mut Frame, app: &App)`:
      vertical layout 30%/55%/15% then dispatches to
      `panes::top::render`, `panes::middle::render`,
      `panes::bottom::render`.  Renders mode overlays
      (`overlays::color_picker`, `help`, `confirm`) on top.
      Implemented as `draw4.rs` (named to coexist with Phase 3 `draw.rs`).
- [x] `mod.rs` — `pub fn run4(config: Config, output_path: PathBuf)
      -> Result<(), Error>` enters raw mode, alternate screen,
      `EnableBracketedPaste`; runs event loop dispatching
      `KeyEvent` to `App::handle`; tears down on `should_quit`.
      `App::handle` lives in `app4_handle.rs` (split for LOC limit).
- [x] non-TTY refusal: at start, if `!std::io::stdout().is_terminal()`,
      return `Error::NotATty`; `--configure` exits non-zero in main.
- [x] modify `src/main.rs` — `--configure` dispatch: load config via
      existing resolver, call `tui::run` (Phase 3, unchanged) exit
      0 on Ok / non-zero on Err.  Phase 4 `run4` not yet wired to
      main — Task 12 swaps the call site.  `--configure` keeps working.
- [x] declare `cfg(test)` mutex `pub(crate) static TUI_MUTEX:
      std::sync::Mutex<()>` if integration tests need it (probably
      not — App tests don't touch global env). Decision: not needed;
      integration tests drive App state only, no global env mutation.
- [x] write integration test: synthetic event sequence that
      navigates pane→pane, tab→tab, toggles a preset, saves,
      and asserts the resulting Config JSON matches expected.
- [x] write integration test: non-TTY invocation returns NotATty.
- [x] write integration test: dirty-quit confirms before exit.
- [x] run `cargo test integration::` — must pass before Task 12.

### Task 12: Delete Phase 3 files; invariants script

**Files:**
- Delete (top-level `src/tui/`): `app_color.rs`, `app_editor.rs`,
  `app_help_tests.rs`, `app_picker.rs`, `app_powerline_tests.rs`,
  `app_save_tests.rs`, `app_save.rs`, `app_tests.rs` (Phase 3
  variant — replaced by Phase 4 `app_tests.rs` in Task 4),
  `color_picker_tests.rs`, `picker_tests.rs`, `preview.rs`,
  `save.rs`, `tests.rs`.
- Delete (entire `src/tui/widgets/`): `color_picker.rs`, `help.rs`,
  `line_list.rs`, `mod.rs`, `placeholder_picker.rs`,
  `segment_editor.rs`, `segment_list.rs`, `status.rs`.  After
  deletion the `src/tui/widgets/` directory itself should be
  removed (empty directory cleanup).
- Modify: `scripts/check-invariants.sh`
- Modify: `CLAUDE.md`

- [ ] delete every Phase 3 TUI file listed above (13 in `src/tui/`
      + 8 in `src/tui/widgets/`, plus the empty widgets directory).
      The full rewrite replaces them; nothing is preserved.
- [ ] retain only Phase 4 files (created in Tasks 1-11): `mod.rs`,
      `app.rs`, `app_tests.rs`, `catalog.rs`, `catalog_tests.rs`,
      `builder.rs`, `builder_tests.rs`, `ansi.rs`, `ansi_tests.rs`,
      `draw.rs`, `panes/*`, `overlays/*`, `preview_fixture.json`,
      `integration_tests.rs`, `filter_tests.rs`.
- [ ] add invariant 12 to `scripts/check-invariants.sh`: a grep
      asserting every preset's template (extracted from
      `catalog.rs`) appears as a `name => …` arm in
      `format/placeholders/mod.rs`.  (If grepping the source is
      brittle, fall back to a Rust unit test in `catalog_tests.rs`
      and skip the shell check.)
- [ ] update `CLAUDE.md`: replace the Phase 3 module tree with the
      Phase 4 tree; add invariant 12 to the hard-invariants list;
      bump "interactive TUI" wording to v1.1.
- [ ] write test: every `*.rs` under `src/tui/` is ≤ 500 LOC
      (already enforced by `scripts/check-loc.sh` — confirm pass).
- [ ] run `bash scripts/check-loc.sh && bash scripts/check-invariants.sh`
      — both must pass before Task 13.
- [ ] run `cargo test --all` — full suite green.

### Task 13: Round-trip golden test for builtin templates

**Files:**
- Create: `tests/golden_phase4.rs`

- [ ] for each `name` in `config::builtins::all_names()`: load the
      builtin → `BuilderState::from_config` → `to_config` →
      serialize with canonical key ordering → assert byte-identical
      to the canonical-serialized original.
- [ ] script a multi-step session via synthetic `KeyEvent`s:
      Tab→Middle, `]→]→]` to rates tab, `Space` on row 0 (5h left%),
      `Tab→Top`, `s` to edit separator → type ` | ` → Enter,
      `Ctrl+S` to save.  Read the saved JSON and assert structure.
- [ ] script a delete-line + dirty-quit-confirm sequence; assert
      file is NOT modified after Esc on confirm.
- [ ] run `cargo test --test golden_phase4` — all green.
- [ ] run full suite: `cargo test --all` — all green.

### Task 14: Verify acceptance criteria

- [ ] verify all design points from Overview are implemented:
      preset checkboxes, top-pane preview with cursor, virtual
      `+ new line` row, line gutter ops (s/J/K/y/x), cursor-aware
      bottom keymap, custom-segment passthrough with DIM rendering,
      Appearance tab settings.
- [ ] verify edge cases: 1-line minimum, 3-line maximum, custom-only
      config, empty fixture rendering with `—` placeholders,
      malformed config recovery.
- [ ] verify all hard invariants: `bash scripts/check-loc.sh`,
      `bash scripts/check-invariants.sh`, `cargo fmt --check`,
      `cargo clippy --all-targets -- -D warnings`,
      `cargo test --all`, `shellcheck scripts/*.sh`.
- [ ] run cold-start benchmark — confirm `--configure` startup is
      not regressed: `time target/release/cc-myasl --configure
      < /dev/tty` (manual smoke).
- [ ] confirm release build still ≤ 1.5 MB stripped: `ls -lh
      target/release/cc-myasl`.

### Task 15: README + CLAUDE.md final

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`

- [ ] rewrite `README.md` `--configure` section with new
      screenshots/asciicast (or markdown-rendered screenshot —
      manual capture step listed in Post-Completion).
- [ ] add a `### Phase 4 builder TUI` paragraph to README explaining
      the preset checkbox model, custom passthrough, and that
      hand-edited templates remain editable in the JSON file.
- [ ] update `CLAUDE.md`: bump module tree to Phase 4 layout;
      reference `2026-05-10-phase4-builder-tui.md` plan; add to the
      "completed" section once moved.
- [ ] update `docs/plans/completed/` references at bottom of
      `CLAUDE.md`.
- [ ] run final full test suite + lint gates.
- [ ] move this plan: `mkdir -p docs/plans/completed && mv
      docs/plans/2026-05-10-phase4-builder-tui.md
      docs/plans/completed/`.

**Note on version bump.** `Cargo.toml` is intentionally NOT bumped
in this phase.  The version bump (`1.0.0 → 1.1.0` or whatever is
current at the time) happens in a separate release-prep commit on
`main` after this branch is merged, gated by the user, before the
`v*` tag push that triggers the GitHub Actions release workflow.

## Technical Details

**Data structures.**

```rust
// catalog.rs (compile-time const)
pub struct Preset {
    pub id: &'static str,            // stable lookup key, e.g. "model"
    pub category: Category,
    pub label: &'static str,
    pub template: &'static str,      // the canonical preset template
    pub hide_when_absent: bool,
    pub default_color: Option<NamedColor>,
    pub default_bg: Option<NamedColor>,
}

// builder.rs
pub enum BuilderSegment {
    Preset { id: &'static str, color: Option<NamedColor>, bg: Option<NamedColor> },
    Custom { template: String, color: Option<NamedColor>, bg: Option<NamedColor>,
             padding: u8, hide_when_absent: bool },
}

pub struct BuilderLine { pub separator: String, pub segments: Vec<BuilderSegment> }
pub struct BuilderState {
    pub lines: Vec<BuilderLine>,
    pub powerline: bool,
    pub default_fg: Option<NamedColor>,
    pub default_bg: Option<NamedColor>,
    pub schema_url: Option<String>,
}
```

**Cursor model.** Three-state enum with a single source of truth:
`Cursor::Gutter` (line-level ops), `Cursor::Segment(usize)`
(segment-level ops), `Cursor::VirtualNewLine` (Enter to add).
Movement keys (`←/→/h/l`) cycle through `Gutter, Segment(0),
Segment(1), …, Segment(N-1)`; `↑/↓/j/k` walk lines and the virtual
row.  Bottom pane re-derives keymap from the cursor variant.

**Render pipeline.**

```
BuilderState
  → builder::to_config(&BuilderState) → Config
  → config::render::render(&Config, &fixture_ctx) → String (with ANSI)
  → tui::ansi::ansi_to_lines(&String) → Vec<Line<'static>>
  → apply Modifier::DIM for Custom segments
  → apply Modifier::REVERSED for cursor segment
  → ratatui::Frame::render
```

The pipeline runs once per keystroke; the `to_config → render →
ansi_to_lines` pass is microseconds against the 65-line fixture.

**Save atomicity.**

```
write(<path>.tmp)
fsync(<path>.tmp)
[if first save and <path> exists] copy <path> → <path>.bak
rename(<path>.tmp, <path>)
fsync(parent dir)
```

**Parallel execution annotation** (for orchestrator agents):

```
Task 1 (ansi.rs)        ── parallel ─┐
Task 2 (catalog.rs)     ── parallel ─┤
                                     │
Task 3 (builder.rs)     ──── after 2 ┘
Task 4 (app.rs)         ──── after 3
                                     ┐
Task 5 (panes/top)      ── after 4 ──┤
Task 6 (panes/middle)   ── after 4 ──┤
Task 7 (panes/appear)   ── after 4 ──┤
Task 8 (panes/bottom)   ── after 4 ──┤  parallel
Task 9 (overlays/*)     ── after 4 ──┤
Task 10 (filter)        ── after 6 ──┘
                                     ┐
Task 11 (draw + entry)  ── after 5,6,7,8,9,10
Task 12 (delete + inv)  ── after 11
Task 13 (golden test)   ── after 11
Task 14 (acceptance)    ── after 12,13
Task 15 (release prep)  ── after 14
```

## Post-Completion

*Items requiring manual intervention or external systems — no
checkboxes, informational only*

**Manual capture:**

- record a fresh asciicast for the README `--configure` section
  (`asciinema rec`, ~30s session: open TUI → tab through tabs →
  toggle 3 presets → save).  Convert to GIF for README inline,
  or host on asciinema.org and link.
- take a screenshot of the 3-pane layout for the README header.

**Manual smoke testing:**

- run `--configure` against an empty config (no `~/.config/cc-myasl/`
  directory) — verify it falls through to baked-in default and
  still renders the TUI.
- run `--configure` against an arbitrary user config with custom
  templates (e.g. a config with `${cost_usd}` or
  `{model} - {version}`) — verify segments appear DIM in top pane,
  reorder/delete still work, save round-trips byte-identically.
- run `--configure` inside tmux + iTerm2 + Alacritty + macOS
  Terminal — verify `[`/`]`, `<`/`>`, `Ctrl+S`, `Tab`/`Shift+Tab`
  all behave as documented.

**Release flow** (gated on PR merge — done by user, not Claude):

- merge feature branch to `main` via squash merge.
- tag `v1.1.0` on the merge commit and push the tag — triggers
  `.github/workflows/release.yml` matrix build (4 targets:
  aarch64/x86_64 darwin, aarch64/x86_64 linux-musl).
- verify GitHub Release page contains all 4 tarballs + sha256
  sidecars.
- update README install command if any URLs changed (none expected).
