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
echo "[1/6] Installing system dependencies..."
apt-get update -qq
apt-get install -y -qq curl git ca-certificates gnupg libpq-dev pkg-config libssl-dev build-essential

# Install Rust
echo "[2/6] Installing Rust toolchain..."
if ! command -v rustc &>/dev/null; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  source "$HOME/.cargo/env"
fi
echo "       Rust $(rustc --version) installed"

# Ensure cargo is in PATH
export PATH="$HOME/.cargo/bin:$PATH"

# Check for .env file
echo "[3/6] Checking environment configuration..."
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
echo "[4/6] Installing Diesel CLI..."
if ! command -v diesel &>/dev/null; then
  cargo install diesel_cli --no-default-features --features postgres --quiet
fi

# Run migrations
echo "[5/6] Running database migrations..."
diesel migration run

# Build release binary
echo "[6/6] Building release binary..."
cargo build --release --quiet
echo "       Binary: $INSTALL_DIR/target/release/tugbot"

# Create systemd service
echo "[7/8] Creating systemd service..."
cat > /etc/systemd/system/${SERVICE_NAME}.service << EOF
[Unit]
Description=Tugbot Discord Bot
After=network.target

[Service]
Type=simple
User=root
WorkingDirectory=$INSTALL_DIR
Environment="PATH=/root/.cargo/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin"
ExecStart=$INSTALL_DIR/target/release/tugbot
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

# Create update script
echo "[8/8] Creating update script..."
cat > /usr/local/bin/update-tugbot << 'EOFSCRIPT'
#!/usr/bin/env bash
set -e

INSTALL_DIR="/opt/tugbot"

echo "Updating Tugbot Discord Bot..."

cd "$INSTALL_DIR"

echo "[1/4] Pulling latest changes..."
git fetch --all
git reset --hard origin/main

echo "[2/4] Running migrations..."
diesel migration run

echo "[3/4] Building release binary..."
cargo build --release

echo "[4/4] Restarting service..."
systemctl restart tugbot.service

echo ""
echo "Done! Bot updated."
systemctl status tugbot.service --no-pager
EOFSCRIPT
chmod +x /usr/local/bin/update-tugbot

# Enable and start service
systemctl daemon-reload
systemctl enable ${SERVICE_NAME}.service
systemctl start ${SERVICE_NAME}.service

# Get IP
IP=$(hostname -I | awk '{print $1}')

echo ""
echo "==================================="
echo " Installation complete!"
echo "==================================="
echo ""
echo " Update:  update-tugbot"
echo " Status:  systemctl status tugbot"
echo " Logs:    journalctl -u tugbot -f"
echo " Stop:    systemctl stop tugbot"
echo " Start:   systemctl start tugbot"
echo ""
