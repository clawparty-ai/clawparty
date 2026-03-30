#!/bin/bash
#
# 033-test-chat-chats.sh - TC-CH-004: Get Chats List
#
# Endpoint: GET /api/chats
# Expected: HTTP 200, JSON array with chats
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 033-test-chat-chats.sh"
echo "TC-CH-004: Get Chats List"
echo "=========================================="
echo ""

curl_api "GET /api/chats" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/chats"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/chats HTTP 200, valid JSON"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Chat app not loaded (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
