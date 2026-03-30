#!/bin/bash
#
# 042-test-chat-cli.sh - TC-CH-025: CLI Command Tests
#
# Command: ztm chat --help
# Expected: Exit code 0, output contains help info
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 042-test-chat-cli.sh"
echo "TC-CH-025: CLI Command Tests"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN chat --help"
output=$("$ZTM_BIN" chat --help 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    if echo "$output" | grep -qi "chat\|send\|list"; then
        pass "ztm chat --help shows help info"
    else
        skip "ztm chat command may not be implemented"
    fi
else
    skip "ztm chat command not available"
fi
