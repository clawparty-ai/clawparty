#!/bin/bash
#
# 067-test-cloud-api-mirrors.sh - TC-CL-013: Mirror Configuration View
#
# Endpoint: GET /api/mirrors
# Expected: HTTP 200 or 404
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 067-test-cloud-api-mirrors.sh"
echo "TC-CL-013: Mirror Configuration View"
echo "=========================================="
echo ""

curl_api "GET /api/mirrors" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/mirrors"

if [ "$CURL_STATUS" = "200" ] || [ "$CURL_STATUS" = "404" ]; then
    pass "GET /api/mirrors HTTP $CURL_STATUS"
else
    fail "HTTP $CURL_STATUS (expected: 200 or 404)"
fi
