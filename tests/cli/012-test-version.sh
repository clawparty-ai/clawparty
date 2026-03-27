#!/bin/bash
#
# 012-test-version.sh - Test ztm version command
#
# Command: ztm version
# Expected: Exit code 0, output contains ZTM and Pipy version info
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

echo "=========================================="
echo "Test: 012-test-version.sh"
echo "Command: $ZTM_BIN version"
echo "Expected: Output contains version info"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN version"
output=$("$ZTM_BIN" version 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -ne 0 ]; then
    fail "Command exit code is non-zero"
fi

# Verify output contains required content
if echo "$output" | grep -qi "ZTM\|ztm"; then
    echo "✓ Contains ZTM version info"
else
    fail "Does not contain ZTM version"
fi

if echo "$output" | grep -qi "pipy"; then
    echo "✓ Contains Pipy version info"
else
    fail "Does not contain Pipy version"
fi

pass "ztm version works correctly"
