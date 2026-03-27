#!/bin/bash
#
# 001-test-hub-status.sh - Test Hub status API
#
# Endpoint: GET /api/status
# Access:   mTLS
# Expected: HTTP 200, JSON with id, zone, since, ports, capacity, load
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../lib/common.sh"

TMP_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)/tests/tmp"
HUB_URL="${HUB_URL:-https://127.0.0.1:18888}"
HUB_CA="${HUB_CA:-$TMP_DIR/certs/ca.pem}"
HUB_CERT="${HUB_CERT:-$TMP_DIR/certs/client.crt}"
HUB_KEY="${HUB_KEY:-$TMP_DIR/certs/client.key}"

echo "=========================================="
echo "Test: 001-test-hub-status.sh"
echo "Endpoint: GET $HUB_URL/api/status"
echo "Expected: HTTP 200, JSON with hub status"
echo "=========================================="
echo ""

[ ! -f "$HUB_CERT" ] && fail "mTLS certificates not found: $HUB_CERT"

curl_api "GET /api/status" \
    --cert "$HUB_CERT" --key "$HUB_KEY" --cacert "$HUB_CA" --insecure \
    "$HUB_URL/api/status"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/status HTTP 200, valid JSON"
    echo "$CURL_BODY" | jq -r '"  id: \(.id // "n/a"), zone: \(.zone // "n/a")"' 2>/dev/null || true
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi