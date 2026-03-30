#!/bin/bash
#
# 026-test-cli-join-party.sh - Test CLI join party command
#
# Command: ztm join party --reg-url http://127.0.0.1:5678
# Expected: Successfully joins mesh or already joined
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 026-test-cli-join-party.sh"
echo "Command: $ZTM_BIN join party --reg-url http://127.0.0.1:5678"
echo "Expected: Successfully joins mesh or already joined"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN join party --reg-url http://127.0.0.1:5678"
output=$("$ZTM_BIN" join party --reg-url http://127.0.0.1:5678 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if echo "$output" | grep -q "Successfully joined"; then
    pass "CLI join party successful"
elif echo "$output" | grep -q "already joined"; then
    pass "Already joined (expected)"
else
    fail "CLI join failed"
fi
