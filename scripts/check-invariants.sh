#!/bin/sh
# check-invariants.sh — enforce Hard Invariants 4, 5, 6.
set -eu

SELF="scripts/check-invariants.sh"
FAILED=0

# Invariant 4: no security dump-keychain invocation.
# Exclude this script itself to avoid matching the grep pattern string.
if grep -r --exclude="$(basename "$SELF")" "dump-keychain" src/ scripts/ 2>/dev/null; then
    echo "FAIL: 'dump-keychain' found in src/ or scripts/" >&2
    FAILED=1
fi

# Invariant 6: no npx …@latest / @latest auto-update paths.
# Exclude this script itself to avoid matching the grep literal pattern strings.
if grep -rEi --exclude="$(basename "$SELF")" "npx.*latest|@latest" src/ scripts/ 2>/dev/null; then
    echo "FAIL: npx.*latest or @latest found in src/ or scripts/" >&2
    FAILED=1
fi

# Invariant 5: install.sh must not reference cargo/rustc/rustup.
# Tolerate the file being absent (it ships in Task 16).
if [ -f scripts/install.sh ]; then
    if grep -Ei "cargo|rustc|rustup" scripts/install.sh 2>/dev/null; then
        echo "FAIL: cargo/rustc/rustup reference found in scripts/install.sh" >&2
        FAILED=1
    fi
fi

# Invariant 7: format/*.rs must NOT import crate::config.
# (One-way dependency: format is a pure rendering engine.)
# Skip comments (lines starting with //) to reduce false positives.
if grep -r "use crate::config" src/format/ 2>/dev/null | grep -v "^\s*//" ; then
    echo "FAIL: 'use crate::config' found in src/format/" >&2
    FAILED=1
fi

# Invariant 8: config/*.rs must NOT import crate::api or crate::cache.
# (Parallel decoupling invariant — config must not reach into HTTP/cache layers.)
if grep -r "use crate::api\|use crate::cache" src/config/ 2>/dev/null | grep -v "^\s*//" ; then
    echo "FAIL: 'use crate::api' or 'use crate::cache' found in src/config/" >&2
    FAILED=1
fi

# Invariant 9: git/*.rs must NOT import crate::format, crate::config, crate::api, or crate::cache.
# (git is a low-level module; it must not depend on rendering or HTTP layers.)
if grep -r "use crate::format\|use crate::config\|use crate::api\|use crate::cache" src/git/ 2>/dev/null | grep -v "^\s*//" ; then
    echo "FAIL: forbidden crate import found in src/git/" >&2
    FAILED=1
fi

# Invariant 10: format/*.rs and format/placeholders/*.rs must NOT import crate::git.
# (Rendering engine must not reach into the git module; git data flows via RenderCtx primitives.)
if grep -r "use crate::git" src/format/ 2>/dev/null | grep -v "^\s*//" ; then
    echo "FAIL: 'use crate::git' found in src/format/ (including placeholders/)" >&2
    FAILED=1
fi

if [ "$FAILED" -eq 1 ]; then
    exit 1
fi

echo "check-invariants: all invariants satisfied"
