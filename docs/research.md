# Research notes — Claude Code remaining-quota status line

Compiled 2026-04-26 from official Claude Code docs, the
`sirmalloc/ccstatusline` source, and the `AlexDobrushskiy/botfarm`
source. Everything below was verified against actual code at that date;
both projects move fast, so re-check before relying on a specific field
name.

## The Claude Code `statusLine` contract

- Configured via `~/.claude/settings.json`:
  ```json
  { "statusLine": { "type": "command", "command": "/path/to/script" } }
  ```
- Claude Code launches the command after every assistant message (and at
  startup), pipes a JSON document to its stdin, reads stdout, and
  renders the first line as the status line.
- The stdin JSON includes (non-exhaustive): `model.display_name`,
  `workspace.current_dir`, `transcript_path`, `session_id`, plus a
  `rate_limits` block — but the rate-limits block is **only** populated
  for Claude.ai Pro/Max subscribers, and **only after the first API
  response of the session**. API-key auth never gets it.
- Shape of the rate-limits block:
  ```json
  "rate_limits": {
    "five_hour": { "used_percentage": 23.5, "resets_at": 1738425600 },
    "seven_day": { "used_percentage": 41.2, "resets_at": 1738857600 }
  }
  ```
  `used_percentage` is 0–100; `resets_at` is a Unix epoch seconds value.
  ⚠️ Note: the OAuth API (below) calls the same field
  `utilization` and uses an ISO-8601 string for the reset time — the
  two surfaces are *not* schema-compatible.

## The undocumented OAuth `usage` endpoint

Both ccstatusline and botfarm hit the same private endpoint that Claude
Code itself uses. This is the only way to get quota numbers
**before** the first API response of a session, and the only way to get
them under API-key auth or in a brand-new shell.

```
GET https://api.anthropic.com/api/oauth/usage
Authorization: Bearer <oauth-access-token>
anthropic-beta: oauth-2025-04-20
User-Agent: <set something — Anthropic rate-limits empty UA>
```

Response shape (verified against `ccstatusline` Zod schema):

```json
{
  "five_hour":  { "utilization": 0..100 | null, "resets_at": "ISO-8601" | null },
  "seven_day":  { "utilization": 0..100 | null, "resets_at": "ISO-8601" | null },
  "extra_usage": {
    "is_enabled":     true | false | null,
    "monthly_limit":  number | null,
    "used_credits":   number | null,
    "utilization":    0..100 | null
  }
}
```

### OAuth bearer token sources

In priority order:

1. **macOS Keychain**
   `security find-generic-password -s "Claude Code-credentials" -w` →
   JSON `{ "claudeAiOauth": { "accessToken": "<token>", ... } }`.
   Extract `claudeAiOauth.accessToken`.
2. **Credentials file (cross-platform fallback)**
   `~/.claude/.credentials.json` — same JSON shape.
3. **(ccstatusline only)** Keychain enumeration via
   `security dump-keychain` to find sibling services with the
   `Claude Code-credentials` prefix (multi-account scenarios). This is
   privacy-noisy — see `security-review.md`.

### Status-code handling that matters

| Status | Meaning                              | Suggested action                                             |
|--------|--------------------------------------|--------------------------------------------------------------|
| 200    | success, parse JSON                   | cache 180s, clear backoff                                    |
| 401    | token expired/rotated                 | drop quota segment, mark token blocked, re-read on next miss |
| 429    | rate-limited by the endpoint itself   | parse `Retry-After` (RFC 9110: int seconds OR HTTP-date)     |
| 5xx    | transient                             | exponential backoff, keep stale cache valid                  |
| timeout| network                                | exponential backoff, keep stale cache valid                  |

botfarm caps adaptive backoff at 1800 s (normal) / 3600 s (401-auth).
ccstatusline uses a simpler 30 s lock + 180 s data cache with
`Retry-After` honoured (closed issue #204 was the bug-report that
caused the latter to be added).

## Patterns from `sirmalloc/ccstatusline` worth borrowing

- **Two-tier cache**: in-memory (per process) + on-disk
  (`~/.cache/ccstatusline/usage.json`). Disk cache survives across the
  many short-lived status-line processes.
- **Lock file** (`usage.lock`) holds either a `blockedUntil` epoch or
  a legacy mtime; throttles network calls when many renders happen
  back-to-back.
- **Cache schema is strict** — Zod schema deliberately omits the bearer
  token; cache file never contains credentials.
- **Stale-cache fallback**: on a transient failure, prefer returning
  the previous good data rather than blanking the widget.
- **`HTTPS_PROXY` honoured** via `https-proxy-agent` — same caveat as
  any proxied auth: the proxy sees the bearer.
- **Widgets**: `WeeklyUsage`, `SessionUsage`, `BlockTimer`,
  `BlockResetTimer`, `WeeklyResetTimer`. Each supports a "raw" mode
  (just the value, no label) and a progress-bar mode.

## Patterns from `AlexDobrushskiy/botfarm/usage.py` worth borrowing

- **Token fingerprint**:
  `sha256(token[-8:]).hexdigest()[:16]` — non-reversible identifier
  used to detect token rotation across runs without ever storing the
  token itself.
- **State machine for token lifecycle**:
  `active → erroring → blocked → recovered | replaced`.
  Three consecutive 401s flip a token to `blocked`; a successful call
  flips it back to `recovered`; a new fingerprint flips it to
  `replaced`. Provides graceful UX during account re-auth.
- **Audit log** (`usage_api_calls` table): one row per HTTP attempt with
  `caller`, `status_code`, `error_type`, `retry_after`, `response_time_ms`.
  Overkill for a status line, but a stripped-down version (append-only
  jsonl with last-N rows) is useful when debugging "why is my widget
  showing `[Rate limited]`".
- **Configurable pause thresholds**: 85 % five-hour, 90 % seven-day
  with reason strings (`"5-hour utilization 87.4% >= 85% threshold"`).
  Translates directly to colour-changing thresholds in a status line.

## Failure modes we should plan for

- Bearer token absent (API-key auth, fresh install) — drop the quota
  segment, render the rest of the line cleanly.
- `rate_limits` field absent on stdin (first turn, non-Pro/Max) — fall
  back to the OAuth endpoint, or skip if Option A.
- `security` CLI prompts the user (rare on macOS for `find-generic-password`
  on already-trusted entries; can happen after OS upgrades).
- Anthropic changes the endpoint shape or path — undocumented,
  no SLA. Fail closed: emit a `[quota n/a]` segment, never crash the
  whole line.
- Multiple Claude Code sessions racing for the cache — atomic writes
  via tmp-file rename, or accept a 180 s stale-data window and skip
  locking.

## What is **not** available

- No documented public REST endpoint for "remaining tokens this day/week".
  The `/usage` slash command and the Console usage dashboard are the only
  blessed surfaces.
- The popular `ccusage` npm package tracks **historical consumption**
  parsed from local transcript JSONL — not remaining vs. limit. Useful
  for cost telling, useless for quota.
- The Anthropic Files API and Messages API don't expose quota in
  response headers (verified: only `request-id`, no `x-ratelimit-*`
  on the OAuth-authenticated path).
