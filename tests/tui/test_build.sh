#!/bin/bash
# TUI Build Tests

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ZTM_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"
TUI_DIR="$ZTM_DIR/tui"
BIN_DIR="$ZTM_DIR/bin"

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

echo "=== TUI Build Tests ==="
echo ""

# Test 1: Cargo build succeeds
echo "Test: cargo build --release"
cd "$TUI_DIR"
if cargo build --release 2>&1 | grep -q "Finished"; then
    pass "cargo build --release succeeds"
else
    fail "cargo build --release failed"
fi

# Test 2: No compiler warnings
echo "Test: no compiler warnings"
WARNINGS=$(cargo build --release 2>&1 | grep -c "warning:" || true)
if [ "$WARNINGS" -eq 0 ]; then
    pass "zero compiler warnings"
else
    fail "$WARNINGS compiler warning(s)"
fi

# Test 3: Binary exists and is executable
echo "Test: binary exists"
if [ -x "$BIN_DIR/clawparty" ]; then
    pass "bin/clawparty exists and is executable"
else
    fail "bin/clawparty not found or not executable"
fi

# Test 4: Binary is properly signed (macOS)
echo "Test: binary signing"
if [ "$(uname)" = "Darwin" ]; then
    if codesign -v "$BIN_DIR/clawparty" 2>/dev/null; then
        pass "binary is properly signed"
    else
        fail "binary is not signed"
    fi
else
    pass "skipped (not macOS)"
fi

# Test 5: Incremental build
echo "Test: incremental build"
cd "$TUI_DIR"
BUILD_OUTPUT=$(cargo build --release 2>&1)
if echo "$BUILD_OUTPUT" | grep -q "Finished"; then
    if echo "$BUILD_OUTPUT" | grep -q "Compiling clawparty"; then
        pass "incremental build (recompiled)"
    else
        pass "incremental build (cached)"
    fi
else
    fail "incremental build failed"
fi

# Test 6: Clean build
echo "Test: clean build flag"
cd "$ZTM_DIR"
if [ -f build.sh ]; then
    # Just verify the flag is accepted
    if grep -q "\-\-clean" build.sh; then
        pass "build.sh supports --clean flag"
    else
        fail "build.sh missing --clean flag"
    fi
else
    fail "build.sh not found"
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
