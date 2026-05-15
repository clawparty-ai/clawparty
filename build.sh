#!/bin/bash

set -e

ZTM_DIR=$(cd "$(dirname "$0")" && pwd)
GUI_DIR="$ZTM_DIR/chat-gui"

CLEAN=false
BUILD_ZTM=false
BUILD_ZEROCLAW=false
BUILD_TUI=false
BUILD_PIPY=false

# Determine build targets from arguments
has_positional=false
for arg in "$@"; do
  case $arg in
    --clean|-c)
      CLEAN=true
      ;;
    --ztm-only|-z)
      # Legacy flag: only skip Rust binaries, build everything else
      BUILD_ZTM=true
      ;;
    ztm|all)
      has_positional=true
      BUILD_ZTM=true
      BUILD_ZEROCLAW=true
      BUILD_TUI=true
      BUILD_PIPY=true
      ;;
    zeroclaw)
      has_positional=true
      BUILD_ZEROCLAW=true
      ;;
    tui)
      has_positional=true
      BUILD_TUI=true
      ;;
    pipy)
      has_positional=true
      BUILD_PIPY=true
      ;;
  esac
done

# Default: build everything if no positional arg was given
if [ "$has_positional" = false ]; then
  BUILD_ZTM=true
  BUILD_ZEROCLAW=true
  BUILD_TUI=true
  BUILD_PIPY=true
fi

# Legacy --ztm-only with no positional arg also triggers full ZTM build
if [ "$BUILD_ZTM" = false ] && [ "$BUILD_ZEROCLAW" = false ] && [ "$BUILD_TUI" = false ] && [ "$BUILD_PIPY" = false ]; then
  BUILD_ZTM=true
  BUILD_ZEROCLAW=true
  BUILD_TUI=true
  BUILD_PIPY=true
fi

if [ ! -d "$GUI_DIR" ] && [ -d "$ZTM_DIR/gui" ]; then
  GUI_DIR="$ZTM_DIR/gui"
fi

if [ ! -d "$GUI_DIR" ]; then
  echo "Cannot find GUI directory (expected chat-gui/ or gui/), exit..."
  exit 1
fi

if [ "$CLEAN" = true ]; then
  echo "=== Clean build ==="
  rm -rf "$ZTM_DIR/pipy/build"
  rm -rf "$ZTM_DIR/tui/target"
  rm -rf "$ZTM_DIR/zeroclaw/target"
  rm -rf "$ZTM_DIR/bin"
  rm -rf "$ZTM_DIR/chat-gui/node_modules"
  rm -rf "$ZTM_DIR/chat-gui/dist"
fi

# GUI is always built when ZTM is requested (no standalone GUI target)
if [ "$BUILD_ZTM" = true ] || [ "$BUILD_PIPY" = true ]; then
  cd "$GUI_DIR"
  yarn install

  cd "$ZTM_DIR"
  build/deps.sh

  cd "$ZTM_DIR"
  build/gui.sh
fi

# Build ZeroClaw (Rust)
if [ "$BUILD_ZEROCLAW" = true ]; then
  echo "Building ZeroClaw..."
  cd "$ZTM_DIR/zeroclaw"
  mkdir -p "$HOME/.clawparty/.zeroclaw"
  cargo build --release --features gateway
  mkdir -p "$ZTM_DIR/bin"
  cp -f "$ZTM_DIR/zeroclaw/target/release/zeroclaw" "$ZTM_DIR/bin/zeroclaw"
  if [ "$(uname)" = "Darwin" ]; then
    codesign -s - --force --deep "$ZTM_DIR/bin/zeroclaw" 2>/dev/null || true
  fi
  echo "ZeroClaw built: $ZTM_DIR/bin/zeroclaw"
fi

# Build TUI (Rust)
if [ "$BUILD_TUI" = true ]; then
  echo "Building TUI..."
  cd "$ZTM_DIR/tui"
  cargo build --release
  mkdir -p "$ZTM_DIR/bin"
  cp -f "$ZTM_DIR/tui/target/release/clawparty" "$ZTM_DIR/bin/clawparty"
  if [ "$(uname)" = "Darwin" ]; then
    codesign -s - --force --deep "$ZTM_DIR/bin/clawparty" 2>/dev/null || true
  fi
  echo "TUI built: $ZTM_DIR/bin/clawparty"
fi

# Build ZTM agent (via pipy.sh which bundles agent JS into bin/ztm)
# NOTE: plain pipy build is skipped, but pipy.sh (which builds the packaged ztm binary) is preserved.
if [ "$BUILD_PIPY" = true ]; then
  cd "$ZTM_DIR"
  build/pipy.sh
fi

# # Build plain pipy binary (disabled — not needed for ZTM agent)
# if [ "$BUILD_PIPY" = true ]; then
#   echo "Building plain pipy..."
#   mkdir -p "$ZTM_DIR/pipy/build-plain"
#   cd "$ZTM_DIR/pipy/build-plain"
#   cmake .. \
#     -DCMAKE_BUILD_TYPE=Release \
#     -DCMAKE_C_COMPILER=clang \
#     -DCMAKE_CXX_COMPILER=clang++ \
#     -DPIPY_GUI=OFF \
#     -DPIPY_SAMPLE_CODEBASES=OFF
#   make -j2
#   mkdir -p "$ZTM_DIR/bin"
#   cp -f "$ZTM_DIR/pipy/bin/pipy" "$ZTM_DIR/bin/pipy"
#   echo "Plain pipy built: $ZTM_DIR/bin/pipy"
#   rm -rf "$ZTM_DIR/pipy/build-plain"
# fi

echo "=== Build complete ==="
