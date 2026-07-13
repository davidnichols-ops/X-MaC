# Changelog

All notable changes to X-MaC are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [2.1.0] - 2026-07-12

### Added
- **Config system** (`xmac config`) — TOML-based configuration at `~/.config/xmac/config.toml` with 7 optimization profiles (Balanced, Gaming, Development, Video Editing, Conservative, Aggressive, Custom)
- **Background daemon** (`xmac daemon`) — long-running mode with PID-file single-instance enforcement, graceful shutdown, automation rule evaluation, auto-purge on memory pressure, auto-clean on disk pressure
- **AI Advisor** (`xmac advisor`) — natural-language system health recommendations with severity, explanation, CLI command, estimated impact, confidence, and auto-safe flags. Adaptive learning adjusts confidence based on user feedback history
- **Zen Mode** (`xmac zen`) — one-click comprehensive optimization with preview/execute modes, before/after health score, memory delta, disk reclaimable summary
- **Multi-dimensional system awareness** — CPU + memory + thermal + battery + disk telemetry with weighted composite health score (0-100)
- **History & analytics** (`xmac history`) — scan history with --summary, --export, --clear
- **Safe automation rules** — user-defined conditions trigger actions with cooldown enforcement
- **Exportable reports** — JSON and text output for all commands
- **SwiftUI GUI** — ZenView, AdvisorView, MenuBarExtra with quick access to core functions
- **Linux platform support** — all engines cross-compile and run on Linux
- 246 new tests (327 total, up from 81)

### Changed
- Config profiles now actually affect engine behavior (min_age thresholds, category toggles, aggressive modes)
- Daemon automation actions now execute (ScanClean, RunMaintenance, KillProcess, PurgeMemory)
- GUI binary path checks inside .app bundle first for self-contained deployment
- Minimum macOS deployment target raised to 13.0 (Ventura)

### Fixed
- Resolved all unused import/variable compiler warnings
- Fixed `Option::max` usage on f64 values
- Fixed global CLI flag conflicts (--format, --verbose)
- Corrected APFS sparse file size calculation

## [2.0.0] - 2026-06-15

### Added
- GNN (Graph Neural Network) safety scoring for scan findings
- CoreML on-device inference — no network required
- SwiftUI GUI with dashboard, disk analyzer, smart scan, clean, maintain, app inventory
- RAM boost / memory optimizer engine
- Crash reporter and adaptive fixer in GUI
- Onboarding flow for first-launch

### Changed
- Complete overhaul of installed app detection
- RAM display improvements

## [1.0.0] - 2026-04-01

### Added
- Rust scan engine with 9 engines: clean, disk, depth, diag, envmap, graph, maintain, map, conflict
- Trash-first safe deletion with undo support
- CLI with clap argument parsing
- NDJSON streaming output for GUI integration
- Time Machine and backup volume exclusion
