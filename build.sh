#!/bin/bash

set -e

ZTM_DIR=$(cd "$(dirname "$0")" && pwd)
GUI_DIR="$ZTM_DIR/chat-gui"

CLEAN=false
for arg in "$@"; do
  case $arg in
    --clean|-c)
      CLEAN=true
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

# Build TUI (Rust)
echo "Building TUI..."
cd "$ZTM_DIR/tui"
cargo build --release
mkdir -p "$ZTM_DIR/bin"
cp -f "$ZTM_DIR/tui/target/release/clawparty" "$ZTM_DIR/bin/clawparty"
# Ad-hoc sign the binary on macOS to avoid Gatekeeper issues
if [ "$(uname)" = "Darwin" ]; then
  codesign -s - --force --deep "$ZTM_DIR/bin/clawparty" 2>/dev/null || true
fi
echo "TUI built: $ZTM_DIR/bin/clawparty"

cd "$ZTM_DIR"
build/pipy.sh
