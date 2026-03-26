#!/bin/bash
#
# test-identity-agent1.sh - Test Agent1 identity command
#
# Command: ztm identity
# Environment variables: ZTM_CONFIG=127.0.0.1:7778, ZTM_API_TOKEN=enjoy-party
# Expected output: Agent1 identity (PEM certificate)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

# Agent1 environment variables
export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "=========================================="
echo "Test: ztm identity (Agent1)"
echo "Command: $ZTM_BIN identity"
echo "Environment variables:"
echo "  ZTM_CONFIG=$ZTM_CONFIG"
echo "  ZTM_API_TOKEN=$ZTM_API_TOKEN"
echo "Expected: Output PEM certificate"
echo "=========================================="
echo ""

echo "Execute: ZTM_CONFIG=$ZTM_CONFIG ZTM_API_TOKEN=$ZTM_API_TOKEN $ZTM_BIN identity"
output=$("$ZTM_BIN" identity 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -ne 0 ]; then
    echo -e "${RED}✗ FAIL${NC}: Command exit code is non-zero"
    exit 1
fi

# Verify output contains PEM certificate or public key
if echo "$output" | grep -q "BEGIN CERTIFICATE\|BEGIN PUBLIC KEY"; then
    echo -e "${GREEN}✓ PASS${NC}: Returns valid certificate/public key"
    echo "Key header: $(echo "$output" | head -1)"
else
    echo -e "${RED}✗ FAIL${NC}: Does not contain valid key"
    exit 1
fi

exit 0