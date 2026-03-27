#!/bin/bash
#
# cleanup.sh - Clean up test environment
#
# Description:
#   1. Kill all processes listening on test ports (Hub, Agent1, Agent2, Registration)
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

# Clean up all test ports
cleanup_ports() {
    log_info "Cleaning up test ports..."
    
    # Test ports
    # 18888 - Hub mTLS port
    # 5678  - Hub registration API
    # 7778  - Agent1
    # 7779  - Agent2
    local ports=("18888" "5678" "7778" "7779")
    
    for port in "${ports[@]}"; do
        # Find processes using this port (IPv4 and IPv6)
        local pids=$(lsof -i tcp:$port 2>/dev/null | grep -v "^COMMAND" | awk '{print $2}' | sort -u || true)
        
        if [ -n "$pids" ]; then
            for pid in $pids; do
                if kill -0 $pid 2>/dev/null; then
                    local cmd=$(ps -p $pid -o comm= 2>/dev/null || echo "unknown")
                    log_warn "Killing process on port $port (PID: $pid, CMD: $cmd)"
                    kill -9 $pid 2>/dev/null || true
                fi
            done
        else
            log_info "Port $port is free"
        fi
    done
    
    # Extra wait to ensure ports are released
    sleep 1
}

# Main function
main() {
    echo "=========================================="
    echo "Cleaning up test environment"
    echo "=========================================="
    
    # Clean up by port
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