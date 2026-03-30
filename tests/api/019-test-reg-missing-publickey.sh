#!/bin/bash
#
# 019-test-reg-missing-publickey.sh - Test registration without PublicKey
#
# Endpoint: POST /invite
# Input: {UserName, EpName, PassKey} - missing PublicKey
# Expected: HTTP 400 "missing PublicKey"
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

REG_PORT="${REG_PORT:-5678}"

echo "=========================================="
echo "Test: 019-test-reg-missing-publickey.sh"
echo "Endpoint: POST http://127.0.0.1:$REG_PORT/invite"
echo "Expected: HTTP 400 (missing PublicKey)"
echo "=========================================="
echo ""

json_payload='{"UserName":"testuser","EpName":"test-ep","PassKey":"testpass"}'

curl_api "POST /invite (missing PublicKey)" \
    -X POST \
    -H "Content-Type: application/json" \
    -d "$json_payload" \
    "http://127.0.0.1:$REG_PORT/invite"

if [ "$CURL_STATUS" = "400" ]; then
    pass "Returns 400 for missing PublicKey"
else
    fail "HTTP $CURL_STATUS (expected: 400)"
fi
