#!/bin/bash
#===============================================================================
# ClawParty macOS Installer & Manager
#===============================================================================
#
# 功能概述
# --------
# 这是 ClawParty 在 macOS 上的一站式脚本，整合了四大核心功能：
#
#   1. 安装    依赖检测 → 编译构建 → 二进制安装 → 桌面 App 安装 → 配置生成 → 验证
#   2. 启停    后台启动 ClawParty 服务、停止、重启、运行状态查看
#   3. 检测   对标 ClawPartyDesktop 的「检查与修复」系统，覆盖 7 大类别
#   4. 修复   自动拷贝缺失二进制、生成默认配置、创建工作区目录
#
#
# 两种运行方式
# ------------
#
#   A) 交互菜单（双击 .sh 文件或直接运行，无参数）
#
#       ./clawparty-macos.sh
#
#       进入终端菜单，显示当前服务状态和二进制位置，
#       用数字键选择：启动 / 停止 / 系统检查 / 检查并修复 / 退出
#
#   B) 命令行模式（带参数，跳过交互菜单）
#
#       ./clawparty-macos.sh <命令> [选项]
#
#       命令一览：
#         install     把当前目录的可执行文件安装到系统 PATH（不编译）
#         start       后台启动 ClawParty 服务
#         stop        停止 ClawParty 服务
#         restart     重启 ClawParty 服务
#         status      查看运行状态、进程详情、最近日志
#         check       系统诊断检查（只读，不修改）
#         check-fix   检查并自动修复发现的问题
#         uninstall   卸载 ClawParty（二进制 / 配置 / 数据 / 桌面 App）
#         help        显示帮助信息
#
#
# 命令详解
# --------
#
#   install [--skip-desktop] [--prefix PATH] [--no-modify-path]
#
#       安装流程：macOS 版本检查 → 检查脚本同目录是否有可执行文件 →
#       拷贝到 /usr/local/bin → 安装桌面 App 到 /Applications →
#       生成 ~/.config/clawparty/config.toml → 验证安装
#
#       注：install 不会编译，只把预编译好的文件拷贝到系统目录。
#           如需编译，先用仓库根目录的 ./build.sh 构建。
#
#       --skip-desktop       跳过桌面 App 安装
#       --prefix PATH        安装目录（默认 /usr/local）
#       --no-modify-path     不修改 shell 配置文件
#
#
#   start [--foreground] [--port PORT]
#
#       查找 clawparty 二进制 → 检查端口占用 → 后台启动 (nohup) →
#       写入 PID 文件 → 等待 2 秒验证启动成功
#
#       --foreground         前台运行（Ctrl+C 停止）
#       --port PORT          指定端口（默认 7778）
#
#
#   check       只读诊断，检查以下 7 个类别：
#
#       1. 可执行文件    clawparty / zeroclaw / ztm / opencode 是否在 PATH 中
#       2. 配置文件      0#Agent opencode.json 是否存在、api_key/model/provider 是否配置
#                         ClawParty config.toml 是否存在、default_model 是否配置
#       3. 端口冲突      7778 端口是否被占用
#       4. 数据库        clawparty.db 的 PRAGMA integrity_check
#       5. 服务状态      clawparty 进程是否存在
#       6. 工作区        各 Agent 的 workspace 目录是否存在且可写
#       7. LLM 连通性    通过 curl 测试各 Agent 的 API 端点（check 模式可选）
#
#   check-fix    先执行 check 全部检测，再自动修复可修复的问题：
#
#       修复项：
#         - 拷贝缺失的二进制文件到 /usr/local/bin
#         - 生成缺失的 0#Agent / ClawParty 默认配置文件
#         - 创建缺失的 Agent 工作区目录并修正权限
#         - 非修复项（如 api_key 未填）标记 warning，需手动处理
#
#
#   安装产出
#   --------
#     /usr/local/bin/clawparty            CLI 二进制（内嵌 Web GUI）
#     /usr/local/bin/zeroclaw             ZeroClaw agent runtime
#     /usr/local/bin/ztm                  ZTM mesh networking
#     /usr/local/bin/opencode             OpenCode CLI
#     /Applications/ClawPartyDesktop.app  macOS 菜单栏 App
#     ~/.config/clawparty/config.toml     配置文件
#     ~/.clawparty/                       运行时数据目录
#
#===============================================================================
set -euo pipefail

# Clear quarantine on this script itself (macOS blocks downloaded .command files)
if command -v xattr &> /dev/null && [ -f "$0" ]; then
    xattr -cr "$0" 2>/dev/null || true
fi

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REPO_DIR="$SCRIPT_DIR"
BIN_DIR="/usr/local/bin"
CONFIG_DIR="${XDG_CONFIG_HOME:-$HOME/.config}/clawparty"
CLAWPARTY_HOME="${CLAWPARTY_HOME:-$HOME/.clawparty}"
PID_FILE="$CLAWPARTY_HOME/clawparty.pid"
DEFAULT_PORT=7778

# ── Colors (terminal-aware) ───────────────────────────────────────
if [ -t 1 ]; then
    GREEN='\033[0;32m'
    RED='\033[0;31m'
    YELLOW='\033[1;33m'
    CYAN='\033[0;36m'
    BOLD='\033[1m'
    NC='\033[0m'
else
    GREEN='' RED='' YELLOW='' CYAN='' BOLD='' NC=''
fi

# ── Helpers ───────────────────────────────────────────────────────
header() {
    echo ""
    echo -e "${CYAN}=========================================${NC}"
    echo -e "${CYAN}  $1${NC}"
    echo -e "${CYAN}=========================================${NC}"
}

info()   { echo -e "  ${CYAN}ℹ${NC}  $1"; }
warn()   { echo -e "  ${YELLOW}⚠${NC}  $1"; }
pass()   { echo -e "  ${GREEN}✓${NC} $1"; }
fail()   { echo -e "  ${RED}✗${NC} $1"; }

error_exit() {
    fail "$1"
    exit 1
}

confirm() {
    if ${NON_INTERACTIVE:-false}; then
        return 0
    fi
    local prompt="$1"
    local default="${2:-y}"
    local yn
    if [ "$default" = "y" ]; then
        read -r -p "  $prompt [Y/n] " yn
        [[ "$yn" != "n" && "$yn" != "N" ]]
    else
        read -r -p "  $prompt [y/N] " yn
        [[ "$yn" = "y" || "$yn" = "Y" ]]
    fi
}

# ── Shell detection ───────────────────────────────────────────────
detect_shell_profile() {
    local shell_name
    shell_name=$(basename "${SHELL:-/bin/zsh}")
    case "$shell_name" in
        zsh)  echo "${ZDOTDIR:-$HOME}/.zshrc" ;;
        fish) echo "$HOME/.config/fish/config.fish" ;;
        *)    echo "$HOME/.bashrc" ;;
    esac
}

detect_shell_name() { basename "${SHELL:-/bin/zsh}"; }

path_export_cmd() {
    case "$(detect_shell_name)" in
        fish) echo "fish_add_path $BIN_DIR" ;;
        *)    echo "export PATH=\"$BIN_DIR:\$PATH\"" ;;
    esac
}

add_to_path() {
    local config_file="$1" cmd="$2"
    [ ! -f "$config_file" ] && return 1
    grep -qxF "$cmd" "$config_file" 2>/dev/null && return 0
    if [ -w "$config_file" ]; then
        printf '\n# ClawParty\n%s\n' "$cmd" >> "$config_file"
        info "Added ClawParty to PATH in $config_file"
        return 0
    fi
    warn "Cannot write to $config_file — add manually:  $cmd"
    return 1
}

configure_path() {
    local config_file
    config_file=$(detect_shell_profile)
    add_to_path "$config_file" "$(path_export_cmd)" || {
        for f in "$HOME/.zshrc" "$HOME/.bash_profile" "$HOME/.bashrc" "$HOME/.config/fish/config.fish"; do
            add_to_path "$f" "$(path_export_cmd)" && return 0
        done
        warn "No writable shell config found. Add to PATH manually:"
        warn "  $(path_export_cmd)"
    }
}

# ── Binary helpers ────────────────────────────────────────────────
is_binary_in_path() {
    local name="$1"
    local env_path="/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin"
    PATH="$env_path" command -v "$name" > /dev/null 2>&1
}

find_binary_source() {
    local name="$1"
    local candidates=()

    # Search from current directory (pwd) and its subdirectories
    local dir
    for dir in "$(pwd)" "$(pwd)/bin" "$(pwd)/src/cli/target/release" \
                "$(pwd)/src/tui/target/release" "$(pwd)/zeroclaw/target/release" \
                "$(pwd)/ztm/bin"; do
        candidates+=("$dir/$name")
    done

    for path in "${candidates[@]}"; do
        [ -f "$path" ] && echo "$path" && return 0
    done

    # Fallback: opencode not found in current dir → check ~/.opencode/bin/
    if [ "$name" = "opencode" ]; then
        local oc_path="$HOME/.opencode/bin/opencode"
        if [ -f "$oc_path" ]; then
            echo "$oc_path"
            return 0
        fi
    fi

    return 1
}

find_clawparty_binary() {
    local paths=(
        "$BIN_DIR/clawparty"
        "$(pwd)/clawparty"
        "$(pwd)/bin/clawparty"
        "$(pwd)/src/cli/target/release/clawparty"
        "$(pwd)/src/tui/target/release/clawparty"
    )
    for p in "${paths[@]}"; do
        [ -x "$p" ] && echo "$p" && return 0
    done
    # fallback to PATH
    local found
    found=$(PATH="/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:$PATH" command -v clawparty 2>/dev/null) || true
    [ -n "$found" ] && echo "$found" && return 0
    return 1
}

# ── Port helpers ──────────────────────────────────────────────────
is_port_in_use() {
    lsof -i ":$1" -P -n 2>/dev/null | grep -q LISTEN
}

port_process_info() {
    local info
    info=$(lsof -i ":$1" -P -n 2>/dev/null | grep LISTEN | awk '{print $1 "(PID:" $2 ")"}' | paste -sd ',' -)
    echo "$info"
}

# ── Process helpers ───────────────────────────────────────────────
is_clawparty_running() {
    pgrep -x clawparty > /dev/null 2>&1
}

clawparty_pids() {
    pgrep -x clawparty 2>/dev/null || true
}

# ====================================================================
#  COMMAND: help
# ====================================================================
cmd_help() {
    cat << EOF
${BOLD}ClawParty macOS Installer & Manager${NC}

Usage: $0 <command> [options]

${BOLD}Commands:${NC}
  ${GREEN}install${NC}     Install ClawParty and all dependencies
  ${GREEN}start${NC}       Start the ClawParty service in the background
  ${GREEN}stop${NC}        Stop the running ClawParty service
  ${GREEN}restart${NC}     Restart the ClawParty service
  ${GREEN}status${NC}      Check whether ClawParty is currently running
  ${GREEN}check${NC}       Run system readiness checks (diagnostics only)
  ${GREEN}check-fix${NC}   Run checks and attempt to fix detected issues
  ${GREEN}uninstall${NC}   Remove ClawParty from the system
  ${GREEN}help${NC}        Show this help message

${BOLD}Install options:${NC} (for ${GREEN}install${NC} command)
  --skip-desktop      Skip installing the Desktop menu bar app
  --prefix PATH       Install prefix (default: /usr/local)
  --no-modify-path    Do not modify shell config files

${BOLD}Start options:${NC} (for ${GREEN}start${NC} command)
  --foreground        Run in foreground instead of detaching
  --port PORT         Override default service port (default: 7778)

${BOLD}Examples:${NC}
  ./clawparty-macos.sh install                           # Install binaries from current dir
  ./clawparty-macos.sh install --skip-desktop --prefix ~/.local
  ./clawparty-macos.sh start                             # Start service
  ./clawparty-macos.sh start --port 8080                 # Start on custom port
  ./clawparty-macos.sh status                            # Check if running
  ./clawparty-macos.sh check                             # Diagnostic scan
  ./clawparty-macos.sh check-fix                         # Scan + auto-repair
  ./clawparty-macos.sh restart                           # Restart service
  ./clawparty-macos.sh stop                              # Stop service
  ./clawparty-macos.sh uninstall                         # Remove completely
EOF
    exit 0
}

# ====================================================================
#  COMMAND: install
# ====================================================================
cmd_install() {
    # ── Parse args ─────────────────────────────────────────────────
    SKIP_DESKTOP=false
    INSTALL_PREFIX="/usr/local"
    NO_MODIFY_PATH=false

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --skip-desktop)   SKIP_DESKTOP=true; shift ;;
            --no-modify-path) NO_MODIFY_PATH=true; shift ;;
            --prefix)
                INSTALL_PREFIX="$2"; shift 2 ;;
            --prefix=*)
                INSTALL_PREFIX="${1#*=}"; shift ;;
            *)
                echo "Unknown install option: $1"; echo "Use: $0 help"; exit 1 ;;
        esac
    done

    BIN_DIR="$INSTALL_PREFIX/bin"

    # Source directory for binaries — same dir as this script
    # Works for release layout (script + binaries in same bin/) and
    # repo layout (script in scripts/, binaries in ../bin/)
    SRC_DIR="$(cd "$(dirname "$0")" && pwd)"
    # If SRC_DIR doesn't contain the binaries, try ../bin relative to script
    if [ ! -f "$SRC_DIR/clawparty" ] && [ -f "$SCRIPT_DIR/bin/clawparty" ]; then
        SRC_DIR="$SCRIPT_DIR/bin"
    fi

    # ── OS check ───────────────────────────────────────────────────
    if [[ "$(uname)" != "Darwin" ]]; then
        error_exit "This installer is for macOS only. Found: $(uname)"
    fi
    info "macOS version: $(sw_vers -productVersion 2>/dev/null || echo 'unknown')"
    info "Source directory: $SRC_DIR"

    # ── Check source files exist ────────────────────────────────────
    header "Checking Source Files"
    local need=("clawparty")
    local extras=("zeroclaw" "ztm" "opencode")
    local all_found=true

    for name in "${need[@]}"; do
        if [ -f "$SRC_DIR/$name" ]; then
            pass "$name"
        else
            fail "$name not found in $SRC_DIR/"
            all_found=false
        fi
    done
    for name in "${extras[@]}"; do
        if [ -f "$SRC_DIR/$name" ]; then
            pass "$name"
        else
            info "$name not found — skipping (optional)"
        fi
    done

    if ! $all_found; then
        error_exit "Required binary 'clawparty' not found in $SRC_DIR/. Run build.sh first."
    fi

    # ── Install binaries ───────────────────────────────────────────
    header "Installing Binaries"
    mkdir -p "$BIN_DIR"

    install_one() {
        local name="$1" src="$SRC_DIR/$name" dst="$BIN_DIR/$name"
        [ -f "$src" ] || return 0
        cp -f "$src" "$dst"
        chmod 755 "$dst"
        if command -v xattr &> /dev/null; then
            xattr -cr "$dst" 2>/dev/null || true
        fi
        if command -v codesign &> /dev/null; then
            codesign -s - --force --deep "$dst" 2>/dev/null || true
        fi
        pass "$name → $dst"
    }

    install_one "clawparty"
    install_one "zeroclaw"
    install_one "ztm"
    install_one "opencode"

    if ! $NO_MODIFY_PATH; then
        if ! echo "$PATH" | tr ':' '\n' | grep -qxF "$BIN_DIR"; then
            configure_path
        fi
    fi

    # ── Install Desktop app ────────────────────────────────────────
    if $SKIP_DESKTOP; then
        info "Skipping Desktop app (--skip-desktop)"
    elif [ -d "$SRC_DIR/ClawPartyDesktop.app" ]; then
        header "Installing Desktop App"
        local app_dst="/Applications/ClawPartyDesktop.app"
        [ -d "$app_dst" ] && rm -rf "$app_dst"
        cp -R "$SRC_DIR/ClawPartyDesktop.app" "$app_dst"
        if command -v xattr &> /dev/null; then
            xattr -cr "$app_dst" 2>/dev/null || true
        fi
        pass "ClawPartyDesktop.app → /Applications/"
    fi

    # ── Setup config ───────────────────────────────────────────────
    header "Setting Up Configuration"
    mkdir -p "$CONFIG_DIR"
    local config_file="$CONFIG_DIR/config.toml"
    if [ ! -f "$config_file" ]; then
        cat > "$config_file" << 'TOML'
# ClawParty Configuration

[core]
default_model = "deepseek-v4-pro"

[ui]
theme = "dark"

[zeroclaw]
gateway_addr = "127.0.0.1:9070"

[ztm]
endpoint = "127.0.0.1:7777"
TOML
        pass "Created $config_file"
    else
        info "Config already exists: $config_file"
    fi

    # ── Verify ─────────────────────────────────────────────────────
    header "Verifying Installation"
    local all_ok=true

    verify_bin() {
        if [ -f "$BIN_DIR/$1" ]; then
            local ver
            ver=$("$BIN_DIR/$1" --version 2>/dev/null || echo "version check failed")
            pass "$1: $ver"
        else
            fail "$1: not found"
            all_ok=false
        fi
    }

    verify_bin "clawparty"
    for name in zeroclaw ztm opencode; do
        [ -f "$BIN_DIR/$name" ] && verify_bin "$name"
    done

    if ! $SKIP_DESKTOP; then
        if [ -d "/Applications/ClawPartyDesktop.app" ]; then
            pass "Desktop app: /Applications/ClawPartyDesktop.app"
        fi
    fi

    echo ""
    $all_ok && pass "All components verified" || warn "Some components failed verification."

    # ── Summary ────────────────────────────────────────────────────
    echo ""
    echo -e "${GREEN}=========================================${NC}"
    echo -e "${GREEN}  ClawParty Installation Complete${NC}"
    echo -e "${GREEN}=========================================${NC}"
    echo ""
    echo -e "  ${BOLD}Binaries:${NC}     $BIN_DIR/"
    for name in clawparty zeroclaw ztm opencode; do
        [ -f "$BIN_DIR/$name" ] && echo "    $name"
    done
    if [ -d "/Applications/ClawPartyDesktop.app" ]; then
        echo ""
        echo -e "  ${BOLD}Desktop:${NC}     /Applications/ClawPartyDesktop.app"
    fi
    echo ""
    echo -e "  ${BOLD}Config:${NC}      $CONFIG_DIR/"
    echo ""
    echo -e "  ${BOLD}Getting Started:${NC}"
    echo "    clawparty-macos.sh start"
    echo "    clawparty-macos.sh status"
    echo ""
    if ! echo "$PATH" | tr ':' '\n' | grep -qxF "$BIN_DIR"; then
        echo -e "  ${YELLOW}⚠ ${BOLD}Add to PATH:${NC}"
        echo "    $(path_export_cmd)"
        echo "    source $(detect_shell_profile)"
        echo ""
    fi
}

# ====================================================================
#  COMMAND: start
# ====================================================================
cmd_start() {
    local FOREGROUND=false
    local PORT="$DEFAULT_PORT"

    while [[ $# -gt 0 ]]; do
        case "$1" in
            --foreground) FOREGROUND=true; shift ;;
            --port)
                PORT="$2"; shift 2 ;;
            --port=*)
                PORT="${1#*=}"; shift ;;
            *)
                echo "Unknown start option: $1"; exit 1 ;;
        esac
    done

    header "Starting ClawParty"

    # Check if already running
    if is_clawparty_running; then
        local pids
        pids=$(clawparty_pids | paste -sd ',' -)
        warn "ClawParty is already running (PID: $pids)"
        status_short
        return 0
    fi

    if is_port_in_use "$PORT"; then
        local info
        info=$(port_process_info "$PORT")
        warn "Port $PORT is already in use by: $info"
        if ! confirm "Proceed anyway?"; then
            return 1
        fi
    fi

    # Find binary
    local binary
    binary=$(find_clawparty_binary) || error_exit "clawparty binary not found. Run '$0 install' first."

    info "Using binary: $binary"
    info "Service port: $PORT"

    # Clear quarantine on all sibling binaries (macOS blocks unsigned downloads)
    if command -v xattr &> /dev/null; then
        local bin_dir
        bin_dir="$(dirname "$binary")"
        for f in "$bin_dir"/clawparty "$bin_dir"/zeroclaw "$bin_dir"/ztm "$bin_dir"/opencode; do
            if [ -f "$f" ]; then
                xattr -cr "$f" 2>/dev/null && info "Cleared quarantine on $(basename "$f")" || true
            fi
        done
    fi

    # Ensure config directories exist
    mkdir -p "$CLAWPARTY_HOME"

    # Build environment
    export PATH="/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin:$PATH"
    export CLAWPARTY_PORT="$PORT"

    if $FOREGROUND; then
        info "Running in foreground (press Ctrl+C to stop)"
        exec "$binary" -s --engine=opencode --no-ztm
    else
        # Detached background
        nohup "$binary" -s --engine=opencode --no-ztm > "$CLAWPARTY_HOME/clawparty.log" 2>&1 &
        local pid=$!
        echo "$pid" > "$PID_FILE"

        # Wait briefly to see if it crashes immediately
        sleep 2
        if kill -0 "$pid" 2>/dev/null; then
            pass "ClawParty started (PID: $pid)"
            info "Logs: $CLAWPARTY_HOME/clawparty.log"
            info "Config: $CLAWPARTY_HOME/"
        else
            fail "ClawParty failed to start. Check logs:"
            if [ -f "$CLAWPARTY_HOME/clawparty.log" ]; then
                tail -20 "$CLAWPARTY_HOME/clawparty.log"
            fi
            rm -f "$PID_FILE"
            return 1
        fi
    fi
}

# ====================================================================
#  COMMAND: stop
# ====================================================================
cmd_stop() {
    header "Stopping ClawParty"

    if ! is_clawparty_running; then
        info "ClawParty is not running"
        rm -f "$PID_FILE"
        return 0
    fi

    local pids
    pids=$(clawparty_pids)
    for pid in $pids; do
        info "Sending TERM to PID $pid"
        kill -TERM "$pid" 2>/dev/null || true
    done

    # Wait up to 10s for graceful shutdown
    local waited=0
    while is_clawparty_running && [ $waited -lt 10 ]; do
        sleep 1
        waited=$((waited + 1))
    done

    # Force kill if still running
    if is_clawparty_running; then
        warn "Graceful shutdown timed out, force-killing..."
        pkill -9 -x clawparty 2>/dev/null || true
        sleep 1
    fi

    if is_clawparty_running; then
        fail "Failed to stop ClawParty"
        return 1
    fi

    rm -f "$PID_FILE"
    pass "ClawParty stopped"
}

# ====================================================================
#  COMMAND: restart
# ====================================================================
cmd_restart() {
    header "Restarting ClawParty"
    cmd_stop
    echo ""
    cmd_start "$@"
}

# ====================================================================
#  COMMAND: status
# ====================================================================
status_short() {
    if is_clawparty_running; then
        local pids
        pids=$(clawparty_pids | paste -sd ',' -)
        local port_info=""
        if is_port_in_use "$DEFAULT_PORT"; then
            port_info=" (port $DEFAULT_PORT)"
        fi
        echo -e "  ${GREEN}●${NC} ClawParty is running${port_info} — PID: $pids"
    else
        echo -e "  ${RED}○${NC} ClawParty is not running"

        if is_port_in_use "$DEFAULT_PORT"; then
            local info
            info=$(port_process_info "$DEFAULT_PORT")
            warn "Port $DEFAULT_PORT is in use by: $info"
        fi
    fi
}

cmd_status() {
    header "ClawParty Status"
    status_short
    echo ""

    # Show process details
    if is_clawparty_running; then
        echo -e "  ${BOLD}Process details:${NC}"
        ps aux | grep -v grep | grep clawparty | while read -r line; do
            echo "    $line"
        done
        echo ""
        echo -e "  ${BOLD}Recent logs:${NC}"
        if [ -f "$CLAWPARTY_HOME/clawparty.log" ]; then
            tail -10 "$CLAWPARTY_HOME/clawparty.log" | while read -r line; do
                echo "    $line"
            done
        else
            info "No log file found"
        fi
    fi

    # Quick binary check
    echo ""
    echo -e "  ${BOLD}Binary:${NC}"
    if find_clawparty_binary > /dev/null 2>&1; then
        pass "clawparty: $(find_clawparty_binary)"
    else
        fail "clawparty not found in PATH or known locations"
    fi
}

# ====================================================================
#  COMMAND: check / check-fix
# ====================================================================
CHECK_FIX_MODE=false

run_checks() {
    header "System Readiness Checks"
    local problem_count=0
    local fixable_count=0
    local passed_count=0

    # ── 1. Binary checks ───────────────────────────────────────────
    echo ""
    echo -e "${BOLD}── Binaries ──${NC}"
    local binaries=("clawparty" "zeroclaw" "ztm" "opencode")
    for binary in "${binaries[@]}"; do
        printf "  %-20s " "$binary"
        if is_binary_in_path "$binary"; then
            local found_path
            found_path=$(PATH="/usr/local/bin:/opt/homebrew/bin:/usr/bin:/bin" command -v "$binary")
            local ver=""
            ver=$("$found_path" --version 2>/dev/null | head -1) || true
            echo -e "${GREEN}✓${NC} $found_path ${ver:+($ver)}"
            passed_count=$((passed_count + 1))
        elif src=$(find_binary_source "$binary"); then
            echo -e "${YELLOW}⚠${NC}  Found at $src, not in PATH"
            problem_count=$((problem_count + 1))
            fixable_count=$((fixable_count + 1))
        else
            echo -e "${RED}✗${NC} Not found"
            problem_count=$((problem_count + 1))
        fi
    done

    # ── 2. Config file check ───────────────────────────────────────
    echo ""
    echo -e "${BOLD}── Configuration ──${NC}"

    # 0#Agent opencode.json
    local zero_config="$CLAWPARTY_HOME/agents/0#Agent/opencode.json"
    printf "  %-40s " "0#Agent opencode.json"
    if [ -f "$zero_config" ]; then
        echo -e "${GREEN}✓${NC} $zero_config"
        passed_count=$((passed_count + 1))
        # Check key fields in JSON
        local model_val api_key_set
        if command -v python3 &> /dev/null; then
            model_val=$(python3 -c "
import json
try:
    with open('$zero_config') as f:
        c = json.load(f)
    print(c.get('model',''))
except: pass
" 2>/dev/null)
            api_key_set=$(python3 -c "
import json
try:
    with open('$zero_config') as f:
        c = json.load(f)
    for p in c.get('provider',{}).values():
        if p.get('options',{}).get('apiKey',''):
            print('yes')
            break
except: pass
" 2>/dev/null)
        fi
        printf "    %-36s " "model"
        if [ -n "$model_val" ] && [ "$model_val" != "default/default" ]; then
            echo -e "${GREEN}✓${NC} $model_val"
        else
            echo -e "${YELLOW}⚠${NC}  not set or default"
            problem_count=$((problem_count + 1))
        fi
        printf "    %-36s " "apiKey"
        if [ -n "$api_key_set" ]; then
            echo -e "${GREEN}✓${NC} Configured"
        else
            echo -e "${YELLOW}⚠${NC}  not set"
            problem_count=$((problem_count + 1))
        fi
    else
        echo -e "${RED}✗${NC} Not found: $zero_config"
        problem_count=$((problem_count + 1))
        fixable_count=$((fixable_count + 1))
    fi

    local clawparty_config="$CONFIG_DIR/config.toml"
    printf "  %-40s " "ClawParty config"
    if [ -f "$clawparty_config" ]; then
        echo -e "${GREEN}✓${NC} $clawparty_config"
        passed_count=$((passed_count + 1))
    else
        echo -e "${YELLOW}⚠${NC}  Not found"
        problem_count=$((problem_count + 1))
        fixable_count=$((fixable_count + 1))
    fi

    # ── 3. Port conflicts ─────────────────────────────────────────
    echo ""
    echo -e "${BOLD}── Ports ──${NC}"
    printf "  %-20s " "Port $DEFAULT_PORT"
    if is_port_in_use "$DEFAULT_PORT"; then
        local info
        info=$(port_process_info "$DEFAULT_PORT")
        if is_clawparty_running; then
            echo -e "${GREEN}✓${NC} In use by ClawParty"
            passed_count=$((passed_count + 1))
        else
            echo -e "${YELLOW}⚠${NC}  In use by: $info"
            problem_count=$((problem_count + 1))
        fi
    else
        echo -e "${CYAN}ℹ${NC}  Available"
    fi

    # ── 4. Database ─────────────────────────────────────────────────
    echo ""
    echo -e "${BOLD}── Database ──${NC}"
    local db_path="$CLAWPARTY_HOME/clawparty.db"

    printf "  %-40s " "clawparty.db"
    if [ -f "$db_path" ]; then
        echo -e "${GREEN}✓${NC} exists"
        passed_count=$((passed_count + 1))

        if ! command -v sqlite3 &> /dev/null; then
            echo -e "  ${YELLOW}⚠${NC}  sqlite3 not installed, skipping table checks"
        else
            local integrity
            integrity=$(sqlite3 "$db_path" "PRAGMA integrity_check;" 2>&1)
            printf "  %-40s " "Integrity"
            if [ "$integrity" = "ok" ]; then
                echo -e "${GREEN}✓${NC} $integrity"
                passed_count=$((passed_count + 1))
            else
                echo -e "${RED}✗${NC} $integrity"
                problem_count=$((problem_count + 1))
            fi

            local admin_exists
            admin_exists=$(sqlite3 "$db_path" "SELECT COUNT(*) FROM users WHERE username='admin';" 2>/dev/null)
            printf "  %-40s " "Admin user"
            if [ "${admin_exists:-0}" -gt 0 ]; then
                echo -e "${GREEN}✓${NC} admin exists"
                passed_count=$((passed_count + 1))
            else
                echo -e "${YELLOW}⚠${NC}  no admin user"
                problem_count=$((problem_count + 1))
                fixable_count=$((fixable_count + 1))
            fi

            local agent_exists
            agent_exists=$(sqlite3 "$db_path" "SELECT COUNT(*) FROM agents WHERE agent_name='0#Agent' AND deleted=0;" 2>/dev/null)
            printf "  %-40s " "0#Agent record"
            if [ "${agent_exists:-0}" -gt 0 ]; then
                echo -e "${GREEN}✓${NC} registered"
                passed_count=$((passed_count + 1))
            else
                echo -e "${YELLOW}⚠${NC}  not registered"
                problem_count=$((problem_count + 1))
                fixable_count=$((fixable_count + 1))
            fi
        fi
    else
        echo -e "${YELLOW}⚠${NC}  Not yet created"
        problem_count=$((problem_count + 1))
        fixable_count=$((fixable_count + 1))
    fi

    # ── 5. Process status ─────────────────────────────────────────
    echo ""
    echo -e "${BOLD}── Service ──${NC}"
    printf "  %-40s " "ClawParty service"
    if is_clawparty_running; then
        local pids
        pids=$(clawparty_pids | paste -sd ',' -)
        echo -e "${GREEN}✓${NC} Running (PID: $pids)"
        passed_count=$((passed_count + 1))
    else
        echo -e "${CYAN}ℹ${NC}  Not running"
    fi

    # ── 6. Workspace directories ───────────────────────────────────
    echo ""
    echo -e "${BOLD}── Workspaces ──${NC}"
    local agents_dir="$CLAWPARTY_HOME/agents"
    if [ -d "$agents_dir" ]; then
        for agent_dir in "$agents_dir"/*/; do
            [ ! -d "$agent_dir" ] && continue
            local agent_name
            agent_name=$(basename "$agent_dir")
            local ws="$agent_dir/workspace"
            printf "  %-40s " "$agent_name workspace"
            if [ -d "$ws" ]; then
                if [ -w "$ws" ]; then
                    echo -e "${GREEN}✓${NC} $ws (writable)"
                    passed_count=$((passed_count + 1))
                else
                    echo -e "${RED}✗${NC} $ws (not writable)"
                    problem_count=$((problem_count + 1))
                    fixable_count=$((fixable_count + 1))
                fi
            else
                echo -e "${YELLOW}⚠${NC}  Not found"
                problem_count=$((problem_count + 1))
                fixable_count=$((fixable_count + 1))
            fi
        done
    else
        echo -e "  ${CYAN}ℹ${NC}  No agent directories found"
    fi

    # ── 7. OpenCode permissions ─────────────────────────────────────
    echo ""
    echo -e "${BOLD}── OpenCode Permissions ──${NC}"

    # Global config: ~/.config/opencode/opencode.jsonc
    local oc_global_config="${XDG_CONFIG_HOME:-$HOME/.config}/opencode/opencode.jsonc"
    printf "  %-40s " "Global opencode.jsonc"
    if [ -f "$oc_global_config" ]; then
        if grep -q 'clawparty/agents' "$oc_global_config" 2>/dev/null; then
            echo -e "${GREEN}✓${NC} ~/.clawparty/agents/** allowed"
            passed_count=$((passed_count + 1))
        else
            echo -e "${YELLOW}⚠${NC}  Missing ~/.clawparty/agents/** permission"
            problem_count=$((problem_count + 1))
            fixable_count=$((fixable_count + 1))
        fi
    else
        echo -e "${YELLOW}⚠${NC}  Not found"
        problem_count=$((problem_count + 1))
        fixable_count=$((fixable_count + 1))
    fi

    # Repo-level config: <repo>/opencode/.opencode/opencode.jsonc
    local oc_repo_config="$REPO_DIR/opencode/.opencode/opencode.jsonc"
    printf "  %-40s " "Repo opencode.jsonc"
    if [ -f "$oc_repo_config" ]; then
        if grep -q 'clawparty/agents' "$oc_repo_config" 2>/dev/null; then
            echo -e "${GREEN}✓${NC} ~/.clawparty/agents/** allowed"
            passed_count=$((passed_count + 1))
        else
            echo -e "${YELLOW}⚠${NC}  Missing ~/.clawparty/agents/** permission"
            problem_count=$((problem_count + 1))
            fixable_count=$((fixable_count + 1))
        fi
    else
        echo -e "${CYAN}ℹ${NC}  Not found (optional)"
    fi

    # ── Summary ────────────────────────────────────────────────────
    echo ""
    echo -e "${BOLD}─────────────────────────────────────${NC}"
    local total=$((passed_count + problem_count))
    echo "  Checked: $total | ${GREEN}Passed: $passed_count${NC} | ${RED}Problems: $problem_count${NC} | Fixable: $fixable_count"
    echo ""

    if $CHECK_FIX_MODE; then
        run_fixes
    fi

    return $problem_count
}

run_fixes() {
    header "Auto-Fixing Issues"
    local fixed=0
    local failed=0
    local target_dir="$BIN_DIR"

    # ── Fix 1: Copy missing binaries ───────────────────────────────
    echo -e "${BOLD}  Copying binaries to $target_dir...${NC}"
    local binaries=("clawparty" "zeroclaw" "ztm" "opencode")
    for binary in "${binaries[@]}"; do
        if is_binary_in_path "$binary"; then
            continue
        fi
        if src=$(find_binary_source "$binary"); then
            printf "    %-20s " "$binary"
            mkdir -p "$target_dir"
            if cp -f "$src" "$target_dir/$binary" 2>/dev/null; then
                chmod 755 "$target_dir/$binary"
                if command -v xattr &> /dev/null; then
                    xattr -cr "$target_dir/$binary" 2>/dev/null || true
                fi
                if command -v codesign &> /dev/null; then
                    codesign -s - --force --deep "$target_dir/$binary" 2>/dev/null || true
                fi
                echo -e "${GREEN}✓${NC} Copied from $src"
                fixed=$((fixed + 1))
            else
                echo -e "${RED}✗${NC} Permission denied — try with sudo"
                failed=$((failed + 1))
            fi
        else
            printf "    %-20s " "$binary"
            echo -e "${RED}✗${NC} Source not found — rebuild needed"
            failed=$((failed + 1))
        fi
    done

    # ── Fix 2: Generate default configs ────────────────────────────
    echo ""
    echo -e "${BOLD}  Generating configs...${NC}"

    # 0#Agent opencode.json
    local zero_agent_dir="$CLAWPARTY_HOME/agents/0#Agent"
    local zero_config="$zero_agent_dir/opencode.json"
    if [ ! -f "$zero_config" ]; then
        mkdir -p "$zero_agent_dir"
        cat > "$zero_config" << 'JSONC'
{
  "$schema": "https://opencode.ai/config.json",
  "model": "",
  "provider": {},
  "permission": {
    "external_directory": {
      "~/.clawparty/agents/**": "allow"
    }
  }
}
JSONC
        pass "Created $zero_config"
        fixed=$((fixed + 1))
    fi

    # ClawParty config
    local cp_config="$CONFIG_DIR/config.toml"
    if [ ! -f "$cp_config" ]; then
        mkdir -p "$CONFIG_DIR"
        cat > "$cp_config" << 'TOML'
# ClawParty Configuration

[core]
default_model = "deepseek-v4-pro"

[ui]
theme = "dark"

[zeroclaw]
gateway_addr = "127.0.0.1:9070"

[ztm]
endpoint = "127.0.0.1:7777"
TOML
        pass "Created $cp_config"
        fixed=$((fixed + 1))
    fi

    # ── Fix 3: Create workspace directories ────────────────────────
    echo ""
    echo -e "${BOLD}  Creating workspace directories...${NC}"
    local agents_dir="$CLAWPARTY_HOME/agents"
    if [ -d "$agents_dir" ]; then
        for agent_dir in "$agents_dir"/*/; do
            [ ! -d "$agent_dir" ] && continue
            local agent_name
            agent_name=$(basename "$agent_dir")
            local ws="$agent_dir/workspace"
            if [ ! -d "$ws" ]; then
                mkdir -p "$ws" 2>/dev/null && {
                    pass "Created $ws"
                    fixed=$((fixed + 1))
                } || {
                    fail "Cannot create $ws"
                    failed=$((failed + 1))
                }
            elif [ ! -w "$ws" ]; then
                chmod u+w "$ws" 2>/dev/null && {
                    pass "Fixed permissions for $ws"
                    fixed=$((fixed + 1))
                } || {
                    fail "Cannot fix permissions for $ws"
                    failed=$((failed + 1))
                }
            fi
        done
    else
        info "No agent dirs found at $agents_dir"
    fi

    # ── Fix 4: Database initialization ──────────────────────────────
    echo ""
    echo -e "${BOLD}  Initializing database...${NC}"
    local db_path="$CLAWPARTY_HOME/clawparty.db"

    _init_clawparty_db() {
        sqlite3 "$db_path" << 'SQL'
PRAGMA journal_mode = WAL;
PRAGMA synchronous = NORMAL;
PRAGMA busy_timeout = 5000;

CREATE TABLE IF NOT EXISTS tasks (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id       TEXT    UNIQUE NOT NULL,
    agent_name    TEXT    NOT NULL,
    group_id      TEXT,
    parent_id     TEXT,
    title         TEXT    NOT NULL,
    short_title   TEXT,
    description   TEXT,
    ai_description TEXT,
    status        TEXT    NOT NULL DEFAULT 'pending',
    progress      INTEGER NOT NULL DEFAULT 0,
    priority      TEXT    NOT NULL DEFAULT 'normal',
    dependencies  TEXT,
    task_number   INTEGER,
    result_summary TEXT,
    prompt        TEXT,
    is_pipeline   INTEGER NOT NULL DEFAULT 0,
    pipeline_definition TEXT,
    created_at    REAL    NOT NULL,
    updated_at    REAL    NOT NULL,
    started_at    REAL,
    completed_at  REAL
);
CREATE INDEX IF NOT EXISTS idx_tasks_agent ON tasks(agent_name);
CREATE INDEX IF NOT EXISTS idx_tasks_group ON tasks(group_id);
CREATE INDEX IF NOT EXISTS idx_tasks_parent ON tasks(parent_id);
CREATE INDEX IF NOT EXISTS idx_tasks_status ON tasks(status);
CREATE INDEX IF NOT EXISTS idx_tasks_task_id ON tasks(task_id);
CREATE INDEX IF NOT EXISTS idx_tasks_number ON tasks(task_number);

CREATE TABLE IF NOT EXISTS task_events (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    task_id     TEXT    NOT NULL,
    event_type  TEXT    NOT NULL,
    from_status TEXT,
    to_status   TEXT,
    progress    INTEGER,
    message     TEXT,
    timestamp   REAL    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_task_events_task ON task_events(task_id);
CREATE INDEX IF NOT EXISTS idx_task_events_timestamp ON task_events(timestamp);

CREATE TABLE IF NOT EXISTS task_analysis_log (
    id                INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name        TEXT    NOT NULL,
    group_id          TEXT,
    last_analyzed_at  REAL    NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_tal_agent ON task_analysis_log(agent_name);
CREATE INDEX IF NOT EXISTS idx_tal_group ON task_analysis_log(group_id);

CREATE TABLE IF NOT EXISTS kanban_configs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    agent_name  TEXT    NOT NULL,
    group_id    TEXT,
    name        TEXT,
    prompt      TEXT,
    config      TEXT,
    updated_at  REAL    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_kanban_agent ON kanban_configs(agent_name);
CREATE INDEX IF NOT EXISTS idx_kanban_group ON kanban_configs(group_id);

CREATE TABLE IF NOT EXISTS agents (
    agent_name      TEXT PRIMARY KEY,
    display_name    TEXT,
    description     TEXT,
    directory       TEXT NOT NULL,
    config_path     TEXT NOT NULL,
    workspace_dir   TEXT NOT NULL,
    port            INTEGER NOT NULL,
    pid             INTEGER,
    status          TEXT NOT NULL DEFAULT 'stopped',
    created_at      REAL    NOT NULL,
    updated_at      REAL    NOT NULL,
    config_json     TEXT,
    error_msg       TEXT,
    deleted         INTEGER NOT NULL DEFAULT 0,
    engine          TEXT NOT NULL DEFAULT 'zeroclaw'
);
CREATE INDEX IF NOT EXISTS idx_agents_status ON agents(status);
CREATE INDEX IF NOT EXISTS idx_agents_deleted ON agents(deleted);

CREATE TABLE IF NOT EXISTS group_chats (
    group_id      TEXT PRIMARY KEY,
    group_name    TEXT    NOT NULL,
    owner_agent   TEXT    NOT NULL,
    members       TEXT    NOT NULL,
    created_at    REAL    NOT NULL,
    updated_at    REAL    NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_groupchats_owner ON group_chats(owner_agent);

CREATE TABLE IF NOT EXISTS users (
    username      TEXT PRIMARY KEY,
    password_hash TEXT NOT NULL,
    salt          TEXT NOT NULL,
    api_token     TEXT NOT NULL,
    share_token   TEXT NOT NULL DEFAULT '',
    role          TEXT NOT NULL DEFAULT 'user',
    created_at    REAL NOT NULL DEFAULT (strftime('%s', 'now')),
    expire        REAL NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS chat_log (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    time        REAL    NOT NULL,
    mesh        TEXT    NOT NULL,
    chat_type   TEXT    NOT NULL,
    chat_id     TEXT    NOT NULL,
    chat_name   TEXT,
    creator     TEXT,
    sender      TEXT    NOT NULL,
    event       TEXT    NOT NULL,
    content     TEXT,
    members     TEXT,
    session_id  TEXT,
    muted       INTEGER NOT NULL DEFAULT 0,
    msg_type    TEXT    NOT NULL DEFAULT 'response'
);
CREATE INDEX IF NOT EXISTS idx_chatlog_chat ON chat_log(chat_id);

-- Migrations
ALTER TABLE agents ADD COLUMN engine TEXT NOT NULL DEFAULT 'zeroclaw';
ALTER TABLE users ADD COLUMN share_token TEXT NOT NULL DEFAULT '';
SQL
    }

    # Create DB schema if file missing
    if [ ! -f "$db_path" ]; then
        if ! command -v sqlite3 &> /dev/null; then
            warn "sqlite3 not available — cannot create database"
        else
            mkdir -p "$CLAWPARTY_HOME"
            _init_clawparty_db 2>/dev/null && {
                pass "Created clawparty.db with full schema"
                fixed=$((fixed + 1))
            } || {
                fail "Failed to create clawparty.db"
                failed=$((failed + 1))
            }
        fi
    fi

    # Check/insert admin user
    if [ -f "$db_path" ] && command -v sqlite3 &> /dev/null; then
        local admin_exists
        admin_exists=$(sqlite3 "$db_path" "SELECT COUNT(*) FROM users WHERE username='admin';" 2>/dev/null)
        if [ "${admin_exists:-0}" -eq 0 ]; then
            local admin_pass salt hash api_token created_at
            admin_pass=$(LC_ALL=C tr -dc 'a-zA-Z0-9' < /dev/urandom 2>/dev/null | head -c 16 || python3 -c "import secrets; print(secrets.token_urlsafe(12)[:16])" 2>/dev/null)
            salt=$(LC_ALL=C tr -dc 'a-zA-Z0-9' < /dev/urandom 2>/dev/null | head -c 16 || python3 -c "import secrets; print(secrets.token_urlsafe(12)[:16])" 2>/dev/null)
            created_at=$(date +%s)
            hash=$(printf '%s%s' "$salt" "$admin_pass" | shasum -a 256 2>/dev/null | awk '{print $1}')
            if [ -z "$hash" ] && command -v python3 &> /dev/null; then
                hash=$(python3 -c "import hashlib; print(hashlib.sha256(('${salt}${admin_pass}').encode()).hexdigest())")
            fi
            api_token=$(LC_ALL=C tr -dc 'a-zA-Z0-9' < /dev/urandom 2>/dev/null | head -c 32 || python3 -c "import secrets; print(secrets.token_urlsafe(24)[:32])" 2>/dev/null)

            sqlite3 "$db_path" \
                "INSERT INTO users (username, password_hash, salt, api_token, role, created_at, expire)
                 VALUES ('admin', '$hash', '$salt', '$api_token', 'admin', $created_at, 0);" 2>/dev/null && {
                pass "Created admin user"
                echo ""
                echo -e "  ${BOLD}========================================${NC}"
                echo -e "  ${BOLD}  Admin credentials (save these!)${NC}"
                echo -e "  ${BOLD}========================================${NC}"
                echo -e "  ${BOLD}Username:${NC} admin"
                echo -e "  ${BOLD}Password:${NC} $admin_pass"
                echo -e "  ${BOLD}API Token:${NC} $api_token"
                echo -e "  ${BOLD}========================================${NC}"
                echo ""
                fixed=$((fixed + 1))
            } || {
                fail "Failed to create admin user"
                failed=$((failed + 1))
            }
        fi

        # Check/insert 0#Agent record
        local agent_exists now
        agent_exists=$(sqlite3 "$db_path" "SELECT COUNT(*) FROM agents WHERE agent_name='0#Agent';" 2>/dev/null)
        if [ "${agent_exists:-0}" -eq 0 ]; then
            now=$(date +%s)
            local agent_dir="$CLAWPARTY_HOME/agents/0#Agent"
            sqlite3 "$db_path" \
                "INSERT INTO agents (agent_name, display_name, description, directory, config_path, workspace_dir, port, status, created_at, updated_at, engine)
                 VALUES ('0#Agent', 'Zerus(0#Agent)', 'Primary orchestrator agent',
                         '$agent_dir', '$agent_dir/opencode.json', '$agent_dir/workspace',
                         42617, 'stopped', $now, $now, 'opencode');" 2>/dev/null && {
                pass "Registered 0#Agent in database"
                fixed=$((fixed + 1))
            } || {
                warn "Failed to register 0#Agent in database"
            }
        fi
    fi

    # ── Fix 5: OpenCode external_directory permission ───────────────
    echo ""
    echo -e "${BOLD}  Configuring OpenCode permissions...${NC}"

    local oc_global_config="${XDG_CONFIG_HOME:-$HOME/.config}/opencode/opencode.jsonc"
    local oc_global_dir
    oc_global_dir=$(dirname "$oc_global_config")

    _fix_opencode_config() {
        local config_file="$1" label="$2"
        if [ ! -f "$config_file" ]; then
            mkdir -p "$(dirname "$config_file")"
            cat > "$config_file" << 'JSONC'
{
  "$schema": "https://opencode.ai/config.json",
  "model": "",
  "provider": {},
  "permission": {
    "external_directory": {
      "~/.clawparty/agents/**": "allow"
    }
  }
}
JSONC
            pass "Created $label with ~/.clawparty/agents/** permission"
            return 0
        elif grep -q 'clawparty/agents' "$config_file" 2>/dev/null; then
            pass "$label already has ~/.clawparty/agents/** permission"
            return 1
        elif command -v python3 &> /dev/null; then
            python3 -c "
import re, json, os, sys
path = os.path.expanduser('$config_file')
with open(path) as f:
    raw = f.read()
# Strip JSONC comments
cleaned = re.sub(r'//.*', '', raw)
try:
    config = json.loads(cleaned)
except json.JSONDecodeError:
    config = {}
config.setdefault('permission', {}).setdefault('external_directory', {})['~/.clawparty/agents/**'] = 'allow'
with open(path, 'w') as f:
    json.dump(config, f, indent=2)
    f.write('\n')
" 2>/dev/null && {
                pass "Added ~/.clawparty/agents/** to $label"
                return 0
            } || {
                warn "Failed to update $label with python3"
                return 2
            }
        else
            warn "$label exists but python3 not available to modify it"
            warn "  Manually add to permission.external_directory: ~/.clawparty/agents/** = allow"
            return 2
        fi
    }

    _fix_opencode_config "$oc_global_config" "Global opencode.jsonc" && fixed=$((fixed + 1)) || true

    local oc_repo_config="$REPO_DIR/opencode/.opencode/opencode.jsonc"
    if [ -f "$oc_repo_config" ] && ! grep -q 'clawparty/agents' "$oc_repo_config" 2>/dev/null; then
        _fix_opencode_config "$oc_repo_config" "Repo opencode.jsonc" && fixed=$((fixed + 1)) || true
    fi

    # ── Summary ────────────────────────────────────────────────────
    echo ""
    echo -e "${BOLD}─────────────────────────────────────${NC}"
    echo -e "  Fixed: ${GREEN}$fixed${NC} | Failed: ${RED}$failed${NC}"
    echo ""

    if [ "$failed" -gt 0 ]; then
        warn "Some fixes require elevated privileges. Try: sudo $0 check-fix"
    fi
}

cmd_check() {
    CHECK_FIX_MODE=false
    run_checks
}

cmd_check_fix() {
    CHECK_FIX_MODE=true
    run_checks
}

# ====================================================================
#  COMMAND: uninstall
# ====================================================================
cmd_uninstall() {
    header "Uninstalling ClawParty"

    # Stop first
    if is_clawparty_running; then
        info "Stopping ClawParty..."
        cmd_stop > /dev/null 2>&1
    fi

    local removed=0

    # Remove binaries
    for bin in clawparty zeroclaw ztm opencode; do
        if [ -f "$BIN_DIR/$bin" ]; then
            rm -f "$BIN_DIR/$bin"
            pass "Removed $BIN_DIR/$bin"
            removed=$((removed + 1))
        fi
    done

    # Remove PID file
    rm -f "$PID_FILE"

    # Remove config
    if [ -d "$CONFIG_DIR" ]; then
        if confirm "Remove configuration directory $CONFIG_DIR?"; then
            rm -rf "$CONFIG_DIR"
            pass "Removed $CONFIG_DIR"
            removed=$((removed + 1))
        fi
    fi

    # Remove ~/.clawparty
    if [ -d "$CLAWPARTY_HOME" ]; then
        if confirm "Remove data directory $CLAWPARTY_HOME?"; then
            rm -rf "$CLAWPARTY_HOME"
            pass "Removed $CLAWPARTY_HOME"
            removed=$((removed + 1))
        fi
    fi

    # Remove Desktop app
    if [ -d "/Applications/ClawPartyDesktop.app" ]; then
        if confirm "Remove /Applications/ClawPartyDesktop.app?"; then
            rm -rf "/Applications/ClawPartyDesktop.app"
            pass "Removed /Applications/ClawPartyDesktop.app"
            removed=$((removed + 1))
        fi
    fi

    if [ $removed -eq 0 ]; then
        warn "No ClawParty installation found."
    else
        echo ""
        pass "Uninstall complete."
    fi
}

# ====================================================================
#  Interactive Menu (shown when run without arguments)
# ====================================================================
interactive_menu() {
    while true; do
        clear 2>/dev/null || printf '\033[2J\033[H'

        echo ""
        echo -e "${CYAN}╔═══════════════════════════════════════╗${NC}"
        echo -e "${CYAN}║${NC}        ${BOLD}ClawParty macOS${NC}              ${CYAN}║${NC}"
        echo -e "${CYAN}╚═══════════════════════════════════════╝${NC}"
        echo ""

        # Status line
        if is_clawparty_running; then
            local pids
            pids=$(clawparty_pids | paste -sd ',' -)
            echo -e "  ${BOLD}Status:${NC} ${GREEN}● Running${NC} (PID: $pids)"
        else
            echo -e "  ${BOLD}Status:${NC} ${RED}○ Stopped${NC}"
        fi

        # Binary check
        local bin_status=""
        if find_clawparty_binary > /dev/null 2>&1; then
            bin_status="${GREEN}Found${NC}"
        else
            bin_status="${RED}Not found${NC}"
        fi
        echo -e "  ${BOLD}Binary:${NC} $bin_status"
        echo ""

        echo -e "  ${BOLD}─── Actions ───${NC}"
        echo ""
        echo -e "  ${GREEN}[1]${NC}  ${BOLD}Start ClawParty${NC}       启动服务"
        if is_clawparty_running; then
            echo -e "  ${YELLOW}[2]${NC}  ${BOLD}Stop ClawParty${NC}        停止服务"
        fi
        echo -e "  ${CYAN}[3]${NC}  ${BOLD}Check System${NC}          系统检查"
        echo -e "  ${CYAN}[4]${NC}  ${BOLD}Check & Fix${NC}           检查并修复"
        echo ""
        echo -e "  [q]  Quit                     退出"
        echo ""
        printf "  ${BOLD}Select:${NC} "
        read -r choice

        case "$choice" in
            1)
                clear 2>/dev/null || true
                cmd_start
                echo ""
                printf "Press Enter to continue..."
                read -r _
                ;;
            2)
                if is_clawparty_running; then
                    clear 2>/dev/null || true
                    cmd_stop
                    echo ""
                    printf "Press Enter to continue..."
                    read -r _
                else
                    echo -e "  ${RED}Invalid choice${NC}"
                    sleep 1
                fi
                ;;
            3)
                clear 2>/dev/null || true
                cmd_check
                echo ""
                printf "Press Enter to continue..."
                read -r _
                ;;
            4)
                clear 2>/dev/null || true
                cmd_check_fix
                echo ""
                printf "Press Enter to continue..."
                read -r _
                ;;
            q|Q)
                echo ""
                echo -e "  ${CYAN}Goodbye.${NC}"
                exit 0
                ;;
            *)
                echo -e "  ${RED}Invalid choice — press Enter to try again${NC}"
                read -r _
                ;;
        esac
    done
}

# ====================================================================
#  MAIN — command dispatch
# ====================================================================
# No arguments → interactive menu (double-click launch)
if [ $# -eq 0 ]; then
    interactive_menu
    exit 0
fi

COMMAND="$1"
shift 2>/dev/null || true

case "$COMMAND" in
    install)   cmd_install "$@" ;;
    start)     cmd_start "$@" ;;
    stop)      cmd_stop "$@" ;;
    restart)   cmd_restart "$@" ;;
    status)    cmd_status "$@" ;;
    check)     cmd_check "$@" ;;
    check-fix) cmd_check_fix "$@" ;;
    uninstall) cmd_uninstall "$@" ;;
    menu)      interactive_menu ;;
    help|-h|--help) cmd_help ;;
    *)
        echo "Unknown command: $COMMAND"
        echo "Usage: $0 <install|start|stop|restart|status|check|check-fix|uninstall|menu|help>"
        echo "Run '$0 help' for details."
        exit 1
        ;;
esac
