#!/bin/bash
# TUI API Tests

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ZTM_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

API_HOST="${API_HOST:-http://localhost:6789}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

PASS=0
FAIL=0

pass() {
    echo "  PASS: $1"
    PASS=$((PASS + 1))
}

fail() {
    echo "  FAIL: $1"
    FAIL=$((FAIL + 1))
}

echo "=== TUI API Tests ==="
echo "API Host: $API_HOST"
echo ""

# Test 1: Health check
echo "Test: health check"
RESP=$(curl -s -o /dev/null -w "%{http_code}" "$API_HOST/ok" 2>/dev/null || echo "000")
if [ "$RESP" = "200" ]; then
    pass "health check returns 200"
else
    fail "health check returned $RESP (agent may not be running)"
fi

# Test 2: Get meshes
echo "Test: get meshes"
RESP=$(curl -s -H "Authorization: Bearer $API_TOKEN" "$API_HOST/api/meshes" 2>/dev/null)
if [ -n "$RESP" ]; then
    pass "get meshes returns data"
else
    fail "get meshes returned empty response"
fi

# Test 3: Get openclaw agents
echo "Test: get openclaw agents"
RESP=$(curl -s -H "Authorization: Bearer $API_TOKEN" "$API_HOST/api/openclaw/agents" 2>/dev/null)
if echo "$RESP" | grep -q '"id"'; then
    pass "get openclaw agents returns agent data"
else
    fail "get openclaw agents returned no agents"
fi

# Test 4: Get chats
echo "Test: get chats"
MESH=$(curl -s -H "Authorization: Bearer $API_TOKEN" "$API_HOST/api/meshes" 2>/dev/null | grep -o '"name":"[^"]*"' | head -1 | cut -d'"' -f4)
if [ -n "$MESH" ]; then
    RESP=$(curl -s -H "Authorization: Bearer $API_TOKEN" "$API_HOST/api/meshes/$MESH/apps/ztm/chat/api/chats" 2>/dev/null)
    if [ -n "$RESP" ]; then
        pass "get chats returns data"
    else
        fail "get chats returned empty response"
    fi
else
    echo "  SKIP: no mesh configured"
fi

# Test 5: Get endpoints
echo "Test: get endpoints"
if [ -n "$MESH" ]; then
    RESP=$(curl -s -H "Authorization: Bearer $API_TOKEN" "$API_HOST/api/meshes/$MESH/endpoints?limit=500" 2>/dev/null)
    if [ -n "$RESP" ]; then
        pass "get endpoints returns data"
    else
        fail "get endpoints returned empty response"
    fi
else
    echo "  SKIP: no mesh configured"
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
