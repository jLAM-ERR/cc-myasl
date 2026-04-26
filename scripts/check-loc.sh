#!/bin/sh
# check-loc.sh — fail if any single .rs file under src/ exceeds 500 LOC.
set -eu

LIMIT=500
FAILED=0

while IFS= read -r file; do
    lines=$(wc -l < "$file")
    if [ "$lines" -gt "$LIMIT" ]; then
        echo "FAIL: $file has $lines lines (limit $LIMIT)" >&2
        FAILED=1
    fi
done << EOF
$(find src -name '*.rs')
EOF

if [ "$FAILED" -eq 1 ]; then
    exit 1
fi

echo "check-loc: all .rs files are within the $LIMIT-line limit"
