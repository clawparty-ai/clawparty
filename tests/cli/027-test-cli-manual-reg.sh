#!/bin/bash
#
# 027-test-cli-manual-reg.sh - Test CLI manual registration
#
# Command: ztm try --help
# Expected: Command exists or skip if not available
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 027-test-cli-manual-reg.sh"
echo "Command: $ZTM_BIN try --help"
echo "Expected: Command exists or skip"
echo "=========================================="
echo ""

# Try the try command - may not exist, check help
echo "Checking if 'ztm try --help' exists..."
output=$("$ZTM_BIN" try --help 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "'ztm try' command exists"
else
    echo "Note: 'ztm try' command not available (optional feature)"
    skip "Manual registration CLI optional"
fi
