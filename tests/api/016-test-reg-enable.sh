#!/bin/bash
#
# 016-test-reg-enable.sh - Test registration API is enabled
#
# Endpoint: GET http://127.0.0.1:5678/
# Expected: HTTP response from registration endpoint
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

REG_PORT="${REG_PORT:-5678}"

echo "=========================================="
echo "Test: 016-test-reg-enable.sh"
echo "Endpoint: GET http://127.0.0.1:$REG_PORT/"
echo "Expected: HTTP response from registration endpoint"
echo "=========================================="
echo ""

curl_api "GET / (registration API)" \
    "http://127.0.0.1:$REG_PORT/"

if [ -n "$CURL_STATUS" ]; then
    pass "Registration API is accessible (HTTP $CURL_STATUS)"
else
    fail "Registration API not accessible"
fi
