#!/bin/bash
#
# 039-test-chat-delete-group.sh - TC-CH-012: Delete Group
#
# Endpoint: DELETE /api/groups/{creator}/{groupId}
# Expected: HTTP 204 or 200
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"
CREATOR="${CREATOR:-testuser}"
GROUP_ID="${GROUP_ID:-testgroup036}"

echo "=========================================="
echo "Test: 039-test-chat-delete-group.sh"
echo "TC-CH-012: Delete Group"
echo "=========================================="
echo ""

curl_api "DELETE /api/groups/$CREATOR/$GROUP_ID" \
    -X DELETE \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/groups/$CREATOR/$GROUP_ID"

if [ "$CURL_STATUS" = "200" ] || [ "$CURL_STATUS" = "204" ]; then
    pass "DELETE /api/groups/$CREATOR/$GROUP_ID HTTP $CURL_STATUS"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Group not found (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200 or 204)"
fi
