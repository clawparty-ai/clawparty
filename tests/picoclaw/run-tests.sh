#!/bin/bash
# Test script for Picoclaw integration

CLAWPARTY_URL="${CLAWPARTY_URL:-http://localhost:6789}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "=== Testing Picoclaw Integration ==="
echo "URL: $CLAWPARTY_URL"

echo -e "\n1. Health Check:"
curl -s "$CLAWPARTY_URL/api/picoclaw/health" \
  -H "X-ZTM-Token: $API_TOKEN" | jq . || echo "(jq not available, raw output above)"

echo -e "\n2. Send Test Message:"
curl -s -X POST "$CLAWPARTY_URL/api/picoclaw/chat" \
  -H "Content-Type: application/json" \
  -H "X-ZTM-Token: $API_TOKEN" \
  -d '{"message": "Hello, I am testing the ClawParty integration. Please respond with a short greeting.", "session_id": "test-session"}' | jq . || echo "(jq not available, raw output above)"

echo -e "\n=== Test Complete ==="