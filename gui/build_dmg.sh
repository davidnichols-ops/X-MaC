#!/bin/bash
set -euo pipefail

# Build a distributable macOS DMG for X-MaC.
#
# Environment variables:
#   XMAC_SKIP_APP_BUILD     - set to 1 to skip building the .app bundle
#   XMAC_UNIVERSAL          - set to 1 when the binaries are universal (macOS DMG name)
#   CODESIGN_IDENTITY       - "Developer ID Application: ..." to code-sign the .app
#   APPLE_ID                - Apple ID for notarytool
#   APPLE_TEAM_ID           - Apple Developer Team ID
#   APPLE_APP_SPECIFIC_PASSWORD - app-specific password for notarytool

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
STAGING_DIR="$SCRIPT_DIR/staging"
APP_BUNDLE="$STAGING_DIR/X-MaC.app"
DMG_STAGING="$SCRIPT_DIR/dmg_staging"

VERSION="$(grep -m1 '^version' "$PROJECT_ROOT/Cargo.toml" | sed 's/.*"\(.*\)".*/\1/')"
ARCH="${XMAC_ARCH:-$(uname -m)}"

if [ "${XMAC_UNIVERSAL:-0}" = "1" ]; then
    DMG_NAME="X-MaC-${VERSION}-macOS.dmg"
else
    DMG_NAME="X-MaC-${VERSION}-${ARCH}.dmg"
fi
DMG_DIR="$PROJECT_ROOT/target/release"
DMG_PATH="$DMG_DIR/$DMG_NAME"

mkdir -p "$DMG_DIR"

# Step 1: Build the .app bundle
echo "=== Building X-MaC DMG v$VERSION ==="
if [ "${XMAC_SKIP_APP_BUILD:-0}" != "1" ]; then
    echo "[1/5] Building .app bundle..."
    (
        export XMAC_UNIVERSAL="${XMAC_UNIVERSAL:-0}"
        bash "$SCRIPT_DIR/build_app.sh"
    )
else
    echo "[1/5] Skipping .app build, expecting: $APP_BUNDLE"
    if [ ! -d "$APP_BUNDLE" ]; then
        echo "  ERROR: App bundle not found at $APP_BUNDLE"
        exit 1
    fi
fi

# Step 2: Optionally code-sign the .app
echo "[2/5] Checking code signing..."
if [ -n "${CODESIGN_IDENTITY:-}" ]; then
    echo "  Signing app bundle with '$CODESIGN_IDENTITY'..."
    codesign --deep --force --verify --verbose \
        --sign "$CODESIGN_IDENTITY" \
        --options runtime \
        "$APP_BUNDLE"
else
    echo "  No CODESIGN_IDENTITY set; app will be unsigned."
    echo "  Gatekeeper will block the app until you right-click -> Open."
fi

# Step 3: Prepare DMG staging
echo "[3/5] Preparing DMG staging..."
rm -rf "$DMG_STAGING"
mkdir -p "$DMG_STAGING"
cp -a "$APP_BUNDLE" "$DMG_STAGING/"
ln -s /Applications "$DMG_STAGING/Applications"

# Step 4: Create the DMG
echo "[4/5] Creating DMG..."
APP_SIZE_MB=$(du -sm "$APP_BUNDLE" | cut -f1)
DMG_SIZE_MB=$((APP_SIZE_MB + 60))

TMP_DMG="/tmp/X-MaC-temp-$$.dmg"
rm -f "$TMP_DMG"

hdiutil create \
    -srcfolder "$DMG_STAGING" \
    -volname "X-MaC $VERSION" \
    -fs HFS+ \
    -format UDRW \
    -size "${DMG_SIZE_MB}m" \
    "$TMP_DMG"

rm -f "$DMG_PATH"
hdiutil convert "$TMP_DMG" -format UDZO -o "$DMG_PATH"
rm -f "$TMP_DMG"

# Step 5: Optionally notarize and staple
echo "[5/5] Checking notarization..."
if [ -n "${APPLE_ID:-}" ] && [ -n "${APPLE_TEAM_ID:-}" ] && [ -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" ]; then
    echo "  Submitting DMG for notarization..."
    xcrun notarytool submit "$DMG_PATH" \
        --apple-id "$APPLE_ID" \
        --team-id "$APPLE_TEAM_ID" \
        --password "$APPLE_APP_SPECIFIC_PASSWORD" \
        --wait
    echo "  Stapling notarization ticket..."
    xcrun stapler staple "$DMG_PATH"
else
    echo "  No Apple notarization credentials set; DMG will be unsigned/unnotarized."
fi

echo ""
echo "=== DMG created ==="
echo "$DMG_PATH"
