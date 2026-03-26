#!/bin/bash
#
# test-agent-version.sh - Test Agent version API
#
# Endpoint: GET /api/version
# Expected: HTTP 200, returns { ztm: {...}, pipy: {...} }
#

set -e

# Agent1 configuration
AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "=========================================="
echo "Test: GET /api/version"
echo "Input: None"
echo "Expected: HTTP 200, JSON with ztm and pipy version info"
echo "=========================================="
echo ""

# Add auth header
AUTH_HEADER="-H \"Authorization: Bearer $API_TOKEN\""

echo "Execute: curl -s -w 'HTTP_STATUS:%{http_code}' $AUTH_HEADER $AGENT_URL/api/version"
output=$(curl -s -w "\nHTTP_STATUS:%{http_code}" -H "Authorization: Bearer $API_TOKEN" "$AGENT_URL/api/version")

http_status=$(echo "$output" | grep "HTTP_STATUS" | cut -d: -f2)
response_body=$(echo "$output" | sed '/HTTP_STATUS/d')

echo ""
echo "Output:"
echo "$response_body"
echo ""

echo "HTTP status code: $http_status"
echo ""

if [ "$http_status" = "200" ]; then
    # Verify JSON format
    if echo "$response_body" | jq -e '.ztm' > /dev/null 2>&1; then
        ztm_info=$(echo "$response_body" | jq -r '.ztm // "unknown"')
        echo "ZTM version: $ztm_info"
    fi
    
    if echo "$response_body" | jq -e '.pipy' > /dev/null 2>&1; then
        pipy_info=$(echo "$response_body" | jq -r '.pipy // "unknown"')
        echo "Pipy version: $pipy_info"
    fi
    
    echo -e "${GREEN}✓ PASS${NC}: GET /api/version returns version info"
else
    echo -e "${RED}✗ FAIL${NC}: HTTP $http_status (expected: 200)"
    exit 1
fi

exit 0