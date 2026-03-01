#!/usr/bin/env bash
# World Wide Swarm — one-line installer
# Usage: curl -sSfL https://install.wws.dev | sh
set -euo pipefail

REPO="Good-karma-lab/WorldWideSwarm"
INSTALL_DIR="${WWS_INSTALL_DIR:-$HOME/.local/bin}"
DATA_DIR="${HOME}/.wws"

# Detect OS and architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)
case "$ARCH" in
  x86_64)        ARCH="amd64" ;;
  aarch64|arm64) ARCH="arm64" ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

# Get latest version from GitHub API
VERSION=$(curl -sSfL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | cut -d'"' -f4)

if [ -z "$VERSION" ]; then
  echo "Failed to fetch latest version" >&2
  exit 1
fi

FILENAME="wws-connector-${VERSION#v}-${OS}-${ARCH}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${FILENAME}"

echo "Installing wws-connector ${VERSION} (${OS}/${ARCH})..."
mkdir -p "$INSTALL_DIR" "$DATA_DIR"
curl -sSfL "$URL" -o /tmp/wws-connector.tar.gz
tar xzf /tmp/wws-connector.tar.gz -C "$INSTALL_DIR" wws-connector
chmod +x "$INSTALL_DIR/wws-connector"
rm -f /tmp/wws-connector.tar.gz

echo ""
echo "✓ Installed: ${INSTALL_DIR}/wws-connector"
echo "✓ Identity directory: ${DATA_DIR}"
echo ""
echo "Add to PATH (if not already):"
echo "  export PATH=\"\$HOME/.local/bin:\$PATH\""
echo ""
echo "Get started:"
echo "  wws-connector --agent-name alice"
