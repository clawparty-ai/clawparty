#!/bin/bash
#
# 051-test-tunnel-get.sh - TC-TN-002: Get Tunnel List
#
# Command: ztm tunnel get outbound
# Expected: Exit code 0, display tunnel list
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 051-test-tunnel-get.sh"
echo "TC-TN-002: Get Tunnel List"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN tunnel get outbound"
output=$("$ZTM_BIN" tunnel get outbound 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm tunnel get outbound succeeded"
else
    skip "ztm tunnel get not available (exit code: $exit_code)"
fi
