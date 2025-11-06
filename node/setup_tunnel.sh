#!/usr/bin/env bash
set -euo pipefail

# ==========================
# ⚙️ CONFIGURATION
# ==========================
CLOUDFLARED_BIN="/usr/local/bin/cloudflared"
CLOUDFLARED_DIR="/home/yaro/.cloudflared"

# Define tunnels independently
INFERENCE_TUNNEL="inference"
INFERENCE_HOST="inference.ktulhu.com"
INFERENCE_SERVICE="http://localhost:30823"

PERSISTENCE_TUNNEL="persistence"
PERSISTENCE_HOST="persistence.ktulhu.com"
PERSISTENCE_SERVICE="http://localhost:8080"

# ==========================
# 🧩 Ensure cloudflared installed
# ==========================
if ! command -v "$CLOUDFLARED_BIN" &>/dev/null && ! command -v cloudflared &>/dev/null; then
  echo "⚠️  Installing cloudflared..."
  curl -L https://github.com/cloudflare/cloudflared/releases/latest/download/cloudflared-linux-amd64 \
       -o /usr/local/bin/cloudflared
  chmod +x /usr/local/bin/cloudflared
fi

mkdir -p "$CLOUDFLARED_DIR"
cd "$CLOUDFLARED_DIR"

# ------------------------------------------------------------
# 🔁 Function: create or reuse a single tunnel
# Usage: ensure_tunnel "name" "hostname" "service"
# ------------------------------------------------------------
ensure_tunnel() {
  local NAME="$1"
  local HOST="$2"
  local SERVICE="$3"
  local CONFIG_FILE="$CLOUDFLARED_DIR/${NAME}.yml"

  echo ""
  echo "=============================================="
  echo "⚙️  Ensuring tunnel: $NAME → $SERVICE ($HOST)"
  echo "=============================================="

  # --- Check if the tunnel already exists ---
  local EXISTING_LINE=$($CLOUDFLARED_BIN tunnel list 2>/dev/null | grep -w "$NAME" || true)
  local EXISTING_ID=$(echo "$EXISTING_LINE" | awk '{print $2}' || true)

  if [[ -n "$EXISTING_ID" ]]; then
    echo "✅ Reusing existing tunnel '$NAME' (ID: $EXISTING_ID)"
  else
    echo "🆕 Creating new tunnel '$NAME'..."
    $CLOUDFLARED_BIN tunnel create "$NAME"
  fi

  # --- Create individual YAML for this tunnel ---
  if [[ ! -f "$CONFIG_FILE" ]]; then
    echo "📝 Writing config for $NAME → $CONFIG_FILE"
    cat > "$CONFIG_FILE" <<EOF
tunnel: $NAME
credentials-file: $CLOUDFLARED_DIR/${NAME}.json
ingress:
  - hostname: $HOST
    service: $SERVICE
  - service: http_status:404
EOF
  else
    echo "✅ Config already exists at $CONFIG_FILE"
  fi

  # --- Ensure DNS route points correctly ---
  echo "🔗 Ensuring DNS route for $HOST..."
  $CLOUDFLARED_BIN tunnel route dns "$NAME" "$HOST" || true

  # --- Start tunnel ---
  echo "🚀 Starting tunnel '$NAME'..."
  nohup $CLOUDFLARED_BIN tunnel --config "$CONFIG_FILE" run "$NAME" >/dev/null 2>&1 &
  sleep 2

  local ID=$($CLOUDFLARED_BIN tunnel list | grep -w "$NAME" | awk '{print $2}' || true)
  if [[ -n "$ID" ]]; then
    echo "✅ '$NAME' is running (Tunnel ID: $ID) → https://$HOST"
  else
    echo "⚠️  Could not verify '$NAME' run state. Check manually."
  fi
}

# ==========================
# 🚀 Create & start both
# ==========================
ensure_tunnel "$INFERENCE_TUNNEL" "$INFERENCE_HOST" "$INFERENCE_SERVICE"
ensure_tunnel "$PERSISTENCE_TUNNEL" "$PERSISTENCE_HOST" "$PERSISTENCE_SERVICE"

echo ""
echo "✅ All tunnels verified. Both inference and persistence online."
exit 0
