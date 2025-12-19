#!/bin/bash
set -euo pipefail

SRC_DIR="/home/ubuntu/sfrustscript"
DEST_DIR="/var/www/html/updates"

echo "=== Running update.sh ==="

sudo mkdir -p "$DEST_DIR"

# --- sfbot.exe ---
if [ -f "$SRC_DIR/sfbot.exe" ]; then
  echo "Updating sfbot.exe..."
  sudo install -m 0644 -o www-data -g www-data "$SRC_DIR/sfbot.exe" "$DEST_DIR/sfbot.exe"
  rm -f "$SRC_DIR/sfbot.exe"
else
  echo "No new sfbot.exe found."
fi

# --- Installer (irgendein sfbot_*_x64-setup.exe) ---
newest=$(ls -t "$SRC_DIR"/sfbot_*_x64-setup.exe 2>/dev/null | head -n1 || true)
if [ -n "$newest" ]; then
  echo "Updating installer: $(basename "$newest") -> sfbot_installer.exe"
  sudo install -m 0644 -o www-data -g www-data "$newest" "$DEST_DIR/sfbot_installer.exe"
  rm -f "$newest"
else
  echo "No new installer found."
fi

# --- Linux builds ---
if [ -f "$SRC_DIR/sfbot-linux-x64" ]; then
  echo "Updating sfbot-linux-x64..."
  sudo install -m 0644 -o www-data -g www-data "$SRC_DIR/sfbot-linux-x64" "$DEST_DIR/sfbot-linux-x64"
  rm -f "$SRC_DIR/sfbot-linux-x64"
else
  echo "No new sfbot-linux-x64 found."
fi

if [ -f "$SRC_DIR/sfbot-linux-arm64" ]; then
  echo "Updating sfbot-linux-arm64..."
  sudo install -m 0644 -o www-data -g www-data "$SRC_DIR/sfbot-linux-arm64" "$DEST_DIR/sfbot-linux-arm64"
  rm -f "$SRC_DIR/sfbot-linux-arm64"
else
  echo "No new sfbot-linux-arm64 found."
fi

if [ -f "$SRC_DIR/sfbot-linux-armv7" ]; then
  echo "Updating sfbot-linux-armv7..."
  sudo install -m 0644 -o www-data -g www-data "$SRC_DIR/sfbot-linux-armv7" "$DEST_DIR/sfbot-linux-armv7"
  rm -f "$SRC_DIR/sfbot-linux-armv7"
else
  echo "No new sfbot-linux-armv7 found."
fi

if [ -f "$SRC_DIR/sfbot-linux-i686" ]; then
  echo "Updating sfbot-linux-i686..."
  sudo install -m 0644 -o www-data -g www-data "$SRC_DIR/sfbot-linux-i686" "$DEST_DIR/sfbot-linux-i686"
  rm -f "$SRC_DIR/sfbot-linux-i686"
else
  echo "No new sfbot-linux-i686 found."
fi

# --- charsToFight.json ---
if [ -f "$SRC_DIR/charsToFight.json" ]; then
  echo "Updating charsToFight.json..."
  sudo install -m 0644 -o www-data -g www-data "$SRC_DIR/charsToFight.json" "$DEST_DIR/charsToFight.json"
  rm -f "$SRC_DIR/charsToFight.json"
else
  echo "No new charsToFight.json found."
fi

# --- latest.json ---
if [ -f "$SRC_DIR/latest.json" ]; then
  echo "Updating latest.json..."
  sudo install -m 0644 -o www-data -g www-data "$SRC_DIR/latest.json" "$DEST_DIR/latest.json"
  rm -f "$SRC_DIR/latest.json"
else
  echo "No new latest.json found."
fi

echo "=== update.sh finished ==="
