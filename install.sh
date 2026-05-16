#!/bin/sh
set -e

# Detect OS and Architecture
OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case "$OS" in
  linux)
    ASSET="routecode-linux-x86_64"
    ;;
  darwin)
    if [ "$ARCH" = "arm64" ]; then
      ASSET="routecode-macos-arm64"
    else
      ASSET="routecode-macos-x86_64"
    fi
    ;;
  *)
    echo "Unsupported OS: $OS"
    exit 1
    ;;
esac

echo "Downloading RouteCode for $OS ($ARCH)..."

# Get latest version tag
LATEST_TAG=$(curl -s https://api.github.com/repos/anasx07/routecode/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/')

if [ -z "$LATEST_TAG" ]; then
  echo "Failed to fetch latest release tag. Defaulting to main branch (not recommended)."
  exit 1
fi

URL="https://github.com/anasx07/routecode/releases/download/$LATEST_TAG/$ASSET"
INSTALL_DIR="/usr/local/bin"

if [ ! -w "$INSTALL_DIR" ]; then
  echo "Install directory $INSTALL_DIR is not writable. Trying with sudo..."
  curl -L "$URL" -o routecode
  chmod +x routecode
  sudo mv routecode "$INSTALL_DIR/routecode"
else
  curl -L "$URL" -o "$INSTALL_DIR/routecode"
  chmod +x "$INSTALL_DIR/routecode"
fi

echo "RouteCode installed successfully to $INSTALL_DIR/routecode"
routecode --version
