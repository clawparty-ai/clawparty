#!/bin/bash
#
# 035-test-chat-get-peer-msg.sh - TC-CH-006: Receive Peer Message
#
# Endpoint: GET /api/peers/{peer}/messages
# Expected: HTTP 200, JSON array
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"
TEST_PEER="${TEST_PEER:-testuser}"

echo "=========================================="
echo "Test: 035-test-chat-get-peer-msg.sh"
echo "TC-CH-006: Receive Peer Message"
echo "=========================================="
echo ""

curl_api "GET /api/peers/$TEST_PEER/messages" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/peers/$TEST_PEER/messages"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/peers/$TEST_PEER/messages HTTP 200, valid JSON"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Chat app not loaded (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
