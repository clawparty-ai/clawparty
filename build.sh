#!/bin/bash

set -e

ZTM_DIR=$(cd "$(dirname "$0")" && pwd)
GUI_DIR="$ZTM_DIR/chat-gui"

CLEAN=false
ZTM_ONLY=false
for arg in "$@"; do
  case $arg in
    --clean|-c)
      CLEAN=true
      ;;
    --ztm-only|-z)
      ZTM_ONLY=true
      ;;
  esac
done

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

cd "$GUI_DIR"
npm ci --no-audit

cd "$ZTM_DIR"
build/deps.sh

cd "$ZTM_DIR"
build/gui.sh

# Build Rust binaries before pipy.sh so its package step can include them.
# (Skipped in --ztm-only mode.)
if [ "$ZTM_ONLY" != true ]; then
  # Build ZeroClaw (Rust)
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

  # Build TUI (Rust)
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

cd "$ZTM_DIR"
build/pipy.sh

if [ "$ZTM_ONLY" = true ]; then
  echo "=== ZTM only build complete ==="
  exit 0
fi
