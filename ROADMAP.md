# Roadmap

This document tracks planned work. Dates are approximate and driven by community interest.

## Current Focus (v2.1.x)

- [ ] Homebrew tap for `xmac` CLI (`brew install xmac`)
- [ ] TestFlight public beta + notarized DMG distribution
- [ ] GNN model improvements with community scan data (opt-in, anonymized)
- [ ] Localization — the UI is English-only right now
- [ ] Dark/light mode theming (currently dark-only)

## Near Term (v2.2)

- [ ] Duplicate file finder with visual diff
- [ ] Space Lens — drill-down treemap (like Disk Diag / DaisyDisk)
- [ ] CSV export for scan results
- [ ] "Largest files" view inside disk donut segments
- [ ] Scheduled scan UI in the GUI (backend exists via daemon)
- [ ] Notification Center integration for scan completion

## Medium Term (v2.3–v2.5)

- [ ] App Store submission
- [ ] Plugin system for custom scan engines
- [ ] Docker container cleanup (images, volumes, build caches)
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

- [ ] Brew formula (`brew install xmac`)
- [ ] Nix flake
- [ ] Arch Linux AUR package
- [ ] NixOS module
- [ ] Snap package

See [GOOD_FIRST_ISSUES.md](GOOD_FIRST_ISSUES.md) for beginner-friendly entry points.
