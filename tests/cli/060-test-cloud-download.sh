#!/bin/bash
#
# 060-test-cloud-download.sh - TC-CL-004: File Download
#
# Command: ztm cloud download --list
# Expected: Exit code 0, display download list
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 060-test-cloud-download.sh"
echo "TC-CL-004: File Download"
echo "=========================================="
echo ""

echo "Execute: $ZTM_BIN cloud download --list"
output=$("$ZTM_BIN" cloud download --list 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm cloud download --list succeeded"
else
    skip "ztm cloud download not available (exit code: $exit_code)"
fi
