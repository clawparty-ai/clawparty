#!/bin/bash
#
# run-tests.sh - Run Hub API tests
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TESTS_DIR="$SCRIPT_DIR"

# Configuration
HUB_URL="${HUB_URL:-http://localhost:18888}"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

test_passed() {
    echo -e "${GREEN}✓ PASS${NC}: $1"
}

test_failed() {
    echo -e "${RED}✗ FAIL${NC}: $1"
}

run_all_tests() {
    echo "Running Hub API tests..."
    
    # Test 1: GET /api/status
    log_info "Test 1: GET /api/status"
    output=$(curl -s -w "\n%{http_code}" "$HUB_URL/api/status")
    status=$(echo "$output" | tail -1)
    body=$(echo "$output" | sed '$d')
    
    if [ "$status" = "200" ]; then
        test_passed "GET /api/status - HTTP $status"
        echo "  Response: $body"
    else
        test_failed "GET /api/status - HTTP $status"
    fi

    # Test 2: GET /api/log
    log_info "Test 2: GET /api/log"
    output=$(curl -s -w "\n%{http_code}" "$HUB_URL/api/log")
    status=$(echo "$output" | tail -1)
    
    if [ "$status" = "200" ]; then
        test_passed "GET /api/log - HTTP $status"
    else
        test_failed "GET /api/log - HTTP $status"
    fi

    # Test 3: GET /api/zones
    log_info "Test 3: GET /api/zones"
    output=$(curl -s -w "\n%{http_code}" "$HUB_URL/api/zones")
    status=$(echo "$output" | tail -1)
    
    if [ "$status" = "200" ]; then
        test_passed "GET /api/zones - HTTP $status"
    else
        test_failed "GET /api/zones - HTTP $status"
    fi

    # Test 4: GET /api/endpoints
    log_info "Test 4: GET /api/endpoints"
    output=$(curl -s -w "\n%{http_code}" "$HUB_URL/api/endpoints")
    status=$(echo "$output" | tail -1)
    
    if [ "$status" = "200" ]; then
        test_passed "GET /api/endpoints - HTTP $status"
    else
        test_failed "GET /api/endpoints - HTTP $status"
    fi

    # Test 5: GET /api/users
    log_info "Test 5: GET /api/users"
    output=$(curl -s -w "\n%{http_code}" "$HUB_URL/api/users")
    status=$(echo "$output" | tail -1)
    
    if [ "$status" = "200" ]; then
        test_passed "GET /api/users - HTTP $status"
    else
        test_failed "GET /api/users - HTTP $status"
    fi

    echo ""
    echo "Hub API tests completed"
}

main() {
    run_all_tests
}

main "$@"