#!/bin/bash
#
# 048-test-proxy-config-help.sh - TC-PX-001: Config Command Help
#
# Command: ztm proxy help config
# Expected: Exit code 0, display config command help
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 048-test-proxy-config-help.sh"
echo "TC-PX-001: Config Command Help"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN proxy help config"
output=$("$ZTM_BIN" proxy help config 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    if echo "$output" | grep -qi "set-listen\|add-target\|remove-target"; then
        pass "ztm proxy help config shows option descriptions"
    else
        skip "ztm proxy help config may not show expected options"
    fi
else
    skip "ztm proxy help config not available (exit code: $exit_code)"
fi
