# X-MaC Agent & Contributor Guide

## Build & Test Commands

```bash
cargo build                              # compile
cargo test                               # run tests (410)
cargo clippy -- -D warnings              # lint (treat warnings as errors)
cargo fmt --check                        # format check
cargo check --target x86_64-unknown-linux-gnu  # Linux cross-compile check
cd gui/XMacApp && swift build            # build SwiftUI app
cd gui && ./build_app.sh                 # build full .app bundle
```

## Project Structure

- **Rust core:** `src/` — engines, util, cli, cleanup, core, config, intelligence
- **Swift GUI:** `gui/XMacApp/` — SwiftUI macOS app
- **GNN:** `gnn/` — PyTorch model + CoreML export
- **10 engines:** clean, disk, depth, diag, envmap, graph, maintain, map, optimize, conflict
- **Intelligence suite:** `src/intelligence/` — advisor, daemon, zen, system_awareness
- **Config system:** `src/config/` — profiles, TOML store

## Key Architecture Decisions

- Engines implement an async `Engine` trait (see `src/core/engine.rs`)
- Findings stream via `mpsc::channel` for real-time GUI updates
- Cleanup is always trash-first with undo support
- Config profiles tune engine thresholds via `with_config()`
- The GNN runs on-device via CoreML — no network calls
- macOS-specific code is `#[cfg(target_os = "macos")]`, Linux has equivalents

## Commit Conventions

Use Conventional Commits format:
```
type(scope): description
```
Types: `feat`, `fix`, `docs`, `test`, `refactor`, `chore`, `perf`, `ci`
Scopes: `clean`, `disk`, `maintain`, `map`, `depth`, `conflict`, `envmap`, `graph`, `optimize`, `config`, `intelligence`, `gui`, `gnn`, `cli`

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full contributor guide.
