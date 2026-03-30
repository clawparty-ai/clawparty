#!/bin/bash
#
# 009-test-agent-meshes-list.sh - Test get mesh list
#
# Endpoint: GET /api/meshes
# Expected: HTTP 200, JSON array
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 009-test-agent-meshes-list.sh"
echo "Endpoint: GET $AGENT_URL/api/meshes"
echo "Expected: HTTP 200, JSON array"
echo "=========================================="
echo ""

curl_api "GET /api/meshes" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/meshes"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/meshes HTTP 200, valid JSON"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
