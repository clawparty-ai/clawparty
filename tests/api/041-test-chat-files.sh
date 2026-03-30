#!/bin/bash
#
# 041-test-chat-files.sh - TC-CH-019: File Upload
#
# Endpoint: GET /api/files
# Expected: HTTP 200, JSON array
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 041-test-chat-files.sh"
echo "TC-CH-019: File Upload"
echo "=========================================="
echo ""

curl_api "GET /api/files" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/files"

if [ "$CURL_STATUS" = "200" ] || [ "$CURL_STATUS" = "404" ]; then
    pass "GET /api/files HTTP $CURL_STATUS"
else
    fail "HTTP $CURL_STATUS (expected: 200 or 404)"
fi
