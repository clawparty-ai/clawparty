#!/bin/bash
#
# 018-test-reg-missing-username.sh - Test registration without UserName
#
# Endpoint: POST /invite
# Input: {PublicKey, EpName, PassKey} - missing UserName
# Expected: HTTP 400 "missing UserName"
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

REG_PORT="${REG_PORT:-5678}"

echo "=========================================="
echo "Test: 018-test-reg-missing-username.sh"
echo "Endpoint: POST http://127.0.0.1:$REG_PORT/invite"
echo "Expected: HTTP 400 (missing UserName)"
echo "=========================================="
echo ""

# Use a fixed public key for testing
json_payload='{"PublicKey":"-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAuIw/G0UWqHNBivP4htroyA16q1ap2zBCBImDASFRwWh/Ni1h5etxumrP9o2N1KXS1oyOKy4wq4Zoh9oi/ohODUBwQ2sydqVXYFTWaeh3hyNac4NIYA9NrLBZlRpFHsnze7N4aDWNq8LvqqeDnFAymRArWBWvTauJZg7l7PGCjGrd6WycCcWV49uC7FVCd9Bhpp6j9QSvD16zPl0ogh/6RripfiVSpuZXTTHT6IruFJ8pL+0h55cIH4SnII+HU1cHbKiQ0SYZAHWf0Jybkz23UtJsk7NDZSrUURtqtSE7ns2aphxZZPtMZUGLIzJztkrfdBIr2T/32jhQ4b7jR5sBPQIDAQAB\n-----END PUBLIC KEY-----","EpName":"test-ep","PassKey":"testpass"}'

curl_api "POST /invite (missing UserName)" \
    -X POST \
    -H "Content-Type: application/json" \
    -d "$json_payload" \
    "http://127.0.0.1:$REG_PORT/invite"

if [ "$CURL_STATUS" = "400" ]; then
    pass "Returns 400 for missing UserName"
else
    fail "HTTP $CURL_STATUS (expected: 400)"
fi
