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

# Step 1: Build the Rust binary
echo "[1/5] Building Rust binary..."
cd "$PROJECT_ROOT"
cargo build --release 2>&1 | tail -3
cp target/release/x-mac ~/.local/bin/xmac
echo "  Rust binary installed to ~/.local/bin/xmac"

# Step 2: Build the Swift app
echo "[2/5] Building Swift app..."
cd "$SCRIPT_DIR/XMacApp"
swift build -c release 2>&1 | tail -5

# Step 3: Create the .app bundle
echo "[3/5] Creating .app bundle..."
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
echo "[4/5] Copying resources..."
# Copy CoreML model
if [ -d "$PROJECT_ROOT/gnn/XMacGNN.mlpackage" ]; then
    cp -r "$PROJECT_ROOT/gnn/XMacGNN.mlpackage" "$APP_BUNDLE/Contents/Resources/"
    echo "  Copied XMacGNN.mlpackage"
elif [ -d "/Applications/X-MaC.app/Contents/Resources/XMacGNN.mlpackage" ]; then
    cp -r "/Applications/X-MaC.app/Contents/Resources/XMacGNN.mlpackage" "$APP_BUNDLE/Contents/Resources/"
    echo "  Copied XMacGNN.mlpackage from existing install"
fi

# Copy label map
if [ -f "$PROJECT_ROOT/gnn/label_map.json" ]; then
    cp "$PROJECT_ROOT/gnn/label_map.json" "$APP_BUNDLE/Contents/Resources/"
elif [ -f "/Applications/X-MaC.app/Contents/Resources/label_map.json" ]; then
    cp "/Applications/X-MaC.app/Contents/Resources/label_map.json" "$APP_BUNDLE/Contents/Resources/"
fi

# Copy or generate icon
if [ -f "$SCRIPT_DIR/AppIcon.icns" ]; then
    cp "$SCRIPT_DIR/AppIcon.icns" "$APP_BUNDLE/Contents/Resources/"
elif [ -f "/Applications/X-MaC.app/Contents/Resources/AppIcon.icns" ]; then
    cp "/Applications/X-MaC.app/Contents/Resources/AppIcon.icns" "$APP_BUNDLE/Contents/Resources/"
fi


# Copy launch agent helper script
if [ -f "$SCRIPT_DIR/install_launch_agent.sh" ]; then
    cp "$SCRIPT_DIR/install_launch_agent.sh" "$APP_BUNDLE/Contents/Resources/"
    chmod +x "$APP_BUNDLE/Contents/Resources/install_launch_agent.sh"
fi

# Step 5: Create Info.plist
echo "[5/5] Creating Info.plist..."
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
    <string>1</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0.0</string>
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
