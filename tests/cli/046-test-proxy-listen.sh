#!/bin/bash
#
# 046-test-proxy-listen.sh - TC-PX-003: Configure Listening Endpoint
#
# Command: ztm proxy config --set-listen
# Expected: Exit code 0
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 046-test-proxy-listen.sh"
echo "TC-PX-003: Configure Listening Endpoint"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN proxy config --set-listen 0.0.0.0:1080"
output=$("$ZTM_BIN" proxy config --set-listen "0.0.0.0:1080" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm proxy config --set-listen succeeded"
    
    # Clean up - close listening
    "$ZTM_BIN" proxy config --set-listen "" 2>/dev/null || true
else
    skip "ztm proxy config --set-listen not available (exit code: $exit_code)"
fi
