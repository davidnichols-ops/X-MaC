# X-MaC — The Intelligence Layer Above macOS

X-MaC is an open-source macOS cleaner, optimizer, and system monitor that builds a live **digital twin** of your machine and reasons over it with an on-device graph neural network. It is written in Rust, ships a SwiftUI app, and runs its predictions locally via CoreML — no network calls, no telemetry, no account required.

- **License:** MIT
- **Current version:** 2.1.1
- **Requires:** macOS 13 (Ventura) or newer
- **Repo:** [github.com/davidnichols-ops/X-MaC](https://github.com/davidnichols-ops/X-MaC)

---

## What it is

X-MaC is a single binary (`xmac`) plus an optional SwiftUI app (`X-MaC.app`) that combine 13 analysis engines, a digital twin model, and an on-device GNN into one tool. It replaces the "scan once, delete blindly" model of commercial cleaners with an observe → understand → predict → simulate → recommend → execute loop, where every recommended action is checked against YAML safety rules before anything is touched.

Everything runs on your machine. The GNN is exported to CoreML and bundled inside the app; the event store is a local SQLite file; the safety rules are plain YAML you can read and edit.

---

## The Digital Twin

The digital twin is a queryable model of your Mac assembled from live observers. It is not a snapshot — it is continuously updated as the system changes.

```
Physical Mac
      |
      v
Live Observers (fs events, process table, power, memory, launchd, Spotlight metadata)
      |
      v
SQLite Event Store (append-only, UUIDv7 keyed, local only)
      |
      v
Graph Projection (hardware + software genome + filesystem graph + process/memory/energy)
      |
      v
AI Reasoning (GNN predictions + causal simulation + safety-checked recommendations)
      |
      v
Actions (trash-first cleanup, optimization, maintenance — all undoable)
```

The twin is composed of these dimensions, each in `src/twin/`:

| Dimension | File | What it models |
|---|---|---|
| HardwareProfile | `hardware.rs` | CPU, GPU, memory, disk, sensors |
| SoftwareGenome | `software_genome.rs` | Installed apps, versions, dependencies, health |
| FilesystemGraph | `fs_graph.rs` | Directory tree as a graph with sizes and duplicates |
| ProcessIntelligence | `process.rs` | Running processes, resource use, ancestry |
| MemoryIntelligence | `memory.rs` | Pressure, swap, per-app footprint |
| EnergyTwin | `energy.rs` | Power draw, battery health, per-app energy |
| AppIntelligenceGraph | `app_agent.rs` | Per-app behavior, launch frequency, caches |
| ReasoningEngine | `reasoning.rs` | Causal analysis, simulation, recommendation |

---

## Safety Model

X-MaC never deletes in place. The safety model is layered:

1. **Trash-first.** Every cleanup action moves files to the user's Trash (or an internal undo store), never `rm`. Undo is always available.
2. **YAML safety rules.** A human-readable ruleset in `rules/` defines what is protected, what is safe to clean, and what requires explicit confirmation. You can read and edit them.
3. **Simulation before execution.** The reasoning engine projects the effect of an action onto the twin and checks the ruleset before proposing it.
4. **Explicit confirmation.** Anything outside the "safe" tier prompts the user (CLI) or the app (GUI) before running.
5. **No telemetry.** No usage data, crash reports, or system profiles leave your machine. There is no analytics SDK and no network call in the cleanup path.

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│  X-MaC.app (SwiftUI)            xmac (CLI)                   │
│         │                              │                      │
│         └────────────┬─────────────────┘                      │
│                      v                                       │
│           13 Engines (src/, async Engine trait)              │
│  clean · disk · depth · diag · envmap · graph · maintain ·   │
│  map · optimize · conflict · duplicate · startup · privacy   │
│                      │                                       │
│                      v                                       │
│           Digital Twin (src/twin/)                           │
│  hardware · software_genome · fs_graph · process ·           │
│  memory · energy · app_agent · reasoning                     │
│                      │                                       │
│         ┌────────────┴────────────┐                          │
│         v                         v                          │
│  SQLite Event Store        GNN (CoreML, on-device)           │
│  (append-only, local)      predictions + simulation          │
│         │                         │                          │
│         └────────────┬────────────┘                          │
│                      v                                       │
│           Reasoning Engine                                    │
│  observe → understand → predict → simulate →                 │
│  recommend → execute safely                                   │
│                      │                                       │
│                      v                                       │
│           YAML Safety Rules (rules/)                         │
│                      │                                       │
│                      v                                       │
│           MCP Server (context for external agents)           │
└─────────────────────────────────────────────────────────────┘
```

Key properties:

- **13 engines** implementing a shared async `Engine` trait; findings stream over `mpsc` channels for live GUI updates.
- **GNN predictions** run on-device via CoreML (`gnn/XMacGNN.mlpackage`). No network calls.
- **SQLite event store** is append-only, UUIDv7 keyed, and lives under your user directory.
- **Live observers** use `notify` (filesystem), the process table, power APIs, and Spotlight metadata to keep the twin current.
- **MCP server** exposes the twin and engine results to external AI agents for context retrieval.
- **YAML safety rules** gate every destructive action.

---

## Comparison

| Feature | X-MaC | CleanMyMac | Gargantua |
|---|---|---|---|
| License | MIT (open source) | Proprietary, paid | Proprietary |
| Source code | Public | Closed | Closed |
| Telemetry | None | Present | Present |
| Digital twin model | Yes (live graph) | No | No |
| On-device GNN predictions | Yes (CoreML) | No | No |
| Causal simulation before action | Yes | No | No |
| SQLite event store | Yes | No | No |
| Live filesystem observers | Yes | No (scan on demand) | No |
| YAML-editable safety rules | Yes | No | No |
| Trash-first with undo | Yes | Yes | Partial |
| CLI + GUI | Both | GUI only | GUI only |
| MCP server for agent context | Yes | No | No |
| Price | Free | Subscription | One-time / subscription |

---

## Installation

### Homebrew (cask — GUI app)

```bash
brew tap davidnichols-ops/xmac
brew install --cask xmac
```

This installs `X-MaC.app` into `/Applications`.

### Homebrew (formula — CLI only)

```bash
brew install davidnichols-ops/xmac/xmac
```

### From source

```bash
git clone https://github.com/davidnichols-ops/X-MaC.git
cd X-MaC
cargo build --release
# CLI binary:
./target/release/xmac --version
# Full app bundle:
cd gui && ./build_app.sh
```

### Direct download

Download the latest DMG from the [releases page](https://github.com/davidnichols-ops/X-MaC/releases), drag `X-MaC.app` to `/Applications`, and verify the SHA256 checksum published alongside each release.

---

## CLI examples

```bash
# Show version and detected hardware
xmac --version
xmac diag

# Scan for cleanable files without deleting anything
xmac clean --dry-run

# Clean with the default safety profile (trash-first, undoable)
xmac clean

# Clean with a stricter safety profile
xmac clean --profile conservative

# Disk usage analysis, top 50 largest paths
xmac disk --top 50

# Find duplicate files by BLAKE3 hash
xmac duplicate scan

# Graph the filesystem under a path
xmac graph ~/Developer

# Run the maintenance engine (caches, logs, permissions)
xmac maintain

# Map all installed apps and their health
xmac map

# Optimize memory (with GNN-backed recommendation)
xmac optimize memory

# Inspect the digital twin
xmac twin show
xmac twin simulate --action "clean caches"

# Export completions
xmac completions --shell zsh > ~/.zfunc/_xmac
```

---

## License

MIT. See [LICENSE](../LICENSE).
