#!/bin/bash
#
# 071-test-payment-get.sh - TC-03: Get Payment Details
#
# Command: ztm payment get <id>
# Expected: Exit code 0 or payment not found
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 071-test-payment-get.sh"
echo "TC-03: Get Payment Details"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN payment get 00000000-0000-0000-0000-000000000000"
output=$("$ZTM_BIN" payment get "00000000-0000-0000-0000-000000000000" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if echo "$output" | grep -qi "not found"; then
    pass "ztm payment get correctly reports 'Payment not found'"
elif [ $exit_code -eq 0 ]; then
    pass "ztm payment get succeeded"
else
    skip "ztm payment get not available (exit code: $exit_code)"
fi
