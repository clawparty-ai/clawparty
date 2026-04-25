#!/bin/bash
#
# Full invite code test: 1 hub + 3 agents
#   - alice, bob: join with root permit (as root user)
#   - charlie: join with invite code (as regular user)
#
# Layout (all under tests/hub-llm-local/tmp/):
#   hub     -> hub data dir, listens on 127.0.0.1:18888 (+ reg :15678)
#   alice   -> agent data dir, listens on 127.0.0.1:7781
#   bob     -> agent data dir, listens on 127.0.0.1:7782
#   charlie -> agent data dir, listens on 127.0.0.1:7783
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMP="$SCRIPT_DIR/tmp"
ZTM="${ZTM_BIN:-$PROJECT_ROOT/bin/ztm}"

HUB_PORT=18888
REG_PORT=15678
ALICE_PORT=7781
BOB_PORT=7782
CHARLIE_PORT=7783
MESH_NAME=clawparty
API_TOKEN=hub-llm-test

green()  { printf '\033[0;32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[1;33m%s\033[0m\n' "$*"; }
red()    { printf '\033[0;31m%s\033[0m\n' "$*"; }
log()    { green "[setup] $*"; }

wait_port() {
  local port=$1 name=$2 tries=30
  while ! nc -z 127.0.0.1 "$port" 2>/dev/null; do
    tries=$((tries - 1))
    if [ "$tries" -le 0 ]; then red "timeout waiting for $name on $port"; exit 1; fi
    sleep 0.5
  done
}

cleanup() {
  log "cleaning previous run"
  for port in $HUB_PORT $REG_PORT $ALICE_PORT $BOB_PORT $CHARLIE_PORT; do
    local pid
    pid=$(lsof -ti:"$port" 2>/dev/null || true)
    [ -n "$pid" ] && kill -9 $pid 2>/dev/null || true
  done
  rm -rf "$TMP"
  mkdir -p "$TMP"
}

prepare_zeroclaw_config() {
  local zeroclaw_config="$TMP/zeroclaw-config.toml"
  local source_config="$HOME/.zeroclaw/config.toml"

  if [ -f "$source_config" ]; then
    log "copying zeroclaw config from $source_config"
    cp "$source_config" "$zeroclaw_config"
  else
    log "creating minimal zeroclaw config (no ~/.zeroclaw/config.toml found)"
    cat > "$zeroclaw_config" <<'EOF'
[llm]
provider = "openai"
api_key = "your-api-key-here"
default_model = "gpt-4o-mini"
temperature = 0.7
timeout_secs = 120

[memory]
backend = "none"
auto_save = false
EOF
  fi

  log "zeroclaw config ready at $zeroclaw_config"
  ZEROCLAW_CONFIG="$zeroclaw_config"
}

start_hub() {
  local zeroclaw_config=$1
  log "starting hub on :$HUB_PORT (registration :$REG_PORT)"
  nohup "$ZTM" run hub \
    --listen "127.0.0.1:$HUB_PORT" \
    --data "$TMP/hub" \
    --names "127.0.0.1:$HUB_PORT" \
    --enable-registration "127.0.0.1:$REG_PORT" \
    --zeroclaw-config "$zeroclaw_config" \
    --permit "$TMP/root.json" \
    > "$TMP/hub.log" 2>&1 &
  echo $! > "$TMP/hub.pid"
  wait_port $HUB_PORT hub
  wait_port $REG_PORT registration
  log "root permit saved to $TMP/root.json"
}

start_agent() {
  local name=$1 port=$2
  log "starting agent '$name' on :$port"
  mkdir -p "$TMP/$name"

  # Run agent from PROJECT_ROOT/agent so relative 'gui' directory can be served at '/'
  (
    cd "$PROJECT_ROOT/agent"
    nohup "$ZTM" run agent \
      --listen "127.0.0.1:$port" \
      --data "$TMP/$name" \
      --api-token "$API_TOKEN" \
      > "$TMP/$name.log" 2>&1 &
    echo $! > "$TMP/$name.pid"
  )

  wait_port "$port" "agent $name"
}

join_with_permit() {
  local name=$1 port=$2
  log "$name joining with root permit"
  ZTM_CONFIG="127.0.0.1:$port" ZTM_API_TOKEN="$API_TOKEN" \
    "$ZTM" join clawparty \
      --as "$name" \
      --permit "$TMP/root.json"
}

generate_invite_code() {
  local username=$1
  log "generating invite code for user '$username'..."

  # Wait for alice to be fully connected
  local tries=10
  while true; do
    local connected
    connected=$(curl -sS -H "Authorization: Bearer $API_TOKEN" \
      "http://127.0.0.1:$ALICE_PORT/api/meshes" 2>/dev/null | \
      python3 -c "import sys,json; meshes=json.load(sys.stdin); print(meshes[0]['connected'] if meshes else False)" 2>/dev/null || echo "false")

    if [ "$connected" = "True" ]; then
      break
    fi

    tries=$((tries - 1))
    if [ "$tries" -le 0 ]; then
      red "alice not connected to mesh, cannot generate invite code"
      exit 1
    fi
    sleep 1
  done

  # Generate invite code via alice (root user)
  ZTM_CONFIG="127.0.0.1:$ALICE_PORT" ZTM_API_TOKEN="$API_TOKEN" \
    "$ZTM" add-invite-code --name "$username" --email "${username}@test.local" > "$TMP/invite-code.txt"

  local code
  code=$(cat "$TMP/invite-code.txt" | grep -o '[A-Z0-9]\{8\}' | head -1)

  if [ -z "$code" ]; then
    red "failed to generate invite code"
    cat "$TMP/invite-code.txt"
    exit 1
  fi

  green "  invite code: $code"
  echo "$code" > "$TMP/invite-code-only.txt"
}

join_with_invite_code() {
  local name=$1 port=$2 username=$3
  local code
  code=$(cat "$TMP/invite-code-only.txt")

  log "$name joining with invite code as user '$username'"

  local payload
  payload=$(python3 - <<PY
import json
print(json.dumps({
  "regUrl": "http://127.0.0.1:$REG_PORT",
  "userName": "$username",
  "inviteCode": "$code"
}))
PY
)

  local http_code
  http_code=$(curl -sS -o "$TMP/${name}-join-party.json" -w "%{http_code}" \
    -H "Authorization: Bearer $API_TOKEN" \
    -H "Content-Type: application/json" \
    -X POST "http://127.0.0.1:$port/api/join-party" \
    -d "$payload")

  if [ "$http_code" != "200" ]; then
    red "$name join-party failed (HTTP $http_code)"
    cat "$TMP/${name}-join-party.json"
    exit 1
  fi

  green "  $name joined via invite code"
}

verify_agent() {
  local name=$1 port=$2
  log "verifying 0#Agent created for $name..."
  local tries=20
  while true; do
    local status
    status=$(curl -sS -H "Authorization: Bearer $API_TOKEN" \
      "http://127.0.0.1:$port/api/agents" 2>/dev/null | \
      python3 -c "import sys,json; agents=json.load(sys.stdin); a=next((x for x in agents if x.get('agent_name')=='0#Agent'),None); print(a['status'] if a else 'not_found')" 2>/dev/null || echo "error")
    if [ "$status" = "running" ]; then
      green "  $name: 0#Agent is running"
      return 0
    elif [ "$status" = "not_found" ]; then
      tries=$((tries - 1))
      if [ "$tries" -le 0 ]; then
        yellow "  $name: 0#Agent not found (hub may not have LLM config)"
        return 0
      fi
      sleep 1
    else
      green "  $name: 0#Agent status = $status"
      return 0
    fi
  done
}

verify_global_config() {
  local name=$1 port=$2
  local cfg
  cfg=$(curl -sS -H "Authorization: Bearer $API_TOKEN" \
    "http://127.0.0.1:$port/api/global-config" 2>/dev/null || echo "{}")
  local model
  model=$(echo "$cfg" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('llm',{}).get('model','not_found'))" 2>/dev/null || echo "error")
  if [ "$model" != "not_found" ] && [ "$model" != "error" ]; then
    green "  $name: global config model = $model"
  else
    yellow "  $name: no global config found"
  fi
}

verify_mesh_user() {
  local name=$1 port=$2 expected_user=$3
  local username
  username=$(curl -sS -H "Authorization: Bearer $API_TOKEN" \
    "http://127.0.0.1:$port/api/meshes" 2>/dev/null | \
    python3 -c "import sys,json; meshes=json.load(sys.stdin); print(meshes[0]['agent']['username'] if meshes else 'unknown')" 2>/dev/null || echo "error")

  if [ "$username" = "$expected_user" ]; then
    green "  $name: joined as user '$username' ✓"
  else
    red "  $name: expected user '$expected_user', got '$username'"
  fi
}

# ── Main ──────────────────────────────────────────────────────────────────────

cleanup
prepare_zeroclaw_config
start_hub "$ZEROCLAW_CONFIG"

# Start alice and bob (will join as root)
start_agent alice $ALICE_PORT
start_agent bob   $BOB_PORT

join_with_permit alice $ALICE_PORT
join_with_permit bob   $BOB_PORT

sleep 3

log "verifying alice and bob..."
verify_global_config alice $ALICE_PORT
verify_global_config bob   $BOB_PORT
verify_agent alice $ALICE_PORT
verify_agent bob   $BOB_PORT
verify_mesh_user alice $ALICE_PORT "root"
verify_mesh_user bob   $BOB_PORT "root"

# Generate invite code for charlie
generate_invite_code "charlie-user"

# Start charlie and join with invite code
start_agent charlie $CHARLIE_PORT
join_with_invite_code charlie $CHARLIE_PORT "charlie-user"

sleep 3

log "verifying charlie..."
verify_global_config charlie $CHARLIE_PORT
verify_agent charlie $CHARLIE_PORT
verify_mesh_user charlie $CHARLIE_PORT "charlie-user"

echo
green "════════════════════════════════════════════════════════════════"
green "✓ Test Complete!"
green "════════════════════════════════════════════════════════════════"
echo
green "Hub:        http://127.0.0.1:$HUB_PORT"
green "RegSrv:     http://127.0.0.1:$REG_PORT"
echo
green "Alice:      http://127.0.0.1:$ALICE_PORT  (user: root, token: $API_TOKEN)"
green "Bob:        http://127.0.0.1:$BOB_PORT  (user: root, token: $API_TOKEN)"
green "Charlie:    http://127.0.0.1:$CHARLIE_PORT  (user: charlie-user, token: $API_TOKEN)"
echo
yellow "Invite code used: $(cat $TMP/invite-code-only.txt)"
echo
yellow "Logs: tail -f $TMP/alice.log $TMP/bob.log $TMP/charlie.log $TMP/hub.log"
echo
green "GUI access:"
green "  Alice:   http://127.0.0.1:$ALICE_PORT/"
green "  Bob:     http://127.0.0.1:$BOB_PORT/"
green "  Charlie: http://127.0.0.1:$CHARLIE_PORT/"
echo
