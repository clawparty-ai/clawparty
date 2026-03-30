#!/bin/bash
#
# 032-test-chat-users.sh - TC-CH-003: Get Users List
#
# Endpoint: GET /api/users
# Expected: HTTP 200, JSON array with user list
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 032-test-chat-users.sh"
echo "TC-CH-003: Get Users List"
echo "=========================================="
echo ""

curl_api "GET /api/users" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/users"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/users HTTP 200, valid JSON"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Chat app not loaded (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
