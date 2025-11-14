#!/usr/bin/env bash
set -euo pipefail

# ==========================
# CONFIGURATION
# ==========================
CLOUDFLARED_BIN="/usr/local/bin/cloudflared"
CLOUDFLARED_DIR="/home/yaro/.cloudflared"

INFERENCE_NAME="inference"
INFERENCE_HOST="inference.ktulhu.com"
INFERENCE_PORT="30823"

PERSISTENCE_NAME="persistence"
PERSISTENCE_HOST="persistence.ktulhu.com"
PERSISTENCE_PORT="8080"

SYSTEMD_DIR="/etc/systemd/system"

# ==========================
# Ensure cloudflared installed
# ==========================
if ! command -v "$CLOUDFLARED_BIN" &>/dev/null; then
  echo "Installing cloudflared..."
  sudo curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 \
    -o "$CLOUDFLARED_BIN"
  sudo chmod +x "$CLOUDFLARED_BIN"
fi

mkdir -p "$CLOUDFLARED_DIR"

# ==========================
# CREATE OR REUSE TUNNEL
# ==========================
ensure_tunnel() {
  local NAME="$1"
  local HOST="$2"
  local PORT="$3"

  echo ""
  echo "=== Ensuring tunnel: $NAME ($HOST → localhost:$PORT) ==="

  # Try to find existing ID
  local EXISTING_ID
  EXISTING_ID=$($CLOUDFLARED_BIN tunnel list 2>/dev/null | awk -v n="$NAME" '$1!~/ID/ && $2==n {print $1}')

  # Create tunnel if needed
  if [[ -z "$EXISTING_ID" ]]; then
    echo "Creating tunnel $NAME..."
    $CLOUDFLARED_BIN tunnel create "$NAME"
    EXISTING_ID=$($CLOUDFLARED_BIN tunnel list | awk -v n="$NAME" '$2==n {print $1}')
  else
    echo "Reusing existing tunnel: $EXISTING_ID"
  fi

  # Ensure DNS
  echo "Configuring DNS for $HOST..."
  $CLOUDFLARED_BIN tunnel route dns "$NAME" "$HOST" || true

  # Write config file
  local CFG="$CLOUDFLARED_DIR/${NAME}.yml"
  local CREDS="$CLOUDFLARED_DIR/${EXISTING_ID}.json"

  echo "Writing config: $CFG"
  cat > "$CFG" <<EOF
tunnel: $EXISTING_ID
credentials-file: $CREDS

ingress:
  - hostname: $HOST
    service: http://localhost:$PORT
  - service: http_status:404
EOF

  # Create systemd service
  echo "Creating systemd service for $NAME..."

  sudo tee "$SYSTEMD_DIR/cloudflared-$NAME.service" >/dev/null <<EOF
[Unit]
Description=Cloudflare Tunnel - $NAME
After=network.target

[Service]
User=yaro
ExecStart=$CLOUDFLARED_BIN tunnel --config $CFG run
Restart=always
RestartSec=5

[Install]
WantedBy=multi-user.target
EOF

  echo "Enabling and starting $NAME..."
  sudo systemctl daemon-reload
  sudo systemctl enable "cloudflared-$NAME.service"
  sudo systemctl restart "cloudflared-$NAME.service"
}

# ==========================
# RUN BOTH TUNNELS
# ==========================
ensure_tunnel "$INFERENCE_NAME" "$INFERENCE_HOST" "$INFERENCE_PORT"
ensure_tunnel "$PERSISTENCE_NAME" "$PERSISTENCE_HOST" "$PERSISTENCE_PORT"

echo ""
echo "=============================================="
echo " DONE! Both tunnels created + systemd enabled "
echo "=============================================="
echo ""
cloudflared tunnel list
