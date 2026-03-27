#!/bin/bash
#
# 036-test-chat-create-group.sh - TC-CH-007: Create Group
#
# Endpoint: POST /api/groups/{creator}/{groupId}
# Expected: HTTP 201 or 200
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"
CREATOR="${CREATOR:-testuser}"
GROUP_ID="${GROUP_ID:-testgroup036}"

echo "=========================================="
echo "Test: 036-test-chat-create-group.sh"
echo "TC-CH-007: Create Group"
echo "=========================================="
echo ""

json_payload='{"name":"Test Group","members":[]}'

curl_api "POST /api/groups/$CREATOR/$GROUP_ID" \
    -X POST \
    -H "Authorization: Bearer $API_TOKEN" \
    -H "Content-Type: application/json" \
    -d "$json_payload" \
    "$AGENT_URL/api/groups/$CREATOR/$GROUP_ID"

if [ "$CURL_STATUS" = "200" ] || [ "$CURL_STATUS" = "201" ]; then
    pass "POST /api/groups/$CREATOR/$GROUP_ID HTTP $CURL_STATUS"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Chat app not loaded (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200 or 201)"
fi
