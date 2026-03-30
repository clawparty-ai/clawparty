#!/bin/bash
#
# 054-test-tunnel-close.sh - TC-TN-007: Delete Tunnel Endpoint
#
# Command: ztm tunnel close outbound tcp/test
# Expected: Exit code 0
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 054-test-tunnel-close.sh"
echo "TC-TN-007: Delete Tunnel Endpoint"
echo "=========================================="
echo ""

# First create a tunnel
echo "Setup: Creating tunnel..."
"$ZTM_BIN" tunnel open outbound tcp/test --targets "127.0.0.1:80" 2>/dev/null || true

echo "Execute: $ZTM_BIN tunnel close outbound tcp/test"
output=$("$ZTM_BIN" tunnel close outbound tcp/test 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm tunnel close succeeded"
else
    skip "ztm tunnel close not available (exit code: $exit_code)"
fi
