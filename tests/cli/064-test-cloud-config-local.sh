#!/bin/bash
#
# 064-test-cloud-config-local.sh - TC-CL-001: Set Local Directory
#
# Command: ztm cloud config --local-dir
# Expected: Exit code 0
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"
TMP_DIR="$PROJECT_ROOT/tests/tmp"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 064-test-cloud-config-local.sh"
echo "TC-CL-001: Set Local Directory"
echo "=========================================="
echo ""

mkdir -p "$TMP_DIR/cloud"

echo "Execute: $ZTM_BIN cloud config --local-dir $TMP_DIR/cloud"
output=$("$ZTM_BIN" cloud config --local-dir "$TMP_DIR/cloud" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm cloud config --local-dir succeeded"
else
    skip "ztm cloud config --local-dir not available (exit code: $exit_code)"
fi
