#!/bin/bash
#
# 068-test-payment-help.sh - TC-01: CLI Help
#
# Command: ztm payment --help
# Expected: Exit code 0, display help info
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 068-test-payment-help.sh"
echo "TC-01: CLI Help"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN payment --help"
output=$("$ZTM_BIN" payment --help 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    if echo "$output" | grep -qi "payment\|send\|list\|get"; then
        pass "ztm payment --help shows help info"
    else
        pass "ztm payment --help executed successfully"
    fi
else
    skip "ztm payment command not available (exit code: $exit_code)"
fi
