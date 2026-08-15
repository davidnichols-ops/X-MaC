#!/bin/bash
set -euo pipefail

# Build script for X-MaC GUI app
# Builds the Rust + Swift binaries and packages them into a .app bundle with
# the bundled CoreML models and label map.
#
# Environment variables (all optional):
#   XMAC_RUST_BIN         - path to an already-built Rust binary
#   XMAC_SWIFT_BIN        - path to an already-built Swift executable
#   XMAC_SKIP_RUST_BUILD  - set to 1 to skip `cargo build` (requires XMAC_RUST_BIN)
#   XMAC_SKIP_SWIFT_BUILD - set to 1 to skip `swift build` (requires XMAC_SWIFT_BIN)
#   XMAC_UNIVERSAL        - set to 1 when the provided Rust/Swift binaries are universal

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
STAGING_DIR="$SCRIPT_DIR/staging"
APP_NAME="X-MaC"
APP_BUNDLE="$STAGING_DIR/$APP_NAME.app"

VERSION="$(grep -m1 '^version' "$PROJECT_ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
BUNDLE_VERSION="${XMAC_BUNDLE_VERSION:-5}"

echo "=== Building X-MaC GUI v$VERSION ==="

# Step 0: Verify CoreML model integrity
echo "[0/6] Verifying CoreML model integrity..."
if [ -f "$PROJECT_ROOT/scripts/verify_models.sh" ]; then
    bash "$PROJECT_ROOT/scripts/verify_models.sh"
else
    echo "  WARNING: verify_models.sh not found, skipping model verification"
fi

# Step 1: Build or locate the Rust binary
echo "[1/6] Preparing Rust binary..."
cd "$PROJECT_ROOT"
if [ -n "${XMAC_RUST_BIN:-}" ]; then
    RUST_BIN="$XMAC_RUST_BIN"
    echo "  Using provided Rust binary: $RUST_BIN"
    if [ ! -f "$RUST_BIN" ]; then
        echo "  ERROR: Provided Rust binary not found: $RUST_BIN"
        exit 1
    fi
elif [ "${XMAC_SKIP_RUST_BUILD:-0}" = "1" ]; then
    echo "  ERROR: XMAC_RUST_BIN must be set when XMAC_SKIP_RUST_BUILD=1"
    exit 1
else
    echo "  Building Rust binary with cargo..."
    cargo build --release 2>&1 | tail -3
    RUST_BIN="$PROJECT_ROOT/target/release/xmac"
fi

if [ ! -f "$RUST_BIN" ]; then
    echo "  ERROR: Rust binary not found at $RUST_BIN"
    exit 1
fi

# Install the CLI locally for dev convenience when building from source.
if [ -z "${XMAC_RUST_BIN:-}" ] && [ -d "$HOME/.local/bin" ]; then
    mkdir -p "$HOME/.local/bin"
    cp "$RUST_BIN" "$HOME/.local/bin/xmac"
    chmod +x "$HOME/.local/bin/xmac"
    echo "  Rust binary installed to ~/.local/bin/xmac"
fi

# Step 2: Build or locate the Swift app
echo "[2/6] Preparing Swift app..."
if [ -n "${XMAC_SWIFT_BIN:-}" ]; then
    SWIFT_BIN="$XMAC_SWIFT_BIN"
    echo "  Using provided Swift binary: $SWIFT_BIN"
    if [ ! -f "$SWIFT_BIN" ]; then
        echo "  ERROR: Provided Swift binary not found: $SWIFT_BIN"
        exit 1
    fi
elif [ "${XMAC_SKIP_SWIFT_BUILD:-0}" = "1" ]; then
    echo "  ERROR: XMAC_SWIFT_BIN must be set when XMAC_SKIP_SWIFT_BUILD=1"
    exit 1
else
    cd "$SCRIPT_DIR/XMacApp"
    echo "  Building Swift app..."
    swift build -c release 2>&1 | tail -5
    SWIFT_BIN="$(swift build -c release --show-bin-path)/XMacApp"
fi

if [ ! -f "$SWIFT_BIN" ]; then
    echo "  ERROR: Swift binary not found at $SWIFT_BIN"
    exit 1
fi

# Step 3: Create the .app bundle
echo "[3/6] Creating .app bundle..."
rm -rf "$STAGING_DIR"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

cp "$SWIFT_BIN" "$APP_BUNDLE/Contents/MacOS/XMacApp"
chmod +x "$APP_BUNDLE/Contents/MacOS/XMacApp"

cp "$RUST_BIN" "$APP_BUNDLE/Contents/MacOS/xmac"
chmod +x "$APP_BUNDLE/Contents/MacOS/xmac"

# Step 4: Copy resources
echo "[4/6] Copying resources..."
if [ -d "$PROJECT_ROOT/gnn/XMacGNN.mlpackage" ]; then
    cp -r "$PROJECT_ROOT/gnn/XMacGNN.mlpackage" "$APP_BUNDLE/Contents/Resources/"
    echo "  Copied XMacGNN.mlpackage"
else
    echo "  ERROR: XMacGNN.mlpackage not found in $PROJECT_ROOT/gnn/"
    echo "  Cannot build without the CoreML model. Run gnn/export_coreml.py first."
    exit 1
fi

if [ -d "$PROJECT_ROOT/gnn/XMacMemoryGNN.mlpackage" ]; then
    cp -r "$PROJECT_ROOT/gnn/XMacMemoryGNN.mlpackage" "$APP_BUNDLE/Contents/Resources/"
    echo "  Copied XMacMemoryGNN.mlpackage"
else
    echo "  WARNING: XMacMemoryGNN.mlpackage not found — memory optimization will be disabled"
fi

if [ -f "$PROJECT_ROOT/gnn/label_map.json" ]; then
    cp "$PROJECT_ROOT/gnn/label_map.json" "$APP_BUNDLE/Contents/Resources/"
    echo "  Copied label_map.json"
else
    echo "  ERROR: label_map.json not found in $PROJECT_ROOT/gnn/"
    exit 1
fi

if [ -f "$SCRIPT_DIR/AppIcon.icns" ]; then
    cp "$SCRIPT_DIR/AppIcon.icns" "$APP_BUNDLE/Contents/Resources/"
    echo "  Copied AppIcon.icns"
fi

if [ -f "$SCRIPT_DIR/install_launch_agent.sh" ]; then
    cp "$SCRIPT_DIR/install_launch_agent.sh" "$APP_BUNDLE/Contents/Resources/"
    chmod +x "$APP_BUNDLE/Contents/Resources/install_launch_agent.sh"
fi

# Step 5: Create Info.plist
echo "[5/6] Creating Info.plist..."
cat > "$APP_BUNDLE/Contents/Info.plist" << PLIST
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>X-MaC</string>
    <key>CFBundleDisplayName</key>
    <string>X-MaC</string>
    <key>CFBundleIdentifier</key>
    <string>com.xmac.gui</string>
    <key>CFBundleVersion</key>
    <string>$BUNDLE_VERSION</string>
    <key>CFBundleShortVersionString</key>
    <string>$VERSION</string>
    <key>CFBundlePackageType</key>
    <string>APPL</string>
    <key>CFBundleExecutable</key>
    <string>XMacApp</string>
    <key>CFBundleInfoDictionaryVersion</key>
    <string>6.0</string>
    <key>LSMinimumSystemVersion</key>
    <string>14.0</string>
    <key>NSHighResolutionCapable</key>
    <true/>
    <key>LSApplicationCategoryType</key>
    <string>public.app-category.utilities</string>
    <key>NSHumanReadableCopyright</key>
    <string>© 2026 X-MaC. All rights reserved.</string>
    <key>NSAppleEventsUsageDescription</key>
    <string>X-MaC needs to run maintenance commands to clean and maintain your Mac.</string>
    <key>NSUserNotificationUsageDescription</key>
    <string>X-MaC uses notifications to alert you when scans complete or cleanup is recommended.</string>
    <key>CFBundleIconFile</key>
    <string>AppIcon</string>
</dict>
</plist>
PLIST

echo ""
echo "=== Build complete ==="
echo "App bundle: $APP_BUNDLE"
echo ""
echo "To install: cp -r '$APP_BUNDLE' /Applications/"
