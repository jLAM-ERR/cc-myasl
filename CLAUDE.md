# CLAUDE.md

Guidance for Claude Code (claude.ai/code) when working in this repository.

## Project in one paragraph

`cc-myasl` (My Yet Another Status Line) is a Rust v1.0 single-binary
tool that renders remaining Claude.ai 5-hour and 7-day token quota in
the Claude Code status line. The shipped binary is named
`cc-myasl` (descriptive of what it does). Reads Claude Code's
stdin JSON; if the official `rate_limits` field is absent (fresh
session, non-Pro/Max, first turn), falls back to
`GET https://api.anthropic.com/api/oauth/usage` using a Bearer token
read from the macOS Keychain entry `Claude Code-credentials` or
`~/.claude/.credentials.json`. Output is fully config-driven — 8 baked-in templates as JSON Configs
plus user-overridable `--config` and `--template <name>` flags — so
users can change which segments appear without rebuilding.

The canonical implementation plan is
`docs/plans/completed/2026-04-26-rust-statusline.md`. Read it first
for any non-trivial change.

## Commands

```shell
# Build (debug)
cargo build

# Build release (≤ 1.5 MB stripped binary on macOS-arm64)
cargo build --release

# Run the full test suite (303 tests, 1 ignored real-HTTPS smoke)
cargo test

# Run only the integration suite
cargo test --test golden

# Lint and format gates
cargo fmt --check
cargo clippy --all-targets -- -D warnings

# Hard-invariant gates (CI-enforced)
bash scripts/check-loc.sh           # ≤ 500 LOC per *.rs
bash scripts/check-invariants.sh    # no dump-keychain / @latest / cargo refs in install.sh

# Shell linting (Linux/macOS with shellcheck installed)
shellcheck scripts/*.sh

# Manual cold-start benchmark (requires hyperfine)
hyperfine --warmup 5 'echo "{}" | target/release/cc-myasl'

# Run the diagnostic
./target/release/cc-myasl --check

# Run with debug trace to stderr
STATUSLINE_DEBUG=1 ./target/release/cc-myasl < some-stdin.json
```

## Big-picture architecture

### Module tree (locked in v1.0)

```
src/
├── main.rs           orchestration: arg parse + dispatch + render flow + debug trace
├── check.rs          --check command (only path that may exit non-zero)
├── debug.rs          Trace struct + emit_to_stderr
├── error.rs          single Error enum + From impls
├── payload.rs        serde for Claude Code stdin JSON (extended in Phase 2)
├── payload_mapping.rs  Payload → RenderCtx mapping (extracted from main.rs in Phase 2)
├── creds.rs          Keychain (macOS) + ~/.claude/.credentials.json fallback
├── time.rs           utc→local clock, ms→"2h13m" countdown, ISO-8601 parser
├── args.rs           hand-rolled CLI parser (no clap)
├── api/{mod,response,retry}.rs       HTTP client + serde + Retry-After
├── cache/{mod,lock,backoff,atomic_helper}.rs   Disk cache + lock + backoff ladder
├── format/{mod,parser,placeholders,values,thresholds}.rs   Template engine (segment rendering)
├── config/{mod,schema,builtins,render}.rs      Structured JSON config + 9 built-ins + multi-line renderer
└── git/{mod,status}.rs               gix-based git discovery + branch/root + status counters
```

### Three-stage render flow (`main.rs`)

```
read stdin → parse Payload
  ├── rate_limits present? → build RenderCtx directly, no HTTP
  └── absent → cache hit? else lock active? else read creds, fetch:
       ├── 200 → write cache, clear lock, render, exit 0
       ├── 401 → lock 3600s, drop quota or use stale, render, exit 0
       ├── 429 → lock = max(Retry-After, 300s), use stale, render, exit 0
       ├── 5xx/timeout → exp backoff lock, use stale, render, exit 0
       └── creds missing → drop quota, no lock, render, exit 0
```

**Render mode ALWAYS exits 0.** `--check` is the only path that may
exit non-zero.

### Hard invariants (CI-gated; do not weaken)

1. **No file in `src/**/*.rs` exceeds 500 LOC** — `scripts/check-loc.sh`.
2. **Bearer token never written to disk** — `UsageCache` schema deliberately
   omits any token field; verified by serialization-substring test in
   `cache::tests` and golden test 8.
3. **`main.rs` never exits non-zero in render mode** — verified by
   adversarial test in `main::tests` and golden test 7.
4. **No `security dump-keychain` invocation** — `scripts/check-invariants.sh`.
5. **`scripts/install.sh` has no cargo/rust references** — same script.
6. **No `npx -y …@latest` style auto-update path** — same script.
7. **`format/*.rs` must NOT import `crate::config`** — one-way dependency;
   verified by string-scan test in `format::mod::tests`.
8. **`config/*.rs` must NOT import `crate::api` or `crate::cache`** —
   parallel to format's decoupling invariant; verified by string-scan test
   in `config::tests`.
9. **`git/*.rs` must NOT import `crate::format`, `crate::config`, `crate::api`,
   or `crate::cache`** — low-level module; verified by string-scan test in
   `git::tests` and by `scripts/check-invariants.sh`.
10. **`format/*.rs` (including `format/placeholders/*.rs`) must NOT import
    `crate::git`** — git data flows into `RenderCtx` as primitives via
    `payload_mapping::populate_git_ctx`; the render engine never reaches the
    git module directly. Verified by string-scan test in
    `format::placeholders::tests` and by `scripts/check-invariants.sh`.

### Format engine decoupling invariant

`format/*.rs` files MUST NOT contain `use crate::api;` or
`use crate::cache;`. `RenderCtx` is composed of primitives only
(`Option<String>`, `Option<f64>`, `Option<u64>`, `Option<bool>`,
`Option<PathBuf>`). The mapping from `UsageResponse` and `UsageCache`
into `RenderCtx` lives in `main.rs::run_render`, never in `format/`.
This keeps the format engine reusable and trivially unit-testable.

A test in `format::placeholders::tests` enforces this invariant
by string-scanning the source files.

### Cross-test env-var serialization

Tests that read or mutate process-global env vars share five mutexes
(`pub(crate)` from their respective modules):

- `creds::HOME_MUTEX` — for tests touching `HOME`. Tests that mutate it
  MUST restore the original value (or unset cleanly) before releasing.
- `format::ENV_MUTEX` — for tests touching `STATUSLINE_RED` / `STATUSLINE_YELLOW`.
- `config::CONFIG_MUTEX` — for tests touching `STATUSLINE_CONFIG` or
  `XDG_CONFIG_HOME`. Declared in `src/config/mod.rs` (test-gated).
- `config::render::COLS_MUTEX` — for tests touching `STATUSLINE_TEST_COLS`.
  `STATUSLINE_TEST_COLS` is a **test-only** escape hatch read by
  `config::render` for deterministic flex-spacer width in unit tests.
  It is never set by production code and must NOT appear in README or
  user-facing docs. Declared in `src/config/render.rs` (test-gated).
- `git::GIT_ENV_MUTEX` — for tests touching `GIT_CEILING_DIRECTORIES` (used
  to prevent gix discovery from walking outside the test tempdir). Declared
  in `src/git/mod.rs` (test-gated).

Without these, parallel `cargo test` interleaves env writes and tests
flap. If you add a new test that reads or writes any of these vars,
acquire the appropriate mutex and restore prior values before releasing.

### Distribution

- `scripts/install.sh` is a curl-pipe POSIX-sh remote installer. Defaults
  `OWNER=jLAM-ERR REPO=cc-myasl`; users override via env. Verifies
  SHA-256 sidecar before extracting. **Never edits `~/.claude/settings.json`** —
  prints the snippet for the user to merge.
- `.github/workflows/release.yml` matrix-builds 4 targets
  (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-unknown-linux-musl`,
  `aarch64-unknown-linux-musl`) on `v*` tag push. A separate
  `create-release` job runs first to materialise the GitHub Release;
  matrix jobs depend on it and upload via
  `taiki-e/upload-rust-binary-action@v1`.
- Release archives are named `cc-myasl-<ver-without-v>-<target>.tar.gz`
  to match `install.sh`'s URL pattern (the workflow strips the leading
  `v` from `GITHUB_REF_NAME` for this).

## Test architecture

- **Unit tests** (`#[cfg(test)] mod tests {}`) in every module — required
  for every code change, no exceptions.
- **Integration tests** (`tests/golden.rs`) spawn the release binary,
  pipe a fixture stdin, mock OAuth via `mockito` (plain HTTP — `ureq`
  with `rustls` only enforces TLS for `https://`), assert structural
  output. The 8 fixtures cover hot-path, OAuth-fallback, 401/429/500,
  malformed-payload, and fixture-hygiene paths.
- **Phase 2 golden tests** (`tests/golden_phase2.rs`) use
  `tests/fixtures/full-payload.json` as the standard fixture — a
  comprehensive stdin payload populating every Phase 2 field. Use this
  fixture as the base for any new placeholder tests.
- **No real-network tests** in CI. The one `#[ignore]`-marked test in
  `api::tests` exercises a real `https://example.com` request only when
  invoked manually via `cargo test -- --ignored`.
- **Cross-platform isolation**: golden tests pin `HOME`, `XDG_CACHE_HOME`,
  and `XDG_CONFIG_HOME` to a tempdir so the binary's `directories::ProjectDirs`
  resolves to the test-controlled location regardless of runner env.

## Things to NOT do

- Do NOT add new top-level dependencies without explicit justification.
  The locked dep set is `ureq + rustls`, `serde + serde_json`,
  `directories`, `anyhow`, `terminal_size` (flex-spacer correctness;
  ioctl alternative requires unsafe libc), `gix` (slim build,
  `default-features = false`; git repo discovery + branch reads;
  avoids fragile shell-out parsing; matches Starship's pattern).
  Dev-deps add `mockito`, `tempfile`, `assert_cmd`, `predicates`.
  No `clap`, no `tokio`/`reqwest`, no `chrono`, no `keyring`.
- The MSRV is `rust-version = "1.85"` (Rust 2024 edition baseline).
  `rust-toolchain.toml` pins channel `1.85`. `gix 0.83` requires the
  `sha1` feature for any commit/tree work — `Cargo.toml` enables only
  `sha1`, no network/signing/async. Older Cargo.lock pins for
  `clru=0.6.2` and `idna_adapter=1.2.0` (workarounds for the prior
  `1.83` floor) are no longer needed and were removed when the
  toolchain bumped. Some deps (icu_*, idna_adapter ≥ 1.2.2) require
  Rust 1.86; the current Cargo.lock holds them at compatible
  versions — do not run `cargo update` without a coordinated MSRV
  bump.
- Do NOT call `security dump-keychain` (or even mention the literal
  string in `src/` or `scripts/` — the invariant grep is naive).
- Do NOT add `@latest` or `npx -y …@latest` patterns anywhere in
  `src/` or `scripts/`.
- Do NOT exit non-zero from `main` in render mode. Every error path
  must collapse to "render line without quota segment + emit trace
  if --debug + exit 0".
- Do NOT log the bearer token. The `Trace` struct only carries the
  fingerprint (`creds::fingerprint` — non-cryptographic SipHash, fine
  for rotation detection only).
- Do NOT pull `format/*.rs` into `crate::api` or `crate::cache` types.
  Use primitives in `RenderCtx`.
- Do NOT import `crate::config` from `format/*.rs`, and do NOT import
  `crate::api` or `crate::cache` from `config/*.rs`. Both are one-way
  dependency invariants enforced by string-scan unit tests.
- Do NOT edit `~/.claude/settings.json` from `install.sh`. Print the
  snippet; let the user merge.
- Do NOT implement `{skills}` or `{git_pr}` in Phase 2 or without a
  separate plan. `{skills}` requires hook data not in stdin;
  `{git_pr}` requires a `gh`/`glab` shell-out plus authenticated API call.
  Both are explicitly deferred to a future plan.

## Reference docs

- `docs/research.md` — API contract + ccstatusline / botfarm patterns.
- `docs/security-review.md` — security audit of the upstream
  ccstatusline; informs which patterns we deliberately avoid.
- `docs/session-2026-04-26.md` — pinned brainstorm session pointer.
- `docs/plan.md` — superseded sh-based plan, kept for historical context.
- `docs/plans/completed/2026-04-26-rust-statusline.md` — the
  authoritative implementation plan, all tasks `[x]`.
- `docs/plans/completed/2026-05-01-phase1-structured-config.md` —
  Phase 1 structured-config rewrite. Implementation complete.
- `docs/plans/completed/2026-05-01-phase2-placeholder-expansion.md` —
  Phase 2 placeholder expansion (stdin extension + gix-based git module).
  Implementation complete.
