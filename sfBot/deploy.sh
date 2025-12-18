#!/bin/bash
set -e  # Exit on any error

SFROOT="$HOME/sfbot/snf/sfBot"

echo "=== Fixing permissions ==="
sudo chown -R $USER:$USER "$SFROOT"
chmod -R u+w "$SFROOT"

echo "=== Removing old src folder ==="
rm -rf "$SFROOT/src"

echo "=== Creating new src folder ==="
mkdir "$SFROOT/src"

echo "=== Unzipping snf.zip into src folder ==="
unzip -o "$SFROOT/snf.zip" -d "$SFROOT/src"

echo "=== Loading Rust environment ==="
source $HOME/.cargo/env

echo "=== Building with cargo tauri ==="
cd "$SFROOT/src-tauri"
cargo tauri build

echo "=== Copying binary to mains ==="
cp target/debug/sfbot "$HOME/sfbot/mains/"

echo "=== Starting sfbot ==="
cd "$HOME/sfbot/mains"
nohup ./sfbot > output.log 2>&1 &

echo "=== Deployment completed successfully! ==="
