# Roadmap

This document tracks planned work. Items are driven by community interest and contributor availability.

## Done

- [x] CSV export (`--format csv`)
- [x] Shell completions (`xmac completions --shell zsh|bash|fish|elvish|powershell`)
- [x] Docker cache detection (`--docker` flag)
- [x] Homebrew formula (`packaging/homebrew/xmac.rb`)
- [x] Daemon signal handling fix (SIGTERM/SIGINT now work across all cycles)
- [x] 410 tests (disk, map, conflict, daemon lifecycle coverage)
- [x] Linux cross-compile support
- [x] Config profiles wired into engines
- [x] Background daemon with auto-purge and automation rules
- [x] AI Advisor with adaptive learning
- [x] Zen Mode comprehensive optimization

## Current Focus (v2.1.x)

- [ ] Publish Homebrew tap (`brew tap davidnichols-ops/xmac`)
- [ ] Notarize the macOS app for distribution
- [ ] GNN model improvement — file scorer is at 99.76% val accuracy, memory model needs final accuracy verification
- [ ] Localization — the UI is English-only right now
- [ ] Dark/light mode theming (currently dark-only)

## Near Term (v2.2)

- [ ] Duplicate file finder with BLAKE3 hashing
- [ ] Space Lens — drill-down treemap (like Disk Diag / DaisyDisk)
- [ ] "Largest files" view inside disk donut segments
- [ ] Scheduled scan UI in the GUI (backend exists via daemon)
- [ ] Notification Center integration for scan completion
- [ ] `xmac doctor` command (system health check with recommendations)
- [ ] HTML report export (`xmac report --format html`)

## Medium Term (v2.3–v2.5)

- [ ] App Store submission
- [ ] Plugin system for custom scan engines
- [ ] Kubernetes resource cleanup
- [ ] Cloud storage cleanup (iCloud, Dropbox, Google Drive stale caches)
- [ ] Network-based scan (find orphaned files on NAS / external drives)

## Long Term (v3.0)

- [ ] Cross-platform GUI (Linux via Tauri or GTK)
- [ ] Team / multi-user mode for shared workstations
- [ ] Centralized fleet management for IT admins
- [ ] Real-time monitoring dashboard (always-on daemon with live UI)

## Community-Requested

Features requested by the community that we'd love help with:

- [ ] Nix flake
- [ ] Arch Linux AUR package
- [ ] NixOS module
- [ ] Snap package

See [GOOD_FIRST_ISSUES.md](GOOD_FIRST_ISSUES.md) for beginner-friendly entry points.
