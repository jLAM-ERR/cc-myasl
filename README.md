# cc-myasl

[![CI](https://github.com/jLAM-ERR/cc-myasl/actions/workflows/ci.yml/badge.svg)](https://github.com/jLAM-ERR/cc-myasl/actions/workflows/ci.yml)
[![Release](https://img.shields.io/github/v/release/jLAM-ERR/cc-myasl)](https://github.com/jLAM-ERR/cc-myasl/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)
[![MSRV](https://img.shields.io/badge/rustc-1.85+-blue.svg)](https://www.rust-lang.org/)

Claude.ai 5h/7d quota in your Claude Code statusline. Single static
binary, ~3–5 ms cold start, no runtime deps, no npm. Includes an
interactive **`--configure` TUI** for visual config editing.

```
claude-opus-4-7 · 5h: 76% · 7d: 59% (resets 18:00)
```

## Table of contents

- [Install](#install)
- [Quickstart](#quickstart)
- [Interactive editor (`--configure`)](#interactive-editor---configure)
- [CLI flags](#cli-flags)
- [Configuration](#configuration)
  - [Resolution order](#resolution-order)
  - [Built-in templates](#built-in-templates)
  - [Demo configs](#demo-configs)
  - [Placeholder reference](#placeholder-reference)
- [Environment variables](#environment-variables)
- [Diagnostics (`--check`, `--debug`)](#diagnostics---check---debug)
- [Troubleshooting](#troubleshooting)
- [Security model](#security-model)
- [Uninstall / upgrade](#uninstall--upgrade)
- [FAQ](#faq)
- [Contributing](#contributing)
- [License](#license)

## Install

```sh
curl -fsSL https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/scripts/install.sh | sh
```

Pin a version: `VERSION=v1.0.0 curl … | sh`. The installer detects
your platform, verifies the SHA-256 sidecar, drops the binary at
`~/.claude/bin/cc-myasl`, copies templates to
`~/.config/cc-myasl/templates/`, and prints the `settings.json`
snippet (it never edits the file).

Manual tarball install for the four shipped targets
(`aarch64-apple-darwin`, `x86_64-apple-darwin`,
`aarch64-unknown-linux-musl`, `x86_64-unknown-linux-musl`):
download from [Releases](https://github.com/jLAM-ERR/cc-myasl/releases),
verify the `.sha256` sidecar, extract, copy `bin/cc-myasl` to
`~/.claude/bin/`.

## Quickstart

Add to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/Users/<you>/.claude/bin/cc-myasl --template default"
  }
}
```

Replace `/Users/<you>` with your home directory.

Sanity-check:

```sh
echo '{}' | ~/.claude/bin/cc-myasl --template default
~/.claude/bin/cc-myasl --check
```

`--check` verifies credentials, network, cache, and config.
**Render mode always exits 0** — failures collapse to a partial line.

## Interactive editor (`--configure`)

```sh
cc-myasl --configure
```

Opens a TUI for editing your config visually. Four panes:

- **Lines** (top-left) — add, remove, reorder lines (max 3).
- **Segments** (top-right) — add, remove, reorder segments on the
  selected line. Pick from 69 placeholders via filter-as-you-type.
- **Editor** (middle) — edit the current segment's template,
  padding, `hide_when_absent` flag, and per-segment fg/bg colors.
  Toggle Powerline mode globally.
- **Preview** (bottom) — live-rendered output against a built-in
  fixture, updates on every keystroke.

Press `?` for the keybinding overlay. `Ctrl+S` saves to
`~/.config/cc-myasl/config.json` with a `.bak` backup. The save
flow validates first — invalid configs are refused with the error
shown in the status bar.

`--configure` requires an interactive terminal (TTY on both stdin
and stdout). Pipes exit with code 1 and a clear stderr message.

To save somewhere other than the default path:

```sh
cc-myasl --configure --output ~/dotfiles/cc-myasl.json
```

## CLI flags

```
cc-myasl [OPTIONS]

  --template <NAME>     Use a built-in or user template by name.
                        User dir (~/.config/cc-myasl/templates/<NAME>.json)
                        shadows the 8 built-ins.
  --config <PATH>       Load a JSON config from PATH. Wins over --template.
  --output <PATH>       Output path for --configure save (defaults to
                        ~/.config/cc-myasl/config.json).
  --configure           Open the interactive TUI editor.
  --print-config        Emit the resolved config as pretty JSON to stdout.
  --check               Run diagnostics: creds, network, cache, config.
                        Only path that exits non-zero in render mode.
  --debug               Emit a trace to stderr: which credentials path,
                        which precedence layer won, HTTP status, etc.
  --version, -V         Print version and exit.
  --help, -h            Print help and exit.
```

## Configuration

Configs are JSON files with a list of lines, each holding a list of
segments. Each segment is either a template (`{ "template": "..." }`)
or a flex spacer (`{ "flex": true }`).

### Resolution order

1. `--config <path>` — explicit file
2. `--template <name>` — user dir then built-in
3. `STATUSLINE_CONFIG` env var
4. `~/.config/cc-myasl/config.json` (default file)
5. Embedded `default` built-in

User templates dir (`~/.config/cc-myasl/templates/<name>.json`)
shadows built-ins. Override `XDG_CONFIG_HOME` if needed.

Reference the schema for IDE auto-completion:

```json
{
  "$schema": "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json"
}
```

### Built-in templates

| Name | Sample output |
|---|---|
| `default` | `claude-opus-4-7 · 5h: 76% · 7d: 59% (resets 18:00)` |
| `minimal` | `claude-opus-4-7 76%/59%` |
| `compact` | `claude-opus-4-7 76/59` |
| `bars` | `claude-opus-4-7 5h:███████░░░ 7d:█████░░░░░` |
| `colored` | `claude-opus-4-7 · 5h: 76% · 7d: 59%` (ANSI green/yellow/red) |
| `emoji` | `claude-opus-4-7 · 🟢 5h 76% · 🟢 7d 59%` |
| `emoji_verbose` | `🤖 claude-opus-4-7 · 🟢 myproject · ⏳ 76%/59% · ⏰ 18:00` |
| `verbose` | `claude-opus-4-7 · myproject · 5h:███████░░░ 76% (in 1h24m) · …` |
| `rich` | 3-line: model + vim + context bar / git branch + cwd / cost + clock + tokens |

### Demo configs

Three copy-pasteable starting points. Run
`cc-myasl --print-config` to see the full resolved shape any time.

**1. Minimal — model + cwd + 5h quota**

```json
{
  "$schema": "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json",
  "lines": [{
    "segments": [
      { "template": "{model}" },
      { "template": " · {cwd_basename}", "hide_when_absent": true },
      { "template": " · 5h:{five_left}%", "hide_when_absent": true }
    ]
  }]
}
```

**2. Git developer — branch, status, cost, tokens**

```json
{
  "$schema": "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json",
  "lines": [
    {
      "segments": [
        { "template": "{model}" },
        { "template": " · ⎇ {git_branch}", "hide_when_absent": true },
        { "template": " ({git_staged}+{git_unstaged}+{git_untracked})", "hide_when_absent": true }
      ]
    },
    {
      "segments": [
        { "template": "{cwd_basename}" },
        { "flex": true },
        { "template": "${cost_usd}", "hide_when_absent": true },
        { "template": " · {tokens_total}tok", "hide_when_absent": true },
        { "template": " · 5h:{five_left}%", "hide_when_absent": true }
      ]
    }
  ]
}
```

**3. Powerline / Nerd Font — colored blocks with chevrons**

```json
{
  "$schema": "https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/cc-myasl.schema.json",
  "powerline": true,
  "lines": [{
    "segments": [
      { "template": " {model}",        "color": "white", "bg": "blue" },
      { "template": "  {git_branch}", "color": "black", "bg": "green",  "hide_when_absent": true },
      { "template": "  {five_left}%", "color": "black", "bg": "yellow", "hide_when_absent": true }
    ]
  }]
}
```

Powerline mode requires a [Nerd Font](https://www.nerdfonts.com/)
in your terminal for the chevron glyph (U+E0B0). Color values:
`red`, `green`, `yellow`, `blue`, `magenta`, `cyan`, `white`,
`default`, or `null` (omitted = no color). 256-color and hex are
not supported.

Save these to `~/.config/cc-myasl/templates/<name>.json` and
activate with `--template <name>`.

### Placeholder reference

cc-myasl ships ~69 placeholders. Run `cc-myasl --check` for the
full catalog, or browse
[`src/format/catalog.rs`](src/format/catalog.rs).

Categories:

- **Session**: `{model}`, `{model_id}`, `{version}`, `{session_id}`,
  `{session_name}`, `{output_style}`, `{vim_mode}`, `{effort}`,
  `{thinking_enabled}`, `{agent_name}`
- **Cost / clock**: `{cost_usd}`, `{session_clock}`, `{api_duration}`,
  `{lines_added}`, `{lines_removed}`, `{lines_changed}`
- **Tokens**: `{tokens_input}`, `{tokens_output}`,
  `{tokens_cached_creation}`, `{tokens_cached_read}`,
  `{tokens_cached_total}`, `{tokens_total}`,
  `{tokens_input_total}`, `{tokens_output_total}`
- **Context**: `{context_size}`, `{context_used_pct}`,
  `{context_remaining_pct}`, `{context_used_pct_int}`,
  `{context_bar}`, `{context_bar_long}`, `{exceeds_200k}`
- **Workspace / worktree**: `{cwd}`, `{cwd_basename}`,
  `{project_dir}`, `{added_dirs_count}`,
  `{workspace_git_worktree}`, `{worktree_name}`,
  `{worktree_path}`, `{worktree_branch}`,
  `{worktree_original_cwd}`, `{worktree_original_branch}`
- **Git**: `{git_branch}`, `{git_root}`, `{git_changes}`,
  `{git_staged}`, `{git_unstaged}`, `{git_untracked}`,
  `{git_status_clean}`
- **Quota** (5h / 7d / extra): `{five_used}`, `{five_left}`,
  `{five_bar}`, `{five_bar_long}`, `{five_reset_clock}`,
  `{five_reset_in}`, `{five_color}`, `{five_state}` and
  `{seven_*}` / `{extra_*}` analogues
- **Combined / formatting**: `{state_icon}`, `{reset}`

Any placeholder that resolves to `None` makes the segment empty.
Pair with `"hide_when_absent": true` to collapse the segment AND
its trailing separator when the data isn't available.

## Environment variables

| Variable | Purpose |
|---|---|
| `STATUSLINE_CONFIG` | Path to a JSON config (overrides default file). |
| `STATUSLINE_DEBUG` | Set to `1` to enable `--debug` traces from settings. |
| `STATUSLINE_RED` | Threshold in % below which `{five_color}` and friends turn red. Default 20. |
| `STATUSLINE_YELLOW` | Yellow threshold (%). Default 50. |
| `XDG_CONFIG_HOME` | Override the config dir (default `~/.config`). |
| `XDG_CACHE_HOME` | Override the cache dir (default `~/.cache`). |
| `STATUSLINE_OAUTH_BASE_URL` | Override the OAuth endpoint (testing only). |

## Diagnostics (`--check`, `--debug`)

`--check` walks the resolution chain and prints status of:

1. Credentials (Keychain on macOS, `~/.claude/.credentials.json`
   fallback).
2. Network reachability to `api.anthropic.com`.
3. On-disk cache state (`~/.cache/cc-myasl/usage.json`).
4. Active config source and validation result.

Exit code: 0 if every section passes; 1 if any failed. This is the
**only** path (alongside `--configure`) allowed to exit non-zero —
render mode always exits 0.

`--debug` adds a structured trace to stderr: which template path
resolved, which credentials store was used, the OAuth fetch outcome
(if any), the cache decision, and validation warnings.

## Troubleshooting

**The status line is empty.**
First call after install: cc-myasl needs `rate_limits` in stdin
(Pro/Max only) or a working OAuth fallback. Run `--check`.

**Quota numbers are stale.**
Force a refresh:

```sh
rm ~/.cache/cc-myasl/usage.json   # macOS / Linux
```

**`{git_*}` segments are blank.**
cc-myasl uses `gix` to discover the repo. Outside a repo (or in a
bare repo), git placeholders return `None` and `hide_when_absent`
collapses them. Confirm with `--debug`.

**Powerline chevrons render as `?` or boxes.**
Install a [Nerd Font](https://www.nerdfonts.com/) (FiraCode NF,
Hack NF, JetBrains Mono NF) and set your terminal to use it.

**OAuth returns 401.**
The cached token may have rotated. Delete
`~/.claude/.credentials.json` and re-authenticate via Claude Code.

## Security model

- **Bearer tokens never written to disk by cc-myasl.** The
  `UsageCache` schema deliberately omits any token field; verified
  by a serialization-substring test.
- The Keychain entry `Claude Code-credentials` (macOS) or
  `~/.claude/.credentials.json` (Linux/fallback) is read once per
  cache miss; the token is held in memory just long enough to make
  the HTTP request, then dropped.
- No `security dump-keychain` or external scripts are invoked.
  CI-gated by a `scripts/check-invariants.sh` grep.
- The installer never edits `~/.claude/settings.json` — it prints
  the snippet for you to merge.
- HTTPS via `rustls`; respects `HTTPS_PROXY` / `HTTP_PROXY` /
  `NO_PROXY` env vars.
- Linux Secret Service / `gnome-keyring` is intentionally NOT used
  to keep the binary statically linkable against musl.

## Uninstall / upgrade

**Uninstall:**

```sh
rm ~/.claude/bin/cc-myasl
rm -rf ~/.config/cc-myasl ~/.cache/cc-myasl
# then remove the statusLine block from ~/.claude/settings.json
```

**Upgrade:** re-run the install one-liner with a new `VERSION=`:

```sh
VERSION=v1.0.1 curl -fsSL https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/scripts/install.sh | sh
```

## FAQ

**Does this work without a Claude.ai Pro/Max subscription?**
Partially. The 5h/7d quota numbers come from `rate_limits` in
Claude Code's stdin, which is only populated for Pro/Max sessions.
The OAuth fallback works for any logged-in account but does not
return rate-limit data outside Pro/Max. Other placeholders (model,
session, git, cost, tokens, context) work unconditionally.

**Why is the line blank for the first 30 seconds of a session?**
On a fresh session, `rate_limits` is absent and the OAuth fallback
fires. cc-myasl caches the result; subsequent renders hit the cache.
The first call's render is empty for quota segments — pair quota
placeholders with `hide_when_absent: true` to collapse them
gracefully.

**Can I use this with `tmux` or a different shell prompt?**
cc-myasl is a Claude Code statusline command — Claude Code pipes
JSON to it on every assistant message. It's not a shell prompt.
For shell prompts, use [Starship](https://starship.rs/).

**Is the binary safe to run in CI / non-interactive contexts?**
Yes for render mode (piped stdin → output). The TUI mode
(`--configure`) requires a real terminal and exits 1 with a clear
message when stdin or stdout isn't a TTY.

**MSRV / how to build from source?**
Rust 1.85 (edition 2024). `cargo build --release` produces a
~1.8 MB stripped binary. See [CONTRIBUTING.md](CONTRIBUTING.md).

## Contributing

Plans live in `docs/plans/`. Open an issue or PR against `main`.
Conventions in [CLAUDE.md](CLAUDE.md). All changes go through:
`cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`,
`cargo test`, `bash scripts/check-loc.sh`,
`bash scripts/check-invariants.sh`, `shellcheck scripts/*.sh`.

## License

MIT. See [LICENSE](LICENSE).

## Acknowledgments

cc-myasl is a Rust replacement for the npm `ccstatusline` —
written to avoid the supply-chain risk of `npx -y …@latest`
auto-updates and to ship a single static binary instead of a
Node runtime. Inspired by ccstatusline's feature surface and
Starship's gix-based repo discovery.
