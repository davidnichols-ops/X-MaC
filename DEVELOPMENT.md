# Development Setup

This guide gets you from `git clone` to running tests in under 10 minutes.

## Prerequisites

| Component | Requirement | Install |
|-----------|-------------|---------|
| Rust | 1.78+ stable | `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs \| sh` |
| Swift | 5.9+ (GUI only) | Install Xcode 15+ from the App Store |
| Python | 3.10+ (GNN only) | `brew install python@3.12` |
| macOS | 13+ (GUI) / any (CLI) | — |

## Quick Start

```bash
git clone https://github.com/davidnichols-ops/X-MaC.git
cd X-MaC

# Build the Rust engine
cargo build

# Run the test suite
cargo test

# Try the CLI
./target/debug/x-mac quick --no-disk
./target/debug/x-mac advisor
./target/debug/x-mac zen --no-clean --no-maintain

# Build the GUI app (macOS only)
cd gui && ./build_app.sh
open staging/X-MaC.app
```

## Project Structure

```
X-MaC/
├── src/                # Rust engine, CLI, intelligence suite
│   ├── core/           # Engine trait, types, context, errors
│   ├── engines/        # 10 scan engines (clean, disk, maintain, ...)
│   ├── cleanup/        # Safe deletion, undo, transaction, verification
│   ├── cli/            # Clap CLI, argument parsing, output
│   ├── config/         # TOML config, optimization profiles
│   ├── intelligence/   # System awareness, AI advisor, daemon, zen mode
│   └── util/           # Disk, memory, macOS, backup utilities
├── gui/                # SwiftUI macOS app
│   └── XMacApp/        # SwiftPM package
├── gnn/                # PyTorch GNN model + CoreML export
├── tests/              # Rust integration tests
├── docs/               # Architecture docs, diagrams
├── scripts/            # Helper scripts
└── examples/           # Example configs and usage
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the full system design.

## Running Tests

```bash
# All tests
cargo test

# Just the library tests (fast)
cargo test --lib

# Just integration tests
cargo test --test integration_tests

# A specific test
cargo test test_advisor_produces_recommendations

# With output
cargo test -- --nocapture
```

The test suite has 327+ tests covering:
- Engine scanning logic (clean, disk, maintain, etc.)
- Cleanup transaction safety and undo
- Config loading and profile application
- AI advisor recommendations and adaptive learning
- Zen mode formatting and serialization
- Daemon PID file management

## Linting and Formatting

```bash
cargo fmt                # format code
cargo fmt --check        # check formatting without changing
cargo clippy             # lint
cargo clippy -- -D warnings   # treat warnings as errors
```

## Building the GUI

```bash
cd gui
./build_app.sh           # builds Rust + Swift, creates .app bundle
open staging/X-MaC.app   # launch it
```

The build script:
1. Compiles the Rust binary (`cargo build --release`)
2. Installs it to `~/.local/bin/xmac`
3. Compiles the Swift app (`swift build -c release`)
4. Creates a `.app` bundle with the Rust binary and CoreML models bundled inside

## GNN Development (Optional)

The GNN model is pre-trained and bundled. You only need this if you want to retrain:

```bash
cd gnn
python -m venv .venv
source .venv/bin/activate
pip install torch torch-geometric numpy

# Train the safety scoring model
python train.py

# Train the memory optimization model
python train_memory_gnn.py

# Export to CoreML
python export_coreml.py
python export_memory_coreml.py
```

See [gnn/README.md](gnn/README.md) for details.

## Linux Cross-Compilation

The CLI works on Linux. To verify it compiles:

```bash
# Install the target
rustup target add x86_64-unknown-linux-gnu

# Check compilation (doesn't link, just type-checks)
cargo check --target x86_64-unknown-linux-gnu
```

Note: macOS-specific features (Spotlight, LaunchServices, purge, etc.) are conditionally compiled with `#[cfg(target_os = "macos")]` and gracefully degrade on Linux.

## Debugging

### Rust
```bash
RUST_LOG=debug cargo run -- quick --no-disk
RUST_LOG=trace cargo run -- clean 2>&1 | head -50
```

### GUI
The GUI logs to `~/Library/Logs/X-MaC/`. Check the crash reporter at `gui/XMacApp/Sources/XMacApp/CrashReporter.swift`.

## Configuration

X-MaC reads config from `~/.config/xmac/config.toml` (or `~/Library/Application Support/xmac/config.toml` on macOS). See [examples/configs/](examples/configs/) for sample configurations.

```bash
xmac config init          # create default config
xmac config profiles      # list available profiles
xmac config set-profile gaming
xmac config get clean.min_age_days
```
