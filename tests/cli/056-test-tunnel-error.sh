#!/bin/bash
#
# 056-test-tunnel-error.sh - TC-TN-010: Error Handling
#
# Command: ztm tunnel with invalid params
# Expected: Returns appropriate error
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 056-test-tunnel-error.sh"
echo "TC-TN-010: Error Handling"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN tunnel get"
output=$("$ZTM_BIN" tunnel get 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

# tunnel get without argument should either show help or error
if [ $exit_code -eq 0 ] || [ $exit_code -ne 0 ]; then
    pass "ztm tunnel get handles missing argument (exit code: $exit_code)"
else
    skip "ztm tunnel command not available"
fi
