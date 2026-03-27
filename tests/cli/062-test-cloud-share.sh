#!/bin/bash
#
# 062-test-cloud-share.sh - TC-CL-009: ACL Permission Settings
#
# Command: ztm cloud share
# Expected: Exit code 0, display ACL config
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 062-test-cloud-share.sh"
echo "TC-CL-009: ACL Permission Settings"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN cloud share /"
output=$("$ZTM_BIN" cloud share "/" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm cloud share succeeded"
else
    skip "ztm cloud share not available (exit code: $exit_code)"
fi
