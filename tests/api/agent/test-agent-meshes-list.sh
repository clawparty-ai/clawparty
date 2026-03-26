#!/bin/bash
#
# test-agent-meshes-list.sh - Test get mesh list
#

set -e

AGENT_URL="${AGENT_URL:-http://localhost:7778}"
API_TOKEN="${API_TOKEN:-enjoy-party}"

echo "Test: GET /api/meshes (list all meshes)"

output=$(curl -s -w "\n%{http_code}" -H "Authorization: Bearer $API_TOKEN" "$AGENT_URL/api/meshes")
status=$(echo "$output" | tail -1)
body=$(echo "$output" | sed '$d')

echo "HTTP: $status"
echo "Response: $body"

[ "$status" = "200" ] && echo "✓ PASS" || { echo "✗ FAIL"; exit 1; }
exit 0