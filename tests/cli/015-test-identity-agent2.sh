#!/bin/bash
#
# 015-test-identity-agent2.sh - Test Agent2 identity command
#
# Command: ztm identity
# Environment: ZTM_CONFIG=127.0.0.1:7779, ZTM_API_TOKEN=enjoy-party
# Expected: PEM certificate or public key
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

# Agent2 environment variables
export ZTM_CONFIG="127.0.0.1:7779"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 015-test-identity-agent2.sh"
echo "Command: $ZTM_BIN identity"
echo "Environment: ZTM_CONFIG=$ZTM_CONFIG"
echo "Expected: PEM certificate or public key"
echo "=========================================="
echo ""

echo "Execute: ZTM_CONFIG=$ZTM_CONFIG ZTM_API_TOKEN=$ZTM_API_TOKEN $ZTM_BIN identity"
output=$("$ZTM_BIN" identity 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -ne 0 ]; then
    fail "Command exit code is non-zero"
fi

# Verify output contains PEM certificate or public key
if echo "$output" | grep -q "BEGIN CERTIFICATE\|BEGIN PUBLIC KEY"; then
    echo "Key header: $(echo "$output" | head -1)"
    pass "Returns valid certificate/public key"
else
    fail "Does not contain valid key"
fi
