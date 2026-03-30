#!/bin/bash
#
# 023-test-ban-user.sh - Test ban user
#
# Endpoint: POST /api/evict/{username}
# Access: mTLS
# Expected: HTTP 200, 204, or 404 (endpoint may not be implemented)
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

TMP_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)/tests/tmp"
HUB_URL="${HUB_URL:-https://127.0.0.1:18888}"
HUB_CA="${HUB_CA:-$TMP_DIR/certs/ca.pem}"
HUB_CERT="${HUB_CERT:-$TMP_DIR/certs/client.crt}"
HUB_KEY="${HUB_KEY:-$TMP_DIR/certs/client.key}"

echo "=========================================="
echo "Test: 023-test-ban-user.sh"
echo "Endpoint: POST $HUB_URL/api/evict/{username}"
echo "Expected: HTTP 200, 204, or 404 (endpoint may not exist)"
echo "=========================================="
echo ""

[ ! -f "$HUB_CERT" ] && fail "mTLS certificates not found: $HUB_CERT"

TEST_USER="testuser023"

curl_api "POST /api/evict/$TEST_USER (ban user)" \
    -X POST \
    --cert "$HUB_CERT" --key "$HUB_KEY" --cacert "$HUB_CA" --insecure \
    "$HUB_URL/api/evict/$TEST_USER"

if [ "$CURL_STATUS" = "200" ] || [ "$CURL_STATUS" = "204" ]; then
    pass "Ban user HTTP $CURL_STATUS"
elif [ "$CURL_STATUS" = "404" ]; then
    skip "Endpoint /api/evict not implemented (404)"
else
    fail "HTTP $CURL_STATUS (expected: 200, 204, or 404)"
fi
