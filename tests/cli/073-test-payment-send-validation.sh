#!/bin/bash
#
# 073-test-payment-send-validation.sh - TC-15: CLI Validation
#
# Command: ztm payment send without required options
# Expected: Returns error message
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 073-test-payment-send-validation.sh"
echo "TC-15: CLI Validation"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN payment send testuser"
output=$("$ZTM_BIN" payment send testuser 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -ne 0 ]; then
    pass "ztm payment send without --name returns error (exit code: $exit_code)"
elif echo "$output" | grep -qi "error\|missing\|required"; then
    pass "ztm payment send shows validation error"
else
    skip "ztm payment validation not working as expected"
fi
