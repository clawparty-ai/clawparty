#!/bin/bash
# test-version-json.sh - Test JSON output

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

output=$("$ZTM_BIN" version --json 2>&1)

# Verify valid JSON
if echo "$output" | jq empty 2>/dev/null; then
    echo "✓ PASS: JSON output valid"
    echo "$output" | jq .
else
    echo "✗ FAIL: Invalid JSON format"
    echo "$output"
    exit 1
fi
exit 0