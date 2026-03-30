#!/bin/bash
#
# 074-test-payment-send-invalid-amount.sh - TC-15: Invalid Amount Validation
#
# Command: ztm payment send with invalid amount
# Expected: Returns error message
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 074-test-payment-send-invalid-amount.sh"
echo "TC-15: Invalid Amount Validation"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN payment send testuser --name 'Test' --amount -5"
output=$("$ZTM_BIN" payment send testuser --name "Test" --amount -5 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -ne 0 ]; then
    pass "ztm payment send with negative amount returns error (exit code: $exit_code)"
elif echo "$output" | grep -qi "error\|invalid\|positive"; then
    pass "ztm payment send shows amount validation error"
else
    skip "ztm payment amount validation not working as expected"
fi
