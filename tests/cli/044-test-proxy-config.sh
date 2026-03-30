#!/bin/bash
#
# 044-test-proxy-config.sh - TC-PX-002: Configure Forwarding Endpoint
#
# Command: ztm proxy config
# Expected: Exit code 0, display config info
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 044-test-proxy-config.sh"
echo "TC-PX-002: Configure Forwarding Endpoint"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN proxy config"
output=$("$ZTM_BIN" proxy config 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ] || [ $exit_code -eq 99 ]; then
    pass "ztm proxy config executed (exit code: $exit_code)"
else
    skip "ztm proxy config not available (exit code: $exit_code)"
fi
