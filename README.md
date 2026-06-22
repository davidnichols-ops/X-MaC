# X-MaC

macOS system sanitizer & discovery tool. Scans your system to detect bloat, conflicts, runtime environments, filesystem integrity issues, and runs built-in package-manager diagnostics. **All operations are read-only — no system state is modified.**

## Quick Start

```bash
# Build
cargo build --release

# Install so you can run `xmac` from anywhere
./target/release/x-mac install

# Run a safe, comprehensive scan (recommended)
xmac --format report scan

# Generate a reviewable remediation script
xmac --format report --fix-script ./fixes.sh scan
```

## Commands

| Command | Description |
|---------|-------------|
| `scan` | **Recommended.** Runs clean + conflict + map + package-manager diagnostics. Depth is opt-in with `--include-depth`. |
| `all` | Run all engines including depth (use `--skip` to exclude specific engines) |
| `clean` | Detect caches, Xcode artifacts, orphan files, and duplicates |
| `conflict` | Detect PATH conflicts, environment variable conflicts, and port usage |
| `map` | Map Python/Node.js environments and container runtimes |
| `depth` | Check filesystem integrity: permissions, symlinks, dylib dependencies |
| `install` | Install `xmac` to a directory on your PATH (default: `/opt/homebrew/bin` on Apple Silicon) |

## Output Formats

| Format | Flag | Description |
|--------|------|-------------|
| JSON | `--format json` | One finding per line (NDJSON) |
| JSON Pretty | `--format json-pretty` | All findings as a pretty-printed JSON array |
| Report | `--format report` | Structured summary with severity/engine/category breakdowns, reclaimable space totals, and system metadata |

## Global Options

```
-f, --format <FORMAT>        Output format: json, json-pretty, report [default: json]
-o, --output <PATH>          Write output to file instead of stdout
-v, --verbose                Increase verbosity (-v info, -vv debug, -vvv trace)
-q, --quiet                  Suppress progress output
    --fix-script <PATH>      After the scan, write a reviewable remediation shell script
    --concurrency <N>        Number of concurrent workers [default: 4]
    --exclude <GLOB>         Exclude paths matching glob pattern
    --include-hidden         Include hidden files/directories
    --follow-symlinks        Follow symbolic links during traversal
    --cache-dir <PATH>       Cache directory for scan results
```

## The `scan` Command

The recommended default. Runs the safe, reliable engines and package-manager diagnostics:

```bash
# Basic scan
xmac scan

# With report format
xmac --format report scan

# Include filesystem integrity checks (symlinks, dylibs, permissions)
xmac scan --include-depth

# Skip specific engines
xmac scan --skip conflict

# Generate a fix script after scanning
xmac --format report --fix-script ./fixes.sh scan
```

Options:
```
--skip <ENGINE>         Skip an engine: clean, conflict, map, depth, diag
--include-depth         Include the depth engine (off by default)
--diagnostics <BOOL>    Run package-manager diagnostics [default: true]
```

## The `install` Command

Installs `xmac` as a symlink to a directory on your PATH so it runs from anywhere:

```bash
# Default: /opt/homebrew/bin (Apple Silicon) or /usr/local/bin (Intel)
xmac install

# Specify a custom directory
xmac install ~/.local/bin

# Overwrite an existing installation
xmac install --force
```

## Remediation Scripts

Use `--fix-script <PATH>` to generate a safe, reviewable shell script after a scan:

```bash
xmac --fix-script ./fixes.sh scan

# Review the script
less ./fixes.sh

# Apply non-destructive fixes (e.g. chmod o-w on world-writable files)
bash ./fixes.sh --yes

# Destructive commands (rm, kill) are commented out — uncomment
# the ones you agree with after reviewing, then re-run
```

The script is safe by default:
- Every destructive command is commented out
- Non-destructive fixes are gated behind `--yes` confirmation
- False-positive-prone categories carry explicit review warnings
- All paths are shell-quoted to prevent injection

## Engine Details

### Clean

Detects reclaimable disk space:
- Cache files (aged + size-filtered)
- Xcode DerivedData, Archives, iOS DeviceSupport
- Orphaned Application Support directories (matched by app bundle name and bundle ID)
- Duplicate files via BLAKE3 hashing (opt-in with `--dedup`)

```bash
xmac clean --min-age 30d --min-size 1M --dedup ~/Downloads
```

### Conflict

Detects environment conflicts:
- Duplicate binaries across PATH directories
- Environment variables set to different values across shell configs
- Ports in use by running processes

```bash
xmac conflict --path --env --ports --port-list 3000,5000,8080
```

### Map

Maps runtime environments:
- Python: venv, conda, poetry, pipenv, uv, pyenv
- Node.js: npm, yarn, pnpm, nvm versions, global installs
- Containers: Docker, Colima, Lima, OrbStack, Podman

```bash
xmac map --python --nodejs --containers ~/Projects
```

### Depth

Filesystem integrity checks (opt-in for `scan`, included in `all`):
- World-writable files, SUID/SGID binaries
- Broken symlinks (relative symlinks resolved correctly against parent dir)
- Missing dylib dependencies (via `otool -L`, skips `@rpath`/`@loader_path`/`@executable_path`)

```bash
xmac depth --permissions --symlinks --dylibs /usr/local/bin
```

### Diagnostics

Runs built-in diagnostics for detected package managers:
- Homebrew: `brew doctor`, `brew missing`, `brew --version`
- MacPorts: `port version`
- Nix: `nix --version`
- Cargo: `cargo --version`
- pip: `pip3 --version`
- npm: `npm --version`, `npm doctor`

Safe mode (default) only runs read-only diagnostics. Use `xmac all` to include network-dependent checks like `brew outdated`.

## Build

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## License

MIT
