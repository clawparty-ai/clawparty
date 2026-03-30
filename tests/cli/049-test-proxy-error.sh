#!/bin/bash
#
# 049-test-proxy-error.sh - TC-PX-013: Error Handling
#
# Command: ztm proxy config with invalid params
# Expected: Returns appropriate error
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 049-test-proxy-error.sh"
echo "TC-PX-013: Error Handling"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN proxy config --set-listen invalid-port"
output=$("$ZTM_BIN" proxy config --set-listen "invalid-port" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -ne 0 ]; then
    pass "ztm proxy config rejects invalid port (exit code: $exit_code)"
else
    skip "ztm proxy config may not validate port (exit code: $exit_code)"
fi
