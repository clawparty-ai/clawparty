#!/bin/bash
#
# 025-test-api-log.sh - Test API log recording
#
# Endpoint: GET /api/log
# Access: mTLS (root user only)
# Expected: HTTP 200 (root) or 403 (non-root)
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

TMP_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)/tests/tmp"
HUB_URL="${HUB_URL:-https://127.0.0.1:18888}"
HUB_CA="${HUB_CA:-$TMP_DIR/certs/ca.pem}"
HUB_CERT="${HUB_CERT:-$TMP_DIR/certs/client.crt}"
HUB_KEY="${HUB_KEY:-$TMP_DIR/certs/client.key}"

echo "=========================================="
echo "Test: 025-test-api-log.sh"
echo "Endpoint: GET $HUB_URL/api/log"
echo "Expected: HTTP 200 (root) or 403 (non-root)"
echo "=========================================="
echo ""

[ ! -f "$HUB_CERT" ] && fail "mTLS certificates not found: $HUB_CERT"

curl_api "GET /api/log" \
    --cert "$HUB_CERT" --key "$HUB_KEY" --cacert "$HUB_CA" --insecure \
    "$HUB_URL/api/log"

# /api/log returns 403 for non-root users, this is expected behavior
if [ "$CURL_STATUS" = "403" ]; then
    pass "GET /api/log HTTP 403 (requires root access - expected)"
elif [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/log HTTP 200 (root user)"
else
    fail "HTTP $CURL_STATUS (expected: 200 or 403)"
fi
