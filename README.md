# cc-myasl — My Yet Another Status Line for Claude Code

Remaining Claude.ai 5h/7d quota in your Claude Code status line — single
binary, ~3–5 ms cold start, no runtime deps.

> The repo is `cc-myasl`. The shipped binary is `claude-statusline`
> (kept descriptive for what it actually does and to avoid colliding with
> existing Claude Code conventions). All install commands and `settings.json`
> snippets below reference the binary name.

## Demo

```
claude-opus-4-7 · 5h: 76% · 7d: 59% (resets 18:00)
```

Real screenshots can land once the binary is installed on a Pro/Max account.
The format above is what the built-in `default` template produces with a
typical Pro/Max session.

---

## Install

### One-liner (recommended)

```sh
curl -fsSL https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/scripts/install.sh | sh
```

Override defaults with environment variables:

```sh
# Pin a specific release
VERSION=v0.1.0 curl -fsSL https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/scripts/install.sh | sh

# Override owner/repo if you've forked
OWNER=jLAM-ERR REPO=cc-myasl VERSION=v0.1.0 \
  curl -fsSL https://raw.githubusercontent.com/jLAM-ERR/cc-myasl/main/scripts/install.sh | sh
```

The installer:

1. Detects your platform (`uname -sm`).
2. Downloads the matching tarball and `.sha256` from GitHub Releases.
3. Verifies the SHA-256 checksum — aborts on mismatch. (If the
   `.sha256` sidecar is missing from the release, the installer warns
   and continues without verification.)
4. Installs the binary to `~/.claude/bin/claude-statusline`.
5. Copies templates to `~/.config/claude-statusline/templates/`.
6. Prints the `settings.json` snippet (never edits the file itself).

### Manual tarball install

1. Go to the [GitHub Releases](https://github.com/jLAM-ERR/cc-myasl/releases) page.
2. Download the tarball for your platform:

   | Platform | Target triple |
   |---|---|
   | macOS Apple Silicon | `aarch64-apple-darwin` |
   | macOS Intel | `x86_64-apple-darwin` |
   | Linux x86_64 (musl) | `x86_64-unknown-linux-musl` |
   | Linux aarch64 (musl) | `aarch64-unknown-linux-musl` |

3. Verify and install:

   ```sh
   # Example for macOS Apple Silicon, v0.1.0
   VERSION=v0.1.0
   TARGET=aarch64-apple-darwin
   TARBALL="claude-statusline-${VERSION#v}-${TARGET}.tar.gz"

   shasum -a 256 -c "${TARBALL}.sha256"
   tar -xzf "$TARBALL"
   mkdir -p ~/.claude/bin
   install -m 0755 "claude-statusline-${VERSION#v}-${TARGET}/bin/claude-statusline" \
     ~/.claude/bin/claude-statusline
   ```

---

## Configure Claude Code

Add this block to `~/.claude/settings.json`:

```json
{
  "statusLine": {
    "type": "command",
    "command": "/Users/<you>/.claude/bin/claude-statusline --template default"
  }
}
```

Replace `/Users/<you>` with your actual home directory (e.g.
`/Users/alice` on macOS or `/home/alice` on Linux). You can find the
path with `echo $HOME`.

After saving, the status line appears at the bottom of every Claude Code
session that has a quota window (Claude.ai Pro or Max subscribers). On
API-key auth, the tool falls back to the OAuth endpoint using your
stored credentials.

---

## Template Gallery

Select a built-in template with `--template <NAME>`, supply an inline
string with `--format "<STRING>"`, or set `STATUSLINE_FORMAT`. Precedence:
`--format` > `STATUSLINE_FORMAT` > `--template` > built-in `default`.

### default

```
{model}{? · 5h: {five_left}%}{? · 7d: {seven_left}%}{? (resets {seven_reset_clock})}
```

Sample output:

```
claude-opus-4-7 · 5h: 76% · 7d: 59% (resets 18:00)
```

### minimal

```
{model}{? {five_left}%/{seven_left}%}
```

Sample output:

```
claude-opus-4-7 76%/59%
```

### compact

```
{model}{? {five_left}/{seven_left}}
```

Sample output:

```
claude-opus-4-7 76/59
```

### bars

```
{model}{? 5h:{five_bar}}{? 7d:{seven_bar}}
```

Sample output:

```
claude-opus-4-7 5h:███████░░░ 7d:█████░░░░░
```

### colored

```
{model}{? · 5h: {five_color}{five_left}%{reset}}{? · 7d: {seven_color}{seven_left}%{reset}}
```

Sample output (colour escapes rendered by your terminal):

```
claude-opus-4-7 · 5h: [green]76%[/] · 7d: [green]59%[/]
```

### emoji

```
{model}{? · {five_state} 5h {five_left}%}{? · {seven_state} 7d {seven_left}%}
```

Sample output:

```
claude-opus-4-7 · 🟢 5h 76% · 🟢 7d 59%
```

### emoji_verbose

```
🤖 {model}{? · {state_icon} {cwd_basename}}{? · ⏳ {five_left}%/{seven_left}%}{? · ⏰ {seven_reset_clock}}
```

Sample output:

```
🤖 claude-opus-4-7 · 🟢 myproject · ⏳ 76%/59% · ⏰ 18:00
```

### verbose

```
{model} · {cwd_basename}{? · 5h:{five_bar} {five_left}% (in {five_reset_in})}{? · 7d:{seven_bar} {seven_left}% (in {seven_reset_in})}{? · extra:{extra_left}}
```

Sample output:

```
claude-opus-4-7 · myproject · 5h:███████░░░ 76% (in 1h24m) · 7d:█████░░░░░ 59% (in 3d02h)
```

---

## Placeholder Reference

All placeholders collapse the surrounding `{? … }` optional block when
the underlying data is absent. Placeholders outside optional blocks emit
an empty string when data is missing.

| Placeholder | Renders | Falsy (collapses optional) when |
|---|---|---|
| `{model}` | Model display name | `display_name` absent in stdin |
| `{cwd}` | Full working directory (`~`-abbreviated) | `workspace.current_dir` absent |
| `{cwd_basename}` | Last path component of cwd | `workspace.current_dir` absent |
| `{five_used}` | 5-hour usage % (decimal, e.g. `23.5`) | `rate_limits.five_hour` absent |
| `{five_left}` | 5-hour remaining % (integer, e.g. `76`) | `rate_limits.five_hour` absent |
| `{five_bar}` | 10-char block-fill bar of remaining | `rate_limits.five_hour` absent |
| `{five_bar_long}` | 20-char block-fill bar of remaining | `rate_limits.five_hour` absent |
| `{five_reset_clock}` | Local HH:MM when 5-hour window resets | `resets_at` absent |
| `{five_reset_in}` | Countdown to 5-hour reset (e.g. `1h24m`) | `resets_at` absent |
| `{five_color}` | ANSI colour escape for 5-hour state | never (always emits) |
| `{five_state}` | Emoji icon for 5-hour state (🟢 🟡 🔴) | never (always emits) |
| `{seven_used}` | 7-day usage % (decimal, e.g. `41.2`) | `rate_limits.seven_day` absent |
| `{seven_left}` | 7-day remaining % (integer, e.g. `58`) | `rate_limits.seven_day` absent |
| `{seven_bar}` | 10-char block-fill bar of remaining | `rate_limits.seven_day` absent |
| `{seven_bar_long}` | 20-char block-fill bar of remaining | `rate_limits.seven_day` absent |
| `{seven_reset_clock}` | Local HH:MM when 7-day window resets | `resets_at` absent |
| `{seven_reset_in}` | Countdown to 7-day reset (e.g. `2d14h`) | `resets_at` absent |
| `{seven_color}` | ANSI colour escape for 7-day state | never (always emits) |
| `{seven_state}` | Emoji icon for 7-day state (🟢 🟡 🔴) | never (always emits) |
| `{extra_left}` | Extra-usage credits remaining | `extra_usage.is_enabled` not true |
| `{extra_used}` | Extra-usage credits used | `extra_usage.is_enabled` not true |
| `{extra_pct}` | Extra-usage utilisation % (decimal) | `extra_usage.is_enabled` not true |
| `{state_icon}` | Emoji icon for the worse of 5h/7d state | both windows absent |
| `{reset}` | ANSI reset escape (`\x1b[0m`) | never (always emits) |

### Optional segments

Wrap any segment in `{? … }` to suppress the whole block — including its
surrounding literal text — when any placeholder inside it resolves to
empty:

```
{model}{? · 5h: {five_left}%}
```

If `five_left` is unavailable (no quota data yet), the output is just
`claude-opus-4-7` with no trailing ` · 5h:` fragment.

---

## Threshold Environment Variables

| Variable | Default | Effect |
|---|---|---|
| `STATUSLINE_RED` | `20` | Remaining % at or below which state is Red |
| `STATUSLINE_YELLOW` | `50` | Remaining % at or below which state is Yellow |
| `STATUSLINE_FORMAT` | _(unset)_ | Inline format string; overrides `--template` |
| `STATUSLINE_DEBUG` | _(unset)_ | Set to `1` to emit a JSON trace to stderr on every render |
| `STATUSLINE_OAUTH_BASE_URL` | `https://api.anthropic.com` | Override the OAuth endpoint base URL (useful for testing) |

Remaining % above `STATUSLINE_YELLOW` is Green. Colour escapes are
emitted by `{five_color}` / `{seven_color}` / `{reset}` and rendered by
your terminal emulator.

---

## `--check` and `--debug`

### `--check` — four-section diagnostic

`claude-statusline --check` runs a diagnostic of all subsystems and exits
non-zero on any failure. Use it first when the status line looks wrong.

```
$ claude-statusline --check

Credentials
  ✓ macOS Keychain: found service "Claude Code-credentials"
    fingerprint: a3f8c20d11e74b92

Network
  ✓ GET https://api.anthropic.com/api/oauth/usage → 200 (142 ms)

Cache
  ✓ ~/.cache/claude-statusline/usage.json — fresh (38 s old, TTL 180 s)

Format
  ✓ default template renders without error
```

If any section fails, the relevant `✗` line explains why:

```
Credentials
  ✗ macOS Keychain: service "Claude Code-credentials" not found
  ✗ ~/.claude/.credentials.json: file not found
```

Exit code is `0` when all sections pass, `1` if any fail.

### `--debug` — per-render JSON trace

`claude-statusline --debug` (or `STATUSLINE_DEBUG=1`) emits a single-line
JSON object to stderr after every render, containing timing, cache state,
and HTTP outcome. The bearer token is never included — only a
non-reversible fingerprint of its last 8 characters.

```json
{"path":"stdin","cache":"hit","http":null,"took_ms":2,"error":null}
{"path":"oauth","cache":"miss","http":"200","took_ms":148,"error":null}
{"path":"lock","cache":"stale","http":null,"took_ms":1,"error":"rate_limited"}
```

You can pipe stderr to a file for post-mortem analysis:

```sh
echo '{}' | claude-statusline --debug 2>trace.jsonl
```

---

## Nerd Font Tips

### What are Nerd Fonts?

[Nerd Fonts](https://www.nerdfonts.com/) are patched versions of popular
developer fonts that include extra glyphs from Powerline, Devicons,
Font Awesome, and other icon sets — all packed into the Private Use Area
of Unicode. They let you embed icons directly in terminal text without
image rendering.

### Why are they useful here?

The default templates use plain ASCII and Unicode emoji (🟢 🟡 🔴 ⏳ ⏰
🤖) which work in any terminal. If you want richer clock, alert, or
battery icons in your status line, Nerd Fonts let you write a custom
template with those glyphs.

### Recommended fonts

| Font | Install (Homebrew) |
|---|---|
| JetBrainsMono Nerd Font | `brew install --cask font-jetbrains-mono-nerd-font` |
| FiraCode Nerd Font | `brew install --cask font-fira-code-nerd-font` |
| Hack Nerd Font | `brew install --cask font-hack-nerd-font` |

On Linux, download from [github.com/ryanoasis/nerd-fonts/releases](https://github.com/ryanoasis/nerd-fonts/releases)
and copy to `~/.local/share/fonts/`, then run `fc-cache -fv`.

After installing, set your terminal emulator's font to the Nerd Font
variant (e.g. "JetBrainsMono Nerd Font" in iTerm2 or WezTerm).

### Test your font

Run this in the terminal you use for Claude Code:

```sh
echo "   "
```

If you see three distinct icons (clock, warning triangle, battery), your
font has the glyphs. If you see boxes or question marks, the font does
not include them.

### Example Nerd Font template

Once your terminal font supports the glyphs, you can write:

```
{model}   {five_left}%   {seven_left}%
```

And use it with:

```sh
claude-statusline --format "{model}   {five_left}%   {seven_left}%"
```

Or set it in your environment:

```sh
export STATUSLINE_FORMAT="{model}   {five_left}%   {seven_left}%"
```

**Note:** `claude-statusline` does not ship a Nerd Font preset — Nerd
Fonts are an opt-in install that you configure in your terminal emulator.
The built-in templates intentionally use only standard emoji and ASCII so
they work out of the box in any terminal.

---

## Troubleshooting

### Blank status line

1. Run `claude-statusline --check` to see which subsystem is failing.
2. If the check passes but the line is still blank, enable debug output:
   ```sh
   echo '{}' | STATUSLINE_DEBUG=1 claude-statusline 2>&1 | head -5
   ```
3. Inspect the cache file directly:
   - macOS: `cat ~/Library/Caches/claude-statusline/usage.json`
   - Linux: `cat ~/.cache/claude-statusline/usage.json`

### macOS quarantine warning ("unidentified developer")

After installing via the curl-pipe script, macOS Gatekeeper may block the
binary. Clear the quarantine attribute:

```sh
xattr -d com.apple.quarantine ~/.claude/bin/claude-statusline
```

Then re-run `claude-statusline --check` to confirm it works.

### "no credentials" error

`claude-statusline --check` reports this when neither the Keychain nor
the credentials file contains a valid Bearer token.

Diagnosis steps:

1. **macOS**: confirm the Keychain entry exists:
   ```sh
   security find-generic-password -s "Claude Code-credentials" -w | head -c 20
   ```
   If this fails, open Claude Code in the browser at least once so the
   OAuth flow can write the Keychain entry.

2. **All platforms**: check the credentials file:
   ```sh
   ls -la ~/.claude/.credentials.json
   ```
   If it is missing, sign in to Claude Code with `claude auth` or by
   completing an OAuth flow that writes `~/.claude/.credentials.json`.

### Linux: Secret Service / kwallet not used

`claude-statusline` deliberately does **not** integrate with
D-Bus Secret Service or kwallet on Linux. Those services are too fragile
in headless and container environments (SSH sessions, CI, Docker). On
Linux the tool falls back to `~/.claude/.credentials.json` only.

If the file does not exist on a Linux box, you can copy it from a
machine where you have already authenticated:

```sh
scp mydesktop:~/.claude/.credentials.json ~/.claude/.credentials.json
chmod 600 ~/.claude/.credentials.json
```

### Status line doesn't update after `/usage`

The cache TTL is 180 seconds. After running `/usage` inside Claude Code,
the status line continues to show cached data until the TTL expires or
the cache is invalidated.

To force a refresh, delete the cache file:

- macOS: `rm ~/Library/Caches/claude-statusline/usage.json`
- Linux: `rm ~/.cache/claude-statusline/usage.json`

Enable `STATUSLINE_DEBUG=1` to confirm whether each render is a cache
hit or a live fetch.

---

## Tips and Caveats

### The OAuth endpoint is undocumented

`GET https://api.anthropic.com/api/oauth/usage` is a private endpoint
used internally by Claude Code itself. Anthropic has not published a
stability guarantee for it. Both `sirmalloc/ccstatusline` and
`AlexDobrushskiy/botfarm` have already had to update once when the
endpoint shape changed.

Watch the [ccstatusline issue tracker](https://github.com/sirmalloc/ccstatusline/issues)
for early notice of endpoint changes.

### Multi-account caching

The cache is per-machine, not per-account. If you switch Claude.ai
accounts (e.g. from a personal Pro account to a work Max account), the
cached quota numbers still reflect the previous account until the cache
expires or you delete it manually:

- macOS: `rm ~/Library/Caches/claude-statusline/usage.json`
- Linux: `rm ~/.cache/claude-statusline/usage.json`

### Proxy / MITM caveat

If `HTTPS_PROXY` is set in your environment, requests to the OAuth
endpoint flow through that proxy. The proxy can see the OAuth Bearer
token in the `Authorization` header. This is standard HTTPS-via-proxy
behaviour — the proxy terminates TLS. Do not set `HTTPS_PROXY` to an
untrusted host.

### 5-hour and 7-day windows (not "day" and "week")

Claude Code's two quota windows are:

- **5-hour** (`five_*` placeholders): a rolling 5-hour window.
- **7-day** (`seven_*` placeholders): a rolling 7-day window.

These are often described as "hourly" and "weekly" in Claude Code UI, but
the actual durations are 5 hours and 7 days respectively. The
`{five_reset_clock}` and `{seven_reset_clock}` placeholders show the
local time when each window resets.

---

## Security Model

`claude-statusline` is built with a small, auditable trusted-code surface:

- **Bearer token never written to disk.** The `UsageCache` struct
  deliberately omits any token field — this is a compile-time guarantee
  verified by a golden test that serialises the cache and asserts the
  JSON contains none of `"token"`, `"bearer"`, `"secret"`, `"auth"`, or
  `"access"`.

- **Bearer token never logged.** `--debug` output contains only a
  non-reversible fingerprint (SipHash of the token's last 8 characters,
  rendered as 16 hex digits). The raw token never appears in stderr
  output.

- **No `security dump-keychain` enumeration.** We call only
  `security find-generic-password -s "Claude Code-credentials" -w` —
  a targeted lookup for the specific service. The broad enumeration call
  used by `ccstatusline` (which dumps metadata for every keychain item —
  Wi-Fi networks, banking, browsers, SSO — into process memory) is
  deliberately absent. See `docs/security-review.md` for details.

- **install.sh verifies SHA-256 checksums.** The installer downloads
  a `.sha256` file alongside each tarball and aborts on mismatch. The
  binary you install is the binary that was built in CI and signed with
  a known checksum.

- **No runtime npm fetch.** There is no `npx`, `bunx`, or any npm-based
  install path. The binary is a self-contained static executable. You
  install it once from a GitHub Release; it never phones home for
  updates. This directly addresses [ccstatusline issue #298](https://github.com/sirmalloc/ccstatusline/issues/298)
  (supply-chain risk via `npx -y ccstatusline` running whatever is tagged
  `latest` on npm at invocation time).

---

## Development

### Build

```sh
cargo build --release
```

The release profile uses `lto = "fat"`, `codegen-units = 1`,
`strip = true`, and `panic = "abort"` to produce a stripped binary of
≤ 1.5 MB.

### Test

```sh
cargo test
```

The test suite covers unit tests for every module, integration tests via
`tests/golden.rs` (spawn the binary, pipe fixture JSON, assert output),
and invariant assertions. The OAuth endpoint is mocked via `mockito` —
no real network calls in CI.

### LOC budget

Each file in `src/**/*.rs` is capped at 500 lines. CI enforces this with
`scripts/check-loc.sh`. If a file approaches 400 lines, split it before
adding more.

### Hard invariants (CI-gated)

`scripts/check-invariants.sh` asserts:

1. No `security dump-keychain` invocation in `src/` or `scripts/`.
2. No `npx.*latest` or `@latest` in `src/` or `scripts/`.
3. No `cargo`, `rustc`, or `rustup` references in `scripts/install.sh`.

Run manually with:

```sh
bash scripts/check-invariants.sh
bash scripts/check-loc.sh
```

---

## License

MIT. (The `license` field in `Cargo.toml` should be set to `"MIT"` in a
future patch — this is a known TODO for the project maintainer.)

---

## Acknowledgments

`claude-statusline` was built by studying two excellent prior projects:

- **[sirmalloc/ccstatusline](https://github.com/sirmalloc/ccstatusline)**
  — TypeScript / Bun implementation that pioneered the two-tier cache,
  lock file, and stale-fallback patterns used here.
- **[AlexDobrushskiy/botfarm](https://github.com/AlexDobrushskiy/botfarm)**
  — Python supervisor with adaptive backoff, token fingerprinting, and
  audit-log patterns that informed this project's debug tracing.

The Rust rewrite was motivated by supply-chain concerns documented in
`docs/security-review.md` and the need for a cross-platform static
binary with sub-10 ms cold start.
