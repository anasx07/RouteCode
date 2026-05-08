#!/bin/sh
# Loom installer — macOS & Linux
# Usage: curl -fsSL https://raw.githubusercontent.com/anasx07/loom/main/install.sh | sh
set -e

REPO="anasx07/loom"
BINARY="loom-cli"
INSTALL_DIR="${LOOM_INSTALL_DIR:-$HOME/.local/bin}"

# ── Colour helpers ────────────────────────────
RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; BOLD='\033[1m'; RESET='\033[0m'
info()    { printf "${BOLD}[loom]${RESET} %s\n" "$1"; }
success() { printf "${GREEN}[loom]${RESET} %s\n" "$1"; }
warn()    { printf "${YELLOW}[loom]${RESET} %s\n" "$1"; }
error()   { printf "${RED}[loom]${RESET} %s\n" "$1" >&2; exit 1; }

# ── Detect OS ────────────────────────────────
OS="$(uname -s)"
case "$OS" in
  Linux*)   OS_NAME="linux"  ;;
  Darwin*)  OS_NAME="macos"  ;;
  *)        error "Unsupported OS: $OS" ;;
esac

# ── Detect architecture ───────────────────────
ARCH="$(uname -m)"
case "$ARCH" in
  x86_64|amd64)   ARCH_NAME="x86_64" ;;
  arm64|aarch64)  ARCH_NAME="arm64"   ;;
  *)              error "Unsupported architecture: $ARCH" ;;
esac

ASSET="${BINARY}-${OS_NAME}-${ARCH_NAME}"

# ── Resolve latest release ────────────────────
info "Fetching latest release..."
if command -v curl >/dev/null 2>&1; then
  FETCH="curl -fsSL"
elif command -v wget >/dev/null 2>&1; then
  FETCH="wget -qO-"
else
  error "curl or wget is required."
fi

LATEST=$($FETCH "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep '"tag_name"' | head -1 | cut -d'"' -f4)

[ -z "$LATEST" ] && error "Could not determine latest release. Check your internet connection."

info "Installing loom ${LATEST} (${OS_NAME}/${ARCH_NAME})..."

URL="https://github.com/${REPO}/releases/download/${LATEST}/${ASSET}"

# ── Download ──────────────────────────────────
mkdir -p "$INSTALL_DIR"
TMP="$(mktemp)"

if command -v curl >/dev/null 2>&1; then
  curl -fsSL --progress-bar "$URL" -o "$TMP" || error "Download failed: $URL"
else
  wget -q --show-progress "$URL" -O "$TMP" || error "Download failed: $URL"
fi

chmod +x "$TMP"
mv "$TMP" "${INSTALL_DIR}/${BINARY}"

success "loom installed to ${INSTALL_DIR}/${BINARY}"

# ── PATH check ────────────────────────────────
case ":$PATH:" in
  *":${INSTALL_DIR}:"*)
    success "Already on PATH. Type 'loom' to get started."
    ;;
  *)
    warn "${INSTALL_DIR} is not on your PATH."

    SHELL_NAME="$(basename "${SHELL:-sh}")"
    case "$SHELL_NAME" in
      zsh)   PROFILE="$HOME/.zshrc"   ;;
      bash)  PROFILE="$HOME/.bashrc"  ;;
      fish)  PROFILE="$HOME/.config/fish/config.fish" ;;
      *)     PROFILE="" ;;
    esac

    if [ -n "$PROFILE" ]; then
      printf '\nexport PATH="$HOME/.local/bin:$PATH"\n' >> "$PROFILE"
      success "Appended to ${PROFILE}. Run: source ${PROFILE}"
    else
      warn "Add this to your shell profile manually:"
      printf "\n  ${BOLD}export PATH=\"\$HOME/.local/bin:\$PATH\"${RESET}\n\n"
    fi
    ;;
esac
