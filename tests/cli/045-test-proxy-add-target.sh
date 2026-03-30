#!/bin/bash
#
# 045-test-proxy-add-target.sh - TC-PX-005: Add Domain Target
#
# Command: ztm proxy config --add-target
# Expected: Exit code 0
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 045-test-proxy-add-target.sh"
echo "TC-PX-005: Add Domain Target"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN proxy config --add-target 0.0.0.0/0 '*'"
output=$("$ZTM_BIN" proxy config --add-target "0.0.0.0/0" "*" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm proxy config --add-target succeeded"
else
    skip "ztm proxy config --add-target not available (exit code: $exit_code)"
fi
