# Changelog

All notable changes to this project will be documented in this file.

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
