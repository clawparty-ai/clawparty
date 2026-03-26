#!/bin/bash
#
# test-agent-identity.sh - Test Agent Identity API
#
# Endpoint: GET /api/identity
# Expected: HTTP 200, returns PEM format certificate
#

set -e

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "=========================================="
echo "Test: GET /api/identity"
echo "Input: None"
echo "Expected: HTTP 200, PEM certificate string"
echo "=========================================="
echo ""

output=$(curl -s -w "\nHTTP_STATUS:%{http_code}" -H "Authorization: Bearer $API_TOKEN" "$AGENT_URL/api/identity")
http_status=$(echo "$output" | grep "HTTP_STATUS" | cut -d: -f2)
response_body=$(echo "$output" | sed '/HTTP_STATUS/d')

echo "HTTP status code: $http_status"
echo ""

if [ "$http_status" = "200" ]; then
    # Returns PUBLIC KEY before joining a mesh, CERTIFICATE after joining
    if echo "$response_body" | grep -q "BEGIN CERTIFICATE\|BEGIN PUBLIC KEY"; then
        echo -e "${GREEN}✓ PASS${NC}: GET /api/identity returns PEM data"
        echo "Header: $(echo "$response_body" | head -1)"
    else
        echo -e "${RED}✗ FAIL${NC}: Response is not PEM format"
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL${NC}: HTTP $http_status"
    exit 1
fi

exit 0