# Code Signing & Notarization Guide

This document explains how X-MaC is signed and how to enable full
notarization for distribution.

## Current State (Phase 1 — Ad-Hoc Signing)

The build script (`gui/build_app.sh`) applies an **ad-hoc signature**
with the **hardened runtime** enabled. This requires no Apple Developer
certificate and provides:

- Hardened runtime protections (W^X, library validation, etc.)
- Entitlements for CoreML, Apple Events, and file access
- `codesign --verify` passes on the built bundle

### What ad-hoc signing does NOT provide

- Gatekeeper will still show "unidentified developer" on first launch
- Users must right-click → Open to bypass (or `xattr -d com.apple.quarantine`)
- No notarization ticket (no `spctl` approval)
- No DMG distribution without user warning

## Phase 2 — Developer ID Signing & Notarization

To enable full notarization (no Gatekeeper warnings), you need:

1. **Apple Developer Program membership** ($99/year)
2. **Developer ID Application certificate** (created in developer.apple.com)
3. **App Store Connect API key** (for `notarytool`)

### Setup

1. **Obtain Developer ID certificate:**
   - Go to developer.apple.com → Certificates, IDs & Profiles
   - Create a "Developer ID Application" certificate
   - Install it in Keychain Access

2. **Generate App Store Connect API key:**
   - Go to App Store Connect → Users and Access → Keys
   - Generate an API key with "Developer" role
   - Download the `.p8` file
   - Note the Key ID and Issuer ID

3. **Set environment variables:**
   ```bash
   export SIGN_IDENTITY="Developer ID Application: Your Name (TEAMID)"
   export NOTARY_API_KEY_ID="YOUR_API_KEY_ID"
   export NOTARY_API_KEY_ISSUER="YOUR_ISSUER_ID"
   export NOTARY_API_KEY_PATH="/path/to/AuthKey_YOUR_API_KEY_ID.p8"
   ```

4. **Run the release script:**
   ```bash
   ./scripts/release.sh
   ```

### Release Script

`scripts/release.sh` handles:
- Building the .app bundle
- Code signing with Developer ID
- Submitting to Apple for notarization
- Stapling the notarization ticket
- Verifying with `spctl`

### Verification

After notarization, verify the app:
```bash
spctl -a -v /path/to/X-MaC.app
# Should output: "X-MaC.app: accepted"
```

## Entitlements

The entitlements file (`gui/XMacApp/XMacApp.entitlements`) includes:

| Entitlement | Purpose |
|-------------|---------|
| `com.apple.security.cs.allow-jit` | CoreML inference |
| `com.apple.security.cs.allow-unsigned-executable-memory` | CoreML model loading |
| `com.apple.security.cs.disable-library-validation` | Load unsigned libraries |
| `com.apple.security.automation.apple-events` | Run maintenance commands |
| `com.apple.security.files.user-selected.read-write` | File cleanup operations |

## Troubleshooting

### "App is damaged" error

This usually means the notarization ticket is missing or stale. Re-run:
```bash
xcrun notarytool submit X-MaC.zip --key-id $NOTARY_API_KEY_ID --key $NOTARY_API_KEY_PATH --issuer $NOTARY_API_KEY_ISSUER --wait
xcrun stapler staple X-MaC.app
```

### CoreML model fails to load

Ensure the entitlements include `com.apple.security.cs.allow-jit` and
`com.apple.security.cs.allow-unsigned-executable-memory`.

### Apple Events denied

Ensure `com.apple.security.automation.apple-events` is in the entitlements.
The app may also need to be added to System Settings → Privacy & Security → Automation.

## File Locations

- **Entitlements**: `gui/XMacApp/XMacApp.entitlements`
- **Build script**: `gui/build_app.sh`
- **Release script**: `scripts/release.sh`
- **Info.plist**: generated inline in `build_app.sh`
