# X-MaC Digital Twin — Integration Plan

## Overview

This document maps 630+ technical operations (300 cleaner/optimizer ops + 330 digital twin ops) to X-MaC's existing architecture and defines a phased integration plan. Each phase identifies which existing engine/module to extend, what new modules to create, and which operations map to each.

## Current State

X-MaC v2.1.1 is a macOS cleaner/optimizer with:

- **10 engines**: clean, disk, maintain, map, depth, conflict, envmap, graph, diag, optimize
- **Intelligence suite**: system_awareness (multi-dimensional snapshot), advisor (NL recommendations), daemon (background automation), zen (one-click optimization)
- **2 GNN models**: XMacGNN (file safety, 99.76% accuracy), XMacMemoryGNN (memory optimization, 6 action classes)
- **Cleanup pipeline**: scan → policy → preflight → transaction → trash-first execute → verify → undo → history
- **416 tests**, clippy clean, 10 CI jobs green

## Target State

A macOS Digital Twin — not just a cleaner, but a live computational model:

```
Physical Mac
      |
      v
Telemetry + Sensors
      |
      v
Digital Twin Graph (hardware, software, files, processes, memory, energy, behavior)
      |
      v
Reasoning Engine (observe → understand → predict → simulate → recommend → execute safely)
      |
      v
Optimization / Prediction / Automation
```

## Operation → Engine Mapping

### Cleaner/Optimizer Operations (1–300)

| Phase | Ops | Existing Engine | Action |
|-------|-----|-----------------|--------|
| 1. Filesystem Discovery & Storage Intelligence | 1–30 | `disk`, `clean` | Extend disk engine with treemap, heat maps, cloud-sync detection, incremental rescanning |
| 2. Cache Analysis Engine | 31–60 | `clean` (scanner, rules) | Extend clean scanner with per-app cache detection (Slack, Discord, Steam, Adobe, Xcode, Electron), cache age, regeneration ability, deletion safety |
| 3. Log Management | 61–80 | `clean` (scanner) | Add log-specific scanning: app logs, system logs, crash reports, diagnostic logs, oversized log detection, log compression |
| 4. Duplicate Detection Engine | 81–110 | **NEW** `duplicate` engine | New engine using BLAKE3 hashing (already a dep), perceptual hashing for images, duplicate clustering, safe deletion candidate selection |
| 5. Application Intelligence | 111–145 | `envmap`, `map` | Extend envmap with app bundle parsing, leftover detection, orphan files, broken uninstallations, app inventory, duplicate app detection |
| 6. Startup & Background Process Management | 146–175 | **NEW** `startup` engine | New engine for Login Items, LaunchAgents/Daemons scanning, plist parsing, boot impact measurement, process monitoring |
| 7. macOS Database Maintenance | 176–205 | `maintain` | Extend maintain with Spotlight rebuild, LaunchServices rebuild, Finder/Dock reset, font database, icon cache, APFS validation, SMART data |
| 8. Privacy & Security Operations | 206–235 | **NEW** `privacy` engine | New engine for browser history/cookies/autofill clearing, permission auditing, malware signature scanning, secure deletion, EXIF stripping |
| 9. Memory & Performance Optimization | 236–260 | `optimize`, `system_awareness` | Extend optimize with RAM stats, memory pressure, GPU/thermal monitoring, battery health, performance profiling, optimization scoring |
| 10. AI/Next-Gen Optimization Layer | 261–300 | `intelligence` (advisor, daemon) | Build system graph, file ownership mapping, behavioral learning, predictive storage exhaustion, Apple Silicon optimization, developer/Docker/ML workload optimization |

### Digital Twin Operations (1–330)

| Phase | Ops | Target Module | Action |
|-------|-----|---------------|--------|
| 11. Hardware Reality Model | 1–40 | `src/twin/hardware.rs` | New twin module: SoC detection, core topology, memory bandwidth, SSD health, battery model, thermal sensors, USB/Thunderbolt topology, hardware fingerprint |
| 12. Complete Software Genome | 41–80 | `src/twin/software_genome.rs` | New twin module: full inventory of apps, executables, frameworks, dylibs, kexts, system extensions, launch agents/daemons, plugins, fonts, dev tools, SDKs, package managers, Python/Node/Rust environments, Docker, VMs, AI models, datasets |
| 13. Filesystem Intelligence Graph | 81–120 | `src/twin/fs_graph.rs` + `graph` engine | New twin module: file ownership graph, creator mapping, dependency mapping, cache relationships, duplicate content graph, file importance prediction, storage growth forecasting |
| 14. Process Intelligence System | 121–160 | `src/twin/process.rs` + `optimize` | New twin module: process trees, CPU/GPU/memory/disk/network tracking, energy impact, anomaly detection, process fingerprints, bottleneck prediction, workload balancing |
| 15. Unified Memory Intelligence | 161–200 | `src/twin/memory.rs` + `optimize` | New twin module: memory topology, allocation patterns, leak detection, fragmentation, pressure forecasting, Apple Silicon unified memory model, ML/GPU/ANE memory management |
| 16. Energy & Battery Twin | 201–235 | `src/twin/energy.rs` + `system_awareness` | New twin module: battery behavior model, charging patterns, energy impact per app, battery life prediction, thermal efficiency, sleep/wake analysis, power mode recommendations |
| 17. Application Intelligence Agent | 236–275 | `src/twin/app_agent.rs` + `envmap` | New twin module: app purpose understanding, dependency analysis, uninstall impact prediction, crash prediction, app health scoring, permission auditing, suspicious behavior detection |
| 18. AI Reasoning Layer | 276–330 | `src/twin/reasoning.rs` + `intelligence` | New twin module: complete Mac knowledge graph, historical snapshots, regression detection, predictive maintenance, "why is my Mac slow?" answering, optimization simulation, reversible plans, trust scoring, system health score, digital twin visualization, API exposure, autonomous optimization |

## New Module Structure

```
src/
├── engines/
│   ├── clean/          (extended: cache analysis, log management)
│   ├── disk/           (extended: treemap, heat maps, cloud detection)
│   ├── maintain/       (extended: database maintenance, SMART)
│   ├── map/            (extended: app intelligence)
│   ├── depth/          (existing)
│   ├── conflict/       (existing)
│   ├── envmap/         (extended: app leftovers, orphan detection)
│   ├── graph/          (extended: twin graph integration)
│   ├── diag/           (existing)
│   ├── optimize/       (extended: memory intelligence, process intelligence)
│   ├── duplicate/      (NEW: phase 4)
│   ├── startup/        (NEW: phase 6)
│   └── privacy/        (NEW: phase 8)
├── twin/               (NEW: digital twin modules)
│   ├── mod.rs
│   ├── hardware.rs     (phase 11)
│   ├── software_genome.rs (phase 12)
│   ├── fs_graph.rs     (phase 13)
│   ├── process.rs      (phase 14)
│   ├── memory.rs       (phase 15)
│   ├── energy.rs       (phase 16)
│   ├── app_agent.rs    (phase 17)
│   ├── reasoning.rs    (phase 18)
│   └── model.rs        (shared twin data structures)
├── intelligence/
│   ├── advisor.rs      (extended: twin-aware recommendations)
│   ├── daemon.rs       (extended: twin-driven automation)
│   ├── zen.rs          (extended: twin-guided optimization)
│   ├── system_awareness.rs (extended: twin dimensions)
│   └── twin_bridge.rs  (NEW: connects twin to intelligence)
├── core/
│   ├── types.rs        (extended: new categories, twin types)
│   ├── engine.rs       (existing trait)
│   └── context.rs      (extended: twin context)
└── ...
```

## Phase Dependencies

```
Phase 1 (Filesystem) ──┬──> Phase 2 (Cache)
                       ├──> Phase 3 (Logs)
                       └──> Phase 4 (Duplicates)
                               │
Phase 5 (App Intel) ───┬──> Phase 6 (Startup)
                       └──> Phase 7 (Maintenance)
                               │
Phase 8 (Privacy) ──────────────┤
                               │
Phase 9 (Memory/Perf) ─────────┤
                               │
Phase 10 (AI Layer) ───────────┤
                               v
                    ┌─── Digital Twin ───┐
                    │                    │
Phase 11 (Hardware) ├─> Phase 12 (Software Genome)
                    │         │
                    │         v
                    │  Phase 13 (FS Graph)
                    │         │
                    v         v
Phase 14 (Process) ─┴─> Phase 15 (Memory)
                              │
                              v
                    Phase 16 (Energy)
                              │
                              v
                    Phase 17 (App Agent)
                              │
                              v
                    Phase 18 (AI Reasoning)
```

## Per-Phase Implementation Pattern

Each phase follows the same workflow:

1. **Retrieve context from MAOS** — `maos_search_memory` for the relevant subsystem
2. **Read existing code** — understand the current engine/module being extended
3. **Add new `Category` variants** to `src/core/types.rs` if needed
4. **Implement scanning logic** — new engine or extended scanner
5. **Add cleanup rules** if the phase produces deletable findings
6. **Wire into config profiles** — `src/config/profiles.rs`
7. **Add CLI args** — `src/cli/args.rs`
8. **Write tests** — unit tests in the engine module, integration tests in `tests/`
9. **Update GUI** — add SwiftUI view if user-facing
10. **Verify** — `cargo test`, `cargo clippy -- -D warnings`, `cargo fmt --check`
11. **Commit** — `feat(<scope>): <description>` on `digital-twin/<phase>` branch
12. **Record in MAOS** — `maos_create_task` for any follow-up work

## MAOS Integration

The Devin session should use MAOS throughout:

- **Start of session**: `maos_get_context` → understand current workspace state
- **Before each phase**: `maos_search_memory` with subsystem-specific queries
- **During work**: `maos_list_tasks` to track progress
- **After each phase**: `maos_create_task` for follow-up items
- **End of session**: `maos_run_action` with `workspace_summary` to record state

## Verification Checklist

For each phase:
- [ ] `cargo build` succeeds
- [ ] `cargo test` passes (all existing + new tests)
- [ ] `cargo clippy -- -D warnings` clean
- [ ] `cargo fmt --check` clean
- [ ] macOS-specific code gated with `#[cfg(target_os = "macos")]`
- [ ] No new dependencies without checking Cargo.toml first
- [ ] Cleanup is trash-first with undo support
- [ ] GNN safety scoring integrated where applicable
- [ ] Config profile thresholds wired
- [ ] MAOS context retrieved and tasks updated
