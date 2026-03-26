#!/bin/bash
#
# test-version.sh - Test ztm version command
#
# Command: ztm version
# Environment variables: None
# Expected output: Contains ZTM and Pipy version info
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "=========================================="
echo "Test: ztm version"
echo "Command: $ZTM_BIN version"
echo "Expected: Output contains version info"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN version"
output=$("$ZTM_BIN" version 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -ne 0 ]; then
    echo -e "${RED}✗ FAIL${NC}: Command exit code is non-zero"
    exit 1
fi

# Verify output contains required content
if echo "$output" | grep -qi "ZTM\|ztm"; then
    echo "✓ Contains ZTM version info"
else
    echo -e "${RED}✗ FAIL${NC}: Does not contain ZTM version"
    exit 1
fi

if echo "$output" | grep -qi "pipy"; then
    echo "✓ Contains Pipy version info"
else
    echo -e "${RED}✗ FAIL${NC}: Does not contain Pipy version"
    exit 1
fi

echo -e "${GREEN}✓ PASS${NC}: ztm version works correctly"
exit 0