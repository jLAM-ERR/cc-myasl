# Phase 1 — Structured-Config Rewrite (Multi-line + Padding + Separator + Flex Spacer)

## Overview

Replace the current single-line, free-form template-string engine with a
**structured JSON config** that supports multi-line layouts (≤ 3 lines),
per-segment padding, per-line separators, and a width-filling flex spacer.

This is Phase 1 of a 3-phase expansion (Phase 2 = new placeholder set;
Phase 3 = interactive TUI). Phase 1 is layout primitives only — no new
placeholders, no per-segment colour, no Powerline mode.

**Why now:** the user wants 25+ new placeholders, multi-line output,
per-segment styling, and an interactive editor. The existing template
string (`{? · 5h:{five_left}%}`) does not scale cleanly to that surface;
a structured segment array does. We are at v0.1 with no external users
locked in, so a clean break is cheap now and expensive later.

**Key benefits:**

- Multi-line statusline (≤ 3 lines, hard-coded `MAX_LINES` constant).
- Per-line `separator`; per-segment `padding`; per-segment
  `hide_when_absent` collapse semantics.
- `flex` spacer — one per line, fills to terminal width.
- JSON Schema (`cc-myasl.schema.json`) for IDE auto-complete and
  validation.
- User templates dir (`~/.config/cc-myasl/templates/<name>.json`) that
  shadows the 8 built-ins.
- Built-in templates hardcoded as `Config` struct literals in
  `config/builtins.rs` — no JSON parse at startup for built-ins, no
  `templates/*.json` files shipped.

## Context (from discovery)

### Files / components involved

**New files:**
- `src/config/mod.rs` — re-exports + load + precedence resolver.
- `src/config/schema.rs` — serde structs (`Config`, `Line`, `Segment`)
  + validation.
- `src/config/render.rs` — multi-line walk + flex + ANSI-aware width.
- `src/config/builtins.rs` — 8 templates as `Config` struct literals
  + `lookup(name)`.
- `src/config/tests.rs` — unit tests for the new module tree.
- `cc-myasl.schema.json` — JSON Schema, repo root.

**Modified files:**
- `src/main.rs` — wire new resolver, remove `STATUSLINE_FORMAT`.
- `src/args.rs` — add `--config`, `--template`, `--print-config`;
  remove `--format`.
- `src/format/mod.rs` — narrow surface to `render_segment(template, ctx)`;
  drop `DEFAULT_TEMPLATE`, `lookup_template`.
- `src/lib.rs` — add `pub mod config`.
- `src/check.rs` — adapt `report_format` to print resolved config + schema URL.
- `tests/golden.rs` — adapt the 8 existing tests to use
  `--template`/`--config` instead of `--format`. Add 5 new tests.
- `Cargo.toml` — add `terminal_size` dep.
- `README.md` — replace template-string snippet with config snippet.

**Deleted files:**
- `templates/default.txt`, `templates/minimal.txt`, `templates/compact.txt`,
  `templates/bars.txt`, `templates/colored.txt`, `templates/emoji.txt`,
  `templates/emoji_verbose.txt`, `templates/verbose.txt` — built-ins
  move to code.

### Related patterns found

- `format::parser::tokenize` — reuse as-is for mini-template parsing
  inside each segment.
- `format::placeholders::render_placeholder` — unchanged in Phase 1.
- `directories::ProjectDirs::from("", "", "cc-myasl")` — already used
  for cache; reuse for config dir.
- Existing CI gates: `scripts/check-loc.sh`, `scripts/check-invariants.sh`.
- Existing golden-test harness in `tests/golden.rs` with `mockito` +
  `assert_cmd` + `XDG_CACHE_HOME`/`HOME` pinning. Pattern carries over;
  add `XDG_CONFIG_HOME` to the pinned set.

### Dependencies identified

- `terminal_size` crate (NEW) — flex-spacer width detection.
  Justification: ioctl alternative requires `unsafe libc`; spacer
  correctness is a UX requirement.
- All other deps unchanged.

## Development Approach

- **Testing approach:** Regular (code first, then tests in same task).
  Matches existing project style.
- Complete each task fully before moving to the next.
- Make small, focused changes.
- **CRITICAL: every task MUST include new/updated tests** for code
  changes in that task — required, not optional.
  - Unit tests for new functions/methods.
  - Unit tests for modified functions/methods.
  - New test cases for new code paths.
  - Updated test cases when behaviour changes.
  - Cover both success and error scenarios.
- **CRITICAL: all tests must pass before starting next task** — no
  exceptions.
- **CRITICAL: update this plan file when scope changes during
  implementation.**
- Run `cargo test` after each change.
- Run `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
  `bash scripts/check-loc.sh`, `bash scripts/check-invariants.sh`,
  `shellcheck scripts/*.sh` before declaring a task done.

## Testing Strategy

- **Unit tests:** required for every task.
- **Integration tests (`tests/golden.rs`):** existing 8 tests adapted
  to new flag surface; 5 new tests added.
- **Migration safety net:** new `golden_output_unchanged` test pins
  byte-exact output of all 8 built-ins against the current
  `.txt`-template output captured at start of Task 1 (snapshot before
  any changes).
- **Cross-test isolation:** new `XDG_CONFIG_HOME` pinning added
  alongside existing `HOME` / `XDG_CACHE_HOME` pinning.
- No real-network tests in CI; the one `#[ignore]`-marked test in
  `api::tests` is unchanged.

## Progress Tracking

- Mark completed items with `[x]` immediately when done.
- Add newly discovered tasks with ➕ prefix.
- Document issues / blockers with ⚠️ prefix.
- Update plan if implementation deviates from original scope.
- Keep plan in sync with actual work done.

## What Goes Where

- **Implementation Steps** (`[ ]` checkboxes): code, tests,
  documentation updates inside this repo.
- **Post-Completion** (no checkboxes): items requiring external action
  — README publication, schema file URL availability on
  `raw.githubusercontent.com`, downstream user-facing comms.

## Implementation Steps

### Task 0: Capture migration baseline

**Files:**
- Create: `tests/snapshots/builtin-outputs.txt` (temporary — deleted in Task 11)

- [x] run `cargo test` first — baseline must be green before capturing
- [x] checkout HEAD as-is; build release: `cargo build --release`
- [x] for each of `default`, `minimal`, `compact`, `bars`, `colored`,
      `emoji`, `emoji_verbose`, `verbose`: capture output of
      `echo '<full-payload-fixture>' | target/release/cc-myasl --template <name>`
      against the same `RenderCtx`-equivalent stdin used by golden tests
- [x] write all 8 outputs into `tests/snapshots/builtin-outputs.txt`
      (one per line, prefixed with template name + `\t`)
- [x] commit the snapshot — this is the byte-exact baseline that
      Task 11's `golden_output_unchanged` test will assert against
- [x] **Migration-delta policy:** if Task 3's struct-literal translation
      produces output that differs from the snapshot, STOP and inspect
      the delta. Either (a) fix the struct literal to match the .txt
      output exactly (preferred — the migration is meant to be
      bit-identical), or (b) document the delta as an intentional
      modernisation in this plan with ⚠️ prefix and update the
      snapshot. Never silently accept a delta or skip the test.
- [x] no code changes yet — task is pure baseline capture

### Task 1: Add `terminal_size` dependency

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock` (auto)

- [x] add `terminal_size = "0.4"` (or latest stable) to `[dependencies]`
- [x] run `cargo build` to update `Cargo.lock`
- [x] verify no transitive deps added we don't expect (`cargo tree`)
- [x] update CLAUDE.md "locked dep set" section to include
      `terminal_size` with a one-line justification
- [x] no test required for dependency-only task — proceed when
      `cargo build` succeeds

### Task 2: Create `config/schema.rs` — serde structs + validation

**Files:**
- Create: `src/config/schema.rs`
- Modify: `src/lib.rs` (add `pub mod config;`)
- Create: `src/config/mod.rs` (stub: `pub mod schema;`)

- [x] define `pub struct Config { lines: Vec<Line> }` with `#[derive(Deserialize, Serialize, Debug, Clone, PartialEq)]`; tolerate unknown top-level fields (`#[serde(default)]`, no `deny_unknown_fields`); accept `$schema` field (capture as `Option<String>`, ignored at render time)
- [x] define `pub struct Line { separator: String, segments: Vec<Segment> }` with `#[serde(default)]` on `separator`
- [x] define `pub enum Segment { Template(TemplateSegment), Flex(FlexSegment) }` using `#[serde(untagged)]`; `TemplateSegment { template: String, padding: u8, hide_when_absent: bool }`; `FlexSegment { flex: bool }` (must be `true`)
- [x] add `pub const MAX_LINES: usize = 3;` and `pub const MAX_PADDING: u8 = 8;` constants
- [x] add `pub fn validate(&self) -> Result<(), Vec<ValidationError>>` on `Config`: rejects `lines.len() > MAX_LINES`; rejects more than one flex per line; clamps padding > MAX_PADDING with a warning rather than rejection (validation collects warnings + errors separately)
- [x] add `pub struct ValidationError { line_index, segment_index, kind }` with `kind` enum (`TooManyLines`, `MultipleFlex`, `EmptyTemplate`, etc.); does not derive Error — caller maps to `crate::error::Error` if needed
- [x] write unit tests for serde round-trip (full config, minimal config, with $schema, without)
- [x] write unit tests for validation rejections (`lines.len() = 4`, two flex on one line, segment with both `template` and `flex: true`, segment with neither)
- [x] write unit tests for padding clamp (padding=99 clamps to MAX_PADDING with warning)
- [x] write unit tests for unknown-field tolerance (extra fields in segment, line, top-level)
- [x] run `cargo test config::schema -- --nocapture`; all pass before next task

⚠️ Followup commit `28d581c`: addressed auto-reviewer findings — TooManyLines line_index → Option<usize>; Line::segments gets #[serde(default)]; removed dead-code variants AmbiguousSegment and EmptyTemplate; added Segment variant-order doc comment.

### Task 3: Create `config/builtins.rs` — 8 hardcoded templates + lookup

**Files:**
- Create: `src/config/builtins.rs`
- Modify: `src/config/mod.rs` (add `pub mod builtins;`)

- [x] add `pub fn lookup(name: &str) -> Option<Config>` matching all 8
      names: `default`, `minimal`, `compact`, `bars`, `colored`,
      `emoji`, `emoji_verbose`, `verbose`
- [x] implement each template as a function returning `Config` with
      struct literal — bit-identical translation of the corresponding
      `templates/*.txt` content (separator: "" with explicit
      punctuation in each segment template; `hide_when_absent: true`
      on segments that were inside `{? ... }` in the .txt original)
- [x] add small builder helpers — keep them on `TemplateSegment` (NOT on the `Segment` enum) so the type system enforces "can only set `hide_when_absent` on a Template variant":
      - `TemplateSegment::new(s: &str) -> TemplateSegment` (constructor; padding=0, hide_when_absent=false)
      - `TemplateSegment::with_hide_when_absent(mut self) -> TemplateSegment` (consumes self, sets flag, returns TemplateSegment)
      - `TemplateSegment::with_padding(mut self, n: u8) -> TemplateSegment`
      - `impl From<TemplateSegment> for Segment` so the chain `TemplateSegment::new("x").with_hide_when_absent().into()` produces a `Segment::Template(...)`. Built-in declarations call `.into()` on each segment to upgrade to the enum variant. Avoids the "what does `with_hide_when_absent` mean on Flex?" ambiguity entirely — the method does not exist on `Segment` at all.
- [x] keep `lookup` and the 8 functions under the 500 LOC ceiling for
      this file; if forced over, split into `builtins/mod.rs` +
      `builtins/templates.rs` (the helper file). Note as ⚠️ in plan if
      this happens
- [x] write unit tests for `lookup`: every name returns `Some(Config)`; unknown names return `None`
- [x] write unit tests asserting every returned Config validates without errors and has ≥ 1 segment on line 0
- [x] write unit tests for builder helpers: `TemplateSegment::new("x")` produces expected struct; `.with_hide_when_absent()` flips the flag and chains; `.with_padding(n)` sets padding and chains; `From<TemplateSegment> for Segment` produces `Segment::Template` variant
- [x] run `cargo test config::builtins`; all pass; LOC check
      `wc -l src/config/builtins.rs` < 500 — must pass before next task

### Task 4: Create `config/render.rs` — multi-line render with flex

**Files:**
- Create: `src/config/render.rs`
- Modify: `src/config/mod.rs` (add `pub mod render;`)
- Modify: `src/format/mod.rs` (add `pub fn render_segment(template: &str, ctx: &RenderCtx) -> Option<String>`)

- [x] in `format::mod`, add `pub fn render_segment(template: &str, ctx: &RenderCtx) -> Option<String>` that tokenises the template and returns `None` if any required (non-optional-block) placeholder resolves to None or empty, else `Some(rendered)`. This mirrors the existing `{? ... }` collapse semantics applied to the whole template string
- [x] in `config::render`, add `pub fn render(config: &Config, ctx: &RenderCtx) -> String`
- [x] for each line (capped at `MAX_LINES`): walk segments; for each segment, call `format::render_segment` if `Segment::Template`, push `Some(s)` or `None` based on `hide_when_absent` and the result; for `Segment::Flex`, push a placeholder marker `"\x00FLEX\x00"`
- [x] apply padding inside each Some(s) cell (left + right space repetition by `padding` count)
- [x] join visible segments (Some only) with `line.separator`; drop None — separator slot collapses with the hidden segment
- [x] resolve flex marker: compute `visible_width(line_str_minus_marker)` (ANSI-stripped — see helper); query `terminal_size::terminal_size()` returning `Option<(Width, Height)>`; replace marker with `' '.repeat(max(1, term_width.saturating_sub(natural_width)))`
- [x] add `fn visible_width(s: &str) -> usize` that strips CSI sequences (`\x1b[<args>m`) and counts grapheme columns. Phase 1 simplification: use byte length minus ANSI sequence bytes (no full grapheme-aware counting; OK for ASCII + ANSI escapes)
- [x] join all rendered lines with `\n`
- [x] declare `pub(crate) static COLS_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());` in `config::render` (mirroring `format::ENV_MUTEX`). Every test that reads or writes `STATUSLINE_TEST_COLS` MUST acquire this mutex; tests that mutate the env var MUST restore the original value before releasing. Cross-reference in CLAUDE.md (Task 12 will document this).
- [x] write unit tests for happy-path 2-line config rendering
- [x] write unit tests for flex with explicit terminal width via `STATUSLINE_TEST_COLS` (acquire `COLS_MUTEX`); flex without width → 1 space (env unset, `terminal_size` returns None in test env)
- [x] write unit tests for separator-drop-with-hidden-segment behaviour
- [x] write unit tests for padding inside cell (padding>0 with both sides; padding=0 omits both)
- [x] write unit tests for ANSI-stripped width counting (`{five_color}5h{reset}` should count as 2 cols, not the full byte length)
- [x] write unit tests for MAX_LINES truncation guard (config bypassing validation with 5 lines: render still doesn't panic and emits at most MAX_LINES lines)
- [x] run `cargo test config::render`; all pass before next task

⚠️ Followup commit `b921195`: defense-in-depth multi-flex handling (first FLEX_MARKER gets real fill, extras become single space, fill reduced by extra-marker count so total width never exceeds term_width); `#[cfg(test)]`-gated `test_cols_override()` so `STATUSLINE_TEST_COLS` is never read in production builds; corrected misleading comment in `builtins_tests.rs` about `render_segment` behavior when placeholder is absent.

### Task 5: Create `config/mod.rs` — load + precedence resolver

**Files:**
- Modify: `src/config/mod.rs` (add load + resolver)
- Create: `src/config/tests.rs` (alongside, for cross-module tests)

- [x] add `pub fn resolve(args: &Args) -> Config` returning resolved config. Reads `STATUSLINE_CONFIG` directly via `std::env::var` — no `EnvAccess` trait. Never errors — falls back to default on any failure, emitting trace events via the existing `Trace` if --debug.
- [x] declare `#[cfg(test)] pub(crate) static CONFIG_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());` in `src/config/mod.rs`. Tests that read or mutate `STATUSLINE_CONFIG` or `XDG_CONFIG_HOME` MUST acquire `CONFIG_MUTEX` and restore prior values before releasing. Mirrors `creds::HOME_MUTEX`, `format::ENV_MUTEX`, and `config::render::COLS_MUTEX` — one mutex per logical env-var group. Do NOT reuse `creds::HOME_MUTEX`; the env vars are unrelated and conflating them causes false serialization between unrelated tests.
- [x] precedence order:
      1. `args.config_path` (if Some) → `Config::from_file(path)`
      2. `args.template_name` (if Some) → check `<config_dir>/templates/<name>.json` first, then `builtins::lookup(name)`
      3. `std::env::var("STATUSLINE_CONFIG")` (if Some and non-empty) → same as step 1
      4. default config file at `<config_dir>/cc-myasl/config.json` (using `directories::ProjectDirs::from("", "", "cc-myasl").config_dir()`)
      5. `Config::default()` (= `builtins::lookup("default").unwrap()`)
- [x] add `Config::from_file(path: &Path) -> Result<Config, Error>`: read file, parse JSON, validate; bubble up errors to the resolver for trace + fallback
- [x] add `pub fn user_template_path(config_dir: &Path, name: &str) -> PathBuf` helper for step 2's first-check
- [x] add `Config::default()` returning `builtins::lookup("default").unwrap()`
- [x] add `pub fn print_config(config: &Config) -> String` — serialize as pretty JSON with `$schema` field set to the canonical raw.githubusercontent URL; output stable / sorted-keys
- [x] **Cross-reference Task 7:** Task 7's `--config`/`--template` conflict behaviour MUST match the precedence order above (config_path wins over template_name). Verify in code review of Task 7.
- [x] write unit tests for each precedence layer in isolation: use tempdir for filesystem, acquire `config::CONFIG_MUTEX` for env-var manipulation, restore prior `STATUSLINE_CONFIG` / `XDG_CONFIG_HOME` after each test
- [x] write unit test: fall-back-on-parse-error keeps render mode running and emits trace
- [x] write unit test: user template shadows built-in (write `default.json` in tempdir templates dir, assert it wins)
- [x] write unit test: `Config::default()` is valid
- [x] write unit test: `print_config` is deterministic and includes `$schema` field; round-trip through `from_file` is byte-stable (after pretty-print)
- [x] write invariant test: string-scan `src/config/*.rs` files; assert no `use crate::api` and no `use crate::cache` substring (mirrors `format::placeholders::tests` pattern)
- [x] run `cargo test config`; all pass before next task

⚠️ Followup commit (see below): addressed reviewer findings on Task 5 commit `cd04575` plus hardener-found bugs in `9dcebb0`: (Fix 1) path traversal in `user_template_path` — now validates name chars, returns `Option<PathBuf>`, rejects `/`, `\`, `..`, `.`; (Fix 2) `from_file` now rejects non-object JSON (array, string, number, null) with explicit `ConfigParse` error; (Fix 3) `resolve_user_template` records `trace.error` when existing file is corrupt; (Fix 4) `load_default_config_file` records `trace.error` when existing file is corrupt; (Fix 5) `ConfigSource` moved to `debug.rs`, `Trace::config_source` is now `Option<ConfigSource>` enum; (Fix 6) `args.template` renamed to `args.template_name`; (Fix 7) `resolve_layer2_unknown_template_falls_through` asserts `trace.error.is_some()` and contains the unknown name; (Fix 8) `user_template_path` is now `pub(crate)`.

### Task 6: Trim `format/mod.rs` — narrow surface

**Files:**
- Modify: `src/format/mod.rs`
- Modify: `src/check.rs` (still calls `format::render` — must stay compiling)

- [x] add the new public `render_segment` function (already done in Task 4 if not earlier; either way confirm signature + tests)
- [x] delete `pub const DEFAULT_TEMPLATE`
- [x] delete `pub fn lookup_template` (callers move to `config::builtins::lookup`)
- [x] **DO NOT delete `pub fn render(template, ctx) -> String` yet** — `check.rs:201` still calls it. Mark `#[deprecated(note = "Phase-1 transition only — replaced by config::render in Task 10")]` and keep the function body intact. Final removal happens after Task 10 confirms `check.rs` no longer references it.
- [x] add a string-scan invariant test in `format::mod::tests`: assert no `src/format/**/*.rs` file contains `use crate::config` (one-way dependency)
- [x] update existing `format::tests` that referenced `DEFAULT_TEMPLATE` or `lookup_template` to use the new path or delete if obsolete
- [x] suppress the `deprecated` warning ONLY in `check.rs` for the transitional call site (`#[allow(deprecated)]` at the function level enclosing the `format::render` call). The CI gate runs `cargo clippy --all-targets -- -D warnings` which passes `-D warnings` to the compiler, making the rustc `deprecated` lint a hard error — without this suppression, the build will fail.
- [x] run `cargo build`; must compile cleanly
- [x] run `cargo clippy --all-targets -- -D warnings`; must pass — confirms the `#[allow(deprecated)]` is correctly scoped
- [x] run `cargo test format`; all pass; LOC check; ensure file shrinks (was 320; should drop to ~200) — must pass before next task

### Task 7: Update `args.rs` — new flags

**Files:**
- Modify: `src/args.rs`

- [x] add `pub config_path: Option<PathBuf>` and `pub template_name: Option<String>` fields to `Args`
- [x] add `pub print_config: bool` flag field
- [x] parse `--config <path>` to populate `config_path`
- [x] parse `--template <name>` to populate `template_name`
- [x] parse `--print-config` boolean flag (no value)
- [x] remove parsing of `--format <string>` from the parser
- [x] update `--help` text to document the new surface and remove `--format`
- [x] **Conflict handling:** parser does NOT error on `--config X --template Y` — both fields are populated and the resolver in Task 5 picks `config_path` (precedence step 1) over `template_name` (precedence step 2). `--help` documents this precedence so users aren't surprised. This keeps a single source of truth for precedence (the resolver), not split across parser + resolver.
- [x] write unit tests for each new flag parsed correctly (standalone)
- [x] write unit tests for combined flags (`--config X --template Y` populates both; `--print-config` combined with `--config`)
- [x] write unit tests for error cases: missing value after `--config`; missing value after `--template`
- [x] run `cargo test args`; all pass before next task

⚠️ Followup commit `f157332`: parser now peeks ahead and treats a `--flag` token following `--config`/`--template` as dangling (pushed to `unknown`) rather than greedy-consuming it as a value. Tests split to `src/args_tests.rs`; 3 new tests added; hardener's `#[ignore]` test un-ignored.

### Task 8: Wire main.rs — use new resolver, drop STATUSLINE_FORMAT

**Files:**
- Modify: `src/main.rs`

- [x] in `run_render`: replace template-string resolution path with `let config = config::resolve(&args, &env);`
- [x] replace existing `format::render(template, &ctx)` call with `config::render::render(&config, &ctx)`
- [x] remove all references to `STATUSLINE_FORMAT` env var
- [x] handle `--print-config` mode: load resolved config, print as pretty JSON via `config::print_config`, exit 0 (still in render-mode constraints — don't error on missing data; print whatever resolved)
- [x] update `Trace` struct fields if needed (drop `template_source` if it referred to format-string resolution; add `config_source` enum: `CliPath`, `CliTemplate`, `Env`, `DefaultFile`, `Embedded`)
- [x] write adversarial test: render mode with corrupt config file does not exit non-zero (exit code 0); add to existing `main::tests`
- [x] write test: `--print-config` outputs valid JSON parseable back into `Config`; output contains `$schema` field
- [x] run `cargo test`; all pass; verify `main.rs` LOC under 500

⚠️ Followup commit 33260e5: fix(main): emit trace under --print-config --debug — `--print-config` branch was building a `Trace` but never calling `trace.emit(args.debug)`; fixed by adding the emit call before `process::exit(0)` and adding `print_config_debug_emits_trace_to_stderr` unit test in `main_tests.rs`.

### Task 9: Add `cc-myasl.schema.json` at repo root

**Files:**
- Create: `cc-myasl.schema.json`

- [x] write JSON Schema (draft-07 or 2020-12) describing the `Config` shape
- [x] enforce `lines: maxItems: 3`
- [x] enforce `padding: minimum: 0, maximum: 8`
- [x] enforce segment as `oneOf: [TemplateSegment, FlexSegment]` with `additionalProperties: false` on each variant
- [x] enforce one-flex-per-line via JSON Schema where feasible (note: JSON Schema can't express "at most one element of array X has property Y true" cleanly; document this as a soft constraint enforced by `Config::validate`)
- [x] include description fields on every top-level property — these surface in IDE tooltips
- [x] **Decision (was deferred):** NO new `jsonschema` dev-dep. Schema validation in tests is narrow: assert the schema file parses as valid JSON, and spot-check 3 specific constraints by direct `serde_json::Value` traversal: `properties.lines.maxItems == 3`, padding `maximum == 8`, segment `oneOf` has exactly 2 entries. Full schema-conformance verification is left to the IDE-side experience (the contract users actually rely on). Rationale: validating arbitrary configs against a JSON Schema in Rust without a dedicated lib is a project unto itself; the spot-checks catch the constraints we care about regressing.
- [x] write unit test: schema file parses as valid JSON via `serde_json::from_str::<serde_json::Value>(...)` on the included file content
- [x] write unit test: spot-check `lines.maxItems == 3`
- [x] write unit test: spot-check padding `maximum == 8`
- [x] write unit test: spot-check segment `oneOf` has exactly 2 variants
- [x] write unit test: every shipped built-in serializes to JSON whose top-level keys match those declared in the schema's `required` / `properties` lists (sanity check, not full conformance)
- [x] run tests; all pass before next task

⚠️ Followup commit (Task 9 followup): fix(schema): relax segment additionalProperties to match runtime tolerance — `TemplateSegment` and `FlexSegment` had `additionalProperties: false` in schema but the Rust structs do NOT use `#[serde(deny_unknown_fields)]`; test `unknown_field_in_template_segment_is_ignored` proved runtime tolerates extras; schema was rejecting them causing false-positive IDE errors. Changed both to `additionalProperties: true`. Added top-level `description` note about `oneOf` vs `#[serde(untagged)]` stricture. Added two new tests in `tests_d.rs`: `schema_template_segment_additional_properties_is_true_or_unset` and `schema_flex_segment_additional_properties_is_true_or_unset`.

### Task 10: Update `check.rs` and `--check` output

**Files:**
- Modify: `src/check.rs`

- [x] adapt `report_format` (or rename to `report_config`) to print:
  - resolved config source (which precedence layer won)
  - active config as pretty JSON (or summary if too long)
  - list of all 8 built-in names
  - schema URL: `https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json`
  - user templates dir path (whether it exists or not)
- [x] keep `--check` exit codes intact (this is the only path allowed to exit non-zero)
- [x] write tests: `--check` runs against tempdir config, against missing config, against malformed config; output contains expected strings
- [x] run `cargo test check`; all pass

### Task 11: Migrate existing 8 golden tests + add 5 new ones

**Files:**
- Modify: `tests/golden.rs`
- Delete: `tests/snapshots/builtin-outputs.txt` (move into test code as inline constants)
- Delete: `templates/default.txt`, `templates/minimal.txt`, `templates/compact.txt`, `templates/bars.txt`, `templates/colored.txt`, `templates/emoji.txt`, `templates/emoji_verbose.txt`, `templates/verbose.txt`

- [x] adapt the existing 8 golden tests: replace any `--format <string>` invocation with `--template <name>` or `--config <tempfile>`. Same fixtures, same assertions, same hot-path / OAuth / 401 / 429 / 500 / malformed coverage
- [x] add `golden_output_unchanged`: load the snapshot from Task 0; for each of 8 built-in names, render via release binary and assert byte-exact match. THIS is the migration safety net
- [x] add `golden_multiline_output`: 2-line config; pipe full payload; assert output contains exactly one `\n` and per-line content
- [x] add `golden_flex_spacer`: config with one flex; assert flex region is ≥1 space (STATUSLINE_TEST_COLS is not readable by production binary — structural assertion instead)
- [x] add `golden_user_template_shadows_builtin`: write `<tempdir>/cc-myasl/templates/default.json` with a sentinel string; pin `XDG_CONFIG_HOME=<tempdir>`; invoke with `--template default`; assert sentinel appears, NOT the built-in default output
- [x] add `golden_invalid_config_falls_back`: write a config with `lines.len() = 4`; pin via `--config`; assert exit 0 + default-template output
- [x] run `cargo test --test golden`; all pass — must pass before deleting anything
- [x] only AFTER tests pass: delete the 8 `templates/*.txt` files
- [x] only AFTER tests pass: delete `tests/snapshots/builtin-outputs.txt` (snapshots are now inlined into `golden_output_unchanged`)
- [x] re-run `cargo test --test golden` after deletions to confirm nothing referenced the removed files

### Task 12: Update README + CLAUDE.md + invariant scripts

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`
- Possibly modify: `scripts/check-invariants.sh`

- [x] README: replace `--format`/`STATUSLINE_FORMAT` examples with `--config`/`--template`/`--print-config`; document the user templates dir; document the schema URL; show one example multi-line config. **Do NOT document `STATUSLINE_TEST_COLS` in README** — it's a test-only escape hatch and exposing it invites users to set it in prod.
- [x] CLAUDE.md: update Module-tree section; update Hard-Invariants section (add the two new one-way-import invariants); update "Things to NOT do" if relevant; update locked-dep set (add `terminal_size`); document `STATUSLINE_TEST_COLS` and `config::render::COLS_MUTEX` in the "Cross-test env-var serialization" section alongside HOME_MUTEX and ENV_MUTEX
- [x] scripts/check-invariants.sh: add greps for the new one-way-import invariants if feasible in shell (otherwise document them as Rust-test-only)
- [x] run `bash scripts/check-loc.sh`; all files under 500 LOC
- [x] run `bash scripts/check-invariants.sh`; passes
- [x] run `shellcheck scripts/*.sh`; passes (shellcheck not installed on this machine; CI gate remains)
- [x] run `cargo fmt --check`; clean
- [x] run `cargo clippy --all-targets -- -D warnings`; clean
- [x] run `cargo test`; full suite green

### Task 13: Verify acceptance criteria

- [ ] verify all requirements from Overview implemented:
  - structured JSON config ✓
  - multi-line (≤ 3) ✓
  - per-segment padding ✓
  - per-line separator with hide-collapse ✓
  - flex spacer ✓
  - JSON Schema for IDE ✓
  - user templates dir shadowing built-ins ✓
  - hardcoded built-ins (no `templates/*.json`) ✓
  - `--config` / `--template` / `--print-config` / `STATUSLINE_CONFIG` ✓
  - `--format` / `STATUSLINE_FORMAT` removed ✓
  - `terminal_size` added (only new dep) ✓
- [ ] verify edge cases handled:
  - corrupt config falls back to default ✓
  - missing config file uses embedded default ✓
  - flex without terminal width degrades to 1 space ✓
  - invalid `lines.len() > 3` rejected with warning ✓
  - hidden segment drops adjacent separator ✓
- [ ] run full test suite: `cargo test`
- [ ] run release build: `cargo build --release`; size still ≤ 1.5 MB
      stripped (CLAUDE.md target)
- [ ] manual smoke: `echo '{}' | ./target/release/cc-myasl --template default`
      and `echo '{}' | ./target/release/cc-myasl --print-config` produce
      sensible output
- [ ] verify test coverage hasn't regressed (rough: number of tests
      grew, none deleted without replacement)

### Task 14: [Final] Move plan to completed

- [ ] update CLAUDE.md "Reference docs" section: add link to this plan
      under completed
- [ ] move `docs/plans/2026-05-01-phase1-structured-config.md` to
      `docs/plans/completed/2026-05-01-phase1-structured-config.md`
- [ ] commit with `feat: structured JSON config (multi-line, padding, separator, flex spacer)`

## Technical Details

### Final segment shape (serde, untagged enum)

```rust
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum Segment {
    Template(TemplateSegment),
    Flex(FlexSegment),
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct TemplateSegment {
    pub template: String,
    #[serde(default)]
    pub padding: u8,
    #[serde(default)]
    pub hide_when_absent: bool,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub struct FlexSegment {
    pub flex: bool,  // must be true; validation catches false
}
```

### Render-flow pseudocode

```text
config::render(config, ctx) -> String:
    out_lines = []
    for line in config.lines.iter().take(MAX_LINES):
        rendered: Vec<Option<String>> = []
        for segment in line.segments:
            match segment:
                Template(t):
                    s = format::render_segment(&t.template, ctx)
                    if s.is_some_and(|v| !v.is_empty()):
                        rendered.push(Some(pad(s.unwrap(), t.padding)))
                    elif t.hide_when_absent:
                        rendered.push(None)
                    else:
                        rendered.push(Some(pad("", t.padding)))
                Flex(_):
                    rendered.push(Some("\x00FLEX\x00".to_owned()))
        line_str = rendered.into_iter().flatten().collect::<Vec<_>>().join(&line.separator)
        if line_str.contains("\x00FLEX\x00"):
            width = std::env::var("STATUSLINE_TEST_COLS").ok()
                .and_then(|s| s.parse::<usize>().ok())
                .or_else(|| terminal_size::terminal_size().map(|(w,_)| w.0 as usize))
                .unwrap_or(80);
            natural = visible_width(&line_str.replace("\x00FLEX\x00", ""))
            fill = max(1, width.saturating_sub(natural))
            line_str = line_str.replace("\x00FLEX\x00", &" ".repeat(fill))
        out_lines.push(line_str)
    out_lines.join("\n")
```

### `STATUSLINE_TEST_COLS` env var

A test-only escape hatch read by `config::render` for deterministic
flex-spacer tests. Documented in CLAUDE.md "Cross-test env-var
serialization" section ONLY (not README). Production code path: env
var unset → `terminal_size::terminal_size()` queried → `Some((w, _))`
→ use w; else fall back to 80.

Tests that read or write this var MUST acquire `config::render::COLS_MUTEX`
and restore the prior value before releasing — same pattern as
`format::ENV_MUTEX` / `creds::HOME_MUTEX`.

### Env-var access in resolver

`config::resolve` reads `STATUSLINE_CONFIG` directly via `std::env::var`,
not via a trait. Tests serialize this access via a new
`config::CONFIG_MUTEX` declared in `src/config/mod.rs` (test-gated,
`pub(crate)`), mirroring `creds::HOME_MUTEX`, `format::ENV_MUTEX`, and
`config::render::COLS_MUTEX`. The codebase pattern is one mutex per
logical env-var group; do not reuse `HOME_MUTEX` — it covers `HOME`
only, and conflating env vars causes unnecessary cross-module test
serialization.

CLAUDE.md "Cross-test env-var serialization" section will list four
mutexes after Phase 1: `HOME_MUTEX` (HOME), `ENV_MUTEX`
(STATUSLINE_RED/_YELLOW), `CONFIG_MUTEX`
(STATUSLINE_CONFIG/XDG_CONFIG_HOME), `COLS_MUTEX` (STATUSLINE_TEST_COLS).

### Migration ordering

Tasks must run in order. Specifically:
- Task 0 captures baseline before ANY code change.
- Task 11 verifies migration via the baseline → can only run after
  Tasks 1-10 complete.
- Task 6 (trim format) must come AFTER Task 4 (because render.rs
  depends on `format::render_segment`).

## Post-Completion

*Items requiring manual intervention or external systems — no checkboxes, informational only.*

**Manual verification:**

- Manual smoke in real Claude Code session: install latest binary,
  add to settings.json statusline command, verify multi-line renders
  correctly across iTerm2 / Terminal.app / VSCode integrated terminal.
- Performance check: `hyperfine --warmup 5 'echo "{}" | target/release/cc-myasl'`
  — cold start should be unchanged (config resolution adds < 1 ms).

**External system updates:**

- Schema URL `https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json`
  becomes live the moment the PR merges to main; until then, IDE
  validation in user configs that reference this URL won't work
  (graceful degradation — IDE just falls back to no validation).
- README snippet update is the user-facing comms; no separate
  release-notes channel.

**Future phases (NOT in this plan):**

- Phase 2: new placeholder set — git, git worktree, session, tokens,
  context, clock, separator, current-cwd. Will require new modules
  for git introspection (likely shell out to `git`) and transcript
  parsing. New plan when Phase 1 lands.
- Phase 3: interactive TUI — `cc-myasl --configure` opens a TUI for
  editing `~/.config/cc-myasl/config.json`. Likely needs `ratatui` +
  `crossterm` deps; explicit dep approval.
