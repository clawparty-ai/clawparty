#!/bin/bash
#
# run-tests.sh - Run Agent API tests
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

echo "=========================================="
echo "Agent API tests"
echo "URL: $AGENT_URL"
echo "Token: $API_TOKEN"
echo "=========================================="

run_test() {
    local name=$1
    local cmd=$2
    
    echo ""
    echo "Test: $name"
    if eval "$cmd"; then
        echo -e "${GREEN}✓ PASS${NC}: $name"
    else
        echo -e "${RED}✗ FAIL${NC}: $name"
    fi
}

# Agent Version
run_test "GET /api/version" \
    'curl -s -o /dev/null -w "%{http_code}" "$AGENT_URL/api/version" | grep -q "200"'

# Agent Identity
run_test "GET /api/identity" \
    'curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $API_TOKEN" "$AGENT_URL/api/identity" | grep -q "200"'

# Agent Meshes List
run_test "GET /api/meshes" \
    'curl -s -o /dev/null -w "%{http_code}" -H "Authorization: Bearer $API_TOKEN" "$AGENT_URL/api/meshes" | grep -q "200"'

# Agent OK
run_test "GET /ok" \
    'curl -s -o /dev/null -w "%{http_code}" "$AGENT_URL/ok" | grep -q "200"'

echo ""
echo "Agent API tests completed"
exit 0