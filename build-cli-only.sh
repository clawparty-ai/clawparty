#!/bin/bash

ZTM_DIR=$(cd "$(dirname "$0")" && pwd)

CLEAN=false
for arg in "$@"; do
  case $arg in
    --clean|-c)
      CLEAN=true
      ;;
  esac
done

if [ "$CLEAN" = true ]; then
  echo "=== Clean build ==="
  rm -rf "$ZTM_DIR/pipy/build"
  rm -rf "$ZTM_DIR/tui/target"
  rm -rf "$ZTM_DIR/zeroclaw/target"
  rm -rf "$ZTM_DIR/bin"
fi

cd "$ZTM_DIR"
git clean -X -f agent

cd "$ZTM_DIR"
build/deps.sh

if [ $? -ne 0 ]; then
  echo "Prepare deps failed, exit..."
  exit 1
fi

cd "$ZTM_DIR"
build/pipy.sh

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

echo "Building TUI..."
cd "$ZTM_DIR/tui"
cargo build --release
mkdir -p "$ZTM_DIR/bin"
cp -f "$ZTM_DIR/tui/target/release/clawparty" "$ZTM_DIR/bin/clawparty"
if [ "$(uname)" = "Darwin" ]; then
  codesign -s - --force --deep "$ZTM_DIR/bin/clawparty" 2>/dev/null || true
fi
echo "TUI built: $ZTM_DIR/bin/clawparty"
