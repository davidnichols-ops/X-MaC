#!/bin/bash
# release.sh — Build, sign, notarize, package, and publish an X-MaC release.
#
# Usage:
#   ./scripts/release.sh              # uses version from Cargo.toml
#   ./scripts/release.sh 2.1.1        # explicit version override
#
# Requirements (populate the placeholders below before first real run):
#   - Apple Developer ID Application certificate in keychain
#   - App Store Connect API key for notarization (or Apple ID + app password)
#   - `gh` CLI authenticated with repo write access
#   - `cargo`, `swift`, `create-dmg` (or `hdiutil`), `shasum`, `xcrun`
set -euo pipefail

# ---------------------------------------------------------------------------
# Configuration — replace these placeholders with real values before running.
# ---------------------------------------------------------------------------
# Developer ID Application certificate name (Keychain).
SIGN_IDENTITY="${SIGN_IDENTITY:-Developer ID Application: Your Name (TEAMID)}"
# App Store Connect API key for notarization (preferred over Apple ID).
NOTARY_API_KEY_ID="${NOTARY_API_KEY_ID:-YOUR_API_KEY_ID}"
NOTARY_API_KEY_ISSUER="${NOTARY_API_KEY_ISSUER:-YOUR_ISSUER_ID}"
NOTARY_API_KEY_PATH="${NOTARY_API_KEY_PATH:-$HOME/.private_keys/AuthKey_YOUR_API_KEY_ID.p8}"
# GitHub repo in <owner>/<name> form.
GH_REPO="${GH_REPO:-davidnichols-ops/X-MaC}"
# Optional: skip signing/notarization for local test builds.
SKIP_NOTARIZE="${SKIP_NOTARIZE:-0}"

# ---------------------------------------------------------------------------
# Resolve version.
# ---------------------------------------------------------------------------
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_ROOT"

if [[ $# -ge 1 ]]; then
  VERSION="$1"
else
  VERSION="$(grep -m1 '^version' Cargo.toml | sed -E 's/.*"([^"]+)".*/\1/')"
fi

if [[ -z "$VERSION" ]]; then
  echo "ERROR: could not determine version from Cargo.toml and none was provided." >&2
  exit 1
fi

APP_NAME="X-MaC"
APP_BUNDLE="$APP_NAME.app"
STAGING_DIR="$PROJECT_ROOT/gui/staging"
APP_PATH="$STAGING_DIR/$APP_BUNDLE"
DMG_PATH="$PROJECT_ROOT/target/release/$APP_NAME-$VERSION.dmg"
DIST_DIR="$PROJECT_ROOT/target/release"

echo "============================================================"
echo " X-MaC release build"
echo "   version:    $VERSION"
echo "   project:    $PROJECT_ROOT"
echo "   app bundle: $APP_PATH"
echo "   dmg:        $DMG_PATH"
echo "   repo:       $GH_REPO"
echo "============================================================"

# ---------------------------------------------------------------------------
# Step 1: Build the Rust release binary.
# ---------------------------------------------------------------------------
echo ""
echo "[1/7] Building Rust release binary..."
cargo build --release
echo "  -> target/release/x-mac"

# ---------------------------------------------------------------------------
# Step 2: Build the Swift app bundle (includes CoreML model + Info.plist).
# ---------------------------------------------------------------------------
echo ""
echo "[2/7] Building Swift app bundle..."
bash "$PROJECT_ROOT/gui/build_app.sh"
if [[ ! -d "$APP_PATH" ]]; then
  echo "ERROR: app bundle not found at $APP_PATH after build_app.sh" >&2
  exit 1
fi
echo "  -> $APP_PATH"

# ---------------------------------------------------------------------------
# Step 3: Code-sign the .app bundle.
# ---------------------------------------------------------------------------
echo ""
echo "[3/7] Code-signing $APP_BUNDLE..."
if [[ "$SKIP_NOTARIZE" == "1" ]]; then
  echo "  SKIP_NOTARIZE=1 — skipping codesign."
else
  codesign --force --deep --options runtime \
    --entitlements "$PROJECT_ROOT/gui/XMacApp/XMacApp.entitlements" \
    --sign "$SIGN_IDENTITY" \
    --timestamp \
    "$APP_PATH"
  codesign --verify --strict --verbose=2 "$APP_PATH"
  echo "  -> signed with '$SIGN_IDENTITY'"
fi

# ---------------------------------------------------------------------------
# Step 4: Notarize the app bundle.
# ---------------------------------------------------------------------------
echo ""
echo "[4/7] Notarizing $APP_BUNDLE..."
if [[ "$SKIP_NOTARIZE" == "1" ]]; then
  echo "  SKIP_NOTARIZE=1 — skipping notarization."
else
  # Zip for submission, then submit via notarytool using an API key.
  ZIP_PATH="$DIST_DIR/$APP_NAME-$VERSION.zip"
  ditto -c -k --keepParent "$APP_PATH" "$ZIP_PATH"
  xcrun notarytool submit "$ZIP_PATH" \
    --key-id "$NOTARY_API_KEY_ID" \
    --key "$NOTARY_API_KEY_PATH" \
    --issuer "$NOTARY_API_KEY_ISSUER" \
    --wait
  # Staple the ticket back onto the app.
  xcrun stapler staple "$APP_PATH"
  xcrun stapler validate "$APP_PATH"
  echo "  -> notarized and stapled"
fi

# ---------------------------------------------------------------------------
# Step 5: Create the DMG installer.
# ---------------------------------------------------------------------------
echo ""
echo "[5/7] Creating DMG..."
mkdir -p "$DIST_DIR"
rm -f "$DMG_PATH"
if command -v create-dmg >/dev/null 2>&1; then
  create-dmg \
    --volname "$APP_NAME $VERSION" \
    --window-pos 200 120 \
    --window-size 600 400 \
    --icon-size 100 \
    --app-drop-link 425 200 \
    --no-internet-enable \
    "$DMG_PATH" \
    "$STAGING_DIR"
else
  # Fallback: hdiutil with a temporary read-write image.
  TMP_DMG="$DIST_DIR/$APP_NAME-tmp.dmg"
  hdiutil create -volname "$APP_NAME $VERSION" \
    -srcfolder "$STAGING_DIR" \
    -fs HFS+ -format UDRW "$TMP_DMG"
  hdiutil convert "$TMP_DMG" -format UDZO -imagekey zlib-level=9 -o "$DMG_PATH"
  rm -f "$TMP_DMG"
fi
echo "  -> $DMG_PATH"

# ---------------------------------------------------------------------------
# Step 6: Generate SHA256 checksum.
# ---------------------------------------------------------------------------
echo ""
echo "[6/7] Generating SHA256..."
SHASUM_PATH="$DMG_PATH.sha256"
shasum -a 256 "$DMG_PATH" | awk '{print $1"  '""$APP_NAME-$VERSION.dmg"'"'}' > "$SHASUM_PATH"
echo "  -> $SHASUM_PATH"
echo "  $(cat "$SHASUM_PATH")"

# ---------------------------------------------------------------------------
# Step 7: Create the GitHub release and upload assets.
# ---------------------------------------------------------------------------
echo ""
echo "[7/7] Creating GitHub release v$VERSION..."
if ! command -v gh >/dev/null 2>&1; then
  echo "  WARNING: 'gh' CLI not found. Skipping GitHub release creation."
  echo "  Upload these assets manually:"
  echo "    $DMG_PATH"
  echo "    $SHASUM_PATH"
else
  if gh release view "v$VERSION" --repo "$GH_REPO" >/dev/null 2>&1; then
    echo "  Release v$VERSION already exists — uploading assets."
    gh release upload "v$VERSION" "$DMG_PATH" "$SHASUM_PATH" \
      --repo "$GH_REPO" --clobber
  else
    gh release create "v$VERSION" "$DMG_PATH" "$SHASUM_PATH" \
      --repo "$GH_REPO" \
      --title "X-MaC $VERSION" \
      --generate-notes
  fi
  echo "  -> https://github.com/$GH_REPO/releases/tag/v$VERSION"
fi

echo ""
echo "============================================================"
echo " Release $VERSION complete."
echo "   DMG:     $DMG_PATH"
echo "   SHA256:  $SHASUM_PATH"
echo ""
echo " Next steps:"
echo "   1. Verify the checksum in $SHASUM_PATH."
echo "   2. Update packaging/homebrew/xmac.rb with the real sha256 and URL."
echo "   3. Announce the release."
echo "============================================================"
