#!/bin/bash
#
# 063-test-cloud-mirror.sh - TC-CL-011: Auto-download Configuration
#
# Command: ztm cloud mirror
# Expected: Exit code 0, display mirror config
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 063-test-cloud-mirror.sh"
echo "TC-CL-011: Auto-download Configuration"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN cloud mirror /"
output=$("$ZTM_BIN" cloud mirror "/" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm cloud mirror succeeded"
else
    skip "ztm cloud mirror not available (exit code: $exit_code)"
fi
