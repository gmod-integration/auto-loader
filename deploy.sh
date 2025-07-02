#!/bin/bash

set -e

VERSION_FILE="./version.txt"

# Check if version file exists, if not create it with version 1
if [ ! -f "$VERSION_FILE" ]; then
  echo "1" > "$VERSION_FILE"
fi

# Read the current version
VERSION=$(cat "$VERSION_FILE")

NAMEFILE="gmsv_gmod_integration_${VERSION}_linux.dll"

LOCAL_FILE="./target/i686-unknown-linux-gnu/debug/libgmod_integration.so"
REMOTE_FILE="/var/lib/pterodactyl/volumes/4541b777-af44-4f01-b9fe-097133365fa9/garrysmod/lua/bin/$NAMEFILE"
REMOTE_HOST="ptero"

echo "Building project..."
cargo build --target i686-unknown-linux-gnu

echo "Uploading $LOCAL_FILE to $REMOTE_HOST:$REMOTE_FILE"
scp "$LOCAL_FILE" "$REMOTE_HOST:$REMOTE_FILE"

# Increment the version
NEW_VERSION=$((VERSION + 1))
echo "$NEW_VERSION" > "$VERSION_FILE"

echo "Deployment complete! Version updated to $NEW_VERSION."
