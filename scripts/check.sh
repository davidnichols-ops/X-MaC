#!/usr/bin/env bash
# Run all pre-commit checks: format, clippy, test, Swift build.
set -e

cd "$(git rev-parse --show-toplevel)"

echo "=== cargo fmt --check ==="
cargo fmt --check

echo "=== cargo clippy (all targets) ==="
cargo clippy --all-targets -- -D warnings

echo "=== cargo test ==="
cargo test

echo "=== swift build ==="
cd gui/XMacApp && swift build 2>&1 | tail -3

echo ""
echo "All checks passed."
