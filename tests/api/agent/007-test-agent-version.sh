#!/bin/bash
#
# 007-test-agent-version.sh - Test Agent version API
#
# Endpoint: GET /api/version
# Expected: HTTP 200, returns { ztm: {...}, pipy: {...} }
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 007-test-agent-version.sh"
echo "Endpoint: GET $AGENT_URL/api/version"
echo "Expected: HTTP 200, JSON with ztm and pipy version info"
echo "=========================================="
echo ""

curl_api "GET /api/version" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/version"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    
    if echo "$CURL_BODY" | jq -e '.ztm' > /dev/null 2>&1; then
        ztm_info=$(echo "$CURL_BODY" | jq -r '.ztm // "unknown"')
        echo "  ZTM version: $ztm_info"
    fi
    
    if echo "$CURL_BODY" | jq -e '.pipy' > /dev/null 2>&1; then
        pipy_info=$(echo "$CURL_BODY" | jq -r '.pipy // "unknown"')
        echo "  Pipy version: $pipy_info"
    fi
    
    pass "GET /api/version returns version info"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
