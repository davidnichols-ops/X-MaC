# Digital Twin Integration Skill

## Purpose

This skill guides a Devin session through integrating 630+ technical operations into X-MaC, evolving it from a macOS cleaner/optimizer into a full macOS Digital Twin — a live computational model of the Mac.

## When to Invoke

Invoke this skill when working on any of the following:
- Adding new engines or extending existing engines in `src/engines/`
- Building the Digital Twin graph (hardware, software, filesystem, process, memory, energy models)
- Implementing AI reasoning layers (prediction, simulation, recommendation)
- Integrating with MAOS for context retrieval
- Working on the integration plan phases

## Context Retrieval via MAOS

**Always start a work session by retrieving context from MAOS:**

1. Call `maos_get_context` to get the full context packet (workspace, session, tasks, knowledge)
2. Call `maos_search_memory` with queries about the specific subsystem you're working on
3. Call `maos_list_tasks` to see pending integration tasks
4. When done with a work session, call `maos_create_task` for any follow-up work

### MAOS Search Queries

Use these search patterns when working on specific subsystems:

| Subsystem | Search Query |
|-----------|-------------|
| Filesystem intelligence | `"xmac filesystem scan clean engine"` |
| Cache analysis | `"xmac cache cleanup scanner rules"` |
| Duplicate detection | `"xmac blake3 hash duplicate"` |
| Application intelligence | `"xmac envmap apps application"` |
| Process intelligence | `"xmac optimize telemetry process"` |
| Memory intelligence | `"xmac memory optimize ram"` |
| Hardware model | `"xmac system_awareness snapshot hardware"` |
| AI advisor | `"xmac advisor recommendation intelligence"` |
| Digital twin graph | `"xmac graph engine gnn"` |
| Cleanup pipeline | `"xmac cleanup transaction undo verification"` |
| Daemon | `"xmac daemon automation background"` |
| Zen mode | `"xmac zen optimization comprehensive"` |

## Architecture Reference

X-MaC has 4 layers:
1. **GUI Layer** (`gui/`) — SwiftUI app, communicates via subprocess + NDJSON
2. **CLI Layer** (`src/cli/`) — Clap args, output formatting
3. **Engine Layer** (`src/engines/`, `src/cleanup/`, `src/intelligence/`) — core logic
4. **GNN Layer** (`gnn/`) — PyTorch GAT, CoreML on-device

### Existing 10 Engines

| Engine | Path | Current Coverage |
|--------|------|-----------------|
| Clean | `src/engines/clean/` | Caches, build artifacts, browser data, iOS backups, temp files, large files, trash |
| Disk | `src/engines/disk/` | APFS disk usage breakdown by directory |
| Maintain | `src/engines/maintain/` | DNS flush, Spotlight reindex, LaunchServices rebuild, periodic scripts, RAM purge |
| Map | `src/engines/map/` | Python/Node/container environments and disk usage |
| Depth | `src/engines/depth/` | Permissions, broken symlinks, missing dylibs |
| Conflict | `src/engines/conflict/` | PATH conflicts, env var collisions, port conflicts |
| Envmap | `src/engines/envmap/` | Environment variables across shells/apps/config |
| Graph | `src/engines/graph/` | GNN integration — builds finding graph for inference |
| Diag | `src/engines/diag/` | System diagnostics |
| Optimize | `src/engines/optimize/` | Memory optimization with GNN-based telemetry |

### Intelligence Suite

| Module | Path | Purpose |
|--------|------|---------|
| SystemSnapshot | `src/intelligence/system_awareness.rs` | Multi-dimensional snapshot: memory, CPU, thermal, battery, disk |
| Advisor | `src/intelligence/advisor.rs` | Natural-language recommendations from system snapshots |
| Daemon | `src/intelligence/daemon.rs` | Background daemon with auto-purge and automation rules |
| Zen | `src/intelligence/zen.rs` | One-click comprehensive optimization |

### Core Types

- `Engine` trait (`src/core/engine.rs`) — async `validate()` + `scan()`, streams `Finding` objects via mpsc channel
- `Finding` (`src/core/types.rs`) — id, engine, severity, category, target, title, description, metadata, size_bytes
- `ScanReport` — aggregated report with severity/category/engine breakdowns
- `ScanContext` — shared state + mpsc::Sender<Finding> for real-time streaming

### Cleanup Pipeline

Scan → Policy → Preflight → Transaction Plan → Execute (Trash-First) → Verify → Undo Metadata → History

### GNN Models

1. **XMacGNN** (file safety scorer) — 27 classes, 99.76% val accuracy, 3-layer GAT, CoreML
2. **XMacMemoryGNN** (memory optimizer) — 6 action classes, 24-dim process features, CoreML

## Integration Phases

See `docs/INTEGRATION_PLAN.md` for the full phased plan. Summary:

- **Phase 1**: Filesystem Discovery & Storage Intelligence (ops 1-30)
- **Phase 2**: Cache Analysis Engine (ops 31-60)
- **Phase 3**: Log Management (ops 61-80)
- **Phase 4**: Duplicate Detection Engine (ops 81-110)
- **Phase 5**: Application Intelligence (ops 111-145)
- **Phase 6**: Startup & Background Process Management (ops 146-175)
- **Phase 7**: macOS Database Maintenance (ops 176-205)
- **Phase 8**: Privacy & Security Operations (ops 206-235)
- **Phase 9**: Memory & Performance Optimization (ops 236-260)
- **Phase 10**: AI/Next-Gen Optimization Layer (ops 261-300)
- **Phase 11**: Hardware Reality Model (ops 1-40 of Digital Twin)
- **Phase 12**: Complete Software Genome (ops 41-80 of Digital Twin)
- **Phase 13**: Filesystem Intelligence Graph (ops 81-120 of Digital Twin)
- **Phase 14**: Process Intelligence System (ops 121-160 of Digital Twin)
- **Phase 15**: Unified Memory Intelligence (ops 161-200 of Digital Twin)
- **Phase 16**: Energy & Battery Twin (ops 201-235 of Digital Twin)
- **Phase 17**: Application Intelligence Agent (ops 236-275 of Digital Twin)
- **Phase 18**: AI Reasoning Layer (ops 276-330 of Digital Twin)

## Build & Test

```bash
cargo build                              # compile
cargo test                               # run tests (416)
cargo clippy -- -D warnings              # lint
cargo fmt --check                        # format check
cd gui/XMacApp && swift build            # build SwiftUI app
cd gui && ./build_app.sh                 # build full .app bundle
```

## Commit Conventions

```
type(scope): description
```
Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `perf`, `ci`
Scopes: `clean`, `disk`, `maintain`, `map`, `depth`, `conflict`, `envmap`, `graph`, `optimize`, `config`, `intelligence`, `gui`, `gnn`, `cli`, `twin`

## Git Safety

- **Never push to main** — push URL is disabled
- All work on `digital-twin/*` branches
- Commit locally, create PRs when ready for review
