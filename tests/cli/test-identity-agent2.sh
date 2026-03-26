#!/bin/bash
#
# test-identity-agent2.sh - Test Agent2 identity command
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
ZTM_BIN="$PROJECT_ROOT/bin/ztm"

export ZTM_CONFIG="127.0.0.1:7779"
export ZTM_API_TOKEN="enjoy-party"

echo "Test: ztm identity (Agent2)"

output=$("$ZTM_BIN" identity 2>&1)

if echo "$output" | grep -q "BEGIN CERTIFICATE\|BEGIN PUBLIC KEY"; then
    echo "✓ PASS: Agent2 identity returns certificate"
else
    echo "✗ FAIL"
    exit 1
fi
exit 0