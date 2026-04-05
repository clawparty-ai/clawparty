#!/bin/bash
# Picoclaw API Tests

set -e

CLAWPARTY_URL="${CLAWPARTY_URL:-http://localhost:6789}"
API_TOKEN="${API_TOKEN:-enjoy-party}"
PASS=0
FAIL=0

echo "=== Picoclaw API Tests ==="
echo ""

# Test 1: Health check
echo "Test 1: Health check endpoint"
RESPONSE=$(curl -s -w "\n%{http_code}" "$CLAWPARTY_URL/api/picoclaw/health" \
  -H "X-ZTM-Token: $API_TOKEN")

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)
BODY=$(echo "$RESPONSE" | head -n-1)

if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "503" ]; then
  echo "  ✓ PASS (HTTP $HTTP_CODE)"
  PASS=$((PASS + 1))
else
  echo "  ✗ FAIL (HTTP $HTTP_CODE, expected 200 or 503)"
  FAIL=$((FAIL + 1))
fi

# Test 2: Chat without message
echo ""
echo "Test 2: Chat without message (should return 400)"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$CLAWPARTY_URL/api/picoclaw/chat" \
  -H "Content-Type: application/json" \
  -H "X-ZTM-Token: $API_TOKEN" \
  -d '{}')

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)

if [ "$HTTP_CODE" = "400" ]; then
  echo "  ✓ PASS (HTTP $HTTP_CODE)"
  PASS=$((PASS + 1))
else
  echo "  ✗ FAIL (HTTP $HTTP_CODE, expected 400)"
  FAIL=$((FAIL + 1))
fi

# Test 3: Chat with message
echo ""
echo "Test 3: Chat with valid message"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$CLAWPARTY_URL/api/picoclaw/chat" \
  -H "Content-Type: application/json" \
  -H "X-ZTM-Token: $API_TOKEN" \
  -d '{"message": "Hello"}')

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)

if [ "$HTTP_CODE" = "200" ] || [ "$HTTP_CODE" = "503" ]; then
  echo "  ✓ PASS (HTTP $HTTP_CODE)"
  PASS=$((PASS + 1))
else
  echo "  ✗ FAIL (HTTP $HTTP_CODE, expected 200 or 503)"
  FAIL=$((FAIL + 1))
fi

# Test 4: Invalid JSON
echo ""
echo "Test 4: Invalid JSON (should return 400)"
RESPONSE=$(curl -s -w "\n%{http_code}" -X POST "$CLAWPARTY_URL/api/picoclaw/chat" \
  -H "Content-Type: application/json" \
  -H "X-ZTM-Token: $API_TOKEN" \
  -d 'not json')

HTTP_CODE=$(echo "$RESPONSE" | tail -n1)

if [ "$HTTP_CODE" = "400" ]; then
  echo "  ✓ PASS (HTTP $HTTP_CODE)"
  PASS=$((PASS + 1))
else
  echo "  ✗ FAIL (HTTP $HTTP_CODE, expected 400)"
  FAIL=$((FAIL + 1))
fi

echo ""
echo "=== Results ==="
echo "Passed: $PASS"
echo "Failed: $FAIL"
echo ""

if [ $FAIL -eq 0 ]; then
  echo "All tests passed!"
  exit 0
else
  echo "Some tests failed!"
  exit 1
fi