#!/bin/bash

set -e

ZTM_DIR=$(cd "$(dirname "$0")" && cd .. && pwd)

cd "$ZTM_DIR"
git submodule update --init || true

if [ ! -f "$ZTM_DIR/pipy/CMakeLists.txt" ]; then
  echo "Pipy submodule is unavailable, cloning pipy repository directly..."
  rm -rf "$ZTM_DIR/pipy"
  git clone --depth 1 --branch v2 https://github.com/flomesh-io/pipy.git "$ZTM_DIR/pipy"
fi

if [ ! -f "$ZTM_DIR/pipy/CMakeLists.txt" ]; then
  echo "Cannot prepare Pipy source at $ZTM_DIR/pipy"
  exit 1
fi

cd "$ZTM_DIR"

if test -e "hub/cluster.js"; then
  EDITION="Enterprise"
else
  EDITION="Community"
fi

if [ -n "$ZTM_VERSION" ]; then
  VERSION="$ZTM_VERSION"
else
  VERSION=$(git describe --abbrev=0 --tags 2>/dev/null || echo "dev")
fi

COMMIT=$(git log -1 --format=%H 2>/dev/null || echo "unknown")
COMMIT_DATE=$(git log -1 --format=%cD 2>/dev/null || date -u)

VERSION_JSON="{
  \"edition\":\"$EDITION\",
  \"tag\": \"$VERSION\",
  \"commit\": \"$COMMIT\",
  \"date\": \"$COMMIT_DATE\"
}"

echo "$VERSION_JSON" > cli/version.json
echo "$VERSION_JSON" > hub/version.json
echo "$VERSION_JSON" > agent/version.json

echo "VERSION=\"$VERSION\"" > version.env
echo "COMMIT=\"$COMMIT\"" >> version.env
echo "COMMIT_DATE=\"$COMMIT_DATE\"" >> version.env
