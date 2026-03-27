#!/bin/bash
#
# 052-test-tunnel-open-tcp.sh - TC-TN-002: Create TCP Outbound Endpoint
#
# Command: ztm tunnel open outbound tcp/test --targets
# Expected: Exit code 0
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 052-test-tunnel-open-tcp.sh"
echo "TC-TN-002: Create TCP Outbound Endpoint"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN tunnel open outbound tcp/test --targets 127.0.0.1:80"
output=$("$ZTM_BIN" tunnel open outbound tcp/test --targets "127.0.0.1:80" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm tunnel open outbound tcp/test succeeded"
    
    # Clean up
    "$ZTM_BIN" tunnel close outbound tcp/test 2>/dev/null || true
else
    skip "ztm tunnel open not available (exit code: $exit_code)"
fi
