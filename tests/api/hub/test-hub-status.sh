#!/bin/bash
#
# test-hub-status.sh - Test Hub status API
#
# Endpoint: GET /api/status
# Access:   mTLS (Hub requires client certificate signed by Hub CA)
# Expected: HTTP 200, JSON with id, zone, since, ports, capacity, load
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TMP_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)/tests/tmp"

HUB_URL="${HUB_URL:-https://127.0.0.1:18888}"
HUB_CA="${HUB_CA:-$TMP_DIR/certs/ca.pem}"
HUB_CERT="${HUB_CERT:-$TMP_DIR/certs/client.crt}"
HUB_KEY="${HUB_KEY:-$TMP_DIR/certs/client.key}"

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

echo "=========================================="
echo "Test: GET /api/status"
echo "Input: mTLS client certificate"
echo "Expected: HTTP 200, JSON with hub status"
echo "=========================================="
echo ""

# Check certificates exist
if [ ! -f "$HUB_CERT" ] || [ ! -f "$HUB_KEY" ] || [ ! -f "$HUB_CA" ]; then
    echo -e "${RED}✗ FAIL${NC}: mTLS certificates not found"
    echo "  CA:   $HUB_CA"
    echo "  Cert: $HUB_CERT"
    echo "  Key:  $HUB_KEY"
    exit 1
fi

echo "Execute: curl --cert $HUB_CERT --key $HUB_KEY --cacert $HUB_CA $HUB_URL/api/status"
output=$(curl -s -w "\nHTTP_STATUS:%{http_code}" \
    --cert "$HUB_CERT" \
    --key "$HUB_KEY" \
    --cacert "$HUB_CA" --insecure \
    "$HUB_URL/api/status")

http_status=$(echo "$output" | grep "HTTP_STATUS" | cut -d: -f2)
response_body=$(echo "$output" | sed '/HTTP_STATUS/d')

echo ""
echo "Output:"
echo "$response_body"
echo ""
echo "HTTP status code: $http_status"
echo ""

if [ "$http_status" = "200" ]; then
    if echo "$response_body" | jq empty 2>/dev/null; then
        echo -e "${GREEN}✓ PASS${NC}: GET /api/status HTTP 200, valid JSON"
        echo "$response_body" | jq -r '. | "  id: \(.id // "n/a"), zone: \(.zone // "n/a")"' 2>/dev/null || true
    else
        echo -e "${RED}✗ FAIL${NC}: Invalid JSON response"
        exit 1
    fi
else
    echo -e "${RED}✗ FAIL${NC}: HTTP $http_status (expected: 200)"
    exit 1
fi

exit 0