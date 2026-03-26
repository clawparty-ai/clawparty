#!/bin/bash
#
# test-hub-zones.sh - Test Hub zones API
#
# Endpoint: GET /api/zones
# Access:   mTLS
# Expected: HTTP 200, JSON array
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
echo "Test: GET /api/zones"
echo "Input: mTLS client certificate"
echo "Expected: HTTP 200"
echo "=========================================="
echo ""

[ ! -f "$HUB_CERT" ] && echo -e "${RED}✗ FAIL${NC}: mTLS certificates not found" && exit 1

output=$(curl -s -w "\nHTTP_STATUS:%{http_code}" \
    --cert "$HUB_CERT" --key "$HUB_KEY" --cacert "$HUB_CA" --insecure \
    "$HUB_URL/api/zones")

http_status=$(echo "$output" | grep "HTTP_STATUS" | cut -d: -f2)
response_body=$(echo "$output" | sed '/HTTP_STATUS/d')

echo "Output: $response_body"
echo "HTTP status code: $http_status"

if [ "$http_status" = "200" ]; then
    echo -e "${GREEN}✓ PASS${NC}: GET /api/zones HTTP 200"
else
    echo -e "${RED}✗ FAIL${NC}: HTTP $http_status (expected: 200)"
    exit 1
fi

exit 0