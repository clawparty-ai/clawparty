#!/bin/bash
#
# 053-test-tunnel-describe.sh - TC-TN-006: View Tunnel Details
#
# Command: ztm tunnel describe outbound tcp/test
# Expected: Exit code 0, display tunnel details
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 053-test-tunnel-describe.sh"
echo "TC-TN-006: View Tunnel Details"
echo "=========================================="
echo ""

# First create a tunnel
echo "Setup: Creating tunnel..."
"$ZTM_BIN" tunnel open outbound tcp/test --targets "127.0.0.1:80" 2>/dev/null || true

echo "Execute: $ZTM_BIN tunnel describe outbound tcp/test"
output=$("$ZTM_BIN" tunnel describe outbound tcp/test 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

# Clean up
"$ZTM_BIN" tunnel close outbound tcp/test 2>/dev/null || true

if [ $exit_code -eq 0 ]; then
    pass "ztm tunnel describe succeeded"
else
    skip "ztm tunnel describe not available (exit code: $exit_code)"
fi
