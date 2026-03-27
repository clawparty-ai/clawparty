#!/bin/bash
#
# 021-test-reg-duplicate.sh - Test duplicate user registration
#
# Endpoint: POST /invite
# Input: Same user registered twice
# Expected: HTTP 201 or 409
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

REG_PORT="${REG_PORT:-5678}"

echo "=========================================="
echo "Test: 021-test-reg-duplicate.sh"
echo "Endpoint: POST http://127.0.0.1:$REG_PORT/invite"
echo "Expected: HTTP 201 or 409 (duplicate handling)"
echo "=========================================="
echo ""

json_payload='{"PublicKey":"-----BEGIN PUBLIC KEY-----\nMIIBIjANBgkqhkiG9w0BAQEFAAOCAQ8AMIIBCgKCAQEAuIw/G0UWqHNBivP4htroyA16q1ap2zBCBImDASFRwWh/Ni1h5etxumrP9o2N1KXS1oyOKy4wq4Zoh9oi/ohODUBwQ2sydqVXYFTWaeh3hyNac4NIYA9NrLBZlRpFHsnze7N4aDWNq8LvqqeDnFAymRArWBWvTauJZg7l7PGCjGrd6WycCcWV49uC7FVCd9Bhpp6j9QSvD16zPl0ogh/6RripfiVSpuZXTTHT6IruFJ8pL+0h55cIH4SnII+HU1cHbKiQ0SYZAHWf0Jybkz23UtJsk7NDZSrUURtqtSE7ns2aphxZZPtMZUGLIzJztkrfdBIr2T/32jhQ4b7jR5sBPQIDAQAB\n-----END PUBLIC KEY-----","UserName":"duplicate021","EpName":"duplicate021-ep","PassKey":"samepass021"}'

# First registration
curl_api "POST /invite (first registration)" \
    -X POST \
    -H "Content-Type: application/json" \
    -d "$json_payload" \
    "http://127.0.0.1:$REG_PORT/invite"

first_status="$CURL_STATUS"
echo "First registration HTTP: $first_status"

# Second registration (duplicate)
curl_api "POST /invite (duplicate registration)" \
    -X POST \
    -H "Content-Type: application/json" \
    -d "$json_payload" \
    "http://127.0.0.1:$REG_PORT/invite"

second_status="$CURL_STATUS"
echo "Second registration HTTP: $second_status"

if [ "$second_status" = "201" ] || [ "$second_status" = "409" ]; then
    pass "Duplicate handling works (HTTP $second_status)"
else
    fail "HTTP $second_status (expected: 201 or 409)"
fi
