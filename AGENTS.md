# X-MaC — Agent & Contributor Guide

## Build & Test Commands

```bash
cargo build                              # compile
cargo test                               # run tests (416+)
cargo clippy -- -D warnings              # lint (treat warnings as errors)
cargo fmt --check                        # format check
cargo check --target x86_64-unknown-linux-gnu  # Linux cross-compile check
cd gui/XMacApp && swift build            # build SwiftUI app
cd gui && ./build_app.sh                 # build full .app bundle
```

## Project Structure

- **Rust core:** `src/` — engines, util, cli, cleanup, core, config, intelligence
- **Digital Twin:** `src/twin/` — hardware, software_genome, fs_graph, process, memory, energy, app_agent, reasoning, model
- **Swift GUI:** `gui/XMacApp/` — SwiftUI macOS app
- **GNN:** `gnn/` — PyTorch model + CoreML export
- **13 engines:** clean, disk, depth, diag, envmap, graph, maintain, map, optimize, conflict, duplicate, startup, privacy
- **Intelligence suite:** `src/intelligence/` — advisor, daemon, zen, system_awareness
- **Config system:** `src/config/` — profiles, TOML store
- **Orchestration:** `.devin/` — Devin session config, skills, MAOS integration

## Digital Twin Architecture

```
Physical Mac
      |
      v
Telemetry + Sensors
      |
      v
Digital Twin Graph (src/twin/)
  ├── HardwareProfile      (hardware.rs)
  ├── SoftwareGenome       (software_genome.rs)
  ├── FilesystemGraph      (fs_graph.rs)
  ├── ProcessIntelligence   (process.rs)
  ├── MemoryIntelligence    (memory.rs)
  ├── EnergyTwin            (energy.rs)
  ├── AppIntelligenceGraph  (app_agent.rs)
  └── ReasoningEngine       (reasoning.rs)
      |
      v
Reasoning Engine (observe → understand → predict → simulate → recommend → execute safely)
```

## MAOS Context Retrieval

This project integrates with MAOS (Mac AI OS) MCP server for context retrieval.
See `.devin/config.json` and `.devin/skills/digital-twin/SKILL.md` for details.

**Always start a work session by:**
1. `maos_get_context` — get full context packet
2. `maos_search_memory` — search for subsystem-specific context
3. `maos_list_tasks` — see pending integration tasks

**See:** `docs/INTEGRATION_PLAN.md` and `docs/OPERATIONS_MANIFEST.md` for the full 630-operation mapping.

## Key Architecture Decisions

- Engines implement an async `Engine` trait (see `src/core/engine.rs`)
- Findings stream via `mpsc::channel` for real-time GUI updates
- Cleanup is always trash-first with undo support
- Config profiles tune engine thresholds via `with_config()`
- The GNN runs on-device via CoreML — no network calls
- macOS-specific code is `#[cfg(target_os = "macos")]`, Linux has equivalents
- The Digital Twin aggregates all dimensions into a single queryable model
- The Reasoning Engine uses the twin for causal analysis and simulation

## Git Safety

- **Never push to main** — push URL is disabled (`DISABLED`)
- All work on `digital-twin/*` branches
- Current branch: `digital-twin/integration`

## Commit Conventions

Use Conventional Commits format:
```
type(scope): description
```
Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `perf`, `ci`
Scopes: `clean`, `disk`, `maintain`, `map`, `depth`, `conflict`, `envmap`, `graph`, `optimize`, `config`, `intelligence`, `gui`, `gnn`, `cli`, `twin`, `duplicate`, `startup`, `privacy`

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contributor guide.
