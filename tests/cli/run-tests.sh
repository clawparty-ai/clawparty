#!/bin/bash
#
# run-tests.sh - Run CLI tests
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

# Test files
test_files=(
    "test-help.sh"
    "test-version.sh"
    "test-version-json.sh"
    "test-identity-agent1.sh"
    "test-identity-agent2.sh"
)

echo "Running CLI tests..."

cd "$SCRIPT_DIR"

passed=0
failed=0

for test_file in "${test_files[@]}"; do
    if [ -f "$test_file" ]; then
        echo ""
        echo "=== $test_file ==="
        if bash "$test_file"; then
            ((passed++))
        else
            ((failed++))
        fi
    fi
done

echo ""
echo "CLI test summary: passed=$passed, failed=$failed"

[ $failed -gt 0 ] && exit 1 || exit 0