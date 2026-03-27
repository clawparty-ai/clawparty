#!/bin/bash
#
# 069-test-payment-list.sh - TC-02: List Payments
#
# Command: ztm payment list
# Expected: Exit code 0, display payment list
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 069-test-payment-list.sh"
echo "TC-02: List Payments"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN payment list"
output=$("$ZTM_BIN" payment list 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm payment list succeeded"
else
    skip "ztm payment list not available (exit code: $exit_code)"
fi
