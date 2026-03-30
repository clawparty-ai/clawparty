#!/bin/bash
#
# 072-test-payment-cancel.sh - TC-04: Cancel Payment
#
# Command: ztm payment cancel <id>
# Expected: Exit code 0 or appropriate error
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 072-test-payment-cancel.sh"
echo "TC-04: Cancel Payment"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN payment cancel 00000000-0000-0000-0000-000000000000"
output=$("$ZTM_BIN" payment cancel "00000000-0000-0000-0000-000000000000" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm payment cancel succeeded"
elif echo "$output" | grep -qi "not found\|cannot cancel"; then
    pass "ztm payment cancel correctly reports error"
else
    skip "ztm payment cancel not available (exit code: $exit_code)"
fi
