# Security review — `sirmalloc/ccstatusline`

Audited 2026-04-26 against `main` at version 2.2.8. Reviewed: open and
closed issues with security-relevant keywords, the `usage-fetch.ts`
credential reader, the `CustomCommand` widget, the `claude-settings`
loader, and `package.json`. Severity tags are mine, not the project's.

## Open / live risks

### [HIGH · OPEN] #298 — Supply-chain attack via `npx -y ccstatusline@latest`

The README's recommended install is `npx -y ccstatusline@latest`. Claude
Code runs the status-line command **after every assistant message**, so
a stale npx cache silently downloads and executes whatever is tagged
`latest` on npm at that moment. `-y` suppresses the only prompt.

If the maintainer's npm account is hijacked, or `NPM_TOKEN` leaks (CI
log, public commit, third-party breach), every user picks up the
malicious release on the next cache refresh — running with full user
permissions (filesystem, env, SSH keys, OAuth tokens). Real precedents:
`ua-parser-js`, `event-stream`, `node-ipc`.

**Mitigation path for users**: pin (`ccstatusline@2.2.8`) or, better,
local install:
```bash
npm install --prefix ~/.claude/statusline-packages \
            --save-exact ccstatusline@2.2.8
```
…and point `settings.json` at the local binary. Eliminates runtime
network access entirely.

### [MEDIUM · BY DESIGN] Keychain enumeration via `security dump-keychain`

`src/utils/usage-fetch.ts` calls `execFileSync('security',
['dump-keychain'], { maxBuffer: 8 MiB })` as a fallback when the
primary `Claude Code-credentials` lookup misses. That dumps **metadata
for every keychain item** (Wi-Fi networks, banking, browsers, work
SSO, internal apps) into the Node process memory on each cache miss.

Only `svce` service-name fields are parsed, and the dump is not written
to disk. Still, a status-line tool that reads your whole keychain index
every ~3 minutes is more invasive than most users would expect.

**Mitigation for our project**: never call `dump-keychain`. Look up
exactly the `Claude Code-credentials` service via `find-generic-password`
and silently drop the quota segment if that misses.

### [MEDIUM · UX FOOTGUN] `CustomCommand` inherits full `process.env`

`src/widgets/CustomCommand.tsx` runs the user-configured command with
`execSync(cmd, { env: process.env, input: jsonInput })`. Two issues:

1. The command string is interpreted by `/bin/sh -c`. Backticks and
   `$(...)` in copy-pasted recipes expand at every render.
2. `process.env` includes every `*_API_KEY`, `*_TOKEN`, etc. in the
   current shell. A widget recipe pasted from a blog has access to all
   of them.

Not exploitable by a remote attacker — purely a foot-gun. Worth flagging
because the README does encourage pasting one-liners.

## Closed but historically relevant

### [HIGH · CLOSED Feb 2026] #156 — Directory traversal in `transcript_path`

`getSessionDuration` and `getTokenMetrics` accepted any path from the
stdin JSON without validation. PoC:

```bash
echo '{"transcript_path":"../../../.ssh/id_rsa"}' | bunx ccstatusline@latest
```

…would read the file. Patched by resolving against the Claude config
dir and checking `realpath` to defeat symlink swaps. Anything
≥ 2.2.x should be safe; verify your installed version is post-fix.

### [MEDIUM · CLOSED] #156 — `CLAUDE_CONFIG_DIR` path injection

Env var was passed straight through `path.resolve` and used to
read/write settings. Allowed redirecting writes to `/etc`, other users'
homes, etc. Patched by restricting to paths within `os.homedir()`
and rejecting `..` patterns.

### [LOW-MEDIUM · CLOSED] #156 — TOCTOU between `existsSync` and `readFile`

Window for symlink swap. Patched by switching to atomic read with
`ENOENT` handling instead of pre-check.

### [INFO · CLOSED] #204 — 429 amplification loop

The 30 s lock didn't honour `Retry-After`. A 429 from
`api.anthropic.com/api/oauth/usage` looped every 30 s, flooded the API,
and showed `[Timeout]` in the status bar. Now respects `Retry-After`.

## Code-level observations (not filed)

- **No certificate pinning** on the `https.request` to the OAuth
  endpoint. A trusted-root MITM (corporate proxy with a custom CA) can
  intercept the bearer. Standard for Node CLIs; mention in our threat
  model but don't bother fixing.
- **`HTTPS_PROXY` honoured.** Same caveat — setting the env var is
  authorising the proxy to see the bearer.
- **OAuth token never lands on disk** in the cache file. Verified via
  the Zod `CachedUsageDataSchema` — it allows only utilisation/reset/
  extra-usage fields plus an error enum. ✓
- **Bundled artefact reduces transitive surface.** The published
  package is a single `dist/ccstatusline.js`. Most listed deps are
  dev-only (eslint, vitest, ink, react). Compromise vector is the
  maintainer's npm account, not 200 transitive packages.
- **`isKnownCommand` regex** matches any command containing
  `ccstatusline.ts`. Used only for "is this already installed?"
  detection during TUI configuration; not a privilege boundary.

## How findings shape our build-vs-install decision

| Risk                                        | A. stdin-only | B. roll-our-own (sh) | C. install ccstatusline |
|---------------------------------------------|:-------------:|:--------------------:|:-----------------------:|
| #298 supply-chain via `npx -y …@latest`     | n/a           | n/a                  | inherits unless pinned  |
| Keychain enumeration / `dump-keychain`       | n/a           | **avoidable**        | inherits                |
| CustomCommand env leak                       | n/a           | n/a                  | inherits if widget used |
| OAuth bearer in process memory               | n/a           | inherits             | inherits                |
| Trusted code surface                         | ~50 lines sh  | ~150 lines sh        | bundled JS (~MB)        |

**Recommendation**: pick **B** with a hard rule that the credentials
fallback is `find-generic-password` only — never `dump-keychain`. If
the user picks **C**, install via the local-pinned `npm install
--prefix` pattern from #298, never `npx -y …@latest`.
