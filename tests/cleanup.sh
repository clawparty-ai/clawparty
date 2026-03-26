#!/bin/bash
#
# cleanup.sh - Clean up test environment
#
# Description:
#   1. Stop all test services (Hub, Agent1, Agent2)
#   2. Delete temp directory
#

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
TMP_DIR="$SCRIPT_DIR/tmp"

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

log_info() {
    echo -e "${GREEN}[INFO]${NC} $1"
}

log_warn() {
    echo -e "${YELLOW}[WARN]${NC} $1"
}

log_error() {
    echo -e "${RED}[ERROR]${NC} $1"
}

# Stop process
stop_process() {
    local name=$1
    local pid_file=$TMP_DIR/$name.pid

    if [ -f "$pid_file" ]; then
        local pid=$(cat "$pid_file")
        if kill -0 $pid 2>/dev/null; then
            log_info "Stopping $name (PID: $pid)"
            kill -9 $pid 2>/dev/null || true
            sleep 1
        fi
        rm -f "$pid_file"
    fi
}

# Clean up ports
cleanup_ports() {
    log_info "Cleaning up test ports..."

    local ports=("18888" "7778" "7779")
    for port in "${ports[@]}"; do
        local pid=$(lsof -t -i:$port 2>/dev/null || true)
        if [ -n "$pid" ]; then
            log_warn "Force killing process on port $port (PID: $pid)"
            kill -9 $pid 2>/dev/null || true
        fi
    done
}

# Main function
main() {
    echo "=========================================="
    echo "Cleaning up test environment"
    echo "=========================================="

    # Stop test processes
    stop_process "hub"
    stop_process "agent1"
    stop_process "agent2"

    # Clean up ports
    cleanup_ports

    # Delete temp directory
    if [ -d "$TMP_DIR" ]; then
        log_info "Deleting temp directory: $TMP_DIR"
        rm -rf "$TMP_DIR"
    fi

    echo ""
    echo "=========================================="
    log_info "Test environment cleanup complete!"
    echo "=========================================="
}

main "$@"