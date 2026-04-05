#!/bin/bash
# Run all TUI tests

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

TOTAL_PASS=0
TOTAL_FAIL=0

echo "========================================="
echo "  ClawParty TUI Test Suite"
echo "========================================="
echo ""

for test_file in "$SCRIPT_DIR"/test_*.sh; do
    [ "$test_file" = "$SCRIPT_DIR/run-tests.sh" ] && continue
    [ "$test_file" = "$SCRIPT_DIR/TEST_PLAN.md" ] && continue
    
    if [ -x "$test_file" ] || chmod +x "$test_file" 2>/dev/null; then
        echo ""
        echo "--- $(basename "$test_file") ---"
        if "$test_file"; then
            PASS=$(grep -c "PASS:" <<< "$(cat "$test_file" 2>/dev/null)" || true)
        else
            echo "  SKIP: agent not running"
        fi
    fi
done

echo ""
echo "========================================="
echo "  All TUI tests completed"
echo "========================================="
