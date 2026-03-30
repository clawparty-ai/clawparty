#!/bin/bash
#
# 040-test-chat-peer-autoreply.sh - TC-CH-014: Peer Auto-reply Configuration
#
# Endpoint: GET /api/peers/{peer}/auto-reply
# Expected: HTTP 200, JSON with auto-reply config
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"
TEST_PEER="${TEST_PEER:-testuser}"

echo "=========================================="
echo "Test: 040-test-chat-peer-autoreply.sh"
echo "TC-CH-014: Peer Auto-reply Configuration"
echo "=========================================="
echo ""

curl_api "GET /api/peers/$TEST_PEER/auto-reply" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/peers/$TEST_PEER/auto-reply"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/peers/$TEST_PEER/auto-reply HTTP 200, valid JSON"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Peer not found (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
