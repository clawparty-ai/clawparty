#!/bin/bash
#
# 031-test-chat-endpoints.sh - TC-CH-002: Get Endpoints List
#
# Endpoint: GET /api/endpoints
# Expected: HTTP 200, JSON array with endpoint info
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 031-test-chat-endpoints.sh"
echo "TC-CH-002: Get Endpoints List"
echo "=========================================="
echo ""

curl_api "GET /api/endpoints" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/endpoints"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/endpoints HTTP 200, valid JSON"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Chat app not loaded (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
