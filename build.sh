#!/bin/bash
#===============================================================================
# ClawParty Build Script
#===============================================================================
#
# Usage: ./build.sh [options]
#
# Options (all optional builds are off by default):
#   --desktop      Build macOS desktop app (Swift 5.9+, macOS only)
#   --ztm          Build ZTM mesh networking (ztm/build.sh)
#   --zeroclaw     Build ZeroClaw agent runtime (cargo build --release --features gateway)
#   --opencode     Build OpenCode CLI binary (bun install + bun run build)
#   --all          Shortcut for --desktop --ztm --zeroclaw --opencode
#   --clean        Remove all build artifacts before building
#   -h, --help     Show help message
#
# Default (no flags): Web (Vite) → CLI (Rust, embeds web via rust-embed)
#
# Outputs: bin/
#   bin/clawparty            CLI binary with embedded Web GUI
#   bin/ztm                  ZTM binary (with --ztm)
#   bin/zeroclaw             ZeroClaw binary (with --zeroclaw)
#   bin/opencode             OpenCode CLI binary (with --opencode)
#   bin/ClawPartyDesktop.app macOS menu bar app (with --desktop)
#===============================================================================
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
BIN_DIR="$SCRIPT_DIR/bin"

BUILD_DESKTOP=false
BUILD_ZTM=false
BUILD_ZEROCLAW=false
BUILD_OPENCODE=false
BUILD_ALL=false
CLEAN=false

# ── Parse arguments ──────────────────────────────────────────────
usage() {
    echo "Usage: $0 [options]"
    echo ""
    echo "Build ClawParty components."
    echo ""
    echo "Options (all optional sub-module builds are off by default):"
    echo "  --desktop      Build macOS desktop app (Swift/macOS only)"
    echo "  --ztm          Build ZTM (ztm/build.sh)"
    echo "  --zeroclaw     Build ZeroClaw (cargo build --release --features gateway)"
    echo "  --opencode     Build OpenCode CLI binary (bun install + bun run build)"
    echo "  --clean        Remove build artifacts before building"
    echo "  --all          Build all components (shortcut for --desktop --ztm --zeroclaw --opencode)"
    echo "  -h, --help     Show this help message"
    echo ""
    echo "Default: builds Web → CLI only (sub-modules require explicit flags)."
    exit 0
}

for arg in "$@"; do
    case "$arg" in
        --desktop)  BUILD_DESKTOP=true ;;
        --ztm)      BUILD_ZTM=true ;;
        --zeroclaw) BUILD_ZEROCLAW=true ;;
        --opencode) BUILD_OPENCODE=true ;;
        --clean)    CLEAN=true ;;
        --all)
            BUILD_ALL=true
            BUILD_DESKTOP=true
            BUILD_ZTM=true
            BUILD_ZEROCLAW=true
            BUILD_OPENCODE=true
            ;;
        -h|--help)  usage ;;
        *)
            echo "Unknown option: $arg"
            echo "Use -h for usage."
            exit 1
            ;;
    esac
done

# ── Helpers ──────────────────────────────────────────────────────
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

check_step() {
    local name="$1"
    if [ $? -eq 0 ]; then
        echo -e "  ${GREEN}✓${NC} $name"
    else
        echo -e "  ${RED}✗${NC} $name"
    fi
}

header() {
    echo ""
    echo "========================================="
    echo "  $1"
    echo "========================================="
}

# ── Clean ────────────────────────────────────────────────────────
if $CLEAN; then
    header "Cleaning build artifacts"
    rm -rf "$BIN_DIR"
    rm -rf "$SCRIPT_DIR/src/web/node_modules"
    rm -rf "$SCRIPT_DIR/src/web/dist"
    rm -rf "$SCRIPT_DIR/src/cli/target"
    rm -rf "$SCRIPT_DIR/src/cli/gui"
    rm -rf "$SCRIPT_DIR/src/desktop/build"
    rm -rf "$SCRIPT_DIR/src/desktop/.build"
    rm -rf "$SCRIPT_DIR/ztm/bin"
    rm -rf "$SCRIPT_DIR/zeroclaw/target"
    rm -rf "$SCRIPT_DIR/opencode/packages/opencode/dist"
    rm -rf "$SCRIPT_DIR/src/tui"

    echo "  ${GREEN}✓${NC} Clean complete"
fi

# ── 0. Init submodules ───────────────────────────────────────────
if [ -f "$SCRIPT_DIR/.gitmodules" ] && $BUILD_ZTM || $BUILD_ZEROCLAW || $BUILD_OPENCODE; then
    echo "  Initializing submodules..."
    git -C "$SCRIPT_DIR" submodule update --init 2>/dev/null || true
fi

# ── 1. Build Web (Vue) ──────────────────────────────────────────
# Skip when --opencode is the ONLY flag (single binary build, no clawparty)
if ! ( $BUILD_OPENCODE && ! $BUILD_ZEROCLAW && ! $BUILD_ZTM && ! $BUILD_DESKTOP && ! $BUILD_ALL ); then
header "Building Web (Vue)"

# Symlink: vite --outDir ../tui/gui expects src/tui/ to exist
rm -rf "$SCRIPT_DIR/src/tui"
ln -sf cli "$SCRIPT_DIR/src/tui"

cd "$SCRIPT_DIR/src/web"

if [ -f "package.json" ]; then
    if [ -d "node_modules" ]; then
        if ! node -e "require('vite')" 2>/dev/null; then
            echo "  Corrupted node_modules detected, cleaning..."
            rm -rf node_modules
        fi
    fi

    yarn install
    check_step "yarn install"

    yarn build
    check_step "yarn build ($(yarn -s run env echo '$npm_package_version') → $SCRIPT_DIR/src/cli/gui/)"
else
    echo -e "  ${YELLOW}⚠${NC}  package.json not found — skipping Web"
fi

# ── 2. Build CLI (Rust) ─────────────────────────────────────────
header "Building CLI (Rust)"
cd "$SCRIPT_DIR/src/cli"

if [ -f "Cargo.toml" ]; then
    cargo build --release
    check_step "cargo build --release"

    mkdir -p "$BIN_DIR"
    cp -f "$SCRIPT_DIR/src/cli/target/release/clawparty" "$BIN_DIR/clawparty"
    if [ "$(uname)" = "Darwin" ]; then
        codesign -s - --force --deep "$BIN_DIR/clawparty" 2>/dev/null || true
    fi
    check_step "clawparty → $BIN_DIR/clawparty"
else
    echo -e "  ${YELLOW}⚠${NC}  Cargo.toml not found — skipping CLI"
fi
fi  # end of ! (opencode-only) guard for Web + CLI

# ── 3. Build ZTM (C++ via pipy) ──────────────────────────────────
if $BUILD_ZTM; then
    header "Building ZTM"
    cd "$SCRIPT_DIR/ztm"

    if [ -f "build.sh" ]; then
        ./build.sh
        check_step "ztm/build.sh"

        # Copy output binary
        ZTM_BIN="$SCRIPT_DIR/ztm/bin/ztm"
        if [ -f "$ZTM_BIN" ]; then
            mkdir -p "$BIN_DIR"
            cp -f "$ZTM_BIN" "$BIN_DIR/ztm"
            check_step "ztm → $BIN_DIR/ztm"
        else
            echo -e "  ${YELLOW}⚠${NC}  ztm binary not found at expected path"
        fi
    else
        echo -e "  ${RED}✗${NC} build.sh not found in ztm/"
        exit 1
    fi
fi

# ── 4. Build ZeroClaw (Rust) ─────────────────────────────────────
if $BUILD_ZEROCLAW; then
    header "Building ZeroClaw"
    cd "$SCRIPT_DIR/zeroclaw"

    if [ -f "Cargo.toml" ]; then
        mkdir -p "$HOME/.clawparty/.zeroclaw"
        cargo build --release --features gateway
        check_step "cargo build --release --features gateway"

        mkdir -p "$BIN_DIR"
        cp -f "$SCRIPT_DIR/zeroclaw/target/release/zeroclaw" "$BIN_DIR/zeroclaw"
        if [ "$(uname)" = "Darwin" ]; then
            codesign -s - --force --deep "$BIN_DIR/zeroclaw" 2>/dev/null || true
        fi
        check_step "zeroclaw → $BIN_DIR/zeroclaw"
    else
        echo -e "  ${RED}✗${NC} Cargo.toml not found in zeroclaw/"
        exit 1
    fi
fi

# ── 5. Build OpenCode (Bun) ──────────────────────────────────────
if $BUILD_OPENCODE; then
    header "Building OpenCode"

    if ! command -v bun &> /dev/null; then
        echo -e "  ${RED}✗${NC} bun not found. Install via: curl -fsSL https://bun.sh/install | bash"
        exit 1
    fi

    cd "$SCRIPT_DIR/opencode"

    if [ -f "package.json" ]; then
        # Install only the opencode package (skip electron desktop app)
        # --ignore-scripts: skip electron postinstall which downloads platform binaries
        bun install --frozen-lockfile --filter=opencode --ignore-scripts
        check_step "bun install --filter=opencode"

        bun run --cwd packages/opencode build
        check_step "bun run build (packages/opencode)"

        # Copy platform-specific binary
        PLATFORM=""
        case "$(uname -s)" in
            Darwin) PLATFORM="darwin-$(uname -m | sed 's/x86_64/x64/;s/arm64/arm64/')" ;;
            Linux)  PLATFORM="linux-$(uname -m | sed 's/x86_64/x64/;s/aarch64/arm64/')" ;;
        esac
        OPENCODE_BIN="$SCRIPT_DIR/opencode/packages/opencode/dist/opencode-${PLATFORM}/bin/opencode"
        if [ -f "$OPENCODE_BIN" ]; then
            mkdir -p "$BIN_DIR"
            cp -f "$OPENCODE_BIN" "$BIN_DIR/opencode"
            check_step "opencode → $BIN_DIR/opencode"
        else
            echo -e "  ${YELLOW}⚠${NC}  opencode binary not found at expected path ($OPENCODE_BIN)"
        fi
    else
        echo -e "  ${RED}✗${NC} package.json not found in opencode/"
        exit 1
    fi
fi

# ── 6. Build Desktop (macOS) ─────────────────────────────────────
if $BUILD_DESKTOP; then
    header "Building Desktop (macOS)"

    if [[ "$(uname)" != "Darwin" ]]; then
        echo -e "  ${RED}✗${NC} Desktop build requires macOS. Found: $(uname)"
        exit 1
    fi

    if ! command -v swift &> /dev/null; then
        echo -e "  ${RED}✗${NC} Swift not found. Install Xcode or Swift toolchain."
        exit 1
    fi

    cd "$SCRIPT_DIR/src/desktop"
    if [ -f "build.sh" ]; then
        ./build.sh
        check_step "desktop build"

        DESKTOP_SRC="$SCRIPT_DIR/src/desktop/build/ClawPartyDesktop.app"
        if [ -d "$DESKTOP_SRC" ]; then
            DESKTOP_DST="$BIN_DIR/ClawPartyDesktop.app"
            rm -rf "$DESKTOP_DST"
            cp -R "$DESKTOP_SRC" "$DESKTOP_DST"
            if command -v xattr &> /dev/null; then
                xattr -cr "$DESKTOP_DST" 2>/dev/null || true
            fi
            check_step "ClawPartyDesktop.app → $DESKTOP_DST"
        fi
    else
        echo -e "  ${RED}✗${NC} build.sh not found in src/desktop/"
        exit 1
    fi
fi

# ── Summary ──────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}=========================================${NC}"
echo -e "${GREEN}  Build complete${NC}"
echo -e "${GREEN}=========================================${NC}"
echo ""
echo "  CLI:      $BIN_DIR/clawparty"
echo "  Web:      src/cli/gui/ (embedded in CLI binary)"
if $BUILD_ZTM; then
    echo "  ZTM:      $BIN_DIR/ztm"
fi
if $BUILD_ZEROCLAW; then
    echo "  ZeroClaw: $BIN_DIR/zeroclaw"
fi
if $BUILD_OPENCODE; then
    echo "  OpenCode: $BIN_DIR/opencode"
fi
if $BUILD_DESKTOP; then
    echo "  Desktop:  $BIN_DIR/ClawPartyDesktop.app"
fi
echo ""
