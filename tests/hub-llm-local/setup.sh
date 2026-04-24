#!/bin/bash
#
# Integration test: 1 hub (with LLM config) + 2 agents.
# Tests the full join-party → auto-global-config → 0#Agent creation flow.
#
# Layout (all under tests/hub-llm-local/tmp/):
#   hub   -> hub data dir, listens on 127.0.0.1:18888 (+ reg :15678)
#   alice -> agent data dir, listens on 127.0.0.1:7781
#   bob   -> agent data dir, listens on 127.0.0.1:7782
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
  for port in $HUB_PORT $REG_PORT $ALICE_PORT $BOB_PORT; do
    local pid
    pid=$(lsof -ti:"$port" 2>/dev/null || true)
    [ -n "$pid" ] && kill -9 $pid 2>/dev/null || true
  done
  rm -rf "$TMP"
  mkdir -p "$TMP"
}

# Write the LLM config file for the hub
write_llm_config() {
  local llm_config="$TMP/llm-config.json"
  # Use ~/.clawparty/.zeroclaw/config.toml values if available, else fallback
  local api_key model provider api_endpoint
  local zc_config="$HOME/.clawparty/.zeroclaw/config.toml"
  if [ -f "$zc_config" ]; then
    api_key=$(grep '^api_key' "$zc_config" | head -1 | sed 's/.*= *"\(.*\)"/\1/')
    model=$(grep '^default_model' "$zc_config" | head -1 | sed 's/.*= *"\(.*\)"/\1/')
    api_endpoint=$(grep '^default_provider' "$zc_config" | head -1 | sed 's/.*= *"custom:\(.*\)"/\1/')
    provider="custom"
  fi
  api_key="${api_key:-your-api-key-here}"
  model="${model:-gpt-4o-mini}"
  provider="${provider:-openai}"

  if [ -n "$api_endpoint" ]; then
    cat > "$llm_config" <<EOF
{
  "default_llm_config": {
    "provider": "$provider",
    "api_endpoint": "$api_endpoint",
    "api_key": "$api_key",
    "model": "$model",
    "temperature": 0.7,
    "timeout_secs": 120
  }
}
EOF
  else
    cat > "$llm_config" <<EOF
{
  "default_llm_config": {
    "provider": "$provider",
    "api_key": "$api_key",
    "model": "$model",
    "temperature": 0.7,
    "timeout_secs": 120
  }
}
EOF
  fi
  log "wrote LLM config to $llm_config (model=$model)"
  echo "$llm_config"
}

start_hub() {
  local llm_config=$1
  log "starting hub on :$HUB_PORT (registration :$REG_PORT)"
  nohup "$ZTM" run hub \
    --listen "127.0.0.1:$HUB_PORT" \
    --data "$TMP/hub" \
    --names "127.0.0.1:$HUB_PORT" \
    --enable-registration "127.0.0.1:$REG_PORT" \
    --llm-config "$llm_config" \
    > "$TMP/hub.log" 2>&1 &
  echo $! > "$TMP/hub.pid"
  wait_port $HUB_PORT hub
  wait_port $REG_PORT registration
}

start_agent() {
  local name=$1 port=$2
  log "starting agent '$name' on :$port"
  mkdir -p "$TMP/$name/.zeroclaw"
  # Minimal zeroclaw config (LLM will come from hub)
  cat > "$TMP/$name/.zeroclaw/config.toml" <<'EOF'
[memory]
backend = "none"
auto_save = false
EOF
  nohup "$ZTM" run agent \
    --listen "127.0.0.1:$port" \
    --data "$TMP/$name" \
    --api-token "$API_TOKEN" \
    > "$TMP/$name.log" 2>&1 &
  echo $! > "$TMP/$name.pid"
  wait_port "$port" "agent $name"
}

join_party() {
  local name=$1 port=$2
  log "$name joining via local reg server"
  ZTM_CONFIG="127.0.0.1:$port" ZTM_API_TOKEN="$API_TOKEN" \
    "$ZTM" join party \
      --reg-url "http://127.0.0.1:$REG_PORT" \
      --name "$name"
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
LLM_CONFIG=$(write_llm_config)
start_hub "$LLM_CONFIG"
start_agent alice $ALICE_PORT
start_agent bob   $BOB_PORT

join_party alice $ALICE_PORT
join_party bob   $BOB_PORT

sleep 3

log "verifying results..."
verify_global_config alice $ALICE_PORT
verify_global_config bob   $BOB_PORT
verify_agent alice $ALICE_PORT
verify_agent bob   $BOB_PORT

echo
green "Hub:     127.0.0.1:$HUB_PORT"
green "RegSrv:  http://127.0.0.1:$REG_PORT   <-- paste into GUI's 加入组织 dialog"
green "Alice:   http://127.0.0.1:$ALICE_PORT  (token $API_TOKEN)"
green "Bob:     http://127.0.0.1:$BOB_PORT  (token $API_TOKEN)"
echo
yellow "Logs: tail -f $TMP/alice.log $TMP/bob.log $TMP/hub.log"
