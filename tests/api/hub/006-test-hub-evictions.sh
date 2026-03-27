#!/bin/bash
#
# 006-test-hub-evictions.sh - Test Hub evictions API
#
# Endpoint: GET /api/evictions
# Access:   mTLS
# Expected: HTTP 200, JSON array
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../../lib/common.sh"

TMP_DIR="$(cd "$SCRIPT_DIR/../../.." && pwd)/tests/tmp"
HUB_URL="${HUB_URL:-https://127.0.0.1:18888}"
HUB_CA="${HUB_CA:-$TMP_DIR/certs/ca.pem}"
HUB_CERT="${HUB_CERT:-$TMP_DIR/certs/client.crt}"
HUB_KEY="${HUB_KEY:-$TMP_DIR/certs/client.key}"

echo "=========================================="
echo "Test: 006-test-hub-evictions.sh"
echo "Endpoint: GET $HUB_URL/api/evictions"
echo "Expected: HTTP 200, JSON array"
echo "=========================================="
echo ""

[ ! -f "$HUB_CERT" ] && fail "mTLS certificates not found: $HUB_CERT"

curl_api "GET /api/evictions" \
    --cert "$HUB_CERT" --key "$HUB_KEY" --cacert "$HUB_CA" --insecure \
    "$HUB_URL/api/evictions"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/evictions HTTP 200, valid JSON"
else
    fail "HTTP $CURL_STATUS (expected: 200)"
fi
