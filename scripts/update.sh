#!/usr/bin/env bash
#
# Tugbot Discord Bot - Update Script
# Pulls latest from GitHub, rebuilds, and restarts the service
#

set -e

INSTALL_DIR="/opt/tugbot"
SERVICE_NAME="tugbot"

echo "Updating Tugbot Discord Bot..."

if [[ ! -d "$INSTALL_DIR" ]]; then
  echo "Error: $INSTALL_DIR not found. Run install.sh first."
  exit 1
fi

cd "$INSTALL_DIR"

# Ensure cargo is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

echo "[1/4] Pulling latest changes..."
git fetch --all
git reset --hard origin/main

echo "[2/4] Running database migrations..."
diesel migration run

echo "[3/4] Installing updated binary..."
cargo install --path .

echo "[4/4] Restarting service..."
systemctl restart ${SERVICE_NAME}.service

echo ""
echo "Done! Bot updated."
systemctl status ${SERVICE_NAME}.service --no-pager
