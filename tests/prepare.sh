#!/bin/bash
#
# prepare.sh - Initialize test environment
#
# Description:
#   1. Delete tests/tmp if it exists
#   2. Create tests/tmp directory structure
#   3. Start Hub service (port 18888, data dir tests/tmp/hub)
#   4. Start Agent1 service (port 7778, data dir tests/tmp/agent1)
#   5. Start Agent2 service (port 7779, data dir tests/tmp/agent2)
#   6. Wait for all services to start
#
# Environment variables:
#   ZTM_BIN - ztm executable path (default: ./bin/ztm)
#   HUB_PORT - Hub port (default: 18888)
#   AGENT1_PORT - Agent1 port (default: 7778)
#   AGENT2_PORT - Agent2 port (default: 7779)
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
TMP_DIR="$SCRIPT_DIR/tmp"
ZTM_BIN="${ZTM_BIN:-$PROJECT_ROOT/bin/ztm}"

# Service configuration
HUB_PORT="${HUB_PORT:-18888}"
REG_PORT="${REG_PORT:-5678}"  # Registration API port
AGENT1_PORT="${AGENT1_PORT:-7778}"
AGENT2_PORT="${AGENT2_PORT:-7779}"
API_TOKEN="enjoy-party"

# Color definitions
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Wait for service to listen on specified port
wait_for_port() {
    local port=$1
    local name=$2
    local max_attempts=30
    local attempt=0

    log_info "Waiting for $name to start (port $port)..."
    while ! nc -z 127.0.0.1 $port 2>/dev/null; do
        attempt=$((attempt + 1))
        if [ $attempt -ge $max_attempts ]; then
            log_error "$name start timeout (port $port)"
            return 1
        fi
        sleep 1
    done
    log_info "$name started (port $port)"
    return 0
}

# Clean up existing processes
cleanup_processes() {
    log_info "Cleaning up existing processes..."

    # Kill processes occupying test ports
    for port in $HUB_PORT $AGENT1_PORT $AGENT2_PORT; do
        local pid=$(lsof -t -i:$port 2>/dev/null || true)
        if [ -n "$pid" ]; then
            log_warn "Killing process on port $port (PID: $pid)"
            kill -9 $pid 2>/dev/null || true
        fi
    done

    sleep 1
}

# Check and create temp directory
setup_tmp_dir() {
    log_info "Setting up temp directory..."
    rm -rf "$TMP_DIR"
    mkdir -p "$TMP_DIR/hub"
    mkdir -p "$TMP_DIR/agent1"
    mkdir -p "$TMP_DIR/agent2"
    log_info "Temp directory created: $TMP_DIR"
}

# Start Hub service
start_hub() {
    log_info "Starting Hub service..."

    nohup "$ZTM_BIN" run hub \
        --listen "127.0.0.1:$HUB_PORT" \
        --data "$TMP_DIR/hub" \
        --api-token "$API_TOKEN" \
        --names "127.0.0.1:$HUB_PORT" \
        --enable-registration \
        > "$TMP_DIR/hub.log" 2>&1 &

    local hub_pid=$!
    echo $hub_pid > "$TMP_DIR/hub.pid"

    log_info "Hub process started (PID: $hub_pid)"
}

# Start Agent1 service
start_agent1() {
    log_info "Starting Agent1 service..."

    nohup "$ZTM_BIN" run agent \
        --listen "127.0.0.1:$AGENT1_PORT" \
        --data "$TMP_DIR/agent1" \
        --api-token "$API_TOKEN" \
        --names "127.0.0.1:$HUB_PORT" \
        > "$TMP_DIR/agent1.log" 2>&1 &

    local agent1_pid=$!
    echo $agent1_pid > "$TMP_DIR/agent1.pid"

    log_info "Agent1 process started (PID: $agent1_pid)"
}

# Start Agent2 service
start_agent2() {
    log_info "Starting Agent2 service..."

    nohup "$ZTM_BIN" run agent \
        --listen "127.0.0.1:$AGENT2_PORT" \
        --data "$TMP_DIR/agent2" \
        --api-token "$API_TOKEN" \
        --names "127.0.0.1:$HUB_PORT" \
        > "$TMP_DIR/agent2.log" 2>&1 &

    local agent2_pid=$!
    echo $agent2_pid > "$TMP_DIR/agent2.pid"

    log_info "Agent2 process started (PID: $agent2_pid)"
}

# Join agent to hub
join_agent_to_hub() {
    local name=$1
    local port=$2
    local mesh=$3

    log_info "Joining $name to $mesh mesh..."

    # Wait for Hub registration port to be ready
    wait_for_port $REG_PORT "Hub registration API"

    # Wait for agent to be ready
    wait_for_port $port "$name"

    export ZTM_CONFIG="127.0.0.1:$port"
    export ZTM_API_TOKEN="$API_TOKEN"

    local reg_url="http://127.0.0.1:$REG_PORT"
    "$ZTM_BIN" join "$mesh" --reg-url "$reg_url" 2>&1 || log_warn "$name join failed"

    log_info "$name joined to $mesh mesh"
}

# Extract mTLS certificates from agent1's ztm.db after joining mesh
# Writes: $TMP_DIR/certs/ca.pem, client.crt, client.key
extract_certs() {
    local db="$TMP_DIR/agent1/ztm.db"
    local cert_dir="$TMP_DIR/certs"

    log_info "Extracting mTLS certificates from agent1 ztm.db..."

    mkdir -p "$cert_dir"

    # Wait until the meshes table is populated (join completed)
    local max_attempts=20
    local attempt=0
    while true; do
        local mesh_count
        mesh_count=$(sqlite3 "$db" "SELECT COUNT(*) FROM meshes" 2>/dev/null || echo "0")
        if [ "$mesh_count" -gt 0 ]; then
            break
        fi
        attempt=$((attempt + 1))
        if [ $attempt -ge $max_attempts ]; then
            log_error "Timed out waiting for agent1 to join mesh"
            return 1
        fi
        sleep 1
    done

    # Extract CA certificate
    sqlite3 "$db" "SELECT ca FROM meshes LIMIT 1" > "$cert_dir/ca.pem"

    # Extract Agent certificate (JSON field: .certificate)
    sqlite3 "$db" "SELECT agent FROM meshes LIMIT 1" \
        | python3 -c "import sys,json; d=json.load(sys.stdin); print(d['certificate'], end='')" \
        > "$cert_dir/client.crt"

    # Extract Agent private key from keys table
    sqlite3 "$db" "SELECT data FROM keys WHERE name='agent'" > "$cert_dir/client.key"

    # Verify all three files are non-empty
    if [ -s "$cert_dir/ca.pem" ] && [ -s "$cert_dir/client.crt" ] && [ -s "$cert_dir/client.key" ]; then
        log_info "Certificates extracted to $cert_dir"
        log_info "  CA cert:     $cert_dir/ca.pem"
        log_info "  Client cert: $cert_dir/client.crt"
        log_info "  Client key:  $cert_dir/client.key"
    else
        log_error "Certificate extraction failed - one or more files are empty"
        return 1
    fi
}

# Wait for all services to be ready
wait_for_services() {
    log_info "Waiting for all services..."

    wait_for_port $HUB_PORT "Hub"
    wait_for_port $AGENT1_PORT "Agent1"
    wait_for_port $AGENT2_PORT "Agent2"

    # Extra wait to ensure services are ready
    sleep 2

    log_info "All services ready!"
}

# Verify services accessibility
verify_services() {
    log_info "Verifying services accessibility..."

    local cert_dir="$TMP_DIR/certs"
    local agent1_url="http://127.0.0.1:$AGENT1_PORT/api/version"
    local agent2_url="http://127.0.0.1:$AGENT2_PORT/api/version"

    # Hub requires mTLS - use client certificates if available
    if [ -f "$cert_dir/client.crt" ] && [ -f "$cert_dir/client.key" ] && [ -f "$cert_dir/ca.pem" ]; then
        curl -s -o /dev/null -w "Hub Status: %{http_code}\n" \
            --cert "$cert_dir/client.crt" \
            --key "$cert_dir/client.key" \
            --cacert "$cert_dir/ca.pem" \
            "https://127.0.0.1:$HUB_PORT/api/status" || log_warn "Hub status check failed"
    else
        log_warn "mTLS certificates not available, skipping Hub status check"
    fi

    curl -s -o /dev/null -w "Agent1 Version: %{http_code}\n" \
        -H "Authorization: Bearer $API_TOKEN" "$agent1_url" || log_warn "Agent1 version check failed"
    curl -s -o /dev/null -w "Agent2 Version: %{http_code}\n" \
        -H "Authorization: Bearer $API_TOKEN" "$agent2_url" || log_warn "Agent2 version check failed"

    log_info "Service verification complete"
}

# Main function
main() {
    echo "=========================================="
    echo "Test Environment Initialization"
    echo "=========================================="
    echo ""
    echo "Configuration:"
    echo "  ZTM_BIN:    $ZTM_BIN"
    echo "  HUB_PORT:   $HUB_PORT"
    echo "  AGENT1_PORT: $AGENT1_PORT"
    echo "  AGENT2_PORT: $AGENT2_PORT"
    echo "  TMP_DIR:    $TMP_DIR"
    echo ""
    echo "=========================================="

    # Step 0: Build ztm binary
    log_info "Building ztm binary..."
    cd "$PROJECT_ROOT" && ./build.sh
    log_info "Build complete"

    # Step 1: Clean up existing processes
    cleanup_processes

    # Step 2: Create temp directory
    setup_tmp_dir

    # Step 3: Start Hub (with registration enabled)
    start_hub

    # Step 4: Start Agent1
    start_agent1

    # Step 5: Join Agent1 to party mesh
    join_agent_to_hub "agent1" "$AGENT1_PORT" "party"

    # Step 6: Start Agent2
    start_agent2

    # Step 7: Join Agent2 to party mesh
    join_agent_to_hub "agent2" "$AGENT2_PORT" "party"

    # Step 8: Wait for all services
    wait_for_services

    # Step 9: Extract mTLS certificates from agent1 ztm.db
    extract_certs

    # Step 10: Verify services
    verify_services

    echo ""
    echo "=========================================="
    log_info "Test environment initialization complete!"
    echo "=========================================="
    echo ""
    echo "Service addresses:"
    echo "  Hub:    http://127.0.0.1:$HUB_PORT"
    echo "  Agent1: http://127.0.0.1:$AGENT1_PORT"
    echo "  Agent2: http://127.0.0.1:$AGENT2_PORT"
    echo ""
    echo "API Token: $API_TOKEN"
    echo ""
    echo "mTLS Certificates (for Hub API tests):"
    echo "  CA cert:     $TMP_DIR/certs/ca.pem"
    echo "  Client cert: $TMP_DIR/certs/client.crt"
    echo "  Client key:  $TMP_DIR/certs/client.key"
    echo ""
    echo "Environment variables:"
    echo "  Agent1: export ZTM_CONFIG=127.0.0.1:$AGENT1_PORT ZTM_API_TOKEN=$API_TOKEN"
    echo "  Agent2: export ZTM_CONFIG=127.0.0.1:$AGENT2_PORT ZTM_API_TOKEN=$API_TOKEN"
    echo "  Hub mTLS: export HUB_CA=$TMP_DIR/certs/ca.pem HUB_CERT=$TMP_DIR/certs/client.crt HUB_KEY=$TMP_DIR/certs/client.key"
    echo ""
}

main "$@"