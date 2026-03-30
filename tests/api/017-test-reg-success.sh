#!/bin/bash
#
# 017-test-reg-success.sh - Test successful user registration
#
# Endpoint: POST /invite
# Input: {PublicKey, UserName, EpName, PassKey}
# Expected: HTTP 201, JSON with UserName and EpName
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"
REG_PORT="${REG_PORT:-5678}"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 017-test-reg-success.sh"
echo "Endpoint: POST http://127.0.0.1:$REG_PORT/invite"
echo "Expected: HTTP 201, JSON with UserName and EpName"
echo "=========================================="
echo ""

echo "Getting public key from agent..."
pubkey=$("$ZTM_BIN" identity 2>/dev/null)

if [ -z "$pubkey" ]; then
    fail "Cannot get public key from agent"
fi

# Build JSON using jq to properly escape the public key
json_payload=$(jq -n \
    --arg pubkey "$pubkey" \
    '{
        "PublicKey": $pubkey,
        "UserName": "testuser017",
        "EpName": "testuser017-ep",
        "PassKey": "testpass017"
    }')

curl_api "POST /invite (user registration)" \
    -X POST \
    -H "Content-Type: application/json" \
    -d "$json_payload" \
    "http://127.0.0.1:$REG_PORT/invite"

if [ "$CURL_STATUS" = "201" ]; then
    echo "$CURL_BODY" | jq empty 2>/dev/null || fail "Invalid JSON response"
    
    if echo "$CURL_BODY" | jq -e '.UserName' > /dev/null 2>&1; then
        username=$(echo "$CURL_BODY" | jq -r '.UserName')
        epname=$(echo "$CURL_BODY" | jq -r '.EpName')
        echo "  UserName: $username"
        echo "  EpName: $epname"
        pass "Registration successful"
    else
        fail "Response missing UserName"
    fi
else
    fail "HTTP $CURL_STATUS (expected: 201)"
fi
