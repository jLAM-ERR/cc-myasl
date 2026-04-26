#!/bin/sh
# claude-statusline remote installer.
#
# Usage:
#   curl -fsSL <raw>/scripts/install.sh | sh
#   VERSION=v0.1.0 curl -fsSL <raw>/scripts/install.sh | sh
#
# Detects platform, downloads the matching tarball + .sha256 from
# GitHub Releases, verifies the checksum, extracts, installs the
# binary and templates. Never edits ~/.claude/settings.json — prints
# the snippet for the user to merge themselves.

set -eu

# ── configuration ─────────────────────────────────────────────────────────
OWNER="${OWNER:-jLAM-ERR}"
REPO="${REPO:-cc-myasl}"
VERSION="${VERSION:-latest}"

DEST_BIN="${DEST_BIN:-$HOME/.claude/bin}"
DEST_TPL="${DEST_TPL:-$HOME/.config/claude-statusline/templates}"

# ── platform detection ────────────────────────────────────────────────────
case "$(uname -sm)" in
    "Darwin arm64")    TARGET="aarch64-apple-darwin" ;;
    "Darwin x86_64")   TARGET="x86_64-apple-darwin" ;;
    "Linux x86_64")    TARGET="x86_64-unknown-linux-musl" ;;
    "Linux aarch64")   TARGET="aarch64-unknown-linux-musl" ;;
    *) printf 'error: unsupported platform: %s\n' "$(uname -sm)" >&2; exit 1 ;;
esac

# ── version resolution ────────────────────────────────────────────────────
if [ "$VERSION" = "latest" ]; then
    api="https://api.github.com/repos/$OWNER/$REPO/releases/latest"
    VERSION=$(curl -fsSL "$api" | sed -n 's/.*"tag_name": *"\([^"]*\)".*/\1/p' | head -n1)
    [ -n "$VERSION" ] || {
        printf 'error: failed to resolve latest version from %s\n' "$api" >&2
        exit 1
    }
fi

# Strip leading "v" for the tarball name.
VER_NUM="${VERSION#v}"
TARBALL="claude-statusline-${VER_NUM}-${TARGET}.tar.gz"
SHA="${TARBALL}.sha256"
URL_BASE="https://github.com/$OWNER/$REPO/releases/download/$VERSION"

# ── download to tempdir ───────────────────────────────────────────────────
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT INT TERM

printf 'Downloading %s...\n' "$TARBALL"
curl -fsSL "$URL_BASE/$TARBALL" -o "$TMP/$TARBALL"

if curl -fsSL "$URL_BASE/$SHA" -o "$TMP/$SHA" 2>/dev/null; then
    : # downloaded successfully
else
    printf 'warn: no .sha256 file at %s — skipping checksum verification\n' "$URL_BASE/$SHA" >&2
    rm -f "$TMP/$SHA"
fi

if [ -f "$TMP/$SHA" ]; then
    ( cd "$TMP" && shasum -a 256 -c "$SHA" >/dev/null ) || {
        printf 'error: checksum mismatch for %s\n' "$TARBALL" >&2
        exit 1
    }
fi

# ── extract + install ─────────────────────────────────────────────────────
tar -xzf "$TMP/$TARBALL" -C "$TMP"
EXTRACTED="$TMP/claude-statusline-${VER_NUM}-${TARGET}"
[ -d "$EXTRACTED" ] || {
    printf 'error: tarball did not produce expected dir %s\n' "$EXTRACTED" >&2
    exit 1
}

mkdir -p "$DEST_BIN" "$DEST_TPL"
install -m 0755 "$EXTRACTED/bin/claude-statusline" "$DEST_BIN/claude-statusline"
cp "$EXTRACTED"/templates/*.txt "$DEST_TPL/"

# ── settings snippet (NEVER edit ~/.claude/settings.json) ─────────────────
printf '\n'
printf 'Installed claude-statusline %s -> %s/claude-statusline\n' "$VERSION" "$DEST_BIN"
printf 'Templates -> %s\n' "$DEST_TPL"
printf '\n'
printf 'Add to ~/.claude/settings.json:\n'
printf '  "statusLine": {\n'
printf '    "type": "command",\n'
printf '    "command": "%s/claude-statusline --template default"\n' "$DEST_BIN"
printf '  }\n'
printf '\n'
printf 'macOS users: if you see an unidentified developer warning,\n'
printf '  xattr -d com.apple.quarantine %s/claude-statusline\n' "$DEST_BIN"
