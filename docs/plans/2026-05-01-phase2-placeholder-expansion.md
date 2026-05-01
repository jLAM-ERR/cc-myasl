# Phase 2 — Placeholder Expansion (stdin extension + git module)

## Overview

Phase 2 expands cc-myasl's render context to cover the full surface of
Claude Code's current stdin JSON payload PLUS git repository
introspection via `gix`. All new placeholders that the upstream
`ccstatusline` project ships (minus `skills` and `git_pr`) become
available in user configs.

This is Phase 2 of a 3-phase expansion. Phase 1 (structured-config
rewrite) merged in PR #4. Phase 3 (interactive TUI) is a separate
plan to be written later.

**Why now:** Phase 1's structured-config engine is in place but only
exposes ~10 placeholders (`{model}`, `{cwd}`, `{five_*}`, `{seven_*}`,
`{extra_*}`, `{state_icon}`, `{reset}`). Users want richer
statuslines: tokens, context %, session clock, vim mode, git branch,
worktree info. Most of this data is already in Claude Code's stdin
payload (just unparsed), and the rest is git repo state.

**Why this design:** the upstream `ccstatusline` (npm) reads its
data from Claude Code's status JSON for almost every widget. The
official Claude Code statusline docs
(https://code.claude.com/docs/en/statusline) confirm a rich stdin
contract: `cost.total_duration_ms` IS the session clock,
`context_window.*` carries tokens and context %, `effort.level`
carries thinking effort, `vim.mode`, `version`, `output_style.name`
are all there. So Phase 2's stdin extension is mostly serde
struct-expansion + new `RenderCtx` primitives + new placeholders —
no transcript JSONL parsing, no settings.json reading.

For git data (branch, root, status counters), Phase 2 adds the `gix`
crate as the second new top-level dep (after `terminal_size` from
Phase 1). Slim build via `default-features = false` to limit binary
bloat. Justification: shell-out to `git` works but parsing
`git worktree list --porcelain` and `git status --porcelain=2` from
Rust is fragile, the project already has a precedent for crate-based
solutions (Phase 1's `terminal_size`), and Starship — the gold-
standard Rust prompt — uses `gix` for repository discovery.

**Key benefits:**

- ~30 new placeholders, all sourced from existing Claude Code stdin
  fields or git repo state.
- No transcript JSONL parser (deferred until proven necessary).
- No settings.json reader (deferred — every metadata field we care
  about is in stdin).
- gix-based git module: branch, root, status counters with graceful
  "no info" fallback when not in a repo.
- Placeholders use the existing `format/placeholders/` infrastructure
  — no engine changes.
- Built-in templates extended (or new ones added) to showcase the
  new placeholders.

## Context (from discovery)

### Files / components involved

**New files:**
- `src/git/mod.rs` — gix-based repo discovery + branch + root.
- `src/git/status.rs` — status counters (changes, staged, etc.).
- `src/git/tests.rs` — unit tests for the git module.

**Modified files:**
- `src/payload.rs` — extend `Payload` struct with ~30 new optional
  fields covering Claude Code's current stdin contract.
- `src/format/placeholders/mod.rs` — extend `RenderCtx` with
  primitives mirroring the new payload fields; add ~30 new
  placeholder match arms.
- `src/format/placeholders/tests.rs` — unit tests per placeholder.
- `src/main.rs` — extend the `Payload → RenderCtx` mapping in
  `run_render` to populate the new fields. Wire the git module
  call (lazy, only when any `{git_*}` placeholder is referenced
  in the resolved config).
- `src/lib.rs` — add `pub mod git;`.
- `src/config/builtins.rs` — optionally extend 1-2 built-in templates
  to use the new placeholders, OR add a new built-in template
  (`rich`) that showcases tokens + context bar + git.
- `cc-myasl.schema.json` — no schema changes needed (placeholder
  catalogue is documentation, not schema-enforced).
- `README.md` — extend the placeholder list.
- `CLAUDE.md` — update Module-tree section, locked-dep set
  (add `gix`).
- `Cargo.toml` — add `gix` with `default-features = false` and
  minimal feature set.

### Related patterns found

- Phase 1's `RenderCtx` discipline: primitives only
  (`Option<String>`, `Option<f64>`, `Option<u64>`, `Option<bool>`,
  `Option<PathBuf>`). Mapping from richer source types lives in
  `main.rs::run_render`. Phase 2 follows this religiously.
- `format/placeholders/` invariant: no `use crate::api;` or
  `use crate::cache;` (string-scan test). Phase 2 must extend to
  `no use crate::git;` either — git data flows via primitives, not
  via direct module import.
- Existing serde pattern: `Payload` uses `#[serde(default)]`,
  no `deny_unknown_fields`, accepts unknown fields silently.
  Phase 2 extends this to all new nested structs.
- gix discovery: `gix::discover(path)` returns the repo if found
  (walks parents); minimal API surface for our needs.

### Dependencies identified

- `gix` crate (NEW, slim build) — git repo discovery + reads.
  Justification: avoids fragile shell-out parsing; matches
  Starship's pattern; Phase 1 already established the
  "small-justified-dep" precedent with `terminal_size`.
  Configure as:
  ```toml
  gix = { version = "0.70", default-features = false, features = [] }
  ```
  Start with `features = []` and add only the minimum needed for
  the API calls (Task 9 finalises the set after a dep-tree audit;
  expected to be `revision`, `status`, or similar — explicitly NOT
  HTTP/SSH/TLS/async/signing).
- All other deps unchanged.

## Development Approach

- **Testing approach:** Regular (code first, then tests in same
  task). Matches Phase 1 pattern and project convention.
- Complete each task fully before moving to the next.
- Make small, focused changes.
- **CRITICAL: every task MUST include new/updated tests** for code
  changes in that task — required, not optional.
  - Unit tests for new functions / fields / placeholders.
  - Tests cover both success and absence (None) scenarios.
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
- **Integration tests (`tests/golden.rs` or sibling files):**
  Add at least one new golden test exercising a config that uses
  several new placeholders; verify rendered output against a
  pinned `RenderCtx`.
- **Fixture extension:** the existing test fixtures pipe a small
  `rate_limits`-only stdin payload. Phase 2 adds a richer fixture
  (`tests/fixtures/full-payload.json`) that populates every new
  field, used as the standard input for new tests.
- **Cross-test isolation:** Phase 2 doesn't add new env vars, so no
  new mutex needed. The 4 existing mutexes
  (`HOME_MUTEX`, `ENV_MUTEX`, `CONFIG_MUTEX`, `COLS_MUTEX`) cover
  all current cases.
- **gix tests:** the git module tests use `tempfile`-created tiny
  repos (`git init` via `std::process::Command` is fine for test
  fixtures — we shell out only in tests, not in production code).
- No real-network tests in CI.

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
  action — README publication, deferred follow-ups (skills, PR
  detection).

## Implementation Steps

### Task 0: Add full-payload fixture

**Files:**
- Create: `tests/fixtures/full-payload.json`

- [ ] run `cargo test` — baseline must be green before any change
- [ ] hand-craft a comprehensive `tests/fixtures/full-payload.json`
      based on the canonical Claude Code stdin contract documented
      at https://code.claude.com/docs/en/statusline. Include every
      field this plan will surface as a placeholder. Use stable
      values (no clock-of-day, no random IDs).
- [ ] commit fixture. Commit message:
      `chore(fixtures): full Claude Code stdin payload for Phase 2`
- [ ] Note: NO standalone "baseline" snapshot file is captured. The
      Phase-1 `golden_output_unchanged` test ALREADY pins the
      BEFORE state for the 8 built-in templates against the
      pre-Phase-1 .txt outputs; Phase 2 doesn't modify those
      built-ins, so the existing test continues to assert
      bit-identity throughout.

### Task 1: Extend `Payload` struct with new stdin fields

**Files:**
- Modify: `src/payload.rs`

- [ ] add nested struct types for: `Cost`, `ContextWindow`,
      `ContextWindowCurrentUsage`, `Effort`, `Thinking`, `OutputStyle`,
      `Vim`, `Agent`, `Worktree`. Each with `#[serde(default)]`,
      `Deserialize`, `Debug`, `Default`, `PartialEq`, all fields
      `Option<T>` where T is primitive.
- [ ] **Note:** `session_id: Option<String>` is ALREADY present in
      the existing `Payload` struct (`src/payload.rs:49`). Do NOT
      add it again. Tasks 2/3 will simply pipe it through to
      `RenderCtx`.
- [ ] extend top-level `Payload` to add:
      - `cwd: Option<String>` (Claude Code now sends both
        `cwd` and `workspace.current_dir`)
      - `version: Option<String>` (Claude Code CLI version)
      - `session_name: Option<String>`
      - `output_style: Option<OutputStyle>` (with `name: Option<String>`)
      - `cost: Option<Cost>` (total_cost_usd, total_duration_ms,
        total_api_duration_ms, total_lines_added, total_lines_removed)
      - `context_window: Option<ContextWindow>` (total_input_tokens,
        total_output_tokens, context_window_size, used_percentage,
        remaining_percentage, current_usage)
      - `exceeds_200k_tokens: Option<bool>`
      - `effort: Option<Effort>` (level)
      - `thinking: Option<Thinking>` (enabled)
      - `vim: Option<Vim>` (mode)
      - `agent: Option<Agent>` (name)
      - `worktree: Option<Worktree>` (name, path, branch,
        original_cwd, original_branch)
- [ ] extend `Workspace` (existing struct) to add:
      - `project_dir: Option<String>`
      - `added_dirs: Option<Vec<String>>`
      - `git_worktree: Option<String>`
- [ ] extend `Model` (existing) to add `id: Option<String>`
- [ ] write unit tests: full payload deserializes; partial payloads
      (most fields absent) deserialize; unknown fields tolerated;
      `current_usage: null` early in session is handled (Option
      semantics)
- [ ] run `cargo test payload` — must pass before next task

### Task 2: Extend `RenderCtx` with new primitive fields

**Files:**
- Modify: `src/format/placeholders/mod.rs`

- [ ] extend `RenderCtx` struct adding (all `Option<T>`):
      - `model_id: Option<String>`
      - `version: Option<String>`
      - `session_id: Option<String>`
      - `session_name: Option<String>`
      - `output_style: Option<String>`
      - `effort_level: Option<String>`
      - `thinking_enabled: Option<bool>`
      - `vim_mode: Option<String>`
      - `agent_name: Option<String>`
      - `cost_usd: Option<f64>`, `total_duration_ms: Option<u64>`,
        `api_duration_ms: Option<u64>`, `lines_added: Option<u64>`,
        `lines_removed: Option<u64>`
      - `tokens_input_total: Option<u64>`,
        `tokens_output_total: Option<u64>`
      - `tokens_input: Option<u64>` (current_usage.input_tokens),
        `tokens_output: Option<u64>` (current_usage.output_tokens),
        `tokens_cache_creation: Option<u64>`,
        `tokens_cache_read: Option<u64>`
      - `context_size: Option<u64>`,
        `context_used_pct: Option<f64>`,
        `context_remaining_pct: Option<f64>`
      - `exceeds_200k: Option<bool>`
      - `project_dir: Option<PathBuf>`
      - `added_dirs_count: Option<u64>`
      - `workspace_git_worktree: Option<String>`
      - `worktree_name: Option<String>`,
        `worktree_path: Option<PathBuf>`,
        `worktree_branch: Option<String>`,
        `worktree_original_cwd: Option<PathBuf>`,
        `worktree_original_branch: Option<String>`
      - `git_branch: Option<String>`,
        `git_root: Option<PathBuf>`,
        `git_changes_count: Option<u64>`,
        `git_staged_count: Option<u64>`,
        `git_unstaged_count: Option<u64>`,
        `git_untracked_count: Option<u64>`
- [ ] keep all existing fields unchanged. `RenderCtx` discipline
      (primitives only) preserved.
- [ ] write a unit test asserting `RenderCtx::default()` has all
      `Option`-typed fields set to `None` (sanity check after large
      struct expansion)
- [ ] run `cargo test format::placeholders` — must pass before next task

### Task 3: Wire `Payload → RenderCtx` mapping in a sibling module

**Files:**
- Create: `src/payload_mapping.rs` (new — required, not optional)
- Modify: `src/main.rs`
- Modify: `src/lib.rs` (add `pub mod payload_mapping;`)

- [ ] **MANDATORY extraction:** `src/main.rs` is at 456 LOC and
      will breach the 500-LOC ceiling once 30+ mapping lines are
      added. CREATE `src/payload_mapping.rs` first, then move the
      `Payload → RenderCtx` mapping into it. main.rs imports the
      function and calls it from `run_render`.
- [ ] in `payload_mapping.rs`, define
      `pub fn build_render_ctx(payload: &Payload, now_unix: u64)
      -> RenderCtx`. Populate every existing RenderCtx field
      (preserving Phase-1 behaviour) PLUS every new RenderCtx
      field from Task 2.
- [ ] **Scope of extraction:** `build_render_ctx` covers the
      FIELD-MAPPING portion only — `Payload` field reads into
      `RenderCtx` primitives. The hot-path control flow in
      `main.rs::run_render` (e.g., "if `payload.rate_limits`
      is present, skip OAuth fetch" and `apply_cache_to_ctx`
      later) STAYS INLINE in `run_render`. Don't move the
      orchestration; only move the pure mapping. After
      extraction, `run_render` should still own:
      stdin parsing → control flow decisions → calling
      `build_render_ctx` at the right point → `config::resolve`
      → `config::render::render` → output.
- [ ] for fields requiring conversion (e.g., `String` →
      `PathBuf`), apply `.map(PathBuf::from)`
- [ ] for `added_dirs: Option<Vec<String>>`, populate
      `added_dirs_count: Option<u64>` as
      `payload.workspace.as_ref().and_then(|w|
       w.added_dirs.as_ref().map(|v| v.len() as u64))`
- [ ] update `run_render` in `main.rs` to call
      `payload_mapping::build_render_ctx` instead of inline
      mapping. main.rs LOC must DECREASE post-extraction.
- [ ] write a unit test in `payload_mapping.rs` that builds a
      fully-populated `Payload` via serde and asserts every
      RenderCtx field is `Some(...)`
- [ ] write a unit test that builds an EMPTY `Payload` and asserts
      every new RenderCtx field is `None`
- [ ] LOC check: `wc -l src/main.rs` < 500, `wc -l
      src/payload_mapping.rs` < 500
- [ ] run `cargo test` — all tests pass before next task

### Task 4: Rewrite the format-placeholders invariant test + add session/Claude metadata placeholders

**Files:**
- Modify: `src/format/placeholders/tests.rs` (FIRST — invariant rewrite)
- Modify: `src/format/placeholders/mod.rs` (THEN — placeholder additions)

- [ ] **FIRST** (before any other change in this task): rewrite
      the `format::placeholders::tests` invariant test that
      currently hardcodes a list of files (~6 entries) to instead
      use a directory walk: `std::fs::read_dir` (or
      `walkdir`-style if a dep-free recursive walk is needed)
      filtered to `*.rs` files under `src/format/`. The scan
      asserts no file contains `use crate::api`, `use crate::cache`,
      or (Phase-2 addition) `use crate::git`. This rewrite happens
      NOW, before Tasks 4-10 split `format/placeholders/` into
      sibling files — so new siblings are covered automatically
      from the moment they're created.
- [ ] verify the rewritten test still passes against the existing
      file tree (no new files yet)
- [ ] add match arms in `format/placeholders/mod.rs` for:
      `model_id`, `version`, `session_id`, `session_name`,
      `output_style`, `effort` (level), `thinking_enabled`
      (returns "thinking" or None — caller decides display),
      `vim_mode`, `agent_name`
- [ ] write unit tests: each placeholder returns Some when
      corresponding ctx field is set; None when absent
- [ ] LOC check on `format/placeholders/mod.rs`. Currently small;
      should still fit. Split into
      `format/placeholders/{mod, session, ...}.rs` if approaching
      500 LOC. The new directory-walk test covers any siblings
      automatically.
- [ ] run `cargo test format::placeholders` — must pass before next task

### Task 5: Add cost / session-clock placeholders

**Files:**
- Modify: `src/format/placeholders/mod.rs`
- Modify: `src/format/placeholders/tests.rs`
- Modify: `src/format/values.rs` (helper functions)

- [ ] add match arms for: `cost_usd` (formatted to 2 decimals),
      `session_clock` (from `total_duration_ms`, formatted as
      "1h23m" via the new `format_duration_ms` helper —
      do NOT reuse `countdown` which subtracts from a future epoch
      and has the wrong direction for elapsed time),
      `api_duration` (from `api_duration_ms`),
      `lines_added`, `lines_removed`,
      `lines_changed` (added + removed convenience)
- [ ] add `format::values::format_duration_ms(ms: u64) -> String`
      helper (e.g., 4500ms → "4s", 60500ms → "1m", 3600500ms →
      "1h0m"). Tests for boundaries: 0ms, 999ms, 1000ms, 60000ms,
      3600000ms, 86400000ms.
- [ ] write unit tests for each placeholder: success + None paths.
- [ ] run `cargo test format` — must pass before next task

### Task 6: Add token placeholders

**Files:**
- Modify: `src/format/placeholders/mod.rs`
- Modify: `src/format/placeholders/tests.rs`
- Modify: `src/format/values.rs` (number-formatting helpers)

- [ ] add match arms for: `tokens_input` (current_usage), `tokens_output`,
      `tokens_cached_creation`, `tokens_cached_read`,
      `tokens_cached_total` (creation + read), `tokens_total`
      (all four summed), `tokens_input_total` (session sum),
      `tokens_output_total` (session sum)
- [ ] add `format::values::format_count(n: u64) -> String` helper
      (1234 → "1.2k", 1234567 → "1.2M") for compact display
- [ ] write unit tests for each placeholder + helper boundaries
      (0, 1, 999, 1000, 1500, 999_999, 1_000_000, u64::MAX)
- [ ] run `cargo test format` — must pass before next task

### Task 7: Add context placeholders + bar

**Files:**
- Modify: `src/format/placeholders/mod.rs`
- Modify: `src/format/placeholders/tests.rs`

- [ ] add match arms for: `context_size`, `context_used_pct`,
      `context_remaining_pct`, `context_used_pct_int` (rounded
      down), `context_bar` (10-char visual bar like the existing
      `five_bar` reusing `format::values::bar`),
      `context_bar_long` (20-char), `exceeds_200k`
- [ ] write unit tests for each placeholder
- [ ] run `cargo test format` — must pass before next task

### Task 8: Add workspace + worktree placeholders

**Files:**
- Modify: `src/format/placeholders/mod.rs`
- Modify: `src/format/placeholders/tests.rs`

- [ ] add match arms for: `project_dir` (with HOME-tilde
      compression like `cwd`), `added_dirs_count`,
      `workspace_git_worktree` (the simple worktree NAME from
      `workspace.git_worktree`), `worktree_name` (from
      `worktree.name`), `worktree_path` (with HOME-tilde),
      `worktree_branch`, `worktree_original_cwd` (with HOME-tilde),
      `worktree_original_branch`
- [ ] note: `workspace_git_worktree` and `worktree_name` are
      different sources (`workspace.git_worktree` vs
      `worktree.name`) — see Claude Code docs. Document the
      distinction in placeholder docstring.
- [ ] write unit tests for each placeholder + edge cases
      (HOME unset, empty paths, missing worktree section)
- [ ] LOC check on `format/placeholders/mod.rs`. If approaching
      500, split into sibling files (`session.rs`, `tokens.rs`,
      `git.rs` etc.) under `format/placeholders/`.
- [ ] run `cargo test format` — must pass before next task

### Task 9: Add `gix` dep (slim build) + create `src/git/` module

**Files:**
- Modify: `Cargo.toml`
- Modify: `Cargo.lock` (auto)
- Create: `src/git/mod.rs`
- Create: `src/git/tests.rs`
- Modify: `src/lib.rs` (add `pub mod git;`)
- Modify: `CLAUDE.md` (locked-dep set)

- [ ] add `gix = { version = "0.70", default-features = false,
      features = [] }` to `Cargo.toml`. **Start with `features =
      []`** and add only what is needed for the API calls we
      actually use (discovery, branch reading, status counters).
      After EACH feature added, run `cargo build` and
      `cargo tree`. The goal: NO transitive deps for HTTP, SSH,
      TLS, async runtimes, or signing. Acceptable feature
      candidates: `revision`, `status`, `index`, `parallel`. Skip
      `blocking-http-transport-curl`, `async-network-client`,
      `gpgme`, anything similarly heavy.
- [ ] document the chosen feature set in a comment above the
      dep declaration (e.g., `# features: revision, status —
      minimal for branch + status counters; no network`)
- [ ] **Cross-target verification:** the CI release matrix builds
      `x86_64-unknown-linux-musl` and
      `aarch64-unknown-linux-musl`. Verify gix slim build compiles
      on musl: run `cargo build --target x86_64-unknown-linux-musl`
      locally if you have rustup target installed, OR push a
      throwaway branch to trigger CI early. If musl fails, document
      the failure in the plan as `⚠️` and switch to shell-out for
      git (the deferred plan B).
- [ ] in `src/git/mod.rs`, add:
      - `pub fn discover(start: &Path) -> Option<Repo>` — wraps
        `gix::discover` and returns a small `Repo` newtype that
        owns the gix repository handle
      - `pub fn branch(&Repo) -> Option<String>` — returns the
        short branch name; None for detached HEAD
      - `pub fn root(&Repo) -> Option<PathBuf>` — returns the
        worktree root (NOT the .git dir)
- [ ] keep all errors INTERNAL — git module returns `Option`,
      never `Result`. The render path can never error out;
      "no info" is the always-safe fallback.
- [ ] write unit tests using `tempfile` + `git init` (shell out
      via `std::process::Command` is fine for test setup):
      - `discover` returns Some inside a repo, None outside
      - `branch` returns Some("main") on a fresh repo with one
        commit
      - `branch` returns None for a detached HEAD
      - `root` returns the canonical repo path
- [ ] update CLAUDE.md "locked dep set" section to add
      `gix` with one-line justification
- [ ] run `cargo build`, `cargo tree`, `cargo test git` — all pass
- [ ] LOC check: `src/git/mod.rs` < 500

### Task 10: Add git-status counters + new placeholders

**Files:**
- Create: `src/git/status.rs`
- Modify: `src/git/mod.rs` (re-export status fns)
- Modify: `src/format/placeholders/mod.rs`
- Modify: `src/format/placeholders/tests.rs`
- Modify: `src/main.rs` (call git module to populate RenderCtx)

- [ ] in `src/git/status.rs`, add:
      - `pub fn counts(&Repo) -> Option<StatusCounts>`
      - `StatusCounts { changes, staged, unstaged, untracked,
        insertions, deletions }` — each `u64`
      - Use gix's status API (research the exact call surface
        in 0.70+ docs). If insertions/deletions require diff
        traversal that's expensive, START with just changes/
        staged/unstaged/untracked counts and defer
        insertions/deletions to a follow-up.
- [ ] in `format/placeholders`, add match arms for:
      `git_branch`, `git_root` (with HOME-tilde),
      `git_changes`, `git_staged`, `git_unstaged`, `git_untracked`,
      `git_status_clean` (returns "clean" or None — convenience
      flag for templates)
- [ ] in `main.rs` (or the new `payload_mapping.rs` from Task 3 —
      pick whichever fits the LOC ceiling), add a function
      `populate_git_ctx(ctx: &mut RenderCtx, cwd: &Path)` that
      calls `git::discover` once and, if Some, populates every
      git-related RenderCtx field.
- [ ] **Path discipline:** ALWAYS pass the cwd (=
      `payload.workspace.current_dir`, falling back to
      `payload.cwd`). NEVER pass `worktree.path` — gix discovery
      walks parents, so cwd is correct in both worktree and
      non-worktree cases. Note this in a docstring on
      `populate_git_ctx`.
- [ ] **Lazy gating:** only call this function when the resolved
      config references any `{git_*}` placeholder. Implementation:
      scan the config's segment templates for the substring
      `{git_` before deciding whether to spawn discovery. No new
      git work if no template uses it.
- [ ] **Escape edge case:** the format parser supports `{{`/`}}` as
      literal braces. A user template containing `{{git_lol}}`
      renders as the literal text `{git_lol}` and SHOULD NOT
      trigger gix discovery. The plain-substring scan
      `template.contains("{git_")` falsely matches `{{git_` too;
      this is a small wasted call (gix discovery is ~1-5ms) and
      acceptable for Phase 2. Document the false-positive in
      the function's docstring; do not over-engineer the scan.
- [ ] write unit tests for placeholders: each git_* returns Some
      when ctx field is set; None when absent
- [ ] write unit tests for `populate_git_ctx`: when config has no
      git_* placeholder, function is not called (or returns early)
- [ ] write a unit test that populates a tempdir repo and asserts
      `git_branch == Some("main")` after `populate_git_ctx`
- [ ] run `cargo test` — must pass before next task

### Task 11: Add `rich` built-in template

**Files:**
- Modify: `src/config/builtins.rs`

- [ ] **Locked decision:** the 8 existing built-in templates
      (`default`, `minimal`, `compact`, `bars`, `colored`, `emoji`,
      `emoji_verbose`, `verbose`) MUST NOT be modified — Phase 1's
      `golden_output_unchanged` test pins them byte-for-byte
      against pre-Phase-1 .txt outputs.
- [ ] add ONE new built-in called `rich` that uses the new
      placeholders: tokens, context bar, git_branch,
      session_clock, vim_mode (if non-None). Document it in the
      function's docstring as the Phase-2 showcase template.
- [ ] update `lookup` to add `rich`. Update any `ALL_NAMES`
      array if present.
- [ ] write unit tests: `rich` validates without errors; `rich`
      renders non-empty output against the full-payload fixture.
- [ ] run `cargo test config::builtins` — all pass

### Task 12: Add Phase 2 golden test

**Files:**
- Create: `tests/golden_phase2.rs`

- [ ] add `golden_full_payload_renders_all_placeholders`: load
      `tests/fixtures/full-payload.json`, run the binary with
      a custom config that references every new placeholder,
      assert each placeholder rendered to its expected value.
      Acquire `HOME_MUTEX` if HOME is touched. Pin
      `XDG_CONFIG_HOME` to a tempdir.
- [ ] add `golden_rich_template_smoke`: run with `--template rich`
      against the full-payload fixture; assert non-empty output
      with all expected segments visible.
- [ ] add `golden_git_placeholders_outside_repo_collapse`: run
      against the full-payload fixture but set cwd to a non-repo
      tempdir; assert `{git_branch}` and friends collapse via
      hide_when_absent.
- [ ] add `golden_git_placeholders_inside_repo_resolve`: create
      a tempdir + git init + commit; run with cwd in the repo;
      assert `{git_branch}` returns "main" (or the test-fixture
      branch).
- [ ] run `cargo test --test golden_phase2` — all pass
- [ ] verify Phase 1's `golden_output_unchanged` STILL passes
      (the 8 original built-ins are untouched)

### Task 13: Update README + CLAUDE.md

**Files:**
- Modify: `README.md`
- Modify: `CLAUDE.md`

- [ ] README: add a "Placeholders" section listing every new
      placeholder grouped by source (Claude Code stdin, git).
      Keep it tabular and brief.
- [ ] README: add the `rich` built-in to the template gallery.
- [ ] README: example config showing tokens + context bar +
      git branch on a 2-line statusline.
- [ ] CLAUDE.md: update Module-tree section to include `git/`.
      Update locked-dep set already updated in Task 9 — verify.
      Update Hard-Invariants section: ADD `format/*.rs` MUST NOT
      import `crate::git`. Same one-way pattern as Phase 1's
      `format/* must not import config/*`.
- [ ] **Verify** the directory-walk invariant test added in Task 4
      now also covers `use crate::git`. Add this substring to the
      test's forbidden-import list if Task 4 didn't include it.
      The test should still discover all sibling files in
      `src/format/` automatically (no hardcoded file list).
- [ ] CLAUDE.md: under "Test architecture", note that Phase 2
      golden tests use `tests/fixtures/full-payload.json` as the
      standard fixture for placeholder tests.
- [ ] update `scripts/check-invariants.sh` to ADD greps for
      `use crate::git` in BOTH `src/format/*.rs` AND
      `src/format/placeholders/*.rs` (the existing Phase-1 grep
      may only cover `src/format/` top-level — verify and extend).
      Mirrors the existing `use crate::config` check pattern.
- [ ] run all gates: `cargo fmt --check`, `cargo clippy
      --all-targets -- -D warnings`, `cargo test`,
      `bash scripts/check-loc.sh`, `bash scripts/check-invariants.sh`,
      `shellcheck scripts/*.sh`

### Task 14: Verify acceptance + finalize

- [ ] verify all requirements from Overview implemented:
  - Payload struct extended with all stdin fields ✓
  - RenderCtx extended with primitives ✓
  - ~30 new placeholders working ✓
  - gix-based git module functional inside repos ✓
  - git placeholders collapse cleanly outside repos ✓
  - `rich` built-in template added ✓
  - Phase 1's `golden_output_unchanged` still passes ✓
  - No transcript JSONL parsing introduced ✓
  - No settings.json reading introduced ✓
  - Only `gix` added as new dep ✓
- [ ] run full test suite: `cargo test`
- [ ] run release build: `cargo build --release`. Confirm size
      after gix slim build is reasonable. Phase 1 binary was
      1.0 MB stripped. Target: ≤ 2.0 MB stripped after gix.
      If over, reconsider feature flags or split git module
      into a feature-gated module (config flag to disable git).
- [ ] manual smoke:
      - `echo '{}' | ./target/release/cc-myasl --template rich` →
        renders sensibly (most segments collapse with empty
        payload)
      - `cat tests/fixtures/full-payload.json |
        ./target/release/cc-myasl --template rich` → renders
        all segments
- [ ] verify test count grew. Phase 1 ended with 534 tests; Phase 2
      should add 60+ for ~30 placeholders, struct extension,
      git module.
- [ ] update CLAUDE.md "Reference docs" section: add a link to
      this plan under completed:
      ```
      - `docs/plans/completed/2026-05-01-phase2-placeholder-expansion.md` —
        Phase 2 placeholder expansion. Implementation complete.
      ```
- [ ] move `docs/plans/2026-05-01-phase2-placeholder-expansion.md`
      to `docs/plans/completed/2026-05-01-phase2-placeholder-expansion.md`
      via `git mv`.
- [ ] mark all Task 14 checkboxes `[x]` in the (now-moved) plan
      file.
- [ ] commit. Suggested message:
      `feat: Phase 2 placeholder expansion (stdin extension + gix-based git module)`

## Technical Details

### New Payload struct shape (sketch)

```rust
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Payload {
    pub model: Option<Model>,
    pub workspace: Option<Workspace>,
    pub transcript_path: Option<String>,
    pub session_id: Option<String>,
    pub session_name: Option<String>,
    pub cwd: Option<String>,
    pub version: Option<String>,
    pub output_style: Option<OutputStyle>,
    pub cost: Option<Cost>,
    pub context_window: Option<ContextWindow>,
    pub exceeds_200k_tokens: Option<bool>,
    pub effort: Option<Effort>,
    pub thinking: Option<Thinking>,
    pub rate_limits: Option<RateLimits>,
    pub vim: Option<Vim>,
    pub agent: Option<Agent>,
    pub worktree: Option<Worktree>,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct ContextWindow {
    pub total_input_tokens: Option<u64>,
    pub total_output_tokens: Option<u64>,
    pub context_window_size: Option<u64>,
    pub used_percentage: Option<f64>,
    pub remaining_percentage: Option<f64>,
    pub current_usage: Option<ContextWindowCurrentUsage>,
}

#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct ContextWindowCurrentUsage {
    pub input_tokens: Option<u64>,
    pub output_tokens: Option<u64>,
    pub cache_creation_input_tokens: Option<u64>,
    pub cache_read_input_tokens: Option<u64>,
}
```

(Other nested structs follow the same pattern — see Task 1.)

### Git module sketch

```rust
// src/git/mod.rs
use std::path::{Path, PathBuf};

pub struct Repo {
    inner: gix::Repository,
}

pub fn discover(start: &Path) -> Option<Repo> {
    gix::discover(start).ok().map(|r| Repo { inner: r })
}

impl Repo {
    pub fn branch(&self) -> Option<String> {
        self.inner.head_name().ok().flatten()
            .and_then(|n| n.shorten().to_str().ok().map(String::from))
    }

    pub fn root(&self) -> Option<PathBuf> {
        self.inner.workdir().map(PathBuf::from)
    }
}
```

(Status counters in `src/git/status.rs` — Task 10.)

### Lazy git invocation

In `main.rs::populate_git_ctx`, gate the gix call on whether the
resolved config references any `{git_*}` placeholder:

```rust
fn config_uses_git(config: &Config) -> bool {
    for line in &config.lines {
        for seg in &line.segments {
            if let Segment::Template(t) = seg {
                if t.template.contains("{git_") {
                    return true;
                }
            }
        }
    }
    false
}
```

This avoids paying the gix discovery cost (~1-5ms) when the user's
config doesn't ask for git data.

### Migration safety

Phase 2 does NOT modify any of the 8 Phase-1 built-in templates.
Phase 1's `golden_output_unchanged` test continues to assert
byte-identity. Adding the `rich` template is a pure addition.

## Post-Completion

*Items requiring manual intervention or external systems — no
checkboxes, informational only.*

**Manual verification:**

- Manual smoke in real Claude Code session: install latest binary,
  add `rich` template via `--template rich` in settings.json
  statusline command, verify all segments render with real data
  across iTerm2 / Terminal.app / VSCode integrated terminal.
- Performance check: `hyperfine --warmup 5 'cat
  tests/fixtures/full-payload.json | target/release/cc-myasl
  --template rich'` — cold start should still be under 50ms with
  the gix dep.

**Deferred / out-of-scope (future plans):**

- `{skills}` placeholder: requires reading hook data (separate
  Claude Code mechanism). Not in stdin. Phase 2.5 or Phase 3
  scope.
- `{git_pr}` placeholder: requires `gh`/`glab` shell-out for
  PR-number lookup, plus authenticated API call. Not in this
  plan; could be a small follow-up plan when needed.
- Modernising the 8 Phase-1 built-in templates with the new
  placeholders: explicitly deferred to keep
  `golden_output_unchanged` passing. Future plan can refresh the
  built-in lineup once we have a different invariant.
- Removing `format::render` deprecated symbol: still has callers
  in `config/builtins.rs` test helpers. Final removal when those
  helpers are rewritten.
- Powerline mode (mentioned in original brainstorm): separate
  plan; affects the rendering path, not just placeholders.

**Phase 3 (NOT in this plan):**

- Interactive TUI for editing `~/.config/cc-myasl/config.json`.
  Likely needs `ratatui` + `crossterm` deps; explicit dep
  approval. Separate plan when Phase 2 lands.
