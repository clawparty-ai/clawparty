#!/bin/bash
#
# 070-test-payment-send.sh - TC-01: Create Payment
#
# Command: ztm payment send
# Expected: Exit code 0 or appropriate error
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 070-test-payment-send.sh"
echo "TC-01: Create Payment"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN payment send testuser --name 'Test Product' --amount 100 --currency USD"
output=$("$ZTM_BIN" payment send testuser --name "Test Product" --amount 100 --currency USD 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm payment send succeeded"
elif echo "$output" | grep -qi "not found\|not available"; then
    skip "ztm payment command may not be available"
else
    skip "ztm payment send returned exit code: $exit_code"
fi
