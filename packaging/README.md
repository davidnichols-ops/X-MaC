# Packaging

Distribution packaging for X-MaC.

## Homebrew

```bash
# Create a tap (one-time setup)
brew tap-new davidnichols-ops/xmac

# Copy the formula to your tap
cp packaging/homebrew/xmac.rb $(brew --repository)/Library/Taps/davidnichols-ops/homebrew-xmac/Formula/

# Install
brew install xmac

# Or install directly from the repo
brew install --HEAD https://raw.githubusercontent.com/davidnichols-ops/X-MaC/main/packaging/homebrew/xmac.rb
```

## Building a Release

```bash
# Tag a release
git tag v2.1.0
git push origin v2.1.0

# Build the release binary
cargo build --release

# Build the macOS app
cd gui && ./build_app.sh

# Create a notarized DMG (requires Apple Developer account)
# TODO: Add notarization script
```

## CI Release Artifacts

The GitHub Actions workflow (`.github/workflows/ci.yml`) builds and tests on every push.
A release workflow (`.github/workflows/release.yml`) should be added to:

1. Build release binaries for macOS (universal) and Linux (x86_64, aarch64)
2. Create a GitHub Release with attached binaries
3. Generate a Homebrew bottle
4. Submit the macOS app for notarization

See [ROADMAP.md](../ROADMAP.md) for distribution plans.
