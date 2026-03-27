#!/bin/bash
#
# 038-test-chat-send-group-msg.sh - TC-CH-010: Send Group Message
#
# Endpoint: POST /api/groups/{creator}/{groupId}/messages
# Expected: HTTP 201 or 200
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"
CREATOR="${CREATOR:-testuser}"
GROUP_ID="${GROUP_ID:-testgroup036}"

echo "=========================================="
echo "Test: 038-test-chat-send-group-msg.sh"
echo "TC-CH-010: Send Group Message"
echo "=========================================="
echo ""

json_payload='{"text":"Hello group from test"}'

curl_api "POST /api/groups/$CREATOR/$GROUP_ID/messages" \
    -X POST \
    -H "Authorization: Bearer $API_TOKEN" \
    -H "Content-Type: application/json" \
    -d "$json_payload" \
    "$AGENT_URL/api/groups/$CREATOR/$GROUP_ID/messages"

if [ "$CURL_STATUS" = "200" ] || [ "$CURL_STATUS" = "201" ]; then
    pass "POST /api/groups/$CREATOR/$GROUP_ID/messages HTTP $CURL_STATUS"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Chat app not loaded (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200 or 201)"
fi
