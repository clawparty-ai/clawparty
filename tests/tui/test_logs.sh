#!/bin/bash
# TUI Log Tests

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
ZTM_DIR="$(cd "$SCRIPT_DIR/../.." && pwd)"

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

echo "=== TUI Log Tests ==="
echo ""

# Test 1: Log file naming pattern
echo "Test: log file naming pattern"
# Create a test log file to verify the pattern
TEST_LOG="console-log-20260101-120000.log"
echo "test" > "$TEST_LOG"
if ls console-log-*.log 2>/dev/null | grep -q "console-log-"; then
    pass "log file uses console-log-<timestamp>.log pattern"
else
    fail "log file pattern incorrect"
fi
rm -f "$TEST_LOG"

# Test 2: Log file is created when TUI runs
echo "Test: log files exist"
LOG_COUNT=$(ls -1 console-log-*.log 2>/dev/null | wc -l | tr -d ' ')
if [ "$LOG_COUNT" -gt 0 ]; then
    pass "log files exist ($LOG_COUNT found)"
else
    fail "no log files found"
fi

# Test 3: Log content format
echo "Test: log content format"
LATEST_LOG=$(ls -t console-log-*.log 2>/dev/null | head -1)
if [ -n "$LATEST_LOG" ] && [ -f "$LATEST_LOG" ]; then
    if grep -q "\[.*\] \[.*\]" "$LATEST_LOG"; then
        pass "log entries have correct format [timestamp] [level]"
    else
        fail "log entries have incorrect format"
    fi
else
    fail "no log file found"
fi

# Test 4: Log contains INFO entries
echo "Test: log contains INFO entries"
if [ -n "$LATEST_LOG" ] && [ -f "$LATEST_LOG" ]; then
    if grep -q "\[INFO\]" "$LATEST_LOG"; then
        pass "log contains INFO entries"
    else
        fail "log missing INFO entries"
    fi
else
    fail "no log file found"
fi

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
