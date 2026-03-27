#!/bin/bash
#
# 034-test-chat-send-peer-msg.sh - TC-CH-005: Send Peer Message
#
# Endpoint: POST /api/peers/{peer}/messages
# Expected: HTTP 201 or 200
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"
TEST_PEER="${TEST_PEER:-testuser}"

echo "=========================================="
echo "Test: 034-test-chat-send-peer-msg.sh"
echo "TC-CH-005: Send Peer Message"
echo "=========================================="
echo ""

json_payload='{"text":"Hello from test"}'

curl_api "POST /api/peers/$TEST_PEER/messages" \
    -X POST \
    -H "Authorization: Bearer $API_TOKEN" \
    -H "Content-Type: application/json" \
    -d "$json_payload" \
    "$AGENT_URL/api/peers/$TEST_PEER/messages"

if [ "$CURL_STATUS" = "200" ] || [ "$CURL_STATUS" = "201" ]; then
    pass "POST /api/peers/$TEST_PEER/messages HTTP $CURL_STATUS"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Chat app not loaded (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200 or 201)"
fi
