# Contributing to cc-myasl

This document describes how to build, test, and contribute changes to
`cc-myasl`. The user-facing usage docs live in [README.md](README.md).

For project conventions, hard invariants, and the "things to NOT do"
list, also read [CLAUDE.md](CLAUDE.md) — it is written for both
human contributors and Claude Code agents working in the tree.

## Build

```sh
cargo build --release
```

The release profile uses `lto = "fat"`, `codegen-units = 1`,
`strip = true`, and `panic = "abort"` to produce a stripped binary of
≤ 1.5 MB.

## Test

```sh
cargo test
```

The test suite covers unit tests for every module, integration tests via
`tests/golden.rs` (spawn the binary, pipe fixture JSON, assert output),
and invariant assertions. The OAuth endpoint is mocked via `mockito` —
no real network calls in CI.

To run only the integration suite:

```sh
cargo test --test golden
```

To run the manual real-HTTPS smoke test that is `#[ignore]`-marked by
default:

```sh
cargo test -- --ignored
```

## Lint and format

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Both gates run in CI on every push and PR.

## LOC budget

Each file in `src/**/*.rs` is capped at 500 lines. CI enforces this with
`scripts/check-loc.sh`. If a file approaches 400 lines, split it before
adding more.

## Hard invariants (CI-gated)

`scripts/check-invariants.sh` asserts:

1. No `security dump-keychain` invocation in `src/` or `scripts/`.
2. No `npx.*latest` or `@latest` in `src/` or `scripts/`.
3. No `cargo`, `rustc`, or `rustup` references in `scripts/install.sh`.

Run manually with:

```sh
bash scripts/check-invariants.sh
bash scripts/check-loc.sh
```

## Shellcheck

```sh
shellcheck scripts/*.sh
```

Required clean. CI runs this on Linux only (`ubuntu-latest` ships
`shellcheck`; `macos-latest` does not — POSIX-`sh` syntax checking is
OS-independent so the Linux leg is sufficient coverage).

## Test isolation

Two `pub(crate)` mutexes serialise tests that read or mutate
process-global env vars:

- `creds::HOME_MUTEX` — for tests touching `HOME`.
- `format::ENV_MUTEX` — for tests touching `STATUSLINE_RED` /
  `STATUSLINE_YELLOW`.

If you add a test that reads or writes any of these env vars, acquire
the appropriate mutex. See `src/format/mod.rs` and `src/creds.rs` for
examples.

## Filing issues and pull requests

- Repository: <https://github.com/jLAM-ERR/cc-myasl>
- For bug reports, please run `cc-myasl --check` and include the output
  (the home-directory path is automatically tilde-redacted in the
  diagnostic).
- For PRs, target `main`. CI runs the full gate suite + per-leg status
  checks.

The `main` branch is protected — direct pushes are rejected. All
changes go through pull requests.
