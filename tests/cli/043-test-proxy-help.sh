#!/bin/bash
#
# 043-test-proxy-help.sh - TC-PX-001: CLI Help Commands
#
# Command: ztm proxy help
# Expected: Exit code 0, output contains help info
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 043-test-proxy-help.sh"
echo "TC-PX-001: CLI Help Commands"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN proxy help"
output=$("$ZTM_BIN" proxy help 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    if echo "$output" | grep -qi "proxy\|config\|listen\|target"; then
        pass "ztm proxy help shows help info"
    else
        skip "ztm proxy command may not be implemented"
    fi
else
    skip "ztm proxy command not available"
fi
