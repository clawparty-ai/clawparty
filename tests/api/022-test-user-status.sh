#!/bin/bash
#
# 022-test-user-status.sh - Test user status query
#
# Endpoint: GET /api/user-log
# Access: mTLS
# Expected: HTTP 200 (root) or 403 (non-root access)
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

TMP_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)/tests/tmp"
HUB_URL="${HUB_URL:-https://127.0.0.1:18888}"
HUB_CA="${HUB_CA:-$TMP_DIR/certs/ca.pem}"
HUB_CERT="${HUB_CERT:-$TMP_DIR/certs/client.crt}"
HUB_KEY="${HUB_KEY:-$TMP_DIR/certs/client.key}"

echo "=========================================="
echo "Test: 022-test-user-status.sh"
echo "Endpoint: GET $HUB_URL/api/user-log"
echo "Expected: HTTP 200 (root) or 403 (non-root)"
echo "=========================================="
echo ""

[ ! -f "$HUB_CERT" ] && fail "mTLS certificates not found: $HUB_CERT"

curl_api "GET /api/user-log" \
    --cert "$HUB_CERT" --key "$HUB_KEY" --cacert "$HUB_CA" --insecure \
    "$HUB_URL/api/user-log"

if [ "$CURL_STATUS" = "200" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    pass "GET /api/user-log HTTP 200, valid JSON"
elif [ "$CURL_STATUS" = "403" ]; then
    pass "GET /api/user-log HTTP 403 (requires root access)"
else
    fail "HTTP $CURL_STATUS (expected: 200 or 403)"
fi
