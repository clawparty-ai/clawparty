#!/bin/bash
#
# Local ACL test: 1 hub + 2 agents with distinct usernames.
#
# Layout (all under tests/acl-local/tmp/):
#   hub     -> hub data dir, listens on 127.0.0.1:18888 (+ reg :15678)
#   alice   -> agent data dir, listens on 127.0.0.1:7781, username "alice"
#   bob     -> agent data dir, listens on 127.0.0.1:7782, username "bob"
#
# Uses the joinParty flow (ztm join party --reg-url ...) so the GUI's
# "加入组织" dialog can target the same local reg server at http://127.0.0.1:15678.
# joinParty() hardcodes the mesh name to `clawparty` internally; the "party"
# in `ztm join party` is a keyword, not the mesh name that ends up stored.
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
# The joinParty flow hardcodes mesh name = clawparty. Tests reference this.
MESH_NAME=clawparty
API_TOKEN=acl-local

green()  { printf '\033[0;32m%s\033[0m\n' "$*"; }
yellow() { printf '\033[1;33m%s\033[0m\n' "$*"; }
red()    { printf '\033[0;31m%s\033[0m\n' "$*"; }

log() { green "[setup] $*"; }

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

init_zeroclaw_config() {
  local data_dir=$1
  local config_dir="$data_dir/.zeroclaw"

  mkdir -p "$config_dir"
  if [ -f "$HOME/.zeroclaw/config.toml" ]; then
    cp "$HOME/.zeroclaw/config.toml" "$config_dir/config.toml"
    log "copied ZeroClaw config from ~/.zeroclaw/config.toml to $config_dir"
  else
    cat > "$config_dir/config.toml" <<'EOF'
default_provider = "openai"
default_model = "gpt-4o-mini"
api_key = "your-api-key-here"

[model_providers]

[memory]
backend = "none"
auto_save = false
EOF
    log "initialized ZeroClaw config at $config_dir/config.toml"
  fi
}

start_hub() {
  log "starting hub on :$HUB_PORT (registration :$REG_PORT)"
  nohup "$ZTM" run hub \
    --listen "127.0.0.1:$HUB_PORT" \
    --data "$TMP/hub" \
    --names "127.0.0.1:$HUB_PORT" \
    --enable-registration "127.0.0.1:$REG_PORT" \
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
  init_zeroclaw_config "$TMP/$name"
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
  log "$name joining via local reg server (username=$name)"
  ZTM_CONFIG="127.0.0.1:$port" ZTM_API_TOKEN="$API_TOKEN" \
    "$ZTM" join party \
      --reg-url "http://127.0.0.1:$REG_PORT" \
      --name "$name"
}

cleanup
start_hub
start_agent alice $ALICE_PORT
start_agent bob   $BOB_PORT

# Both agents join via the local reg server. The reg server issues unique
# usernames if collision occurs (alice-<suffix>, bob-<suffix>).
join_party alice $ALICE_PORT
join_party bob   $BOB_PORT

sleep 2
log "done"
echo
green "Hub:     127.0.0.1:$HUB_PORT"
green "RegSrv:  http://127.0.0.1:$REG_PORT   <-- paste into GUI's 加入组织 dialog"
green "Alice:   127.0.0.1:$ALICE_PORT  (token $API_TOKEN)"
green "Bob:     127.0.0.1:$BOB_PORT  (token $API_TOKEN)"
echo
yellow "Actual assigned usernames (the reg server may append a suffix):"
for ep in alice bob; do
  port=$([ "$ep" = alice ] && echo $ALICE_PORT || echo $BOB_PORT)
  user=$(curl -sS -H "Authorization: Bearer $API_TOKEN" \
    "http://127.0.0.1:$port/api/meshes" | \
    python3 -c "import sys,json; d=json.load(sys.stdin); print(d[0]['agent']['username'])" 2>/dev/null || echo "?")
  printf '  %-6s username = %s\n' "$ep" "$user"
done
echo
yellow "tail -f $TMP/alice.log | $TMP/bob.log | $TMP/hub.log"
