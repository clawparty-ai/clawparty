#!/bin/bash
#
# lib/common.sh - Shared helper functions for all test scripts
#
# Usage: source "$(dirname "$0")/../../lib/common.sh"
#        or   source "$(dirname "$0")/../lib/common.sh"
#

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() { echo -e "${GREEN}✓ PASS${NC}: $*"; }
fail() { echo -e "${RED}✗ FAIL${NC}: $*"; exit 1; }
skip() { echo -e "${YELLOW}⊘ SKIP${NC}: $*"; exit 99; }

# curl_api <description> <curl args...>
# Runs curl with -i, prints full command + response, returns HTTP status code in CURL_STATUS
curl_api() {
    local desc="$1"; shift
    
    echo "--------------------------------------------"
    echo "Request: $desc"
    echo "Command: curl -i $*"
    echo ""
    
    local response
    response=$(curl -i -s "$@" 2>&1)
    
    echo "Response:"
    echo "$response"
    echo "--------------------------------------------"
    echo ""
    
    # Extract HTTP status from response headers (first HTTP/ line)
    CURL_STATUS=$(echo "$response" | grep -m1 "^HTTP/" | awk '{print $2}')
    CURL_BODY=$(echo "$response" | awk 'BEGIN{body=0} /^\r?$/{body=1; next} body{print}')
}