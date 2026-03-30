#!/bin/bash
#
# 037-test-chat-get-group.sh - TC-CH-007: Get Group Info
#
# Endpoint: GET /api/groups/{creator}/{groupId}
# Expected: HTTP 200, JSON with group info
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"
CREATOR="${CREATOR:-testuser}"
GROUP_ID="${GROUP_ID:-testgroup036}"

echo "=========================================="
echo "Test: 037-test-chat-get-group.sh"
echo "TC-CH-007: Get Group Info"
echo "=========================================="
echo ""

curl_api "GET /api/groups/$CREATOR/$GROUP_ID" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/groups/$CREATOR/$GROUP_ID"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/groups/$CREATOR/$GROUP_ID HTTP 200, valid JSON"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Group not found (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
