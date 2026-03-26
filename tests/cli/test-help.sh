#!/bin/bash
#
# test-help.sh - Test ztm help command
#
# Command: ztm help
# Environment variables: None
# Expected output: Contains "Usage:", "Commands:" help info
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "=========================================="
echo "Test: ztm help"
echo "Command: $ZTM_BIN help"
echo "Expected: Output contains Usage: and Commands:"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN help"
output=$("$ZTM_BIN" help 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

# Check exit code
if [ $exit_code -ne 0 ]; then
    echo -e "${RED}✗ FAIL${NC}: Command exit code is non-zero (exit code: $exit_code)"
    exit 1
fi

# Verify output contains required content
if echo "$output" | grep -q "Usage:"; then
    echo "✓ Contains 'Usage:'"
else
    echo -e "${RED}✗ FAIL${NC}: Does not contain 'Usage:'"
    exit 1
fi

if echo "$output" | grep -q "Commands:"; then
    echo "✓ Contains 'Commands:'"
else
    echo -e "${RED}✗ FAIL${NC}: Does not contain 'Commands:'"
    exit 1
fi

echo -e "${GREEN}✓ PASS${NC}: ztm help works correctly"
exit 0