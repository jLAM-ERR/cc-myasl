# Changelog

All notable changes to this project will be documented in this file.

## [v0.1.1] — 2026-04-27

Branding + first-release shakedown fixes. No runtime behaviour change.

### Changed

- README rebranded to `cc-myasl — My Yet Another Status Line for
  Claude Code` to match the GitHub repo. Binary keeps the descriptive
  name `claude-statusline`.
- New `CLAUDE.md` with project guidance, hard invariants, and a
  "do-not" list for future contributors (and for Claude Code itself
  when working in this tree).

### Fixed (CI shakedown)

- `release.yml`: added a `create-release` job; matrix jobs depend on
  it. Without this, `taiki-e/upload-rust-binary-action@v1` retried
  "release not found" until timeout.
- `release.yml`: dry-run job (PR-only) now skips musl Linux targets,
  which need `cross` / `musl-tools` not present on `ubuntu-latest`.
- `ci.yml`: `shellcheck` step gated to Linux runners (not pre-installed
  on macOS GHA images).
- `cwd_substitutes_home` test: now self-contained — sets HOME to a
  fixed value under `creds::HOME_MUTEX`, restores after the test, no
  longer races with `creds::tests` which clear HOME on tear-down.
- Golden test 5 (`api_key_oauth_429_writes_lock`): the helper
  `cache_dir_for_home` was using the macOS path layout
  (`<home>/Library/Caches/<qual>.<org>.<app>`) on Linux. Linux uses
  just the application name (`<home>/.cache/<app>`) per
  `directories::ProjectDirs`. Helper now branches per-OS correctly.
- Golden tests now also pin `XDG_CACHE_HOME` (not just HOME), so
  `directories::ProjectDirs` lands inside the test's tempdir on Linux
  CI runners that export their own `XDG_CACHE_HOME`.

### Internal

- Auto-review pass found and fixed:
  - Doc comment lie in `cache::UsageWindowCache::utilization` saying
    `[0.0, 1.0]` — actual values are 0..=100.
  - Duplicated `DEFAULT_TEMPLATE` between `main.rs` and `check.rs`
    hoisted to `format::DEFAULT_TEMPLATE`. Same for
    `DEFAULT_OAUTH_BASE_URL` → `api::DEFAULT_OAUTH_BASE_URL`.
  - `templates/verbose.txt` had `{cwd_basename}` outside any optional
    block, producing dangling separators on API-key sessions.
  - README's "aborts on mismatch" claim about install.sh checksum was
    inaccurate (script warns and continues if `.sha256` is absent).

## [v0.1.0] — 2026-04-27

Initial release.

### Features

- Reads Claude Code's stdin JSON, displays remaining 5h/7d token quota.
- Falls back to `api.anthropic.com/api/oauth/usage` (OAuth Bearer from
  macOS Keychain or `~/.claude/.credentials.json`) when stdin's
  `rate_limits` field is absent.
- 9 baked-in templates (`default`, `minimal`, `compact`, `bars`,
  `colored`, `emoji`, `emoji_verbose`, `verbose`).
- Fully template-driven format with `{? optional }` segments,
  threshold-driven colour and icon picking.
- `--check` diagnostic, `--debug` JSON trace.
- Cross-platform: macOS arm64/x86_64 + Linux x86_64/aarch64 (musl).

### Security

- Bearer token never written to disk.
- Bearer token never logged (only an opaque fingerprint).
- No `security dump-keychain` enumeration.
- No runtime npm fetch in install path.
- install.sh verifies SHA-256 checksums on tarballs.

### Internal

- 500-LOC-per-file ceiling enforced by CI.
- 303 tests covering unit + integration + invariant assertions.
- Single binary per target, ≤1.5 MB stripped (release profile).
