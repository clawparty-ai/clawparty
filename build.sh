#!/bin/bash

set -e

ZTM_DIR=$(cd "$(dirname "$0")" && pwd)
GUI_DIR="$ZTM_DIR/chat-gui"

if [ ! -d "$GUI_DIR" ] && [ -d "$ZTM_DIR/gui" ]; then
  GUI_DIR="$ZTM_DIR/gui"
fi

if [ ! -d "$GUI_DIR" ]; then
  echo "Cannot find GUI directory (expected chat-gui/ or gui/), exit..."
  exit 1
fi

cd "$GUI_DIR"
npm ci --no-audit

cd "$ZTM_DIR"
build/deps.sh

cd "$ZTM_DIR"
build/gui.sh
build/pipy.sh
