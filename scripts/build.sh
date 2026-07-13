#!/usr/bin/env bash
# Build the full macOS app (Rust + Swift + .app bundle).
set -e
cd "$(git rev-parse --show-toplevel)"
exec gui/build_app.sh "$@"
