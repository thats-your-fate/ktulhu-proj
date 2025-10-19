#!/usr/bin/env bash
set -e

# ──────────────────────────────────────────────
# 🧠 Local Node.js installer for Ktulhu project
# Installs Node v22.x into project root and
# maintains a stable symlink for Rust runner.
# ──────────────────────────────────────────────

echo "📦 Initializing local Node.js v22 environment..."

# Always run from script directory
cd "$(dirname "$0")"

NODE_VERSION="v22.11.0"
NODE_DIST="node-${NODE_VERSION}-linux-x64"
NODE_TAR="${NODE_DIST}.tar.xz"
NODE_URL="https://nodejs.org/dist/${NODE_VERSION}/${NODE_TAR}"

# Destination paths
STABLE_DIR="node-v22-linux-x64"        # what Rust uses
EXTRACT_DIR="${NODE_DIST}"             # actual extracted name

# 1️⃣ Skip if already installed
if [ -x "${STABLE_DIR}/bin/node" ]; then
  echo "✅ Node.js already installed at ${STABLE_DIR}/bin/node"
  "${STABLE_DIR}/bin/node" -v
  exit 0
fi

# 2️⃣ Clean any old leftovers
rm -rf "${STABLE_DIR}" "${EXTRACT_DIR}" "${NODE_TAR}"

# 3️⃣ Download tarball
echo "⬇️ Downloading ${NODE_URL} ..."
wget -q "${NODE_URL}" -O "${NODE_TAR}"

# 4️⃣ Extract
echo "📦 Extracting ${NODE_TAR} ..."
tar -xf "${NODE_TAR}"
rm -f "${NODE_TAR}"

# 5️⃣ Create stable symlink
ln -s "${EXTRACT_DIR}" "${STABLE_DIR}"

# 6️⃣ Verify
NODE_BIN="./${STABLE_DIR}/bin/node"
if [ -x "${NODE_BIN}" ]; then
  echo "✅ Node.js v22 installed locally."
  echo "   Binary: ${NODE_BIN}"
  echo "   Version: $(${NODE_BIN} -v)"
else
  echo "❌ Installation failed — ${NODE_BIN} not found."
  exit 1
fi
