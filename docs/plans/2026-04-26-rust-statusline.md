# Rust v0.1.0 — `claude-statusline` implementation plan

> Revised 2026-04-26 to incorporate plan-review feedback (decoupled
> `RenderCtx`, testable Keychain seam, CI-gated invariants, Task 11
> split, integer-only `Retry-After`, structural golden assertions).

## Overview

Implement a single-binary Rust status-line tool for Claude Code that
displays remaining 5-hour and 7-day token quota. Reads Claude Code's
stdin JSON; if the official `rate_limits` field is absent (fresh
session, non-Pro/Max, or first turn), falls back to
`GET https://api.anthropic.com/api/oauth/usage` using a Bearer token
read from the macOS Keychain entry `Claude Code-credentials` or
`~/.claude/.credentials.json`. Output is fully template-driven so
users can change which segments appear without rebuilding.

This plan **supersedes** the POSIX-`sh` plan in `docs/plan.md` (Option B
there). Treat that file as historical reference; this one is
authoritative for v0.1.0.

Solves three problems with the alternatives:

- **`sh + jq + curl`** — fragile schema handling for an undocumented
  API, ~30–80 ms cold start (5+ forks per render), no Linux quoting
  parity, hard to test.
- **`bunx -y ccstatusline@latest`** — supply-chain risk
  (`docs/security-review.md` issue #298); `dump-keychain` privacy
  footprint; runtime npm fetch on every Claude Code launch.
- **`pip install`-style Python** — 50–100 ms cold start, runtime
  dependency hell, breaks on Homebrew Python's PEP-668.

Rust gives us: single static binary, ~3–5 ms cold start, true
cross-platform, type-safe parsing, no runtime deps for end users.

## Context (from discovery)

Source-of-truth documents already in this repo:

- `docs/research.md` — `statusLine` contract, OAuth endpoint shape,
  ccstatusline / botfarm patterns worth borrowing.
- `docs/security-review.md` — security audit of the upstream
  `ccstatusline` project; informs what we deliberately do **not**
  implement (no `dump-keychain`, no custom-command widget, no
  `npx -y` install path).
- `docs/plan.md` — earlier sh-based plan; superseded by this file.
- `docs/session-2026-04-26.md` — pinned brainstorm session.

External references:

- `sirmalloc/ccstatusline` (`src/utils/usage-fetch.ts`) — endpoint,
  token sources, caching pattern, lock file.
- `AlexDobrushskiy/botfarm` (`botfarm/usage.py`) — adaptive backoff,
  token fingerprint, audit log.
- Anthropic statusLine docs — confirmed JSON shape on stdin.

## Hard Invariants (CI-gated)

These are checked by `cargo test` and/or CI greps. A violation fails
the build. Listed here so every task can reference them.

1. **No file in `src/**/*.rs` exceeds 500 LOC** (CI step in Task 1).
2. **Bearer token never written to disk** — `UsageCache` struct
   omits any token field; verified by golden test in Task 14.
3. **`main.rs` never exits non-zero in render mode** — verified by
   adverse-env test in Task 11c and Task 18.
4. **No `security dump-keychain` invocation** — `! grep -r
   "dump-keychain" src/ scripts/` (CI step in Task 1).
5. **`install.sh` has no cargo/rust references** — `! grep -iE
   "cargo|rustc|rustup" scripts/install.sh` (CI step in Task 1).
6. **No `npx -y …@latest` style auto-update path** — `! grep -iE
   "npx.*latest|@latest" src/ scripts/` (CI step in Task 1).

## Development Approach

- **Testing approach**: Regular (code-then-tests, same task). Rust
  convention is `#[cfg(test)] mod tests {}` alongside the code; each
  task delivers code + tests as one unit. TDD remains acceptable per
  task at the implementer's discretion, but tests are a hard
  deliverable of every task — non-optional.
- Complete each task fully before the next; small focused changes.
- Every task MUST include new/updated tests for its code:
  - unit tests for new functions/methods
  - tests for both success and error scenarios
  - tests for edge cases (missing data, malformed input)
- All tests MUST pass (`cargo test`) before starting the next task.
- Update this plan file when scope changes during implementation
  (`➕` for new tasks, `⚠️` for blockers).
- Maintain the **500-LOC-per-file ceiling** at all times. If a file
  approaches 400 LOC, draft the split before writing more.

## Execution Strategy — agent-per-task

Each task is implemented by **one spawned `general-purpose` agent
running on Sonnet** (`model: "sonnet"`), not directly by the
orchestrator. The orchestrator's job is to brief the agent, verify
its output against the task's acceptance criteria, mark `[x]`, and
move on.

**Why Sonnet, not Opus, for the workers**: implementing a single
task — write file, write tests, run gates — is execution work, not
deep reasoning. Sonnet (`claude-sonnet-4-6`) handles it faster and
cheaper than Opus, with quality on par for this scope. Opus stays
on the orchestrator (this conversation) where cross-task reasoning
and plan revision happens.

**Why agent-per-task at all**: each agent gets a fresh context
window (no prior turns crowding it), the audit trail is clean
(one agent = one task), and independent tasks can run in parallel.

**Hard rule**: every `Agent` tool invocation in this project's
implementation MUST set `model: "sonnet"`. The orchestrator does
not delegate to default-model agents; it always specifies.

### Per-task brief template

When spawning an agent for Task N, the orchestrator prompt MUST
include:

1. **Task scope**: the full Task N section verbatim from this plan
   (Files block + checkboxes).
2. **Hard Invariants** section (verbatim) — every agent must respect
   all 6 invariants regardless of which task it owns.
3. **Reference docs** to read first:
   `docs/research.md`, `docs/security-review.md`, this plan file.
4. **Acceptance gates** the agent must run before reporting done:
   `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
   `cargo test`, `bash scripts/check-loc.sh`,
   `bash scripts/check-invariants.sh`.
5. **Output expectation**: a short report (≤200 words) listing
   files created/modified, tests added, gates that passed, and any
   `➕` new tasks or `⚠️` blockers discovered. Do **not** ask the
   agent to update this plan file — only the orchestrator does that.

### Parallel-eligible task groups

These groups have no inter-dependencies and CAN be spawned
concurrently (one agent per task in the group, dispatched in a
single multi-tool message):

- **Group A (after Task 4)**: Task 5 (creds.rs) ‖ Task 6 (api types)
  ‖ Task 8 (cache primitives). All are leaf modules with no shared
  state.
- **Group B (after Task 12)**: Task 13 (templates) ‖ Task 16
  (install.sh). Entirely separate concerns.
- **Group C (after Task 14)**: Task 15 (CI release.yml) ‖ Task 17
  (README polish). Different files, no overlap.

### Strictly sequential tasks

These have hard dependencies and MUST NOT be parallelised:

- Task 1 → 2 → 3 → 4 (foundation; each depends on the previous)
- Task 11a → 11b → 11c (time → args → orchestration)
- Task 7 depends on Task 6 (api types)
- Task 9 depends on Task 8 (cache primitives)
- Task 18 → 19 (verify must be last but one)

### Conflict resolution

If two parallel agents both want to edit `src/lib.rs` (the export
manifest), the orchestrator serialises those edits — agents return
their needed `pub mod …` line as part of their report; the
orchestrator applies all of them in one final edit before running
`cargo test`. Agents do NOT race on `Cargo.toml` either — any new
dep is requested in the agent's report, applied by the orchestrator.

### Verification cycle

After each agent reports done:
1. Orchestrator reads the agent's listed files to spot-check.
2. Runs the same acceptance gates locally.
3. Marks `[x]` in this plan; commits if `--auto-commit` mode (not
   default).
4. If any gate fails, sends a follow-up message to the same agent
   (via `SendMessage`) with the failure detail; agent fixes;
   re-run gates.
5. Only after all gates pass does the orchestrator move to the
   next task (or next group).

## Testing Strategy

- **Unit tests** (`#[cfg(test)] mod tests {}` in every module): required
  for every task.
- **Integration tests** (`tests/golden.rs`): spawn the compiled binary,
  pipe a fixture stdin JSON, assert stdout. OAuth endpoint mocked via
  `mockito` so tests never hit `api.anthropic.com`.
- **No e2e UI tests** — this is a CLI tool.
- **No real-network tests** in CI — they would expose credentials and
  rate-limit us. Mocked only.
- **Fixtures** stored in `tests/fixtures/*.json`:
  - `pro_max_with_rate_limits.json` (hot path)
  - `api_key_no_rate_limits.json` (OAuth fallback path)
  - `extra_usage_enabled.json` (extra-usage placeholders)
  - `malformed_field.json` (graceful degrade path)

## Progress Tracking

- Mark completed items with `[x]` immediately when done — no batching.
- Add newly discovered tasks with `➕` prefix.
- Document blockers with `⚠️` prefix.
- Update plan if implementation deviates from original scope.
- Keep plan in sync with actual work done.

## What Goes Where

- **Implementation Steps** (`[ ]` checkboxes): all tasks achievable in
  this repo — code, tests, CI workflows, install script, README.
- **Post-Completion** (no checkboxes): items needing human action —
  manual `claude-statusline --check` on a real Claude Code install,
  pushing the first `v0.1.0` tag, verifying the CI release artefacts
  download and install on a clean macOS-arm64 box and a clean
  Linux-x86_64 box.

---

## Implementation Steps

### Task 1: Bootstrap Cargo project + CI lint/test/invariant scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `Cargo.lock` (auto)
- Create: `src/main.rs` (skeleton)
- Create: `src/lib.rs` (skeleton, re-exports)
- Create: `.github/workflows/ci.yml`
- Create: `rust-toolchain.toml`
- Create: `scripts/check-loc.sh` (POSIX, ~15 LOC)
- Create: `scripts/check-invariants.sh` (POSIX, ~15 LOC)
- Modify: `.gitignore` (add `target/`, `dist/`, `*.profraw`)

- [x] `cargo init --bin` at repo root; set name `claude-statusline`,
      edition `2021`, version `0.1.0`
- [x] add deps to `Cargo.toml`: `ureq` 2.x with `rustls` feature
      (no `native-tls`); `serde` + `serde_json` with `derive`;
      `directories` 5.x; `anyhow` 1.x. Dev-dep: `mockito` 1.x,
      `tempfile` 3.x, `assert_cmd` 2.x, `predicates` 3.x.
- [x] add `[profile.release]`: `lto = "fat"`, `codegen-units = 1`,
      `strip = true`, `panic = "abort"` to hit ≤ 1.5 MB target
- [x] create empty `src/main.rs` that prints a placeholder line and
      exits 0
- [x] add `rust-toolchain.toml`: `channel = "1.83"`,
      `components = ["clippy", "rustfmt"]` (pin tools so CI and dev align)
- [x] write `scripts/check-loc.sh`: `find src -name '*.rs' -exec
      wc -l {} +` and exit 1 if any single-file count > 500
- [x] write `scripts/check-invariants.sh`: runs three greps,
      fails on any hit:
      `grep -r "dump-keychain" src/ scripts/`,
      `grep -iE "npx.*latest|@latest" src/ scripts/`,
      `grep -iE "cargo|rustc|rustup" scripts/install.sh` (the file
      may not exist yet — script must tolerate that and skip)
- [x] `chmod +x` both scripts; `shellcheck` clean
- [x] add `.github/workflows/ci.yml`: jobs run on
      `ubuntu-latest` and `macos-latest`:
      `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
      `cargo test`, `bash scripts/check-loc.sh`,
      `bash scripts/check-invariants.sh`,
      `shellcheck scripts/*.sh`
- [x] write a smoke test in `tests/smoke.rs` that spawns the binary,
      pipes empty stdin, and asserts non-empty stdout + exit code 0
- [x] write tests for `scripts/check-loc.sh`: stub fixture files
      sized 499 / 500 / 501 LOC; verify pass/fail behaviour
- [x] run `cargo test`, `cargo clippy`, both shell scripts —
      must pass before Task 2

### Task 2: Implement `payload.rs` (Claude Code stdin parser)

**Files:**
- Create: `src/payload.rs`
- Modify: `src/lib.rs` (export `payload` module)
- Modify: `src/main.rs` (read stdin, parse, print debug repr behind `--debug`)

- [x] declare `pub struct Payload` with serde-derived fields:
      `model.display_name`, `workspace.current_dir`, `transcript_path`,
      `session_id`, optional `rate_limits.{five_hour, seven_day}` with
      `used_percentage` and `resets_at`
- [x] make every field `Option<…>` where Claude Code may omit it;
      never panic on unknown shapes (use `#[serde(default)]`; serde
      ignores unknown fields by default — do **not** add
      `deny_unknown_fields`)
- [x] `pub fn parse(reader: impl Read) -> Result<Payload, Error>` that
      returns a typed error on JSON parse failure; never panics
- [x] write tests for: full payload, payload missing `rate_limits`,
      payload missing `workspace`, malformed JSON, empty stdin
- [x] write tests confirming unknown fields don't break parsing
- [x] run `cargo test` — must pass before Task 3

### Task 3: Implement `format/parser.rs` and `format/values.rs`

**Files:**
- Create: `src/format/mod.rs` (skeleton, exports submodules)
- Create: `src/format/parser.rs`
- Create: `src/format/values.rs`
- Modify: `src/lib.rs` (export `format`)

- [x] `format::parser`: tokenize a template string into
      `Vec<Token>` where `Token = Text(String) | Placeholder(String) |
      Optional(Vec<Token>)`. Recognise `{name}` and `{? ... }`
- [x] write tests for parser: plain text, single placeholder, multiple
      placeholders, optional segment, nested optional, malformed
      `{...` (unterminated → treat as literal)
- [x] `format::values`: pure render helpers — `bar(percent, width) ->
      String` (block-fill ASCII), `percent_int(p) -> String`,
      `percent_decimal(p) -> String`, `clock_local(epoch_or_iso) ->
      String` (HH:MM in local TZ — **UTC-only stub in Task 3,
      replaced by `time::format_clock_local` in Task 11a**),
      `countdown(epoch_or_iso, now) -> String` (`"2h13m"` form)
- [x] write tests for values: bar at 0%, 50%, 100%, width 1 and 20;
      countdown for 0 s, 1 min, 1 hour, 1 day; clock around midnight;
      epoch-vs-ISO input parity
- [x] run `cargo test` — must pass before Task 4

### Task 4: Implement `format/thresholds.rs`, `format/placeholders.rs`, `format/mod.rs`

**Files:**
- Create: `src/format/thresholds.rs`
- Create: `src/format/placeholders.rs`
- Modify: `src/format/mod.rs` (assemble `pub fn render`)

**Decoupling invariant** (CI-checked indirectly via Cargo's module
graph — `format/*` must NOT `use` anything from `crate::api` or
`crate::cache`): `RenderCtx` contains only primitives and stdlib
types. The mapping from `UsageResponse` (Task 6) and `UsageCache`
(Task 9) into `RenderCtx` happens in `main.rs` (Task 11c). This
prevents Task 4 work from being rewritten when Tasks 6/9 land.

- [ ] `format::thresholds`: `enum State { Green, Yellow, Red, Unknown }`
      and `pub fn classify(left: Option<f64>) -> State` honouring env
      vars `STATUSLINE_RED` (default 20) and `STATUSLINE_YELLOW`
      (default 50). `pick_color(state)` and `pick_icon(state)`
      returning `&'static str`
- [ ] `format::placeholders`: define `pub struct RenderCtx` with
      ONLY primitive Option types — `model: Option<String>`,
      `cwd: Option<PathBuf>`, `five_used: Option<f64>`,
      `five_reset_unix: Option<u64>`, `seven_used: Option<f64>`,
      `seven_reset_unix: Option<u64>`, `extra_*: Option<…>`,
      `now_unix: u64`. **No `use crate::api;` or `use crate::cache;`
      anywhere in `format/`.**
- [ ] catalogue (15+) of placeholder name → render fn mappings:
      `{model}`, `{cwd}`, `{cwd_basename}`, `{five_used}`,
      `{five_left}`, `{five_bar}`, `{five_bar_long}`,
      `{five_reset_clock}`, `{five_reset_in}`, `{five_color}`,
      `{five_state}`, parallel `{seven_*}` set, `{extra_left}`,
      `{extra_used}`, `{extra_pct}`, `{state_icon}`, `{reset}`
- [ ] `format::mod::render(template: &str, ctx: &RenderCtx) -> String`:
      tokenize via `parser`, evaluate. Each `Optional` block: if any
      placeholder inside resolves to empty, emit empty for the whole
      block; else substitute and emit
- [ ] write tests for the catalogue: each placeholder rendered with
      sample data and with missing data
- [ ] write tests for `render`: full template, optional collapse,
      colour/icon output bytes, threshold env-var overrides
- [ ] write a test asserting `format/*.rs` files contain zero
      `use crate::api` or `use crate::cache` substrings
- [ ] run `cargo test` — must pass before Task 5

### Task 5: Implement `creds.rs` (Keychain + file fallback) with testable seam

**Files:**
- Create: `src/creds.rs`
- Modify: `src/lib.rs` (export `creds`)

**Testability seam**: split into pure parser + thin caller.

- [ ] `pub fn parse_keychain_output(stdout: &str) -> Result<String,
      Error>` — pure function, no I/O. Parses the JSON returned by
      `security find-generic-password -w`, extracts
      `claudeAiOauth.accessToken`. Testable on **every** platform.
- [ ] `pub fn parse_credentials_file(content: &str) -> Result<String,
      Error>` — same shape, for `~/.claude/.credentials.json`.
      Testable on every platform.
- [ ] `fn keychain_command_output() -> Result<String, Error>` — the
      thin caller that invokes `security find-generic-password -s
      "Claude Code-credentials" -w` via `std::process::Command` and
      pipes stdout into `parse_keychain_output`. **Never** call
      `dump-keychain`. `#[cfg(target_os = "macos")]` gated.
- [ ] `pub fn read_token() -> Result<String, Error>` — orchestrator:
      try Keychain on macOS first, fall back to credentials file.
      Returns the token; never logs it.
- [ ] `pub fn fingerprint(token: &str) -> String` — SHA-256 of last 8
      chars, truncated to 16 hex. Never reverses to the token.
- [ ] write parser tests on every platform (no `cfg`-gating):
      valid Keychain JSON, missing `claudeAiOauth`, missing
      `accessToken`, malformed JSON, empty string. Same set for
      `parse_credentials_file`.
- [ ] write `tempfile`-based tests for the credentials-file path:
      file present + valid, file present + malformed, file missing.
- [ ] write a `#[cfg(target_os = "macos")]` integration smoke test
      that exercises `keychain_command_output` only when an env var
      is set (`CLAUDE_STATUSLINE_KEYCHAIN_TEST=1`); skip otherwise.
      Documented opt-in test for human runs.
- [ ] write a test asserting the token never appears in
      `format!("{:?}", error)` for any error variant.
- [ ] run `cargo test` — must pass before Task 6

### Task 6: Implement `api/response.rs` + `api/retry.rs`

**Files:**
- Create: `src/api/mod.rs` (skeleton, re-exports)
- Create: `src/api/response.rs`
- Create: `src/api/retry.rs`
- Modify: `src/lib.rs` (export `api`)

- [ ] `api::response`: `pub struct UsageResponse` matching the OAuth
      endpoint shape (`five_hour.utilization`, `five_hour.resets_at`,
      `seven_day.*`, `extra_usage.*`). All fields `Option<…>`.
      Include serde tests.
- [ ] `api::retry::parse_retry_after(value: &str) -> Option<Duration>`
      — **integer seconds only**. RFC 9110 also permits HTTP-date,
      but a stdlib-only HTTP-date parser is non-trivial; we
      deliberately accept only `"\d+"` and fall back to the default
      300 s lock if the header is HTTP-date format. Document this
      deviation in a doc-comment on the function.
- [ ] write tests for the response parser: full response, partial
      response, all-null response, malformed JSON
- [ ] write tests for `parse_retry_after`: `"60"`, `"0"`, `"3600"`,
      whitespace, empty string, negative `"-1"`, malformed `"abc"`,
      HTTP-date `"Fri, 13 Mar 2026 12:00:00 GMT"` (returns None →
      caller uses default 300)
- [ ] run `cargo test` — must pass before Task 7

### Task 7: Implement `api/mod.rs` (HTTP fetch with mockito tests)

**Files:**
- Modify: `src/api/mod.rs`

`ureq` with the `rustls` feature ONLY validates TLS for `https://`
URLs; plain `http://` URLs (which `mockito` serves) bypass TLS
entirely and are accepted. The `base_url` parameter exists so tests
swap in `http://127.0.0.1:PORT` from `mockito::Server::url()`.

- [ ] `pub fn fetch_usage(token: &str, base_url: &str) ->
      Result<FetchOutcome, Error>` where `FetchOutcome` is
      `Ok(UsageResponse) | RateLimited(Duration) | AuthFailed |
      ServerError | TimedOut`
- [ ] Use `ureq::AgentBuilder` with `tls_config` set to rustls,
      `timeout(Duration::from_secs(5))`. Headers: `Authorization:
      Bearer <token>`, `anthropic-beta: oauth-2025-04-20`, a
      meaningful `User-Agent` (`claude-statusline/<env!CARGO_PKG_VERSION>`)
- [ ] Status dispatch:
      `200 → Ok`; `401 → AuthFailed`; `429 → RateLimited(parse_retry_after)`;
      `5xx → ServerError`; `timeout/io error → TimedOut`
- [ ] write tests using `mockito`: 200 with body, 200 with empty body,
      401, 429 with `Retry-After: 60`, 429 with HTTP-date header (→
      defaults to 300 s), 429 with no Retry-After, 500, slow response
      (timeout)
- [ ] write a test asserting that pointing `base_url` at a real
      `https://` URL still works (point at `https://example.com`,
      expect a non-200 outcome — the goal is to confirm rustls
      is wired)
- [ ] run `cargo test` — must pass before Task 8

### Task 8: Implement `cache/lock.rs`, `cache/backoff.rs`, `cache/atomic_helper.rs`

**Files:**
- Create: `src/cache/mod.rs` (skeleton)
- Create: `src/cache/lock.rs`
- Create: `src/cache/backoff.rs`
- Create: `src/cache/atomic_helper.rs`
- Modify: `src/lib.rs` (export `cache`)

- [ ] `cache::atomic_helper::write_atomic(path, bytes) -> io::Result<()>`:
      write to `path.tmp`, `fsync`, `rename(2)` over `path`
- [ ] `cache::lock`: `pub struct Lock { blocked_until: u64, error:
      LockError }` (where LockError = RateLimited|AuthFailed|Network).
      `pub fn read(path) -> Option<Lock>`, `write(path, lock)` via
      atomic helper. Treats invalid JSON as no lock.
- [ ] `cache::backoff::next_lock_seconds(consecutive_failures: u32,
      err_kind: LockError) -> u64`: 401 → 3600; 429 with retry_after →
      that value (≥ 300); 5xx/timeout → exponential ladder
      60→120→240→480→960→1800
- [ ] write tests for atomic_helper: write succeeds, partial-write
      simulation never leaves a corrupt target
- [ ] write tests for lock: read missing, read malformed, write+read
      round-trip, expired vs active
- [ ] write tests for backoff: each error kind, ladder progression,
      cap at 1800 s, 401 always 3600 s
- [ ] run `cargo test` — must pass before Task 9

### Task 9: Implement `cache/mod.rs` (orchestrator)

**Files:**
- Modify: `src/cache/mod.rs`

- [ ] Define `CacheDir` resolver using `directories::ProjectDirs::from
      ("ai", "claude-statusline", "claude-statusline")` falling back
      to `~/.cache/claude-statusline/` on Linux
- [ ] `pub struct UsageCache { fetched_at: u64, five_hour: …,
      seven_day: …, extra_usage: … }` — **no token field, ever**
- [ ] `pub fn read(dir: &Path) -> Option<UsageCache>` (returns None
      on parse error; never panics)
- [ ] `pub fn write(dir: &Path, cache: &UsageCache)` via atomic_helper
- [ ] `pub fn is_fresh(cache: &UsageCache, ttl_secs: u64, now: u64) ->
      bool` — TTL = 180
- [ ] `pub fn read_stale(dir) -> Option<UsageCache>` — returns even
      expired data, used as fallback when network fails
- [ ] write tests using `tempfile::tempdir()`: write+read round-trip;
      stale detection; corrupt cache file; missing cache dir.
- [ ] write a concurrent-safety test with N=20 threads spawning
      writes; assert that **every** read during the storm returns
      either `None` or a parseable `UsageCache` (never a corrupt
      file). Loosened from "all writers succeed" — `rename(2)` last-
      writer-wins is acceptable; corrupt-file observation is not.
- [ ] write a test asserting `UsageCache` never round-trips a token:
      construct a `UsageCache`, serialize to JSON, assert the JSON
      string contains none of `"token"`, `"bearer"`, `"secret"`,
      `"auth"`, `"access"` (case-insensitive). Simple, no
      `cargo expand` or AST parsing required.
- [ ] run `cargo test` — must pass before Task 10

### Task 10: Implement `error.rs` and `debug.rs`

**Files:**
- Create: `src/error.rs`
- Create: `src/debug.rs`
- Modify: `src/lib.rs` (export both)

- [ ] `error::Error`: a single enum covering every error our binary
      can encounter (StdinParse, CredsRead, ApiTransport, ApiAuth,
      ApiRateLimited, CacheRead, CacheWrite, FormatRender). Implement
      `Display` and `From` for the common conversions
- [ ] All variants are recoverable in render mode — `main.rs` will
      degrade rather than crash. `--check` is the only path that
      surfaces an error to the user
- [ ] `debug::Trace`: struct collecting `path`, `cache`, `http`,
      `took_ms`, `error`. `pub fn emit(self)` writes a single-line
      JSON object to stderr **only when** `--debug` flag or env
      `STATUSLINE_DEBUG=1` is set
- [ ] Trace MUST never include the bearer token, only its fingerprint
- [ ] write tests for `Error::Display` (each variant), for
      `Trace::emit` writing valid JSON, and for the redaction
      invariant (assert no input string equal to a fixture token ever
      appears in the emitted JSON)
- [ ] run `cargo test` — must pass before Task 11a

### Task 11a: Implement `time.rs`

**Files:**
- Create: `src/time.rs`
- Modify: `src/lib.rs` (export `time`)

- [ ] `time::now_unix() -> u64`, `time::iso_to_unix(&str) ->
      Option<u64>`, `time::format_clock_local(unix) -> String`,
      `time::format_countdown(target_unix, now_unix) -> String`. Use
      stdlib `SystemTime` + tiny manual ISO-8601 parser (no `chrono`)
- [ ] write tests: ISO parsing valid/invalid, clock across midnight
      in fixed timezone (use `TZ=UTC` env in tests), countdown for
      0/1m/1h/1d, invalid epoch input
- [ ] run `cargo test` — must pass before Task 11b

### Task 11b: Hand-roll arg parser

**Files:**
- Create: `src/args.rs`
- Modify: `src/lib.rs` (export `args`)

- [ ] `pub struct Args { format: Option<String>, template:
      Option<String>, debug: bool, check: bool, version: bool,
      help: bool, _unknown: Vec<String> }`
- [ ] `pub fn parse(argv: &[String]) -> Args` — recognises
      `--format <STR>`, `--template <NAME>`, `--debug`, `--check`,
      `--version`, `--help`. Anything else collected into `_unknown`
      (do not error on unknown for compat — render with default and
      ignore)
- [ ] write tests: each flag parsed, multi-flag, malformed
      (`--format` with no value), unknown flags don't error,
      `--help` and `--version` set their bools
- [ ] run `cargo test` — must pass before Task 11c

### Task 11c: Wire `main.rs` orchestration + render flow

**Files:**
- Modify: `src/main.rs`

- [ ] read env `STATUSLINE_OAUTH_BASE_URL` (default
      `"https://api.anthropic.com"`); thread it into the
      `api::fetch_usage(token, base_url)` call. This is the seam
      that lets `tests/golden.rs` (Task 14) point at `mockito`
      without code changes.
- [ ] dispatch: if `args.help` → print usage, exit 0; if `args.version`
      → print version, exit 0; if `args.check` → call `check::run()`,
      exit with its code; else render flow.
- [ ] Render flow:
      1. read stdin → `Payload`
      2. if `payload.rate_limits` populated → build `RenderCtx`
         directly (mapping happens here, not in `format/`), no HTTP
      3. else → check cache (fresh? hit). If miss + lock active,
         fall back to stale or render without quota. Else → read
         creds, call `api::fetch_usage`, write cache or lock based
         on outcome
      4. resolve template source (`--format` > `STATUSLINE_FORMAT` >
         `--template` > built-in `default`)
      5. `format::render(template, &ctx)` → stdout
      6. if `args.debug`, emit trace to stderr
      7. exit 0 (always — never non-zero in render mode)
- [ ] every error path collapses to "render line without quota
      segment + emit trace + exit 0"
- [ ] write a test asserting that even when every subsystem returns
      an error (mock creds missing, network unreachable, cache dir
      read-only), `main` still exits 0 and stdout is non-empty
- [ ] write a test that pipes a `pro_max_with_rate_limits` fixture
      and confirms zero HTTP calls escape (assert via mockito's
      `expect(0)` on any registered mock)
- [ ] run `cargo test` — must pass before Task 12

### Task 12: Implement `check.rs` (`--check` setup verification)

**Files:**
- Create: `src/check.rs`
- Modify: `src/main.rs` (dispatch `--check` to `check::run`)

- [ ] `pub fn run() -> ExitCode`: print a human-readable diagnostic
      to stdout in sections (Credentials, Network, Cache, Format).
      For each: ✓ on success, ✗ on failure with a one-line reason.
      Exit non-zero if any section fails.
- [ ] Credentials section: try Keychain (if macOS), then file. Report
      which path succeeded and the token's fingerprint (NOT the token).
- [ ] Network section: actual `fetch_usage` call against
      `api.anthropic.com`. Report HTTP status and round-trip time.
- [ ] Cache section: try to read existing cache file; report freshness
      and whether the JSON is parseable.
- [ ] Format section: render the built-in `default` template with a
      stub `RenderCtx`; ensure no panic.
- [ ] write tests covering: all-pass scenario (mocked subsystems),
      network-mocked-success, network-mocked-failure (401/429/500),
      creds-missing, cache-corrupt scenario. Assert exit code matches
      expectation per scenario.
- [ ] run `cargo test` — must pass before Task 13

### Task 13: Author seed templates and wire `include_str!`

**Files:**
- Create: `templates/README.md`
- Create: `templates/default.txt`
- Create: `templates/minimal.txt`
- Create: `templates/compact.txt`
- Create: `templates/bars.txt`
- Create: `templates/colored.txt`
- Create: `templates/emoji.txt`
- Create: `templates/emoji_verbose.txt`
- Create: `templates/verbose.txt`
- Modify: `src/format/mod.rs` (add `BUILTIN_TEMPLATES` table via
  `include_str!`)

- [ ] author each template's one-line content per the catalogue in
      `docs/research.md` and the brainstorm. `default.txt` is what
      ships in the printed `settings.json` snippet
- [ ] `templates/README.md`: explain the format-string syntax,
      placeholder list, threshold env vars. (Detailed Nerd Font /
      troubleshooting tips deferred to Task 17.)
- [ ] in `format::mod`, expose
      `pub fn lookup_template(name: &str) -> Option<&'static str>`
      backed by a `match` over
      `include_str!("../../templates/<n>.txt")` (path is relative
      to the file containing the macro: `src/format/mod.rs` → `..`
      = `src/`, `../..` = repo root, `../../templates/<n>.txt`
      = the template file)
- [ ] verify the include path: `cargo build` must succeed; if it
      fails with "file not found", the include path is wrong and
      needs `../../../templates/<n>.txt` instead
- [ ] precedence resolver in `main.rs` consumes this lookup
- [ ] write tests: `lookup_template("default")` returns `Some(s)`
      where `!s.is_empty()`; unknown name returns `None`; every
      shipped template renders successfully against a known-good
      `RenderCtx` (no panics, no empty output)
- [ ] run `cargo test` — must pass before Task 14

### Task 14: End-to-end golden tests in `tests/golden.rs`

**Files:**
- Create: `tests/golden.rs`
- Create: `tests/fixtures/pro_max_with_rate_limits.json`
- Create: `tests/fixtures/api_key_no_rate_limits.json`
- Create: `tests/fixtures/extra_usage_enabled.json`
- Create: `tests/fixtures/malformed_field.json`

- [ ] each fixture is a realistic Claude Code stdin payload
- [ ] use `assert_cmd` to spawn the release binary, pipe each fixture
      to stdin, capture stdout. Use `mockito` to override the OAuth
      endpoint (passed via env `STATUSLINE_OAUTH_BASE_URL` for tests)
- [ ] **structural assertions** (not byte-exact) for stability:
      `pro_max_with_rate_limits` → output matches regex
        `^[^·]+ · 5h: \d{1,3}% · 7d: \d{1,3}% \(resets \d{2}:\d{2}\)$`
      `api_key_no_rate_limits` + mocked OAuth 200 → similar regex
      `api_key_no_rate_limits` + mocked OAuth 401 → no `%` symbol
        in output, line still non-empty, exit 0
      `api_key_no_rate_limits` + mocked OAuth 429 → fallback line,
        cache lock written
      `extra_usage_enabled` → contains `Extra:` segment
      `malformed_field` → graceful degrade, exit 0
- [ ] add **one** byte-exact "snapshot" test on a frozen
      template/fixture pair as a brittle-but-useful canary; mark with
      a comment that it's expected to break on intentional formatter
      changes and is updated by hand
- [ ] write a test asserting `tests/fixtures/*.json` contains no
      string that looks like a real bearer token (length > 30 alnum
      sequence) — fixture hygiene
- [ ] verify total integration suite < 5 s wall time
- [ ] run `cargo test` — must pass before Task 15

### Task 15: CI release workflow `.github/workflows/release.yml`

**Files:**
- Create: `.github/workflows/release.yml`

- [ ] `on: push: tags: ['v*']`
- [ ] matrix: 4 targets (`aarch64-apple-darwin` on `macos-14`,
      `x86_64-apple-darwin` on `macos-13`,
      `x86_64-unknown-linux-musl` on `ubuntu-latest`,
      `aarch64-unknown-linux-musl` on `ubuntu-latest` via `cross`)
- [ ] use `taiki-e/upload-rust-binary-action@v1` with archive name
      `claude-statusline-<ref>-<target>.tar.gz`. Include
      `bin/claude-statusline`, `templates/*`, top-level `README.md`.
      The action emits `.sha256` automatically — do NOT add a
      separate sha256 step.
- [ ] add `permissions: contents: write` for the release-upload step
- [ ] write a tiny dry-run job (gated `if: github.event_name ==
      'pull_request'`) that runs `cargo build --release` only — no
      upload — sanity-check the build still works on PRs touching
      this file
- [ ] verify by tagging a `v0.0.0-test` on a throwaway branch (clean
      up afterwards)
- [ ] run `cargo test` — must pass before Task 16

### Task 16: Remote installer `scripts/install.sh`

**Files:**
- Create: `scripts/install.sh`

- [ ] POSIX `sh`, ~80 LOC, no `cargo`/`rust`/`rustup` references
      (asserted by Task 1's `check-invariants.sh`)
- [ ] detect platform via `uname -sm` → one of the 4 targets
- [ ] resolve version: env `VERSION` (default `latest`) → if `latest`,
      `curl -fsSL api.github.com/repos/<owner>/<repo>/releases/latest`
      and extract `tag_name` via `sed`
- [ ] `curl -fsSL` the tarball + `.sha256`; verify with
      `shasum -a 256 -c`; abort on mismatch
- [ ] `tar -xzf` into `mktemp -d`; `install -m 0755` the binary to
      `~/.claude/bin/`; `cp templates/*` to
      `~/.config/claude-statusline/templates/`
- [ ] print a settings.json snippet — never edit settings.json
- [ ] `trap 'rm -rf "$TMP"' EXIT` for cleanup
- [ ] `chmod +x scripts/install.sh`
- [ ] manual smoke test: run on a clean macOS box (no install),
      verify it works end-to-end (also a Post-Completion item)
- [ ] confirm Task 1's `check-invariants.sh` still passes now that
      `install.sh` exists (no cargo/rust references in it)
- [ ] run `cargo test` and `bash scripts/check-invariants.sh` —
      must pass before Task 17

### Task 17: README polish — Nerd Font, troubleshooting, gallery

**Files:**
- Modify: `README.md`
- Modify: `templates/README.md` (cross-reference)
- Modify: `docs/plan.md` (add deprecation banner)

- [ ] root `README.md` covers: install (`curl … | sh` line + manual
      tarball steps), settings.json snippet, every shipped template
      rendered as a fenced code sample, placeholder reference table,
      threshold env vars, `--debug` and `--check` usage
- [ ] **Nerd Font tips** section: list of common glyphs (`` clock,
      `` warning, `` battery), recommended fonts (Nerd Font
      family overview), how to test rendering, fallback behaviour
- [ ] **Troubleshooting** section:
      - macOS quarantine: `xattr -d com.apple.quarantine
        ~/.claude/bin/claude-statusline`
      - blank status line: run `claude-statusline --check`, then
        `--debug` for stderr trace
      - "no credentials" diagnosis path
      - Linux Secret Service note (we don't use it; file fallback only)
- [ ] **Tip section**: per `docs/research.md` "Failure modes we should
      plan for" — what users should watch out for (Anthropic shape
      changes, multi-account caching, proxy/MITM caveats)
- [ ] add a `CHANGELOG.md` with `## v0.1.0` entry summarising the
      feature set
- [ ] `docs/plan.md` gets a top banner: "**Superseded by
      `docs/plans/2026-04-26-rust-statusline.md`** — kept for
      historical reference of the sh-based design. See also
      `docs/research.md` (API contract) and `docs/security-review.md`
      (why we built our own instead of installing ccstatusline)."
- [ ] no test changes (docs only); confirm `cargo test` still passes
      to be safe

### Task 18: Verify acceptance criteria

- [ ] verify all requirements from Overview are implemented
- [ ] verify the 500-LOC ceiling holds: `bash scripts/check-loc.sh`
      passes
- [ ] verify every Hard Invariant: `bash scripts/check-invariants.sh`
      passes
- [ ] verify binary size: `cargo build --release && ls -la
      target/release/claude-statusline` < 1.5 MB on macOS-arm64
- [ ] verify cold-start time:
      - macOS-arm64: `hyperfine --warmup 5 "echo '{}' |
        target/release/claude-statusline"` median < 10 ms
      - Linux-x86_64-musl: same command, median < 15 ms (factor in
        slower CI/SSH-loop runners)
- [ ] verify the binary never exits non-zero in render mode by
      running each fixture with adverse env (no creds, no network)
      and asserting `$?` is 0
- [ ] verify the bearer token is never written to disk: stress-run
      the binary with a fixture token, then `grep -r "<token>"
      ~/.cache/claude-statusline/ ~/Library/Caches/claude-statusline/`
      → must return zero matches
- [ ] verify all docs cross-links resolve: `grep -RhoE
      '\[.+\]\([^)]+\)' README.md docs/ | sort -u | check`
- [ ] run full test suite: `cargo test --release`
- [ ] run `cargo clippy --all-targets -- -D warnings` and
      `cargo fmt --check`
- [ ] run `shellcheck scripts/install.sh scripts/check-loc.sh
      scripts/check-invariants.sh`

### Task 19: Final — move plan to completed

- [ ] update root `README.md` if any new patterns emerged during impl
- [ ] update CLAUDE.md if needed (probably not — this is a single-tool
      project, conventions live in `docs/`)
- [ ] `mkdir -p docs/plans/completed && git mv
      docs/plans/2026-04-26-rust-statusline.md
      docs/plans/completed/`
- [ ] commit + tag `v0.1.0` (after CI green)

---

## Technical Details

### Data structures

- `Payload` (`src/payload.rs`): mirrors Claude Code's stdin JSON.
  Every field `Option<…>`. Unknown fields ignored.
- `UsageResponse` (`src/api/response.rs`): mirrors the OAuth endpoint.
  Every field `Option<…>`.
- `UsageCache` (`src/cache/mod.rs`): on-disk cache schema.
  **Deliberately omits any token field** — compile-time guarantee
  asserted by Task 9 test.
- `RenderCtx` (`src/format/placeholders.rs`): primitives only — no
  dependency on `api::` or `cache::` types. Built fresh per
  invocation; not persisted. Mapping from `UsageResponse` /
  `UsageCache` to `RenderCtx` lives in `main.rs` (Task 11c).
- `Lock` (`src/cache/lock.rs`): `{ blocked_until: u64, error: LockError }`.
- `Trace` (`src/debug.rs`): `{ path, cache, http, took_ms, error }`,
  emitted as one JSON line to stderr if debug enabled.
- `Args` (`src/args.rs`): parsed CLI flags + collected unknowns.

### Wire formats and parameters

- **OAuth endpoint**: `GET https://api.anthropic.com/api/oauth/usage`
  with `Authorization: Bearer …`, `anthropic-beta: oauth-2025-04-20`,
  `User-Agent: claude-statusline/<version>`.
- **Cache file**: `~/.cache/claude-statusline/usage.json` (Linux) /
  `~/Library/Caches/claude-statusline/usage.json` (macOS) via
  `directories` crate.
- **Lock file**: same dir, `usage.lock`.
- **Template lookup root** (user-supplied):
  `~/.config/claude-statusline/templates/<name>.txt` if present; else
  fall back to baked-in copy.

### Processing flow (render mode)

```
read stdin → parse Payload
  ├── rate_limits present? → build RenderCtx, render, exit 0
  └── absent → check cache:
       ├── fresh (≤180 s) → build RenderCtx from cache, render, exit 0
       └── stale or missing → check lock:
            ├── active → use stale cache or drop quota → render → exit 0
            └── inactive → read creds → call fetch_usage:
                 ├── 200 → write cache, clear lock → render → exit 0
                 ├── 429 → write lock with Retry-After → use stale → render → exit 0
                 ├── 401 → write lock 3600 s → drop quota → render → exit 0
                 └── 5xx/timeout → write lock per backoff → use stale → render → exit 0
```

Always exit 0 in render mode. Always emit something to stdout.

## Post-Completion

*Items requiring manual intervention or external systems — no checkboxes, informational only.*

**Manual verification on real installs**:
- run `claude-statusline --check` against a real Claude.ai Pro/Max
  account; verify Credentials/Network/Cache/Format all green
- install on a clean macOS-arm64 box via the curl-pipe install.sh and
  verify the line renders within 5 s of the first message
- install on a clean Linux-x86_64-musl Alpine container; same check
- verify the macOS quarantine xattr workaround actually clears
  Gatekeeper warnings
- soak test for 24 h on a heavily-used Pro/Max account; check that
  the lock file backs off correctly when 429s occur

**External system updates**:
- push first `v0.1.0` tag to GitHub once tests are green
- verify `taiki-e/upload-rust-binary-action@v1` produced all 4
  platform tarballs in the GitHub Release
- verify the `install.sh` curl-pipe URL resolves and works end-to-end
  for at least one new install
- (optional) submit to `awesome-claude-code` once stable

**Sanity caveats** (not action items, but worth re-stating):
- The OAuth `usage` endpoint is undocumented; Anthropic can change
  it. ccstatusline and botfarm have both already had to update.
  Monitor `sirmalloc/ccstatusline` for shape changes.
- If a future Claude Code release surfaces quota in stdin under all
  auth types (not just Pro/Max post-first-API-call), the OAuth
  fallback layer becomes dead code and we should remove it.
