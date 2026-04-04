#!/bin/bash
# TUI Startup Tests

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ZTM_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
BIN_DIR="$ZTM_DIR/bin"

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

echo "=== TUI Startup Tests ==="
echo ""

# Test 1: TUI fails without TTY
echo "Test: TUI fails without TTY"
OUTPUT=$(echo "" | "$BIN_DIR/clawparty" 2>&1 || true)
if echo "$OUTPUT" | grep -qi "TTY"; then
    pass "TUI correctly rejects non-TTY input"
else
    fail "TUI did not reject non-TTY input"
fi

# Test 2: TUI --help works
echo "Test: TUI --help"
OUTPUT=$("$BIN_DIR/clawparty" --help 2>&1 || true)
if echo "$OUTPUT" | grep -qi "Terminal UI for ClawParty"; then
    pass "TUI --help shows correct description"
else
    fail "TUI --help output unexpected"
fi

# Test 3: TUI accepts --api-host argument
echo "Test: TUI --api-host argument"
OUTPUT=$("$BIN_DIR/clawparty" --help 2>&1 || true)
if echo "$OUTPUT" | grep -q "\-\-api-host"; then
    pass "TUI accepts --api-host argument"
else
    fail "TUI missing --api-host argument"
fi

# Test 4: TUI accepts --token argument
echo "Test: TUI --token argument"
OUTPUT=$("$BIN_DIR/clawparty" --help 2>&1 || true)
if echo "$OUTPUT" | grep -q "\-\-token"; then
    pass "TUI accepts --token argument"
else
    fail "TUI missing --token argument"
fi

# Test 5: TUI accepts --pipy-bin argument
echo "Test: TUI --pipy-bin argument"
OUTPUT=$("$BIN_DIR/clawparty" --help 2>&1 || true)
if echo "$OUTPUT" | grep -q "\-\-pipy-bin"; then
    pass "TUI accepts --pipy-bin argument"
else
    fail "TUI missing --pipy-bin argument"
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
