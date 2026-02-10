#!/usr/bin/env bash
#
# Tugbot Discord Bot - Install Script
# Run this AFTER cloning the repo
#
# Usage:
#   git clone git@github.com:danielcherubini/tugbot-rs.git /opt/tugbot
#   bash /opt/tugbot/scripts/install.sh
#

set -e

INSTALL_DIR="/opt/tugbot"
SERVICE_NAME="tugbot"

echo "==================================="
echo " Tugbot Discord Bot Installer"
echo "==================================="
echo ""

# Check root
if [[ $EUID -ne 0 ]]; then
  echo "Error: Run as root"
  exit 1
fi

# Check repo exists
if [[ ! -d "$INSTALL_DIR/.git" ]]; then
  echo "Error: Repo not found at $INSTALL_DIR"
  echo ""
  echo "Clone first:"
  echo "  git clone git@github.com:danielcherubini/tugbot-rs.git $INSTALL_DIR"
  exit 1
fi

cd "$INSTALL_DIR"

# Install dependencies
echo "[1/5] Installing system dependencies..."
apt-get update -qq
apt-get install -y -qq curl git ca-certificates gnupg libpq-dev pkg-config libssl-dev build-essential

# Install Rust
echo "[2/5] Installing Rust toolchain..."
if ! command -v rustc &>/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  source "$HOME/.cargo/env"
fi
echo "       Rust $(rustc --version) installed"

# Ensure cargo is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Check for .env file
echo "[3/5] Checking environment configuration..."
if [[ ! -f "$INSTALL_DIR/.env" ]]; then
  echo "Error: .env file not found!"
  echo ""
  echo "Create $INSTALL_DIR/.env with:"
  echo "  DISCORD_TOKEN=your_token_here"
  echo "  APPLICATION_ID=your_app_id_here"
  echo "  DATABASE_URL=postgresql://user:pass@host/database"
  exit 1
fi
echo "       .env file found"

# Install Diesel CLI
echo "[4/5] Installing Diesel CLI..."
if ! command -v diesel &>/dev/null; then
  cargo install diesel_cli --no-default-features --features postgres --quiet
fi

# Run migrations
diesel migration run

# Install tugbot binary
echo "[5/5] Installing tugbot binary..."
cargo install --path .
echo "       Binary installed to: /root/.cargo/bin/tugbot"

# Install systemd service (use existing service file, update paths)
echo "[6/6] Installing systemd service..."
cat > /etc/systemd/system/${SERVICE_NAME}.service << EOF
[Unit]
Description=Tugbot Service
After=network.target
StartLimitIntervalSec=0

[Service]
Type=simple
Restart=always
RestartSec=1
User=root
WorkingDirectory=$INSTALL_DIR
ExecStart=/root/.cargo/bin/tugbot

[Install]
WantedBy=multi-user.target
EOF

# Enable and start service
systemctl daemon-reload
systemctl enable ${SERVICE_NAME}.service
systemctl start ${SERVICE_NAME}.service

echo ""
echo "==================================="
echo " Installation complete!"
echo "==================================="
echo ""
echo " Status:  systemctl status tugbot"
echo " Logs:    journalctl -u tugbot -f"
echo " Stop:    systemctl stop tugbot"
echo " Start:   systemctl start tugbot"
echo ""
echo "To update: cd $INSTALL_DIR && git pull && cargo install --path . && systemctl restart tugbot"
echo ""
