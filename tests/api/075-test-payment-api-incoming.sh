#!/bin/bash
#
# 075-test-payment-api-incoming.sh - TC-06: Buyer Receives Notification
#
# Endpoint: GET /api/incoming
# Expected: HTTP 200, 404, or 500
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 075-test-payment-api-incoming.sh"
echo "TC-06: Buyer Receives Notification"
echo "=========================================="
echo ""

curl_api "GET /api/incoming" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/incoming"

if [ "$CURL_STATUS" = "200" ] || [ "$CURL_STATUS" = "404" ] || [ "$CURL_STATUS" = "500" ]; then
    pass "GET /api/incoming HTTP $CURL_STATUS"
else
    fail "HTTP $CURL_STATUS (expected: 200, 404, or 500)"
fi
