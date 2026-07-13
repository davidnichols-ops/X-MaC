<div align="center">

<img src="logo.png" alt="X-MaC" width="180" />

# X-MaC

### Open-source macOS cleaner, optimizer & system monitor — with on-device GNN intelligence

[![CI](https://github.com/davidnichols-ops/X-MaC/actions/workflows/ci.yml/badge.svg)](https://github.com/davidnichols-ops/X-MaC/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.78+-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![Swift](https://img.shields.io/badge/Swift-5.9+-orange?style=flat-square&logo=swift)](https://swift.org)
[![Platform](https://img.shields.io/badge/macOS-13%2B-blue?style=flat-square&logo=apple)](https://www.apple.com/macos)
[![License](https://img.shields.io/badge/license-MIT-lightgrey?style=flat-square)](LICENSE)
[![Tests](https://img.shields.io/badge/tests-410+-brightgreen?style=flat-square)](#testing)

[Install](#installation) · [Features](#features) · [Architecture](#architecture) · [Contributing](CONTRIBUTING.md) · [Roadmap](ROADMAP.md)

</div>

---

X-MaC is a free, open-source Mac cleaner that combines a fast Rust scan engine, a Graph Neural Network safety scorer, and a native SwiftUI app — all running entirely on your device. Nothing ever leaves your Mac.

## Why X-MaC?

| | CleanMyMac | CleanerOne Pro | **X-MaC** |
|---|:---:|:---:|:---:|
| Free & open-source | ✗ | ✗ | ✅ |
| On-device GNN scoring | ✗ | ✗ | ✅ |
| Rust scan engine | ✗ | ✗ | ✅ |
| CLI + GUI | ✗ | ✗ | ✅ |
| Never deletes without asking | sometimes | sometimes | ✅ always |
| No subscription | ✗ | ✗ | ✅ |
| Config profiles (Gaming, Dev, etc.) | ✗ | ✗ | ✅ |
| Background daemon | ✗ | ✗ | ✅ |
| AI advisor | ✗ | ✗ | ✅ |

## Features

### CLI

```bash
xmac quick              # clean + maintain + disk overview in one shot
xmac clean              # find reclaimable space (caches, build artifacts, browsers, Docker)
xmac purge              # clean + delete with confirmation and undo
xmac disk               # disk usage breakdown with APFS-accurate stats
xmac maintain           # flush DNS, reindex Spotlight, rebuild LaunchServices
xmac scan               # full system scan (all engines)
xmac map                # map Python/Node/container environments
xmac conflict           # detect PATH and environment conflicts
xmac depth              # filesystem integrity (permissions, symlinks, dylibs)
xmac advisor            # AI advisor — natural-language system health recommendations
xmac zen                # one-click comprehensive optimization (preview or execute)
xmac config             # manage config, profiles, settings
xmac daemon             # background daemon with auto-purge and automation rules
xmac history            # scan history and analytics
xmac completions        # generate shell completions (zsh, bash, fish, elvish, powershell)
```

### GUI

- **Dashboard** — action-first hero with one-tap Quick Clean and reclaimable total
- **Zen Mode** — one-click comprehensive optimization with before/after health score
- **AI Advisor** — health score ring, system status, prioritized recommendations
- **Disk Analyzer** — interactive donut chart with live hover tooltips
- **Smart Scan (GNN)** — graph neural network scores every finding by safety
- **Clean / Maintain / Map / Depth** — full engine access with category breakdowns
- **Menu Bar Extra** — quick access to Zen Mode, AI Advisor, and Quick Clean
- **Onboarding** — first-launch walkthrough
- **Crash reporter + adaptive fixer** — logs errors, auto-applies known recovery patterns

### Intelligence Suite

- **Config profiles** — 7 profiles (Balanced, Gaming, Development, Video Editing, Conservative, Aggressive, Custom) that tune engine thresholds
- **Background daemon** — auto-purge on memory pressure, auto-clean on disk pressure, automation rules with cooldowns
- **AI Advisor** — multi-dimensional system awareness (CPU + memory + thermal + battery + disk) with natural-language recommendations
- **Zen Mode** — comprehensive optimization with preview, before/after health score, memory delta, disk reclaimable summary
- **Adaptive learning** — tracks user feedback to adjust advisor confidence over time
- **History & analytics** — scan history with export and trend tracking

## Architecture

```
┌─────────────────────────────────────────────────────┐
│              SwiftUI App  (gui/)                    │  ← What users see
│  Dashboard · Zen · Advisor · Disk · Clean · Menu Bar │
├─────────────────────────────────────────────────────┤
│         Intelligence Suite  (src/intelligence/)     │  ← AI + automation
│  Advisor · Daemon · Zen Mode · System Awareness     │
├─────────────────────────────────────────────────────┤
│          Config System  (src/config/)               │  ← User preferences
│  Profiles · TOML Store · Automation Rules           │
├─────────────────────────────────────────────────────┤
│       GNN Inference  (gnn/ + CoreML on-device)      │  ← Safety scoring
│  PyTorch GCN → CoreML → on-device inference         │
├─────────────────────────────────────────────────────┤
│         Rust Scan Engine  (src/engines/)            │  ← Speed + safety
│  Clean · Disk · Maintain · Map · Depth · Optimize   │
│  Conflict · Envmap · Graph · Diag                   │
├─────────────────────────────────────────────────────┤
│         Safe Cleanup  (src/cleanup/)                │  ← Trash-first
│  Transaction · Undo · Verification · Preflight      │
└─────────────────────────────────────────────────────┘
```

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for detailed diagrams and module relationships.

## Installation

### macOS App (GUI + CLI)

**Requirements:** macOS 13 Ventura or later, Apple Silicon or Intel.

```bash
git clone https://github.com/davidnichols-ops/X-MaC.git
cd X-MaC/gui
./build_app.sh
cp -r staging/X-MaC.app /Applications/
open /Applications/X-MaC.app
```

The build script compiles the Rust binary, bundles it inside the `.app` along with the CoreML model — no external dependencies at runtime.

### CLI only

```bash
git clone https://github.com/davidnichols-ops/X-MaC.git
cd X-MaC
cargo build --release
./target/release/x-mac install   # installs xmac to ~/.local/bin
xmac quick
```

### Linux

```bash
git clone https://github.com/davidnichols-ops/X-MaC.git
cd X-MaC
cargo build --release
./target/release/x-mac quick --no-disk
```

macOS-specific features (Spotlight, LaunchServices, purge) gracefully degrade on Linux.

### Requirements

| Component | Requirement |
|---|---|
| CLI build | Rust 1.78+ (`rustup update stable`) |
| GUI build | Xcode 15+, Swift 5.9+, macOS 13+ SDK |
| GNN training | Python 3.10+, PyTorch 2.x (optional — pre-trained model included) |

## Quick Start

```bash
# See what can be cleaned (no deletion)
xmac clean

# Get AI recommendations for your system
xmac advisor

# Preview a comprehensive optimization
xmac zen --no-clean --no-maintain

# Run safe cleanup + maintenance + disk overview
xmac quick

# Set a gaming profile (aggressive memory cleanup)
xmac config set-profile gaming

# Start the background daemon
xmac daemon --start
```

## Configuration

X-MaC reads config from `~/.config/xmac/config.toml` (or `~/Library/Application Support/xmac/config.toml` on macOS).

```bash
xmac config init              # create default config
xmac config profiles          # list available profiles
xmac config set-profile gaming  # switch to gaming profile
xmac config get clean.min_age_days
xmac config set clean.min_age_days 7
```

See [examples/configs/](examples/configs/) for sample configurations and [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for config system details.

## Project Structure

```
X-MaC/
├── src/                    # Rust engine (the core)
│   ├── core/               # Engine trait, types, context, errors
│   ├── engines/            # 10 scan engines
│   │   ├── clean/          # Cache, build artifact, browser, iOS backup scanner
│   │   ├── disk/           # APFS-aware disk usage analyzer
│   │   ├── maintain/       # macOS/Linux maintenance tasks
│   │   ├── optimize/       # Memory optimizer with GNN telemetry
│   │   ├── graph/          # GNN integration (Rust side)
│   │   ├── map/            # Python/Node/container environment mapper
│   │   ├── depth/          # Filesystem integrity checker
│   │   ├── conflict/       # PATH and environment conflict detector
│   │   ├── envmap/         # Environment variable mapper
│   │   └── diag/           # System diagnostics
│   ├── cleanup/            # Safe deletion: trash-first, dry-run, undo
│   ├── cli/                # Clap CLI, argument parsing, output
│   ├── config/             # TOML config, optimization profiles
│   ├── intelligence/       # System awareness, AI advisor, daemon, zen mode
│   └── util/               # Disk, memory, macOS, backup utilities
│
├── gui/                    # Native SwiftUI macOS app
│   └── XMacApp/
│       └── Sources/XMacApp/
│           ├── XMacApp.swift        # App entry point + menu bar
│           ├── XMacRunner.swift     # Rust bridge (process runner)
│           ├── ContentView.swift    # Sidebar + navigation
│           ├── DashboardView.swift  # Hero dashboard
│           ├── ZenView.swift        # Zen Mode optimization
│           ├── AdvisorView.swift    # AI Advisor
│           ├── DiskView.swift       # Donut chart disk analyzer
│           ├── NeuralScanView.swift # GNN smart scan
│           └── ...
│
├── gnn/                    # On-device Graph Neural Network
│   ├── model/              # PyTorch GNN architecture
│   ├── data/               # Training data
│   ├── train.py            # Training script
│   └── XMacGNN.mlpackage   # Pre-trained CoreML model
│
├── tests/                  # Rust integration tests
├── docs/                   # Architecture docs, diagrams, style guide
├── examples/               # Example configs and usage
├── scripts/                # Helper scripts (lint, format, build)
└── .github/                # CI workflows, issue templates
```

## Testing

```bash
cargo test                  # run all 410+ tests
cargo test --lib            # library tests only (fast)
cargo test -- --nocapture   # with output
cargo clippy -- -D warnings # lint
cargo fmt --check           # format check
```

See [DEVELOPMENT.md](DEVELOPMENT.md) for detailed testing instructions.

## Contributing

All contributions welcome — from a one-line typo fix to a new scan engine.

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full guide, and [GOOD_FIRST_ISSUES.md](GOOD_FIRST_ISSUES.md) for beginner-friendly tasks.

Quick start:
```bash
git clone https://github.com/davidnichols-ops/X-MaC.git
cd X-MaC
cargo build && cargo test
```

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the full roadmap.

- [ ] **v2.2** — Duplicate file finder, Space Lens treemap, CSV export
- [ ] **v2.3** — Homebrew tap, TestFlight beta, notarized DMG
- [ ] **v3.0** — Cross-platform GUI, plugin system, App Store submission

## License

MIT — see [LICENSE](LICENSE). Do whatever you want, attribution appreciated.

## Acknowledgements

Built with:
- [Rust](https://www.rust-lang.org/) + [Tokio](https://tokio.rs/) — async scan engine
- [SwiftUI](https://developer.apple.com/xcode/swiftui/) — native macOS UI
- [PyTorch](https://pytorch.org/) + [Core ML](https://developer.apple.com/documentation/coreml) — on-device GNN
- [WalkDir](https://github.com/BurntSushi/walkdir) — fast filesystem traversal
- [Clap](https://clap.rs/) — CLI argument parsing
