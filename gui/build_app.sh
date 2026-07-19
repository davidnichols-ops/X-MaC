#!/bin/bash
set -e

# Build script for X-MaC GUI app
# Builds the Swift app and packages it into a .app bundle with CoreML model

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
STAGING_DIR="$SCRIPT_DIR/staging"
APP_NAME="X-MaC"
APP_BUNDLE="$STAGING_DIR/$APP_NAME.app"

echo "=== Building X-MaC GUI ==="

# Step 0: Verify CoreML model integrity
echo "[0/7] Verifying CoreML model integrity..."
if [ -f "$PROJECT_ROOT/scripts/verify_models.sh" ]; then
    bash "$PROJECT_ROOT/scripts/verify_models.sh"
else
    echo "  WARNING: verify_models.sh not found, skipping model verification"
fi

# Step 1: Build the Rust binary
echo "[1/7] Building Rust binary..."
cd "$PROJECT_ROOT"
cargo build --release 2>&1 | tail -3
cp target/release/x-mac ~/.local/bin/xmac
echo "  Rust binary installed to ~/.local/bin/xmac"

# Step 2: Build the Swift app
echo "[2/7] Building Swift app..."
cd "$SCRIPT_DIR/XMacApp"
swift build -c release 2>&1 | tail -5

# Step 3: Create the .app bundle
echo "[3/7] Creating .app bundle..."
rm -rf "$STAGING_DIR"
mkdir -p "$APP_BUNDLE/Contents/MacOS"
mkdir -p "$APP_BUNDLE/Contents/Resources"

# Copy the Swift binary
SWIFT_BIN=$(swift build -c release --show-bin-path)/XMacApp
cp "$SWIFT_BIN" "$APP_BUNDLE/Contents/MacOS/XMacApp"
chmod +x "$APP_BUNDLE/Contents/MacOS/XMacApp"

# Copy Rust binary into bundle (self-contained, no external dependency)
RUST_BIN="$PROJECT_ROOT/target/release/x-mac"
if [ -f "$RUST_BIN" ]; then
    cp "$RUST_BIN" "$APP_BUNDLE/Contents/MacOS/xmac"
    chmod +x "$APP_BUNDLE/Contents/MacOS/xmac"
    echo "  Bundled Rust binary (xmac)"
fi

# Step 4: Copy resources
echo "[4/7] Copying resources..."
# Copy CoreML model
if [ -d "$PROJECT_ROOT/gnn/XMacGNN.mlpackage" ]; then
    cp -r "$PROJECT_ROOT/gnn/XMacGNN.mlpackage" "$APP_BUNDLE/Contents/Resources/"
    echo "  Copied XMacGNN.mlpackage"
else
    echo "  ERROR: XMacGNN.mlpackage not found in $PROJECT_ROOT/gnn/"
    echo "  Cannot build without the CoreML model. Run gnn/export_coreml.py first."
    exit 1
fi

# Copy Memory Optimizer CoreML model
if [ -d "$PROJECT_ROOT/gnn/XMacMemoryGNN.mlpackage" ]; then
    cp -r "$PROJECT_ROOT/gnn/XMacMemoryGNN.mlpackage" "$APP_BUNDLE/Contents/Resources/"
    echo "  Copied XMacMemoryGNN.mlpackage"
else
    echo "  WARNING: XMacMemoryGNN.mlpackage not found — memory optimization will be disabled"
fi

# Copy label map
if [ -f "$PROJECT_ROOT/gnn/label_map.json" ]; then
    cp "$PROJECT_ROOT/gnn/label_map.json" "$APP_BUNDLE/Contents/Resources/"
else
    echo "  ERROR: label_map.json not found in $PROJECT_ROOT/gnn/"
    exit 1
fi

# Copy or generate icon
if [ -f "$SCRIPT_DIR/AppIcon.icns" ]; then
    cp "$SCRIPT_DIR/AppIcon.icns" "$APP_BUNDLE/Contents/Resources/"
fi


# Copy launch agent helper script
if [ -f "$SCRIPT_DIR/install_launch_agent.sh" ]; then
    cp "$SCRIPT_DIR/install_launch_agent.sh" "$APP_BUNDLE/Contents/Resources/"
    chmod +x "$APP_BUNDLE/Contents/Resources/install_launch_agent.sh"
fi

# Step 5: Create Info.plist
echo "[5/7] Creating Info.plist..."
cat > "$APP_BUNDLE/Contents/Info.plist" << 'PLIST'
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
    <string>4</string>
    <key>CFBundleShortVersionString</key>
    <string>2.1.1</string>
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

# Step 6: Code sign with hardened runtime (ad-hoc, no certificate needed)
echo "[6/7] Signing with hardened runtime (ad-hoc)..."
ENTITLEMENTS="$SCRIPT_DIR/XMacApp/XMacApp.entitlements"

# Sign the Rust binary first (innermost)
if [ -f "$APP_BUNDLE/Contents/MacOS/xmac" ]; then
    codesign --force --options runtime \
        --entitlements "$ENTITLEMENTS" \
        --sign - \
        "$APP_BUNDLE/Contents/MacOS/xmac" 2>&1 | head -3
    echo "  Signed xmac (Rust binary)"
fi

# Sign the Swift binary
codesign --force --options runtime \
    --entitlements "$ENTITLEMENTS" \
    --sign - \
    "$APP_BUNDLE/Contents/MacOS/XMacApp" 2>&1 | head -3
echo "  Signed XMacApp (Swift binary)"

# Deep sign the entire bundle (catches any helper binaries)
codesign --force --deep --options runtime \
    --entitlements "$ENTITLEMENTS" \
    --sign - \
    "$APP_BUNDLE" 2>&1 | head -3
echo "  Deep signed bundle"

# Step 7: Verify the signature
echo "[7/7] Verifying signature..."
if codesign --verify --strict --verbose=2 "$APP_BUNDLE" 2>&1 | head -5; then
    echo "  Signature verification: PASSED"
else
    echo "  WARNING: Signature verification failed (non-fatal for ad-hoc)"
fi

# Show signature details
echo ""
echo "  Signature details:"
codesign --display --entitlements - "$APP_BUNDLE" 2>&1 | grep -E "Identifier|Format|CodeDirectory|Entitlements" | head -5

echo ""
echo "=== Build complete ==="
echo "App bundle: $APP_BUNDLE"
echo ""
echo "The app is signed with an ad-hoc signature (hardened runtime enabled)."
echo "Users may still need to right-click -> Open on first launch."
echo ""
echo "To install: cp -r '$APP_BUNDLE' /Applications/"
echo ""
echo "For distribution with notarization, see gui/SIGNING.md"
