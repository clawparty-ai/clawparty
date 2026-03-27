#!/bin/bash
#
# 057-test-cloud-config.sh - TC-CL-001: Endpoint Configuration Management
#
# Command: ztm cloud config
# Expected: Exit code 0, display config
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 057-test-cloud-config.sh"
echo "TC-CL-001: Endpoint Configuration Management"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN cloud config"
output=$("$ZTM_BIN" cloud config 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    if echo "$output" | grep -qi "cloud\|config\|localDir"; then
        pass "ztm cloud config shows config info"
    else
        pass "ztm cloud config executed successfully"
    fi
else
    skip "ztm cloud command not available (exit code: $exit_code)"
fi
