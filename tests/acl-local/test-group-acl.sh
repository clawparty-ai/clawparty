#!/bin/bash
#
# Verify group visibility is driven by ACL at the hub:
#   1. Group with alice only        -> bob does NOT see it
#   2. Group with alice + bob       -> bob DOES see it
#   3. Remove bob from the group    -> bob (after restart) no longer sees it
#
# Prerequisite: ./setup.sh has been run (two agents on :7781, :7782).
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TMP="$SCRIPT_DIR/tmp"

ALICE="http://127.0.0.1:7781"
BOB="http://127.0.0.1:7782"
MESH=clawparty
TOKEN=acl-local
HDR="Authorization: Bearer $TOKEN"

green() { printf '\033[0;32m%s\033[0m\n' "$*"; }
red()   { printf '\033[0;31m%s\033[0m\n' "$*"; }

fetch_username() {
  curl -sS -H "$HDR" "$1/api/meshes" \
    | python3 -c "import sys,json; d=json.load(sys.stdin); print(d[0]['agent']['username'])"
}

ALICE_USER=$(fetch_username "$ALICE")
BOB_USER=$(fetch_username "$BOB")
green "alice's username = $ALICE_USER"
green "bob's  username  = $BOB_USER"

CREATOR="$ALICE_USER"
fail=0
SETTLE=8

chats_for() {
  curl -sS -H "$HDR" "$1/api/meshes/$MESH/apps/ztm/chat/api/chats"
}

create_or_update_group() {
  local group=$1 name=$2 members_json=$3
  curl -sS -X POST -H "$HDR" -H 'Content-Type: application/json' \
    -d "{\"name\":\"$name\",\"members\":$members_json}" \
    "$ALICE/api/meshes/$MESH/apps/ztm/chat/api/groups/$CREATOR/$group" > /dev/null
}

expect_visible_to_bob() {
  local group=$1 should_see=$2 label=$3
  sleep $SETTLE
  local out
  out=$(chats_for $BOB)
  echo "  bob /chats: $out"
  if echo "$out" | grep -q "$group"; then
    if [ "$should_see" = "yes" ]; then
      green "  ✓ $label: bob sees the group as expected"
    else
      red   "  ✗ $label: LEAK — bob can see the group"; fail=1
    fi
  else
    if [ "$should_see" = "yes" ]; then
      red   "  ✗ $label: bob cannot see the group (but should)"; fail=1
    else
      green "  ✓ $label: bob does not see the group"
    fi
  fi
}

# ─── Scenario 1: alice-only group ─────────────────────────────────────
GROUP1="solo-$(date +%s)"
green "Scenario 1: alice creates '$GROUP1' with members=[$ALICE_USER]; bob NOT invited"
create_or_update_group "$GROUP1" "Alice Solo" "[\"$ALICE_USER\"]"
expect_visible_to_bob "$GROUP1" no "alice-only"

# ─── Scenario 2: group that includes bob ──────────────────────────────
GROUP2="shared-$(date +%s)"
echo
green "Scenario 2: alice creates '$GROUP2' with members=[$ALICE_USER, $BOB_USER]"
create_or_update_group "$GROUP2" "Shared" "[\"$ALICE_USER\",\"$BOB_USER\"]"
expect_visible_to_bob "$GROUP2" yes "shared"

# ─── Scenario 3: alice removes bob; bob restarts ──────────────────────
# NB: bob's in-memory chat list does not auto-prune on membership change while
# his agent keeps running (/chats reads from local cache). A restart rebuilds
# bob's state from the hub, which is the right condition for verifying the
# hub's ACL revocation actually took effect.
echo
green "Scenario 3: alice removes bob from '$GROUP2'; restart bob; bob must not see it"
create_or_update_group "$GROUP2" "Shared" "[\"$ALICE_USER\"]"
sleep $SETTLE

ZTM="$SCRIPT_DIR/../../bin/ztm"
BOB_PORT=7782

for pid in $(lsof -ti:$BOB_PORT 2>/dev/null); do kill -9 "$pid" 2>/dev/null || true; done
sleep 2
nohup "$ZTM" run agent \
  --listen "127.0.0.1:$BOB_PORT" \
  --data "$TMP/bob" \
  --api-token "$TOKEN" \
  > "$TMP/bob.log.restart" 2>&1 &
echo $! > "$TMP/bob.pid"
for i in 1 2 3 4 5 6 7 8 9 10 11 12; do
  curl -sSf -H "$HDR" "$BOB/api/meshes" > /dev/null 2>&1 && break
  sleep 1
done
sleep 4

expect_visible_to_bob "$GROUP2" no "after-removal-restart"

echo
if [ $fail -eq 0 ]; then
  green "======== PASS ========"
else
  red   "======== FAIL ========"
  exit 1
fi
