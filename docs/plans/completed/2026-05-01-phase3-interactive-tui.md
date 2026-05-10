# Phase 3 — Interactive TUI for Config Editing (+ colors + Powerline)

## Overview

Phase 3 adds an interactive TUI launched via `cc-myasl --configure` that
lets users edit their config visually: add/remove/reorder segments,
pick placeholders from a list, set padding, set per-segment colors
(foreground + background), toggle Powerline mode, and see a LIVE
preview rendered against the standard full-payload fixture as they
edit. Saves to `~/.config/cc-myasl/config.json` with a `.bak` backup
and pre-save validation.

This is Phase 3 of a 3-phase expansion. Phase 1 (structured-config
rewrite) merged in PR #4. Phase 2 (placeholder expansion) merged in
PR #5. Phase 3 closes out the original brainstorm scope.

**Why now:** Phase 1+2 shipped 54+ placeholders behind a JSON config
that users must edit by hand. Hand-editing a multi-line segment array
with mini-templates and `hide_when_absent` flags is tedious. A TUI
flattens this — pick segments from a menu, see the result instantly.
The Phase 1 plan called this out explicitly:
> Phase 3: interactive TUI — `cc-myasl --configure` opens a TUI for
> editing `~/.config/cc-myasl/config.json`. Likely needs `ratatui`
> + `crossterm` deps; explicit dep approval.

**Why this design:**
- **ratatui + crossterm**: ecosystem standard (gitui, helix, atuin).
  Immediate-mode rendering trivially supports live preview. ~700-900
  KB binary impact (current 1.5 MB → ~2.3 MB) — within reasonable 2×
  ceiling for a tool with documented `--configure` mode. No lighter
  alternative supports live preview without ~500-800 LOC of
  hand-rolled widget code (`crossterm`-only path) or a heavier
  retained-mode model (`cursive`, `iocraft`).
- **`--configure` flag (NOT auto-detect)**: explicit invocation
  prevents accidental TUI launch during a piped Claude Code call.
  Render path stays unchanged.
- **Schema extension**: Segments gain `color` (fg) and `bg` (bg),
  both named ANSI-16. Config gains `powerline: bool`. Existing
  configs continue to work — new fields default to None/false.
- **Save with backup + validation**: writes to canonical path, copies
  old to `.bak`, validates via existing `Config::validate_and_clamp`
  before write. Refuses save on validation error.
- **Color encoding**: named ANSI-16 only (`red`, `green`, `yellow`,
  `blue`, `magenta`, `cyan`, `white`, `default`, `null`). Smallest
  parser, smallest TUI picker, matches the existing `{five_color}`
  placeholder semantics. 256-color and hex deferred to a future plan.

**Key benefits:**
- Visual config editing for the ~54 placeholders.
- Live preview pane (split layout) — see rendered output as you edit.
- Powerline mode with chevron-separated colored blocks.
- Per-segment foreground + background colors.
- No new render-path complexity: TUI is a separate module that
  produces a `Config`, then the existing render path takes over.
- Render mode (when stdin is piped) is COMPLETELY UNCHANGED.

## Context (from discovery)

### Files / components involved

**New files:**
- `src/tui/mod.rs` — TUI app entry, main loop, event handling.
- `src/tui/app.rs` — `App` struct (state machine), input handlers.
- `src/tui/draw.rs` — frame composition, layout (lines pane,
  segments pane, editor pane, preview pane).
- `src/tui/widgets/{line_list, segment_list, segment_editor,
  placeholder_picker, color_picker, help, status}.rs` — per-pane
  widgets. Split into siblings to stay under 500 LOC.
- `src/tui/save.rs` — save flow: validation, backup, atomic write.
- `src/tui/preview.rs` — invokes the existing `config::render::render`
  on a fixture-loaded `RenderCtx` to produce the live-preview string.
- `src/tui/tests.rs` — unit tests (per-widget pure-state tests; no
  terminal I/O in tests).

**Modified files:**
- `src/config/schema.rs` — extend `TemplateSegment` with `color:
  Option<String>` and `bg: Option<String>`. Extend `Config` with
  `powerline: bool` (`#[serde(default)]`).
- `src/config/render.rs` — apply fg/bg ANSI codes around segment
  output when fields are set. Implement Powerline mode (chevron
  transitions + bg color flow) when `config.powerline` is true.
- `src/format/values.rs` — extend ANSI-stripping `visible_width`
  helper to handle the new fg/bg sequences emitted by the renderer
  (existing logic handles CSI sequences generically; verify).
- `src/args.rs` — add `--configure` flag (boolean) and
  `--output <path>` flag (optional output override).
- `src/main.rs` — dispatch on `args.configure`: if true, run TUI
  and exit; else current render flow. The TUI dispatch may need
  extraction to a sibling module if main.rs LOC pressure remains.
- `src/lib.rs` — add `pub mod tui;`.
- `cc-myasl.schema.json` — add `color`, `bg` to TemplateSegment;
  add `powerline` to Config root. Document the named ANSI-16 enum.
- `Cargo.toml` — add `ratatui` (default-features = false) and
  `crossterm` (default-features = false).
- `README.md` — document `--configure` mode + key bindings.
- `CLAUDE.md` — locked-dep set, module tree, hard invariants.

### Related patterns found

- Phase 1's `RenderCtx` discipline: primitives only. TUI emits a
  `Config` struct, never touches `RenderCtx` directly.
- `format::values::bar` and `pick_color`/`pick_icon` already exist
  and emit ANSI escape codes — fg/bg additions should follow the
  same pattern (small `&'static str` constants per color).
- Phase 1's serde tolerance: `#[serde(default)]`, no
  `deny_unknown_fields`. New fields naturally default to None/false.
- gix slim build precedent (Phase 2): `default-features = false`
  for ratatui mirrors that pattern.

### Dependencies identified

- `ratatui = { version = "0.30", default-features = false,
   features = ["macros", "crossterm"] }` (NEW) — TUI primitives.
   Justification: only viable option with live preview. Ecosystem
   standard. ~25.5M downloads. Active multi-maintainer project.
- `crossterm = { version = "0.29", default-features = false }` (NEW)
   — terminal I/O backend for ratatui. Pulled by ratatui's
   `crossterm` feature; pinned at top-level for clarity.
   Justification: required by ratatui's `crossterm` backend.
- All other deps unchanged.

## Development Approach

- **Testing approach:** Regular (code first, then tests in same
  task). Matches Phase 1 + Phase 2 pattern.
- Complete each task fully before moving to the next.
- Make small, focused changes.
- **CRITICAL: every task MUST include new/updated tests** for code
  changes in that task — required, not optional.
  - Unit tests for new structs / functions / pure-state logic.
  - Tests cover both success and absence (None) scenarios.
  - TUI tests stay PURE-STATE: build an `App` struct, feed
    synthetic key events, assert state transitions. NO real
    terminal I/O in tests.
- **CRITICAL: all tests must pass before starting next task** — no
  exceptions.
- **CRITICAL: update this plan file when scope changes during
  implementation.**
- Run `cargo test` after each change.
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `bash scripts/check-loc.sh`, `bash scripts/check-invariants.sh`,
  `shellcheck scripts/*.sh` before declaring a task done.

## Testing Strategy

- **Unit tests:** required for every task. TUI widgets tested as
  pure-state machines.
- **Integration tests (`tests/golden.rs` or sibling files):** the
  TUI itself is hard to integration-test without a real terminal;
  cover the render path's NEW behavior (fg/bg ANSI escapes,
  Powerline rendering) via standard golden tests with fixtures.
  Add `tests/golden_phase3.rs` for Phase-3-specific assertions:
  - Save flow writes valid JSON.
  - `--configure` flag is accepted by arg parser (doesn't crash).
  - Schema accepts new color/bg/powerline fields (round-trip).
- **Cross-test isolation:** Phase 3 introduces no new env vars.
  The 5 existing mutexes cover all current cases. The TUI itself
  doesn't read env vars beyond what `config::resolve` already
  handles.
- **No real-network tests** in CI.

## Progress Tracking

- Mark completed items with `[x]` immediately when done.
- Add newly discovered tasks with ➕ prefix.
- Document issues / blockers with ⚠️ prefix.
- Update plan if implementation deviates from original scope.
- Keep plan in sync with actual work done.

## What Goes Where

- **Implementation Steps** (`[ ]` checkboxes): code, tests,
  documentation updates inside this repo.
- **Post-Completion** (no checkboxes): items requiring external
  action — README publication, manual TUI smoke test, deferred
  follow-ups.

## Implementation Steps

### Task 0: Add `ratatui` + `crossterm` deps; verify slim build

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock` (auto)
- Modify: `CLAUDE.md` (locked-dep set)

- [x] run `cargo test` — baseline must be green before any change
- [x] add to Cargo.toml:
      ```toml
      ratatui = { version = "0.29", default-features = false, features = ["macros", "crossterm"] }
      crossterm = { version = "0.28", default-features = false }
      ```
      NOTE: ratatui 0.30 requires Rust edition 2024 (incompatible with
      rust-version = "1.83"); pinned to 0.29. ratatui 0.29 requires
      crossterm 0.28; pinned accordingly. Two ratatui transitive deps
      also pinned in Cargo.lock via `cargo update --precise`:
      `instability` → 0.3.5 (0.3.12 requires rustc 1.88),
      `unicode-segmentation` → 1.12.0 (1.13.2 requires rustc 1.85).
- [x] run `cargo build`; investigate any failures.
- [x] run `cargo tree --no-dev-dependencies | head -50` and confirm
      NO new HTTP/SSH/TLS/async-runtime crates were pulled.
      Specifically watch for `tokio`, `async-std`, `reqwest`.
      CONFIRMED: no tokio/async-std/reqwest. mio + signal-hook present
      (expected).
- [x] **`mio` and `signal-hook` are EXPECTED to be pulled** by
      crossterm for terminal event polling. Document them in the
      Cargo.toml comment above the crossterm declaration:
      `# crossterm pulls: mio (event loop), signal-hook (terminal resize)`.
      Do NOT block the task on these — they're crossterm's
      legitimate runtime needs. Only block on async runtimes
      that crossterm should NOT need (tokio etc.).
- [x] verify musl cross-compile: `cargo build --target
      x86_64-unknown-linux-musl` if rustup target is available
      locally; otherwise document `⚠️ MUSL CHECK DEFERRED` and
      let the PR's CI matrix catch it.
      ⚠️ MUSL CHECK DEFERRED — no musl target installed locally;
      PR CI matrix will verify.
- [x] update CLAUDE.md "Things to NOT do" locked-dep list to add
      both crates with one-line justifications:
      - `ratatui` (TUI primitives for `--configure` mode; only
        crate supporting live preview without hand-rolled widgets)
      - `crossterm` (terminal backend for ratatui)
- [x] run `cargo build --release` and report stripped binary size
      via `wc -c`/`ls -l target/release/cc-myasl`. Target ≤ 2.5 MB.
      RESULT: 1,548,504 bytes (~1.48 MB) — within target.
- [x] no test required for dep-only task; proceed when build green
- [x] commit. Suggested message:
      `chore: add ratatui + crossterm for Phase 3 TUI (slim build)`

### Task 1: Extend schema with color, bg, powerline fields

**Files:**
- Modify: `src/config/schema.rs`
- Modify: `cc-myasl.schema.json`
- Modify: `src/config/tests*.rs` (existing test files)

- [x] add `pub color: Option<String>` and `pub bg: Option<String>`
      fields to `TemplateSegment` with `#[serde(default)]`.
- [x] add `pub powerline: bool` field to `Config` with
      `#[serde(default)]` (defaults to false).
- [x] add a `pub const NAMED_COLORS: &[&str] = &["red", "green",
      "yellow", "blue", "magenta", "cyan", "white", "default"];`
      array — used by validation and the TUI color picker.
- [x] extend `validate_and_clamp` (or add a sibling validator) to
      reject `color` / `bg` values not in `NAMED_COLORS` (and not
      `null`). Error variant: `ValidationErrorKind::InvalidColor
      { field: &'static str, value: String }`.
- [x] update `cc-myasl.schema.json`:
      - `TemplateSegment.color` and `TemplateSegment.bg`:
        `{ "type": "string", "enum": ["red", "green", "yellow",
        "blue", "magenta", "cyan", "white", "default"] }`
      - `Config.powerline`: `{ "type": "boolean", "default": false }`
      - Update `description` fields with brief docs.
- [x] write unit tests:
      - serde round-trip with `color` / `bg` populated
      - serde round-trip with both absent (back-compat with Phase 1
        configs — no color emitted)
      - validation rejects `color: "purple"` (not in NAMED_COLORS)
      - validation accepts each of the 8 named colors
      - validation rejects empty string vs null (clarify which is
        valid; null = absent, empty string = invalid)
      - validation accepts `powerline: true` and `powerline: false`
- [x] run `cargo test config::schema`; all pass before next task

### Task 2: Render fg + bg color in non-Powerline mode

**Files:**
- Modify: `src/format/values.rs` (color name → ANSI escape helper)
- Modify: `src/config/render.rs` (consumes the helpers)

- [x] **Locked location:** `ansi_fg` and `ansi_bg` live in
      `src/format/values.rs`. `config/render.rs` calls them via
      `use crate::format::values::{ansi_fg, ansi_bg}`. This is the
      ALLOWED direction (config→format). Do NOT put the color
      table or escape codes inside `config/` — that would
      duplicate format/'s color knowledge and create a bidirectional
      coupling.
- [x] If `pick_color` (currently in `format::placeholders` per
      Phase 1) is needed by the new helpers, MOVE it to
      `format::values` before adding the new functions. Update
      callers in `format::placeholders` to import from
      `format::values`. This ensures all color-table knowledge
      lives in one module.
      NOTE: `pick_color` is in `format::thresholds` (not placeholders)
      and takes a `State` enum — not needed by ansi_fg/ansi_bg which
      take `&str`. Left in thresholds; no move required.
- [x] add `pub fn ansi_fg(name: &str) -> &'static str` and
      `pub fn ansi_bg(name: &str) -> &'static str` in
      `format::values` returning the ANSI escape codes (e.g.,
      `"\x1b[31m"` for fg red, `"\x1b[41m"` for bg red). Returns
      empty string for unknown names.
- [x] in `config::render::render`, when emitting a Template
      segment, wrap the rendered output as
      `<fg-code><bg-code><value><reset>` if either `color` or `bg`
      is Some. The reset code (`\x1b[0m`) is always appended.
- [x] DO NOT yet implement Powerline mode (Task 3). For this task,
      `config.powerline` is read but ignored — color/bg apply per
      segment regardless.
- [x] verify `visible_width` (the ANSI-stripping function used by
      the flex spacer) still correctly counts width when fg/bg
      escapes are emitted. Add a test if the existing CSI parser
      doesn't already cover bg codes (`\x1b[4Xm`).
      NOTE: CSI parser is generic (`\x1b[<digits>m`); bg codes handled
      identically to fg. Test `visible_width_strips_bg_escape_code`
      added in `render_color_tests.rs`.
- [x] write unit tests:
      - segment with `color: Some("red")` and no bg → output
        contains `\x1b[31m`, `\x1b[0m`
      - segment with both `color: Some("red")` and `bg:
        Some("blue")` → output contains `\x1b[31m\x1b[44m`
      - segment with neither → output unchanged from Phase-2
        behavior
      - flex spacer width still correct when adjacent segment has
        fg+bg colors
- [x] run `cargo test config::render`; all pass before next task

### Task 3: Render Powerline mode (chevrons + bg flow)

**Files:**
- Modify: `src/config/render.rs`
- Possibly: `src/config/render_powerline.rs` (sibling, if render.rs
  approaches 500 LOC)

- [x] when `config.powerline == true`, change the rendering of each
      line as follows:
      - Each visible segment is rendered with its `bg` color as a
        BLOCK (text is drawn over `bg`). Segments without `bg` use
        a default block color (pick `bg: "default"` semantics or
        skip the block — document the choice).
      - Between adjacent visible segments, emit a chevron character
        (`` for the standard Powerline glyph, requires Nerd Font;
        document the font requirement in README and CLAUDE.md).
      - The chevron's fg color = previous segment's bg; the
        chevron's bg color = next segment's bg. This gives the
        classic Powerline transition.
      - The line's `separator` is OVERRIDDEN by the chevron when
        Powerline is on (or: separator is rendered BEFORE the
        chevron, but that looks weird; document the choice — I
        recommend OVERRIDE).
      - The flex spacer + Powerline interaction: flex still pads
        with spaces; the chevron rendering happens around the flex
        — the chevron preceding flex uses (previous bg, default bg)
        and chevron after flex uses (default bg, next bg). Document.
- [x] keep non-Powerline rendering (`config.powerline == false`)
      UNCHANGED from Task 2.
- [x] **Locked decision: chevron placement.** SKIP the leading
      chevron. Render `seg_0`, then `chevron(prev=bg_0, next=bg_1)`
      + `seg_1`, then `chevron(prev=bg_1, next=bg_2)` + `seg_2`,
      then trailing `chevron(prev=bg_last, next=default)`. So a
      3-segment line has exactly 3 chevrons total: 2 between
      segments + 1 trailing. This matches typical Powerline
      shells (oh-my-zsh, p10k).
- [x] write unit tests:
      - 3-segment line with Powerline → assert exactly 3 chevron
        chars in output
      - segments with same bg → chevron between is invisible (fg
        and bg match) — verify
      - hidden segment in middle → chevron transitions skip it
        cleanly (no double chevron)
      - flex spacer interaction
- [x] run `cargo test config::render`; all pass before next task

### Task 4: Create `src/tui/` skeleton + main loop + invariant gate

**Files:**
- Create: `src/tui/mod.rs`
- Create: `src/tui/app.rs`
- Create: `src/tui/draw.rs`
- Modify: `src/lib.rs` (add `pub mod tui;`)
- Modify: `scripts/check-invariants.sh` (NEW invariant for tui/)

- [x] in `src/tui/app.rs`, define `pub struct App { config: Config,
      cwd_lines: Vec<Line>, selected_line: usize, selected_segment:
      Option<usize>, mode: Mode, dirty: bool, ... }`. The `Mode`
      enum has variants: `Browsing`, `EditingTemplate`,
      `PickingPlaceholder`, `PickingFgColor`, `PickingBgColor`,
      `Saving`, `Help`, `ConfirmQuit`.
- [x] in `src/tui/mod.rs`, add `pub fn run(config: Config,
      output_path: PathBuf) -> std::io::Result<()>`:
      1. Setup terminal (alternate screen, raw mode) via crossterm.
      2. Build `App` from the input config.
      3. Event loop: `crossterm::event::read()` → `app.handle(event)
         ` → `terminal.draw(|f| draw(f, &app))` → repeat until
         `app.should_quit`.
      4. Restore terminal on exit (any panic or normal return —
         use a guard struct with `Drop` to ensure restoration even
         on panic).
- [x] in `src/tui/draw.rs`, add `pub fn draw(frame: &mut Frame,
      app: &App)` that lays out 3 panes via `Layout::vertical`:
      - top (60%): horizontal split of LineList (left) + SegmentList
        (right) (Task 5)
      - middle (30%): SegmentEditor pane (Task 6)
      - bottom (10%): live preview + status bar (Tasks 8, 11)
      - For Task 4 STUB: just draw 3 empty `Block`s with titles.
- [x] write unit tests for `App` state machine: pure-state, no
      terminal I/O. Feed synthetic `KeyEvent`s and assert state
      transitions (e.g., pressing `q` in `Browsing` mode sets
      `should_quit = true`). Use `crossterm::event::KeyEvent::new
      (KeyCode::Char('q'), KeyModifiers::NONE)`.
- [x] LOC check: each new file under 500 LOC. The `App` struct
      itself may grow — if it does, split state into sub-structs
      (e.g., `EditorState`, `PickerState`) by mode.
- [x] **Add the tui/ import invariant to `scripts/check-invariants.sh`
      NOW** (not Task 15). New invariant 11: `tui/*.rs` MUST NOT
      contain `use crate::api`, `use crate::cache`, or
      `use crate::git`. Mirror the existing greps for
      `format/`, `config/`, `git/`. Recursive grep over `src/tui/`.
      ALSO add a Rust string-scan unit test in `src/tui/mod.rs`
      `#[cfg(test)] mod tests` extending the directory-walk test
      pattern from Phase 2's Task 4 to cover `src/tui/`. This
      ensures every subsequent task (5-12) runs the gate.
- [x] run `cargo test tui`; all pass before next task

### Task 5: TUI — line list + segment list panes

**Files:**
- Create: `src/tui/widgets/line_list.rs`
- Create: `src/tui/widgets/segment_list.rs`
- Modify: `src/tui/draw.rs` (wire up the panes)
- Modify: `src/tui/app.rs` (add input handlers)

- [x] LineList: ratatui `List` widget displaying line indexes 0..N
      (max MAX_LINES=3) with a "+" entry at the bottom to add a new
      line (when count < MAX_LINES). Selected line is highlighted.
- [x] SegmentList: ratatui `List` showing the segments of the
      currently-selected line. Each entry shows
      `<index>: {placeholder} (padding=N, hide=B)` for Template
      segments or `<index>: <flex>` for Flex segments. Last entry
      is "+ add segment" when not at limit.
- [x] Key handlers (in `App::handle` for Mode::Browsing):
      - `j`/`Down`: move selection down (within current pane)
      - `k`/`Up`: move up
      - `Tab`: switch focus between LineList / SegmentList
      - `Enter` on a segment: enter EditingTemplate mode
      - `Enter` on "+ add segment": insert a default Template
        segment, focus it, enter EditingTemplate mode
      - `d` on a segment: delete it (with confirm? — skip confirm
        for now, dirty bit makes save explicit)
      - `J`/`K` (shift): move segment down/up (reorder)
      - `n` on LineList: add a new line if count < MAX_LINES
      - `D` on LineList: delete the line (only if count > 1)
      - `q`: enter ConfirmQuit if dirty, else quit
- [x] **Mode-aware key dispatch:** the `q` handler ONLY applies in
      `Mode::Browsing`. In `EditingTemplate` mode (Task 6), `q` is
      a literal text character and inserts into the template. In
      placeholder/color picker modes (Tasks 7, 9), `q` filters the
      list. The global `App::handle` MUST dispatch on `app.mode`
      first, then route the key event to the active mode's handler.
      No global "q quits" shortcut. To quit from any non-Browsing
      mode, user presses `Esc` first to return to Browsing, then
      `q`.
- [x] write unit tests: each handler advances state correctly.
      Round-trip: load a config, simulate key sequence, save back,
      assert resulting Config matches expected.
- [x] LOC check; split widgets if needed
- [x] run `cargo test tui`; all pass before next task

### Task 6: TUI — segment editor pane

**Files:**
- Create: `src/tui/widgets/segment_editor.rs`
- Modify: `src/tui/draw.rs`
- Modify: `src/tui/app.rs`

- [x] SegmentEditor pane shows fields for the currently-selected
      Template segment:
      - Template string (text input)
      - Padding (number input, 0..=8)
      - Hide when absent (checkbox)
      - Color (label showing current; press `c` to enter
        PickingFgColor mode — Task 9)
      - Background (label showing current; press `b` to enter
        PickingBgColor mode — Task 9)
      For a Flex segment, show only "Flex spacer (no editable
      fields)".
- [x] Key handlers:
      - When SegmentEditor pane is focused: `Tab` cycles fields,
        `Enter` enters edit-text or toggle mode, `Esc` exits to
        Browsing.
      - In EditingTemplate mode: text input via crossterm
        keystrokes, `Enter` commits, `Esc` aborts.
      - `+`/`-` on padding field: increment/decrement (clamp 0..=8).
      - `space` on hide_when_absent: toggle.
- [x] Mark `app.dirty = true` on any change.
- [x] write unit tests for each editor action: state mutates
      correctly, padding clamps, template input handles
      backspace/cursor.
- [x] run `cargo test tui`; all pass before next task

### Task 7: TUI — placeholder picker overlay

**Files:**
- Create: `src/tui/widgets/placeholder_picker.rs`
- Modify: `src/tui/draw.rs`
- Modify: `src/tui/app.rs`

- [x] Add a `pub const ALL_PLACEHOLDERS: &[(&str, &str)] = &[...]`
      list in `src/format/placeholders/mod.rs` (or a new
      `src/format/catalog.rs`) — name + brief description per
      placeholder. Update Phase 1+2 docstring comments to derive
      this list.
- [x] PlaceholderPicker overlay: full-screen popup `List` with
      filter-as-you-type at the top (text input). User selects
      a placeholder; on `Enter`, it's inserted into the current
      segment's template at cursor position (or replaces template
      if empty).
- [x] Mode `PickingPlaceholder` — entered when user presses `p` in
      EditingTemplate mode. Returns to EditingTemplate on `Enter`
      or `Esc`.
- [x] write unit tests: filter narrows the list; selection inserts
      the right name; Esc cancels.
- [x] run `cargo test tui`; all pass before next task

### Task 8: TUI — live preview pane

**Files:**
- Create: `src/tui/preview.rs`
- Create: `src/tui/preview_fixture.json` (NEW — dedicated to TUI,
  separate from `tests/fixtures/full-payload.json`)
- Modify: `src/tui/draw.rs`

- [x] `pub fn render_preview(config: &Config, fixture: &Payload,
      now_unix: u64) -> String` — calls
      `payload_mapping::build_render_ctx(fixture, now_unix)` then
      `config::render::render(config, &ctx)`.
- [x] **Locked decision:** create `src/tui/preview_fixture.json`
      with a minimal stable payload that populates every Phase
      1 + 2 placeholder. Embed via `include_str!("preview_fixture.json")`.
      Do NOT use `include_str!("../../tests/fixtures/full-payload.json")`
      because that couples production code to a test-only path.
      The new `preview_fixture.json` lives next to the production
      code that consumes it. Content can be a copy of
      `tests/fixtures/full-payload.json` initially, but they are
      independently maintained going forward.
- [x] Bottom pane in `draw.rs` shows the multi-line preview output.
      Update on every keystroke (immediate-mode renders cheaply).
      Truncate or scroll if output is wider than the pane.
- [x] Show a small dirty/clean indicator (e.g., "●" if dirty).
- [x] write unit tests for `render_preview` with a known
      Config + fixture pair, assert output matches expected.
- [x] run `cargo test tui`; all pass before next task

### Task 9: TUI — color picker overlays (fg + bg)

**Files:**
- Create: `src/tui/widgets/color_picker.rs`
- Modify: `src/tui/draw.rs`
- Modify: `src/tui/app.rs`

- [x] ColorPicker overlay shows a `List` of the 8 named colors plus
      "default" plus "(none)" (sets the field to `None`). Each
      entry is rendered IN ITS OWN COLOR for visual cue.
- [x] Modes `PickingFgColor` and `PickingBgColor` — entered from
      SegmentEditor by pressing `c` / `b`. On `Enter`, write the
      selected color to the segment's `color` or `bg` field. On
      `Esc`, return to Browsing without changes.
- [x] write unit tests: selection writes to the right field;
      "(none)" sets None.
- [x] run `cargo test tui`; all pass before next task

### Task 10: TUI — Powerline toggle + visual indicator

**Files:**
- Modify: `src/tui/widgets/segment_editor.rs` (or a new
  `src/tui/widgets/global_settings.rs`)
- Modify: `src/tui/app.rs`

- [x] Add a global settings line at the top of the SegmentEditor
      pane (or a new pane): `Powerline: [ON|OFF]`. Press `P` (shift+p)
      to toggle. Mark dirty.
- [x] When Powerline is ON, the live preview (Task 8) renders
      with chevrons (handled automatically since `config.powerline`
      flows through).
- [x] write unit tests: toggle flips bit; dirty bit set; serde
      round-trip preserves value.
- [x] run `cargo test tui`; all pass before next task

### Task 11: TUI — save action with backup + validation

**Files:**
- Create: `src/tui/save.rs`
- Modify: `src/tui/app.rs`

- [x] `pub fn save(config: &Config, output_path: &Path) ->
      Result<(), SaveError>`:
      1. Validate via `Config::validate_and_clamp`. If errors,
         return `SaveError::Validation(Vec<ValidationError>)`.
      2. If `output_path` exists, copy to
         `<output_path>.bak` (overwrite prior `.bak`). If copy
         fails, return `SaveError::BackupFailed`.
      3. Serialize via `config::print_config`. (Task 5 of Phase 1
         provided this — emits pretty JSON with $schema field.)
      4. Atomic write: write to `<output_path>.tmp`, then rename
         to `<output_path>`. If write/rename fails, return
         `SaveError::WriteFailed`.
- [x] In `App`, `Ctrl+S` (in Browsing mode) triggers save. On
      success: clear `dirty` bit, show "Saved to <path>" status
      message for 3 seconds. On `SaveError::Validation`: show
      validation errors in a popup; user can fix and retry.
- [x] On `q` while dirty: enter `ConfirmQuit` mode. `y`/`Y` quits
      without saving; `n`/`N` returns to Browsing; `s`/`S` saves
      then quits.
- [x] write unit tests:
      - successful save creates target + backup
      - validation failure returns errors, no file written, no
        backup touched
      - backup-failure path: target file is read-only or backup
        path is read-only — assert clean error message
      - round-trip: save, load, compare — Config round-trips
        through the save path
- [x] run `cargo test tui`; all pass before next task

### Task 12: TUI — help overlay + status bar

**Files:**
- Create: `src/tui/widgets/help.rs`
- Modify: `src/tui/widgets/status.rs` (create if not present)
- Modify: `src/tui/app.rs`
- Modify: `src/tui/draw.rs`

- [x] HelpOverlay: full-screen popup showing keybindings. Trigger
      via `?`. Dismiss via any key.
- [x] Status bar: bottom-most row showing current mode +
      transient messages (3-second timeout) + dirty indicator +
      cursor position.
- [x] write unit tests for help mode entry/exit + status message
      timeout (mock the clock via a `now: u64` parameter on
      `App`).
- [x] run `cargo test tui`; all pass before next task

### Task 13: --configure CLI flag + main.rs dispatch

**Files:**
- Modify: `src/args.rs`
- Modify: `src/main.rs`

- [x] Add `pub configure: bool` and `pub output: Option<PathBuf>`
      fields to `Args`.
- [x] Parse `--configure` (boolean flag) and `--output <path>`.
- [x] In `main.rs`: if `args.configure` is true:
      1. Resolve current config via `config::resolve` (loads user
         config or default).
      2. Determine output path: `args.output` if set, else
         `<config_dir>/cc-myasl/config.json`.
      3. Call `tui::run(config, output_path)`.
      4. Exit 0 on success; exit 1 on TUI error (this is the
         ONE non-render path that may exit non-zero, alongside
         `--check`).
- [x] When `--configure` flag is on, do NOT enter the render flow,
      do NOT touch HTTP/cache.
- [x] LOC check on main.rs. Currently 472 LOC. The `--configure`
      dispatch is small (~10 LOC). Should fit.
      RESULT: 493 LOC after changes — within 500 limit.
- [x] **Locked decision: non-TTY behavior.** Before calling
      `tui::run`, check that BOTH `std::io::stdin().is_terminal()`
      AND `std::io::stdout().is_terminal()` return true (stable
      `IsTerminal` trait since Rust 1.70; `rust-version = "1.83"`
      floor satisfies). The check lives IN `main.rs`, NOT inside
      `tui::run` — so crossterm raw-mode setup never starts when
      there's no real terminal. On non-TTY (either stdin OR stdout
      is not a TTY): print a one-line message to stderr ("cc-myasl
      --configure requires an interactive terminal") and exit 1.
      Test (integration): spawn `cc-myasl --configure --output
      /tmp/<rand>` with stdin redirected from `/dev/null` AND
      stdout captured by `assert_cmd` (which makes stdout a pipe,
      not a TTY). Assert exit code 1 and stderr contains
      "interactive terminal". This exercises BOTH the stdin
      check (since /dev/null is not a TTY) and the stdout check
      (since assert_cmd captures it).
- [x] write tests:
      - `--configure` flag parses correctly (in args.rs tests)
      - `--configure --output /tmp/x.json` parses both
      - integration: see locked decision above.
- [x] run `cargo test`; all pass before next task

### Task 14: Phase 3 integration tests + golden tests

**Files:**
- Create: `tests/golden_phase3.rs`

- [x] `golden_save_writes_valid_json`: build a minimal Config with
      a fg+bg segment + powerline=true; call `tui::save::save` to
      a tempdir; read the file back; deserialize; compare.
- [x] `golden_save_creates_backup`: pre-populate the target file;
      save a different config; assert `<target>.bak` exists with
      OLD content.
- [x] `golden_save_validates_before_write`: try to save a
      hand-constructed Config with `lines.len() = 4`; assert no
      file is written and the SaveError carries the validation
      error.
- [x] `golden_render_with_color_in_phase3_segments`: render a
      config with color/bg fields against the full-payload
      fixture; assert ANSI escape codes present.
- [x] `golden_render_powerline_mode`: render with `powerline:
      true`; assert chevron character present + bg color
      transitions.
- [x] verify Phase-1 `golden_output_unchanged` AND Phase-2
      `golden_phase2.rs` tests STILL PASS — Phase 3 must not
      regress prior phases.
- [x] run `cargo test --test golden_phase3`; all pass

### Task 15: README + CLAUDE.md updates

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`

- [x] README: new "Interactive editor" section documenting
      `cc-myasl --configure`. Show 1-line example. Document
      keybindings (or link to the in-TUI help overlay).
- [x] README: document the new `color`, `bg` fields on segments
      and the `powerline` top-level field. Show one example config
      using each.
- [x] README: note the Powerline mode requires a Nerd Font for the
      chevron glyph.
- [x] CLAUDE.md: update Module-tree section to add `tui/` block.
- [x] CLAUDE.md: locked-dep set already updated in Task 0; verify.
- [x] CLAUDE.md: Hard Invariants section — ADD invariant 11
      that `tui/*.rs` MUST NOT import `crate::api`, `crate::cache`,
      or `crate::git` (TUI consumes Config + Payload via existing
      modules; doesn't need direct backend access). The string-scan
      test and `scripts/check-invariants.sh` grep were added in
      Task 4 — this Task 15 step is the docs sync only.
- [x] CLAUDE.md: under "Things to NOT do", note that the `--configure`
      mode IS allowed to exit non-zero (alongside `--check`). All
      OTHER paths still always exit 0.
- [x] run all gates: fmt, clippy, check-loc, check-invariants,
      shellcheck, cargo test

### Task 16: Verify acceptance + finalize

- [x] verify all requirements from Overview implemented:
  - TUI launches via `--configure` ✓
  - Edit segments + lines + separator ✓
  - Live preview pane ✓
  - Per-segment fg + bg colors ✓
  - Powerline mode toggle + chevron rendering ✓
  - Save with `.bak` backup + pre-save validation ✓
  - Phase 1 + Phase 2 invariants preserved ✓
  - Render mode (piped stdin) UNCHANGED ✓
  - Only `ratatui` + `crossterm` added as new deps ✓
- [x] run full test suite: `cargo test`
      RESULT: 940 passed, 1 ignored (real-HTTPS smoke), 0 failed
- [x] run release build: `cargo build --release`. Confirm size
      ≤ 2.5 MB stripped.
      RESULT: 1,863,704 bytes (~1.78 MB stripped) — within 2.5 MB target.
- [x] manual smoke (REQUIRES a real terminal — cannot be CI-tested):
      NOTE: smoke test cannot run in CI/non-TTY environment. The
      integration test in golden_phase3.rs + non-TTY test in
      golden_phase3.rs (--configure exits 1 when not a TTY) cover the
      automated portion. Manual steps for human verification:
      - `cc-myasl --configure` opens TUI. Edit something. Save.
        Confirm `~/.config/cc-myasl/config.json.bak` was created.
      - Re-open: changes persisted.
      - Try to construct an invalid config (4 lines via direct file
        edit, then `--configure`); assert TUI loads (fallback to
        default per resolver) or shows the validation error.
      Recommended terminals: iTerm2, Terminal.app, VSCode integrated
      terminal. Verify Powerline chevrons with a Nerd Font installed.
- [x] update CLAUDE.md "Reference docs" — add Phase 3 plan.
      RESULT: updated to point to completed path.
- [x] move plan file:
      `git mv docs/plans/2026-05-01-phase3-interactive-tui.md
       docs/plans/completed/2026-05-01-phase3-interactive-tui.md`
      DONE.
- [x] mark all Task 16 checkboxes [x] in the (now-moved) plan.
      DONE.
- [x] commit with message:
      `feat: Phase 3 interactive TUI for config editing
       (ratatui + Powerline + colors)`

## Technical Details

### Schema additions (final shape)

```json
{
  "$schema": "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json",
  "powerline": false,
  "lines": [
    {
      "separator": " · ",
      "segments": [
        {
          "template": "{model}",
          "padding": 0,
          "hide_when_absent": false,
          "color": "cyan",
          "bg": null
        },
        {
          "template": "5h:{five_left}%",
          "hide_when_absent": true,
          "color": "yellow",
          "bg": null
        }
      ]
    }
  ]
}
```

### TUI layout (3-pane vertical, top split horizontally)

```
┌─Lines──────┐ ┌─Segments──────────────────────┐
│ ▶ Line 0   │ │ ▶ 0: {model} (pad=0, vis)     │
│   Line 1   │ │   1: 5h:{five_left}% (hide)   │
│   + add    │ │   + add segment                │
└────────────┘ └────────────────────────────────┘
┌─Editor──────────────────────────────────────────┐
│ Template: {model}                               │
│ Padding:  0                                     │
│ Hide:     [ ]                                   │
│ Color:    cyan       Bg: (none)                 │
│ Powerline: [OFF]                                │
└─────────────────────────────────────────────────┘
┌─Preview─────────────────────────────────────────┐
│ claude-opus-4-7 · 5h:70% · 7d:40%               │
└─────────────────────────────────────────────────┘
[Browsing] q quit · ? help · Ctrl+S save · Tab ●
```

### Powerline rendering algorithm

```text
For each line in config.lines:
    visible_segs = render each segment (skip hidden)
    out = ""
    prev_bg = "default"
    for i, seg in enumerate(visible_segs):
        cur_bg = seg.bg.unwrap_or("default")
        if i == 0:
            // optional leading chevron — pick "skip"
            out += ansi_bg(cur_bg) + ansi_fg(seg.color)
        else:
            // chevron transition: fg = prev_bg, bg = cur_bg
            out += ansi_fg(prev_bg) + ansi_bg(cur_bg) + "" + ansi_fg(seg.color)
        out += seg.value (padded)
        prev_bg = cur_bg
    // trailing chevron: fg = prev_bg, bg = default (terminal)
    out += ansi_fg(prev_bg) + "\x1b[0m" + "" + "\x1b[0m"
    out (line)
```

(The exact algorithm is task 3's responsibility to finalize.)

### Cross-test mutex inventory

Phase 3 introduces no new env vars. The 5 existing mutexes
(`HOME_MUTEX`, `ENV_MUTEX`, `CONFIG_MUTEX`, `COLS_MUTEX`,
`GIT_ENV_MUTEX`) cover all current cases.

## Post-Completion

*Items requiring manual intervention or external systems — no
checkboxes, informational only.*

**Manual verification:**

- TUI smoke in a real terminal across iTerm2 / Terminal.app /
  VSCode integrated terminal / Linux x11 terminal. Resize,
  small/large windows, missing Nerd Font (chevron renders as `?`
  or `□`).
- Validate Powerline visual quality with one of the popular Nerd
  Fonts (FiraCode NF, Hack NF, JetBrains Mono NF).
- Performance check: opening the TUI on a low-end machine, ensure
  it doesn't lag or block.

**Deferred / out-of-scope (future plans):**

- 256-color and hex color encoding (current Phase 3 = named
  ANSI-16 only).
- Theme presets (e.g., "tokyo-night") — manual config copy works
  today.
- Custom-command widget (ccstatusline-style) — would need shell-out
  + caching plus a security review.
- Auto-detect TUI launch when no stdin (ccstatusline pattern) —
  explicitly rejected for safety.
- Mouse support in the TUI — keyboard-only is enough for v1.
- Undo/redo history — single-level undo via "Esc cancels current
  edit" is the only undo today.
