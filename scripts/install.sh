#!/usr/bin/env bash
# Install the xmac CLI binary to ~/.local/bin (or a custom dir).
set -e

INSTALL_DIR="${1:-$HOME/.local/bin}"
cd "$(git rev-parse --show-toplevel)"

echo "Building release binary..."
cargo build --release

echo "Installing to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
cp target/release/x-mac "$INSTALL_DIR/xmac"
chmod +x "$INSTALL_DIR/xmac"

echo ""
echo "Installed: $INSTALL_DIR/xmac"
echo "Make sure $INSTALL_DIR is on your PATH."
echo ""
echo "Verify: xmac --help"
