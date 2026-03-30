#!/bin/bash
#
# 008-test-agent-identity.sh - Test Agent Identity API
#
# Endpoint: GET /api/identity
# Expected: HTTP 200, returns PEM format certificate or public key
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../lib/common.sh"

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=========================================="
echo "Test: 008-test-agent-identity.sh"
echo "Endpoint: GET $AGENT_URL/api/identity"
echo "Expected: HTTP 200, PEM certificate or public key"
echo "=========================================="
echo ""

curl_api "GET /api/identity" \
    -H "Authorization: Bearer $API_TOKEN" \
    "$AGENT_URL/api/identity"

if [ "$CURL_STATUS" = "200" ]; then
    # Returns PUBLIC KEY before joining a mesh, CERTIFICATE after joining
    if echo "$CURL_BODY" | grep -q "BEGIN CERTIFICATE\|BEGIN PUBLIC KEY"; then
        pass "GET /api/identity returns PEM data"
        echo "  Header: $(echo "$CURL_BODY" | head -1)"
    else
        fail "Response is not PEM format"
    fi
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
