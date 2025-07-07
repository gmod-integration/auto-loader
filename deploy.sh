#!/bin/bash

set -e

VERSION_FILE="./version.txt"

# Check if version file exists, if not create it with version 1
if [ ! -f "$VERSION_FILE" ]; then
  echo "1" > "$VERSION_FILE"
fi

# Read the current version
VERSION=$(cat "$VERSION_FILE")

LOCAL_FILE_REAL="$HOME/gmod-integration/auto-loader/target/i686-unknown-linux-gnu/release/libgmod_integration.so"
REMOTE_FILE_REAL="/var/lib/pterodactyl/volumes/4541b777-af44-4f01-b9fe-097133365fa9/garrysmod/lua/bin/gmsv_gmod_integration_dev_${VERSION}_linux.dll"

LOCAL_FILE_LOADER="$HOME/gmod-integration/auto-loader/target/i686-unknown-linux-gnu/release/libgmod_integration_loader.so"
REMOTE_FILE_LOADER="/var/lib/pterodactyl/volumes/4541b777-af44-4f01-b9fe-097133365fa9/garrysmod/lua/bin/gmsv_gmod_integration_dev_loader_${VERSION}_linux.dll"

REMOTE_HOST="ptero"

echo "Building project..."

cd ~/gmod-integration/auto-loader/crates/loader
cargo build --release --target i686-unknown-linux-gnu

cd ~/gmod-integration/auto-loader/crates/real
cargo build --release --target i686-unknown-linux-gnu

echo "Uploading $LOCAL_FILE_REAL to $REMOTE_HOST:$REMOTE_FILE_REAL"
scp "$LOCAL_FILE_REAL" "$REMOTE_HOST:$REMOTE_FILE_REAL"

echo "Uploading $LOCAL_FILE_LOADER to $REMOTE_HOST:$REMOTE_FILE_LOADER"
scp "$LOCAL_FILE_LOADER" "$REMOTE_HOST:$REMOTE_FILE_LOADER"

cd ~/gmod-integration/auto-loader
# Increment the version
NEW_VERSION=$((VERSION + 1))
echo "$NEW_VERSION" > "$VERSION_FILE"

echo "Deployment complete! Version updated to $NEW_VERSION."
