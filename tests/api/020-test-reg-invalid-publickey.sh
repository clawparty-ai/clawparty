#!/bin/bash
#
# 020-test-reg-invalid-publickey.sh - Test registration with invalid PublicKey
#
# Endpoint: POST /invite
# Input: {PublicKey: "invalid", ...}
# Expected: HTTP 400 "invalid PublicKey"
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

REG_PORT="${REG_PORT:-5678}"

echo "=========================================="
echo "Test: 020-test-reg-invalid-publickey.sh"
echo "Endpoint: POST http://127.0.0.1:$REG_PORT/invite"
echo "Expected: HTTP 400 (invalid PublicKey)"
echo "=========================================="
echo ""

json_payload='{"PublicKey":"invalid-key","UserName":"testuser","EpName":"test-ep","PassKey":"testpass"}'

curl_api "POST /invite (invalid PublicKey)" \
    -X POST \
    -H "Content-Type: application/json" \
    -d "$json_payload" \
    "http://127.0.0.1:$REG_PORT/invite"

if [ "$CURL_STATUS" = "400" ]; then
    pass "Returns 400 for invalid PublicKey"
else
    fail "HTTP $CURL_STATUS (expected: 400)"
fi
