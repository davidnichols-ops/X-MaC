# Good First Issues

A guide for new contributors looking for approachable tasks.

## How to Use This File

Pick an issue that interests you, comment on it (or just open a PR), and ask for help if you get stuck. We're friendly.

## Beginner-Friendly Tasks

### Documentation
- [ ] Add screenshots to README.md (run the app, take screenshots, add them)
- [ ] Write a "How X-MaC compares to CleanMyMac" comparison page in `docs/`
- [ ] Document the `Finding` struct fields in `src/core/types.rs` with doc comments
- [ ] Add inline doc comments to all public functions in `src/config/store.rs`

### Tests
- [x] Add tests for the `disk` engine (9 tests added)
- [x] Add tests for the `map` engine (18 tests added — Python/Node env detection)
- [x] Add tests for the `conflict` engine (11 tests added — shell config parsing)
- [x] Add daemon lifecycle integration tests (7 tests added)
- [ ] Add integration test for `xmac zen --no-clean --no-maintain` (dry-run preview)

### CLI
- [x] Add `--version` flag output with build metadata (git hash, build date)
- [ ] Add `xmac doctor` command that checks system health and prints recommendations
- [x] Add shell completion generation (`xmac completions --shell zsh`)
- [x] Add CSV export (`--format csv`) for scan results

### GUI
- [ ] Add a "Largest files" view when clicking a disk donut segment
- [ ] Add CSV export button to scan results view
- [ ] Add light/dark mode toggle (currently dark-only)
- [ ] Add localization for one additional language (Spanish, French, German, Japanese)

### GNN
- [ ] Add more training data categories
- [ ] Improve model accuracy on edge cases (very small files, system files)
- [ ] Add a confusion matrix visualization to the training notebook

## Medium-Difficulty Tasks

- [ ] Implement duplicate file finder with BLAKE3 hashing (CLI flag exists, logic not implemented)
- [x] Add Homebrew formula for `xmac` CLI (`packaging/homebrew/xmac.rb`)
- [x] Add Docker image cache detection to the clean engine (`--docker` flag)
- [ ] Implement scheduled scan UI in the GUI (backend exists via daemon)
- [ ] Add Notification Center integration for scan completion on macOS
- [ ] Implement Space Lens (drill-down treemap) in the GUI
- [ ] Add `xmac report --format html` for exportable HTML reports

## Advanced Tasks

- [ ] Implement a plugin system for custom scan engines (trait + dynamic loading)
- [ ] Add cross-platform GUI (Linux via Tauri or GTK)
- [ ] Implement real-time monitoring dashboard (always-on daemon with live UI)
- [ ] Add Kubernetes resource cleanup engine
- [ ] Implement team / multi-user mode for shared workstations
- [ ] Add centralized fleet management for IT admins

## Getting Help

- Open a [GitHub Discussion](https://github.com/davidnichols-ops/X-MaC/discussions)
- Join the conversation on issues
- Read [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) to understand the system
- Read [DEVELOPMENT.md](DEVELOPMENT.md) for setup instructions
