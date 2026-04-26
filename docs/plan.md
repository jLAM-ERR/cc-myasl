> **Superseded by [`docs/plans/2026-04-26-rust-statusline.md`](plans/2026-04-26-rust-statusline.md)**
>
> This file documents the original POSIX `sh + jq + curl` approach (Option B
> in the original brainstorm). It was replaced by the Rust v0.1.0 implementation
> for cross-platform reliability, perf, and type-safe API parsing.
>
> Kept for historical reference. See also:
> - [`docs/research.md`](research.md) — API contract discovery
> - [`docs/security-review.md`](security-review.md) — why we built our own instead of installing ccstatusline

# Implementation plan — remaining-quota status line

Source for this plan: `docs/research.md` (the API contract +
ccstatusline / botfarm patterns) and `docs/security-review.md` (which
risks are inherited by which option).

## Three options

| | A. stdin-only | **B. stdin + OAuth fallback (recommended)** | C. install ccstatusline |
|---|---|---|---|
| Effort | low | medium | tiny |
| Blank on first turn | yes | **no** | no |
| Works under API-key auth | no | yes (with `.credentials.json`) | yes |
| Adds dep | none | `jq`, `curl` | Bun + ccstatusline |
| Customisable | full | **full** | TUI only |
| Hits private API | no | yes | yes |
| Inherits #298 supply-chain risk | no | no | yes (unless pinned) |

Decision still pending. Default recommendation is **B** with
`find-generic-password`-only credential lookup (no `dump-keychain`).

## Phased build for Option B

### Phase 0 — Decide
- Pick A / B / C. (We default to B.)
- Confirm Pro/Max plan auth with `/usage` in Claude Code.
- Confirm `~/.claude/.credentials.json` exists *or* the
  `Claude Code-credentials` Keychain entry is present:
  ```bash
  security find-generic-password -s "Claude Code-credentials" -w >/dev/null \
    && echo "Keychain ✓"
  test -f "$HOME/.claude/.credentials.json" && echo "File ✓"
  ```
- Decide display string. Proposed default:
  `<model> · <cwd> · 5h: 76% left · 7d: 41% left (resets 18:00)`.

### Phase 1 — Script (`statusline.sh`, POSIX `#!/bin/sh`)
Constraints: POSIX `sh`, `jq` and `curl` required, no zsh-isms,
graceful degrade on every missing field.

Skeleton:
```sh
#!/bin/sh
set -u
input="$(cat)"
get() { printf '%s' "$input" | jq -r "$1 // empty"; }

model=$(get '.model.display_name')
dir=$(get '.workspace.current_dir' | sed "s|^$HOME|~|")

five_used=$(get '.rate_limits.five_hour.used_percentage')
seven_used=$(get '.rate_limits.seven_day.used_percentage')
seven_reset=$(get '.rate_limits.seven_day.resets_at')

# Fallback to OAuth endpoint when stdin field is missing.
if [ -z "$five_used" ] || [ -z "$seven_used" ]; then
    # ... cache check, lock check, token read,
    #     curl with --max-time 5 + Authorization + anthropic-beta,
    #     write cache on 200, write lock with Retry-After on 429,
    #     mark token blocked for 1h on 401.
    :
fi

# Format output, omitting any segment whose data is missing.
```

Key rules baked in:
- **Credentials**: try `security find-generic-password` first on
  macOS, fall back to `~/.claude/.credentials.json`. **Never**
  enumerate the keychain.
- **Cache**: `~/.cache/claude-statusline/usage.json` (TTL 180 s) +
  `usage.lock` (lock-only-when-needed; honour `Retry-After`).
- **Portability**: file-mtime via
  `stat -f %m "$f" 2>/dev/null || stat -c %Y "$f"` (BSD vs GNU).
- **Fail closed**: any error path emits the line *without* the quota
  segment; never blocks Claude Code.
- **Never log the bearer**.

### Phase 2 — Wire it up
Edit `~/.claude/settings.json` and merge:
```json
{ "statusLine": { "type": "command", "command": "~/.claude/statusline.sh" } }
```
`chmod +x ~/.claude/statusline.sh`.

### Phase 3 — Verify
- Stub stdin without `rate_limits` → confirm OAuth fallback fires
  exactly once and writes cache.
- Stub stdin *with* synthetic `rate_limits` → confirm formatting,
  no network call.
- Hammer test: `for i in $(seq 1 20); do … script </dev/null; done`
  — exactly one HTTP call escapes thanks to the cache.
- Reload Claude Code; eyeball.

### Phase 4 — Optional polish
Borrow from ccstatusline / botfarm references:
- Colour thresholds (red <20% left, yellow 20–49%, green ≥50%).
- Progress bar variant: `[████████░░] 41%`.
- Reset countdown (`resets in 2h13m`) — friendlier than wall-clock.
- Token-rotation log: write `sha256(token[-8:])[:16]` fingerprint to
  a tiny on-disk log so you can correlate "why did the bearer
  change?" with re-auth events.
- Adaptive backoff: bump cache TTL when consecutive failures grow,
  cap at 1800 s.

### Phase 5 — Sanity caveats (not blocking)
- The OAuth `usage` endpoint is **undocumented**; Anthropic can change
  it. ccstatusline and botfarm have both already had to update.
- Keychain prompt may pop the first time `security` runs from a new
  binary path — expected.
- Under API-key auth the endpoint will return 401 forever — handle by
  silently dropping the quota segment.

## What "done" looks like

A single executable `statusline.sh` in this repo, plus a small README
in this repo describing how to install it (file path, settings.json
snippet, jq prerequisite). No npm, no Bun, no Node. ~150 lines of
POSIX `sh` total.

## Out of scope (for now)

- Block-timer widget (5-hour usage countdown).
- Context-window percentage.
- Per-account session affinity (relevant if you run multiple
  `CLAUDE_CONFIG_DIR`s).
- Anything that requires a TUI editor to reconfigure widgets.
