# Design Principles

The architectural decisions behind X-MaC, and the constraints that guide future development.

## 1. Safety First

**The most important principle.** X-MaC touches user files. A bug that deletes the wrong file is catastrophic.

- **Trash-first, always.** Files are moved to Trash, never `rm -rf`'d. Permanent deletion requires a second explicit confirmation.
- **Dry-run by default.** `xmac clean` scans but does not delete. Deletion requires `xmac purge` with confirmation.
- **Undo support.** Every cleanup transaction records undo metadata. `xmac undo` can reverse the last cleanup.
- **Verification.** Post-cleanup verification confirms files were actually moved to Trash.
- **No root escalation without consent.** Sudo is only requested for specific maintenance tasks and is always optional.

## 2. On-Device Only

**No network calls. Ever.**

- The GNN model runs entirely on-device via CoreML. No data is sent to any server.
- No telemetry, no analytics, no crash reporting to a remote server.
- No auto-update mechanism that phones home.
- The user's filesystem contents never leave their machine.

This is a hard constraint. Any PR that adds a network call will be rejected.

## 3. Engine Trait Uniformity

Every scanner implements the same `Engine` trait. This means:

- New engines are easy to add (implement 3 methods).
- The CLI and GUI don't need to know about specific engines — they just run all registered engines.
- Output is uniform — every engine produces `Finding` objects with the same structure.
- Engines are composable — `xmac quick` runs clean + maintain + disk in sequence.

## 4. Streaming Architecture

Findings are streamed via async channels (`mpsc`), not collected into a giant `Vec` and returned at the end. This means:

- The GUI shows results in real-time as they're discovered.
- Memory usage stays low even when scanning millions of files.
- The user can cancel a scan mid-stream.

## 5. Platform Graceful Degradation

X-MaC targets macOS as the primary platform, with Linux as a supported secondary.

- macOS-specific features (Spotlight, LaunchServices, purge, Quick Look) are behind `#[cfg(target_os = "macos")]`.
- Linux has equivalent features where possible (systemd journal vacuum, drop_caches, etc.).
- The CLI works on both platforms. The GUI is macOS-only (SwiftUI).
- Never `#[cfg(windows)]` — Windows is not a target.

## 6. Config-Driven, Not Flag-Driven

User preferences live in `config.toml`, not in long CLI flag strings. This means:

- `xmac clean` uses config defaults. Users don't need to remember 15 flags.
- Profiles (Gaming, Development, Conservative) bundle related settings.
- The GUI reads the same config as the CLI — no separate settings.
- CLI flags override config when explicitly passed, but config is the source of truth.

## 7. No External Runtime Dependencies

The `.app` bundle is self-contained:

- The Rust binary is bundled inside `Contents/MacOS/xmac`.
- The CoreML model is bundled inside `Contents/Resources/`.
- No Homebrew, no Python, no external libraries needed at runtime.
- A user can install the app by dragging it to `/Applications/` — that's it.

## 8. Testable by Design

- Engines are trait objects — easy to mock in tests.
- Config is a plain struct — easy to construct in tests.
- The advisor and zen modules are pure functions of `SystemSnapshot` — no I/O needed for unit tests.
- 410 tests cover the core logic.

## 9. Transparent Output

- The CLI supports text, JSON, and NDJSON output.
- Every finding has a clear `severity`, `category`, `title`, `description`, and `remediation_hint`.
- The user can always see exactly what X-MaC found and what it plans to do before anything happens.
- No "magic" cleanup — every action is logged and explainable.

## 10. Open Source, Community-Driven

- MIT licensed — no commercial restrictions.
- No CLA — contributors keep their copyright.
- All development happens in the open on GitHub.
- Community contributions are welcomed and reviewed promptly.
- The roadmap is public and community-influenced.
