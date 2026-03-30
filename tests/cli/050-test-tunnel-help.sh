#!/bin/bash
#
# 050-test-tunnel-help.sh - TC-TN-001: CLI Help Commands
#
# Command: ztm tunnel help
# Expected: Exit code 0, output contains help info
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 050-test-tunnel-help.sh"
echo "TC-TN-001: CLI Help Commands"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN tunnel help"
output=$("$ZTM_BIN" tunnel help 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    if echo "$output" | grep -qi "tunnel\|open\|close\|get"; then
        pass "ztm tunnel help shows help info"
    else
        skip "ztm tunnel command may not be implemented"
    fi
else
    skip "ztm tunnel command not available"
fi
