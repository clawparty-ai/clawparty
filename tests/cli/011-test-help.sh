#!/bin/bash
#
# 011-test-help.sh - Test ztm help command
#
# Command: ztm help
# Expected: Exit code 0, output contains "Usage:" and "Commands:"
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

echo "=========================================="
echo "Test: 011-test-help.sh"
echo "Command: $ZTM_BIN help"
echo "Expected: Output contains Usage: and Commands:"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN help"
output=$("$ZTM_BIN" help 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

# Check exit code
if [ $exit_code -ne 0 ]; then
    fail "Command exit code is non-zero (exit code: $exit_code)"
fi

# Verify output contains required content
if echo "$output" | grep -q "Usage:"; then
    echo "✓ Contains 'Usage:'"
else
    fail "Does not contain 'Usage:'"
fi

if echo "$output" | grep -q "Commands:"; then
    echo "✓ Contains 'Commands:'"
else
    fail "Does not contain 'Commands:'"
fi

pass "ztm help works correctly"
