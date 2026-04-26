# statusline

Working repo for a Claude Code status-line project that displays remaining
5-hour and 7-day token quota.

This repo was started from research conducted on 2026-04-26 against two
reference projects (`sirmalloc/ccstatusline`, `AlexDobrushskiy/botfarm`)
and the Claude Code `statusLine` mechanism. Nothing is implemented yet —
this is the planning + research vault.

## Layout

- `docs/research.md` — what `statusLine` exposes, where the OAuth `usage`
  endpoint lives, how ccstatusline and botfarm consume it, plus the
  patterns worth borrowing.
- `docs/security-review.md` — security audit of `sirmalloc/ccstatusline`
  (open issues, closed-but-historical, code-level concerns) and how each
  finding maps onto our build-vs-install decision.
- `docs/plan.md` — three implementation options (stdin-only,
  stdin + OAuth fallback, install ccstatusline) with phased steps for the
  recommended option B.
- `docs/session-2026-04-26.md` — pointer to the pinned Claude Code
  session that produced this research, plus a one-paragraph recap.

## Status

- Decision pending: A vs B vs C (see `docs/plan.md`).
- No script written yet.
- No `settings.json` change applied yet.

## Reference repos

- https://github.com/sirmalloc/ccstatusline — TypeScript / Bun status-line
  with Weekly Usage, Block Timer, etc. Bundled into a single `dist`
  artefact.
- https://github.com/AlexDobrushskiy/botfarm — Python supervisor that
  polls the same OAuth `usage` endpoint, with adaptive backoff, audit
  log, and token-rotation detection.
