# Template Gallery

`claude-statusline` ships eight built-in templates. Select one with
`--template <NAME>`, override with `--format "<INLINE>"`, or set
`STATUSLINE_FORMAT` in your environment. Precedence (highest first):
`--format` > `STATUSLINE_FORMAT` > `--template` > built-in `default`.

## Templates

**default** — The recommended template. Shows model name, then optional
5-hour and 7-day percentage remaining, and a reset clock for the 7-day
window.
Sample: `claude-opus-4 · 5h: 72% · 7d: 41% (resets 18:30)`

**minimal** — Both quota percentages on one short segment, nothing else.
Sample: `claude-opus-4 72%/41%`

**compact** — Same as minimal but integer-only (no `%` suffix), useful
for very narrow status bars.
Sample: `claude-opus-4 72/41`

**bars** — Replaces percentages with 10-character block-fill progress
bars for a quick visual scan.
Sample: `claude-opus-4 5h:███████░░░ 7d:████░░░░░░`

**colored** — Like `default` but wraps each percentage in an ANSI colour
escape (green/yellow/red by threshold) and resets afterwards.
Sample: `claude-opus-4 · 5h: [green]72%[reset] · 7d: [red]12%[reset]`

**emoji** — Uses state-icon emojis (🟢 🟡 🔴) in front of each quota
segment, otherwise same layout as `default`.
Sample: `claude-opus-4 · 🟢 5h 72% · 🟡 7d 41%`

**emoji_verbose** — Full robot-prefixed line: model, overall state icon,
current project directory, both quota percentages, and the 7-day reset
clock.
Sample: `🤖 claude-opus-4 · 🟢 myproject · ⏳ 72%/41% · ⏰ 18:30`

**verbose** — Prose-style with bar and countdown for each window, plus
optional extra-usage segment.
Sample: `claude-opus-4 · myproject · 5h:███████░░░ 72% (in 1h12m) · 7d:████░░░░░░ 41% (in 2d14h)`

## Placeholder Reference

| Placeholder | Renders | Falsy (collapses optional) when |
|---|---|---|
| `{model}` | Model display name | `display_name` absent in stdin |
| `{cwd}` | Full working directory (`~`-abbreviated) | `workspace.current_dir` absent |
| `{cwd_basename}` | Last path component of cwd | `workspace.current_dir` absent |
| `{five_used}` | 5-hour usage % (decimal) | `rate_limits.five_hour` absent |
| `{five_left}` | 5-hour remaining % (integer) | `rate_limits.five_hour` absent |
| `{five_bar}` | 10-char block-fill bar of remaining | `rate_limits.five_hour` absent |
| `{five_bar_long}` | 20-char block-fill bar of remaining | `rate_limits.five_hour` absent |
| `{five_reset_clock}` | Local HH:MM when 5-hour window resets | `resets_at` absent |
| `{five_reset_in}` | Countdown to 5-hour reset (`1h12m`) | `resets_at` absent |
| `{five_color}` | ANSI colour escape for 5-hour state | never (always emits) |
| `{five_state}` | Emoji icon for 5-hour state | never (always emits) |
| `{seven_used}` | 7-day usage % (decimal) | `rate_limits.seven_day` absent |
| `{seven_left}` | 7-day remaining % (integer) | `rate_limits.seven_day` absent |
| `{seven_bar}` | 10-char block-fill bar of remaining | `rate_limits.seven_day` absent |
| `{seven_bar_long}` | 20-char block-fill bar of remaining | `rate_limits.seven_day` absent |
| `{seven_reset_clock}` | Local HH:MM when 7-day window resets | `resets_at` absent |
| `{seven_reset_in}` | Countdown to 7-day reset (`2d14h`) | `resets_at` absent |
| `{seven_color}` | ANSI colour escape for 7-day state | never (always emits) |
| `{seven_state}` | Emoji icon for 7-day state | never (always emits) |
| `{extra_left}` | Extra-usage credits remaining | `extra_usage.is_enabled` not true |
| `{extra_used}` | Extra-usage credits used | `extra_usage.is_enabled` not true |
| `{extra_pct}` | Extra-usage utilisation % (decimal) | `extra_usage.is_enabled` not true |
| `{state_icon}` | Emoji icon for the worse of 5h/7d state | both windows absent |
| `{reset}` | ANSI reset escape (`\x1b[0m`) | never (always emits) |

Optional segments `{? … }` suppress the entire block (including
surrounding literal text) when any placeholder inside them resolves to
empty.

## Threshold Environment Variables

| Variable | Default | Effect |
|---|---|---|
| `STATUSLINE_RED` | `20` | Remaining % at or below which state is Red |
| `STATUSLINE_YELLOW` | `50` | Remaining % at or below which state is Yellow |

Remaining % above `STATUSLINE_YELLOW` is Green.

## How to Use

```
# Named built-in template
claude-statusline --template minimal

# Inline format string (highest precedence)
claude-statusline --format "{model} · {five_left}%"

# Via environment variable
export STATUSLINE_FORMAT="{model} · {five_left}%"
claude-statusline
```

For Nerd Font glyphs and troubleshooting tips, see the root `README.md`
(added in Task 17).
