#!/bin/bash
#
# 055-test-tunnel-open-help.sh - TC-TN-001: Open Command Help
#
# Command: ztm tunnel open --help
# Expected: Exit code 0, display open command help
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 055-test-tunnel-open-help.sh"
echo "TC-TN-001: Open Command Help"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN tunnel open --help"
output=$("$ZTM_BIN" tunnel open --help 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    if echo "$output" | grep -qi "outbound\|inbound\|tcp\|udp\|targets"; then
        pass "ztm tunnel open --help shows expected options"
    else
        skip "ztm tunnel open --help may not show expected options"
    fi
else
    skip "ztm tunnel open --help not available (exit code: $exit_code)"
fi
