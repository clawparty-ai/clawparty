#!/bin/bash
#
# 013-test-version-json.sh - Test ztm version --json command
#
# Command: ztm version --json
# Expected: Exit code 0, valid JSON output
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

echo "=========================================="
echo "Test: 013-test-version-json.sh"
echo "Command: $ZTM_BIN version --json"
echo "Expected: Valid JSON output"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN version --json"
output=$("$ZTM_BIN" version --json 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -ne 0 ]; then
    fail "Command exit code is non-zero"
fi

# Verify valid JSON
if echo "$output" | jq empty 2>/dev/null; then
    echo "$output" | jq .
    pass "JSON output valid"
else
    fail "Invalid JSON format"
fi
