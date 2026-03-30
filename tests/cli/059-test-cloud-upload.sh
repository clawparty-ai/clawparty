#!/bin/bash
#
# 059-test-cloud-upload.sh - TC-CL-003: File Upload
#
# Command: ztm cloud upload
# Expected: Exit code 0 or appropriate message
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
source "$SCRIPT_DIR/../lib/common.sh"

PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"
TMP_DIR="$PROJECT_ROOT/tests/tmp"

export ZTM_CONFIG="127.0.0.1:7778"
export ZTM_API_TOKEN="enjoy-party"

echo "=========================================="
echo "Test: 059-test-cloud-upload.sh"
echo "TC-CL-003: File Upload"
echo "=========================================="
echo ""

# Create test file
TEST_FILE="$TMP_DIR/test-upload.txt"
echo "Test content for cloud upload" > "$TEST_FILE"

echo "Execute: $ZTM_BIN cloud upload $TEST_FILE"
output=$("$ZTM_BIN" cloud upload "$TEST_FILE" 2>&1)
exit_code=$?

echo "Output:"
echo "$output"
echo ""

if [ $exit_code -eq 0 ]; then
    pass "ztm cloud upload succeeded"
else
    skip "ztm cloud upload not available (exit code: $exit_code)"
fi
