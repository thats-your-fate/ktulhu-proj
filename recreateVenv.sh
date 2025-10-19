#!/usr/bin/env bash
set -e

# ────────────────────────────────────────────────
# 📦 Recreate Python venv for Ktulhu project
# Automatically detects current path and reinstalls dependencies.
# Run:  ./scripts/recreate_venv.sh
# ────────────────────────────────────────────────

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
VENV_DIR="$PROJECT_ROOT/.venv"
REQ_FILE="$PROJECT_ROOT/requirements.txt"

echo "📁 Project root: $PROJECT_ROOT"
echo "🐍 Virtualenv path: $VENV_DIR"

# 1️⃣ Remove existing venv
if [ -d "$VENV_DIR" ]; then
  echo "🧹 Removing old venv..."
  rm -rf "$VENV_DIR"
fi

# 2️⃣ Create fresh one
echo "🔧 Creating new venv..."
python3 -m venv "$VENV_DIR"

# 3️⃣ Activate
source "$VENV_DIR/bin/activate"

# 4️⃣ Upgrade core tools
echo "⬆️ Upgrading pip & setuptools..."
pip install --upgrade pip setuptools wheel

# 5️⃣ Install dependencies
if [ -f "$REQ_FILE" ]; then
  echo "📦 Installing from requirements.txt..."
  pip install -r "$REQ_FILE"
else
  echo "⚠️ No requirements.txt found, installing core ML packages..."
  pip install torch torchvision torchaudio transformers accelerate sentencepiece huggingface_hub
fi

# 6️⃣ Summary
echo ""
echo "✅ Virtualenv ready!"
echo "   To activate: source $VENV_DIR/bin/activate"
echo ""
python -V
which python
echo ""
echo "🎉 Done."
