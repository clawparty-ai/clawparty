#!/bin/bash
#
# Integration test: 1 hub (with zeroclaw config) + 3 agents.
# Tests the full invite-only flow for test users:
#   admin joins with root permit (bootstrap only)
#   alice/bob join via invite code -> auto-global-config -> 0#Agent creation
#
# Layout (all under tests/hub-llm-local/tmp/):
#   hub   -> hub data dir, listens on 127.0.0.1:18888 (+ reg :15678)
#   admin -> bootstrap admin endpoint, listens on 127.0.0.1:7780
#   alice -> test user endpoint, listens on 127.0.0.1:7781
#   bob   -> test user endpoint, listens on 127.0.0.1:7782
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
TMP="$SCRIPT_DIR/tmp"
ZTM="${ZTM_BIN:-$PROJECT_ROOT/bin/ztm}"

HUB_PORT=18888
REG_PORT=15678
ADMIN_PORT=7780
ALICE_PORT=7781
BOB_PORT=7782
MESH_NAME=clawparty
API_TOKEN=hub-llm-test

green()  { printf '\033[0;32m%s\033[0m\n' "$*" >&2; }
yellow() { printf '\033[1;33m%s\033[0m\n' "$*" >&2; }
red()    { printf '\033[0;31m%s\033[0m\n' "$*" >&2; }
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
  for port in $HUB_PORT $REG_PORT $ADMIN_PORT $ALICE_PORT $BOB_PORT; do
    local pid
    pid=$(lsof -ti:"$port" 2>/dev/null || true)
    [ -n "$pid" ] && kill -9 $pid 2>/dev/null || true
  done
  rm -rf "$TMP"
  mkdir -p "$TMP"
}

# Prepare zeroclaw config for the hub
prepare_zeroclaw_config() {
  local zeroclaw_config="$TMP/zeroclaw-config.toml"
  local source_config="$HOME/.zeroclaw/config.toml"

  if [ -f "$source_config" ]; then
    log "copying zeroclaw config from $source_config"
    cp "$source_config" "$zeroclaw_config"
  else
    log "creating minimal zeroclaw config (no ~/.zeroclaw/config.toml found)"
    cat > "$zeroclaw_config" <<'EOF'
# Minimal zeroclaw config for testing
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
  # Run from agent/ dir so relative 'gui' directory resolves correctly
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
  log "$name joining with root permit (admin bootstrap only)"
  ZTM_CONFIG="127.0.0.1:$port" ZTM_API_TOKEN="$API_TOKEN" \
    "$ZTM" join clawparty \
      --as "$name" \
      --permit "$TMP/root.json"
}

generate_invite_code() {
  local username=$1
  log "generating invite code for '$username' via admin..."

  # Wait for admin to be connected to mesh
  local tries=15
  while true; do
    local connected
    connected=$(curl -sS -H "Authorization: Bearer $API_TOKEN" \
      "http://127.0.0.1:$ADMIN_PORT/api/meshes" 2>/dev/null | \
      python3 -c "import sys,json; m=json.load(sys.stdin); print(m[0]['connected'] if m else False)" 2>/dev/null || echo "False")
    [ "$connected" = "True" ] && break
    tries=$((tries - 1))
    [ "$tries" -le 0 ] && red "admin not connected to mesh" && exit 1
    sleep 1
  done

  local out code
  out=$(ZTM_CONFIG="127.0.0.1:$ADMIN_PORT" ZTM_API_TOKEN="$API_TOKEN" \
    "$ZTM" add-invite-code --name "$username" --email "${username}@test.local" 2>&1)
  code=$(echo "$out" | grep -o '[A-Z0-9]\{8\}' | head -1)
  [ -z "$code" ] && red "failed to generate invite code: $out" && exit 1
  green "  invite code for $username: $code"
  echo "$code"
}

join_with_invite_code() {
  local name=$1 port=$2 username=$3 code=$4
  log "$name joining as '$username' via invite code $code"

  local payload
  payload=$(python3 -c "import json; print(json.dumps({'regUrl':'http://127.0.0.1:$REG_PORT','userName':'$username','inviteCode':'$code'}))")

  local http_code
  http_code=$(curl -sS -o "$TMP/${name}-join.json" -w "%{http_code}" \
    -H "Authorization: Bearer $API_TOKEN" \
    -H "Content-Type: application/json" \
    -X POST "http://127.0.0.1:$port/api/join-party" \
    -d "$payload")

  if [ "$http_code" != "200" ]; then
    red "$name join-party failed (HTTP $http_code)"
    cat "$TMP/${name}-join.json"
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

# ── Main ──────────────────────────────────────────────────────────────────────

cleanup
prepare_zeroclaw_config
start_hub "$ZEROCLAW_CONFIG"

# Admin: bootstrap endpoint using root permit (not a test user)
start_agent admin $ADMIN_PORT
join_with_permit admin $ADMIN_PORT

# Generate invite codes for alice and bob via admin
ALICE_CODE=$(generate_invite_code "alice")
BOB_CODE=$(generate_invite_code "bob")

# Alice and bob: join via invite code (triggers zeroclaw_config + 0#Agent creation)
start_agent alice $ALICE_PORT
start_agent bob   $BOB_PORT

join_with_invite_code alice $ALICE_PORT "alice" "$ALICE_CODE"
join_with_invite_code bob   $BOB_PORT  "bob"   "$BOB_CODE"

sleep 3

log "verifying results..."
verify_global_config alice $ALICE_PORT
verify_global_config bob   $BOB_PORT
verify_agent alice $ALICE_PORT
verify_agent bob   $BOB_PORT

echo
green "Hub:     127.0.0.1:$HUB_PORT"
green "RegSrv:  http://127.0.0.1:$REG_PORT   <-- paste into GUI's 加入组织 dialog"
green "Admin:   http://127.0.0.1:$ADMIN_PORT  (token $API_TOKEN, user: root)"
green "Alice:   http://127.0.0.1:$ALICE_PORT  (token $API_TOKEN, user: alice)"
green "Bob:     http://127.0.0.1:$BOB_PORT    (token $API_TOKEN, user: bob)"
echo
yellow "Logs: tail -f $TMP/alice.log $TMP/bob.log $TMP/hub.log"
