# X-MaC

macOS system sanitizer & discovery tool. Scans your system to detect bloat, conflicts, runtime environments, and filesystem integrity issues. **All operations are read-only — no system state is modified.**

## Quick Start

```bash
cargo run --release -- --format report all
```

## Commands

| Command | Description |
|---------|-------------|
| `clean` | Detect caches, Xcode artifacts, orphan files, and duplicates |
| `conflict` | Detect PATH conflicts, environment variable conflicts, and port usage |
| `map` | Map Python/Node.js environments and container runtimes |
| `depth` | Check filesystem integrity: permissions, symlinks, dylib dependencies |
| `all` | Run all engines (use `--skip` to exclude specific engines) |

## Output Formats

| Format | Flag | Description |
|--------|------|-------------|
| JSON | `--format json` | One finding per line (NDJSON) |
| JSON Pretty | `--format json-pretty` | All findings as a pretty-printed JSON array |
| Report | `--format report` | Structured summary with severity/engine/category breakdowns, reclaimable space totals, and system metadata |

### Report Example

```json
{
  "scan_id": "01923a4b5c6d7e8f",
  "timestamp": "1718923200",
  "macos_version": "14.5.0",
  "apple_silicon": true,
  "engines": [...],
  "findings_by_severity": {
    "info": 12,
    "low": 5,
    "medium": 3,
    "high": 1,
    "critical": 0
  },
  "findings_by_engine": {
    "clean": 8,
    "conflict": 4,
    "map": 7,
    "depth": 2
  },
  "findings_by_category": {
    "cache": 5,
    "xcode_artifact": 3,
    "port_conflict": 2,
    ...
  },
  "total_reclaimable_bytes": 5368709120,
  "total_findings": 21,
  "total_items_scanned": 1543,
  "total_duration_secs": 2.84
}
```

## Global Options

```
-f, --format <FORMAT>        Output format: json, json-pretty, report [default: json]
-o, --output <PATH>          Write output to file instead of stdout
-v, --verbose                Increase verbosity (-v info, -vv debug, -vvv trace)
-q, --quiet                  Suppress progress output
    --concurrency <N>        Number of concurrent workers [default: 4]
    --exclude <GLOB>         Exclude paths matching glob pattern
    --include-hidden         Include hidden files/directories
    --follow-symlinks        Follow symbolic links during traversal
    --cache-dir <PATH>       Cache directory for scan results
```

## Engine Details

### Clean

Detects reclaimable disk space:
- Cache files (aged + size-filtered)
- Xcode DerivedData, Archives, iOS DeviceSupport
- Orphaned Application Support directories (app uninstalled but data remains)
- Duplicate files via BLAKE3 hashing (opt-in with `--dedup`)

```bash
cargo run -- clean --min-age 30d --min-size 1M --dedup ~/Downloads
```

### Conflict

Detects environment conflicts:
- Duplicate binaries across PATH directories
- Environment variables set to different values across shell configs
- Ports in use by running processes

```bash
cargo run -- conflict --path --env --ports --port-list 3000,5000,8080
```

### Map

Maps runtime environments:
- Python: venv, conda, poetry, pipenv, uv, pyenv
- Node.js: npm, yarn, pnpm, nvm versions, global installs
- Containers: Docker, Colima, Lima, OrbStack, Podman

```bash
cargo run -- map --python --nodejs --containers ~/Projects
```

### Depth

Filesystem integrity checks:
- World-writable files, SUID/SGID binaries
- Broken symlinks
- Missing dylib dependencies (via `otool -L`)

```bash
cargo run -- depth --permissions --symlinks --dylibs /usr/local/bin
```

## Build

```bash
cargo build --release
```

Requires Rust stable with targets `aarch64-apple-darwin` and `x86_64-apple-darwin`.

## Testing

```bash
cargo test
```

## License

MIT
