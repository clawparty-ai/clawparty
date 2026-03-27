#!/bin/bash
#
# 030-test-chat-appinfo.sh - TC-CH-001: Get App Info
#
# Endpoint: GET /api/appinfo
# Expected: HTTP 200, JSON with name, provider, username, endpoint
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 030-test-chat-appinfo.sh"
echo "TC-CH-001: Get App Info"
echo "=========================================="
echo ""

curl_api "GET /api/appinfo" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/appinfo"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    
    # Verify required fields
    if echo "$CURL_BODY" | jq -e '.name' > /dev/null 2>&1; then
        echo "  name: $(echo "$CURL_BODY" | jq -r '.name')"
    else
        fail "Response missing 'name' field"
    fi
    
    if echo "$CURL_BODY" | jq -e '.username' > /dev/null 2>&1; then
        echo "  username: $(echo "$CURL_BODY" | jq -r '.username')"
    fi
    
    if echo "$CURL_BODY" | jq -e '.endpoint' > /dev/null 2>&1; then
        echo "  endpoint: $(echo "$CURL_BODY" | jq -r '.endpoint')"
    fi
    
    pass "GET /api/appinfo HTTP 200, valid JSON with required fields"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Chat app not loaded (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
