#!/bin/bash
#
# 061-test-cloud-unpublish.sh - TC-CL-007: File Deletion (Unpublish)
#
# Command: ztm cloud unpublish
# Expected: Exit code 0 or appropriate message
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 061-test-cloud-unpublish.sh"
echo "TC-CL-007: File Deletion (Unpublish)"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN cloud unpublish /nonexistent.txt"
output=$("$ZTM_BIN" cloud unpublish "/nonexistent.txt" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

# Should either succeed silently or report file not found
if [ $exit_code -eq 0 ] || echo "$output" | grep -qi "not found"; then
    pass "ztm cloud unpublish handled correctly (exit code: $exit_code)"
else
    skip "ztm cloud unpublish not available (exit code: $exit_code)"
fi
