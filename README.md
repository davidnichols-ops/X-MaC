# xmac

macOS cleaner, optimizer, and system scanner — the CLI equivalent of CleanMyMac, OnyX, and Cleaner One Pro.

All scan operations are **read-only**. Use `--fix-script` to generate a remediation shell script you can review and run.

## Quick Start

```bash
# Install
cargo build --release
./target/release/x-mac install

# One-shot health check (clean scan + maintenance + disk breakdown)
xmac quick

# Find reclaimable disk space
xmac clean

# Run system maintenance (flush DNS, reindex Spotlight, purge RAM…)
xmac maintain

# What's taking up space?
xmac disk

# Full system scan (clean + conflict + map + envmap + diagnostics)
xmac scan

# Generate a cleanup script you can review and run
xmac --fix-script ./fixes.sh clean
```

## Commands

| Command | What it does |
|---------|-------------|
| `quick` | **One-shot:** clean scan + maintenance + disk breakdown. Fastest way to check system health. |
| `clean` | Find reclaimable space: caches, browser data, build artifacts, trash, iOS backups, language files, large files, duplicates. |
| `maintain` | Run macOS maintenance: flush DNS, reindex Spotlight, rebuild LaunchServices, periodic scripts, purge RAM, clear Quick Look. |
| `disk` | Show disk usage breakdown — top directories and files by size. |
| `scan` | Full system scan: clean + conflict + map + envmap + diagnostics. (Alias: `doctor`) |
| `doctor` | Alias for `scan` (familiar to Homebrew users). |
| `conflict` | Detect PATH conflicts, environment variable conflicts, and port usage. |
| `map` | Map Python/Node.js environments and container runtimes. |
| `envmap` | Map system environment: OS, packages, installed apps. Privacy-first. |
| `depth` | Check filesystem integrity: permissions, symlinks, dylib dependencies. |
| `all` | Run everything including depth (use `--skip` to exclude). |
| `install` | Install `xmac` to your PATH. |

## Output Formats

Default is `report` (human-readable summary). Use `json` for scripting.

```bash
xmac clean                          # report (default, human-readable)
xmac --format json clean | jq .     # JSON for scripting
xmac -o results.json clean          # write to file
```

| Format | Flag | Description |
|--------|------|-------------|
| Report | `--format report` (default) | Structured summary with breakdowns and reclaimable space totals |
| JSON | `--format json` | One finding per line (NDJSON) |
| JSON Pretty | `--format json-pretty` | Indented JSON array |

## Common Workflows

### Free up disk space

```bash
# See what can be cleaned
xmac clean

# Also find duplicate files
xmac clean --dedup

# Generate a cleanup script
xmac --fix-script ./cleanup.sh clean
less ./cleanup.sh       # review it
bash ./cleanup.sh --yes  # run non-destructive fixes

# Find large files (>= 500MB)
xmac clean --min-large-size 500M

# Only scan specific directories
xmac clean ~/Downloads ~/Desktop
```

### Run system maintenance

```bash
# Run all safe maintenance tasks
xmac maintain

# Only flush DNS and purge RAM
xmac maintain --spotlight false --launchservices false --periodic false --quicklook false

# Include sudo-requiring tasks (emitted as reviewable findings)
xmac maintain --repair-permissions true --dyld true
```

### See what's using disk space

```bash
# Top 20 entries in home dir >= 100MB
xmac disk

# Top 50 entries in a specific directory >= 1GB
xmac disk --top 50 --min-size 1G /Applications

# Analyze a project directory
xmac disk ~/Projects
```

### Full system health check

```bash
# Full scan (clean + conflict + map + envmap + diagnostics)
xmac scan

# Include filesystem integrity checks
xmac scan --include-depth

# Skip specific engines
xmac scan --skip conflict

# Generate fix script from full scan
xmac --fix-script ./fixes.sh scan
```

### Quick health check (Smart Scan)

```bash
# Clean scan + maintenance + disk breakdown in one shot
xmac quick

# Also find duplicates
xmac quick --dedup

# Skip maintenance, just scan + disk
xmac quick --no-maintain

# Generate fix script from quick scan
xmac --fix-script ./fixes.sh quick
```

## Remediation Scripts

Use `--fix-script <PATH>` to generate a safe, reviewable shell script:

```bash
xmac --fix-script ./fixes.sh clean

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

## Clean Categories

The `clean` command detects all major categories of reclaimable disk space:

| Category | What it finds | CLI flag |
|----------|--------------|----------|
| Caches | User & system cache files (aged + size-filtered) | always on |
| Xcode artifacts | DerivedData, Archives, iOS DeviceSupport | `--xcode` |
| Package-manager caches | npm, pip, cargo, Homebrew, go, gradle, maven | `--pkg-caches` |
| Temp files | .DS_Store, editor swap files, /tmp contents | `--temp` |
| Build artifacts | node_modules, target, __pycache__, dist, .pyc, .o | `--build-artifacts` |
| Browser caches | Safari, Chrome, Firefox, Edge, Brave, Arc | `--browser` |
| Mail attachments | Mail downloads & attachment directories | `--mail` |
| iOS backups | Old device backups (per-device with sizes) | `--ios-backups` |
| Language files | Removable .lproj dirs (preserves English + Base) | `--languages` |
| Trash bins | Trash on all mounted volumes | `--trash` |
| Large files | Files >= 100MB (configurable) | `--large-files` |
| Rotated logs | .gz, .bz2, .0–.9 in /var/log and ~/Library/Logs | always on |
| Document versions | .DocumentRevisions-V100 stores | always on |
| Duplicates | Duplicate files via BLAKE3 hashing | `--dedup` |

All categories default to `true`. Disable any with `--<flag> false`:

```bash
# Only scan browser caches and trash
xmac clean --xcode false --temp false --build-artifacts false --pkg-caches false \
           --languages false --large-files false --mail false --ios-backups false
```

Build-artifact and temp-file sweeps are scoped to the home directory and skip
editor extensions (`.cursor`, `.windsurf`, `.vscode`), toolchain binaries
(`.cargo/bin`, `.rustup`), and system paths (`/System`, `/usr`).

## Maintenance Tasks

The `maintain` command runs macOS system maintenance — the CLI equivalent of
OnyX's maintenance module:

| Task | What it does | Default |
|------|-------------|---------|
| Flush DNS | `dscacheutil -flushcache` + `killall -HUP mDNSResponder` | on |
| Reindex Spotlight | `mdutil -E /` | on |
| Rebuild LaunchServices | `lsregister -kill -r` (fixes "Open With" menu) | on |
| Periodic scripts | `periodic daily/weekly/monthly` | on |
| Purge RAM | `purge` (frees inactive memory) | on |
| Clear Quick Look | `qlmanage -r cache` | on |
| Repair permissions | `sudo diskutil repairPermissions /` (HFS+ only) | off |
| Rebuild dyld cache | `sudo update_dyld_shared_cache` | off |

Safe tasks run automatically. Sudo-requiring tasks are emitted as findings
with the command in the remediation hint for review via `--fix-script`.

## Global Options

```
-f, --format <FORMAT>        Output format: report (default), json, json-pretty
-o, --output <PATH>          Write output to file instead of stdout
-v, --verbose                Increase verbosity (-v info, -vv debug, -vvv trace)
-q, --quiet                  Suppress progress output
    --fix-script <PATH>      Generate a reviewable remediation shell script
    --concurrency <N>        Number of concurrent workers [default: 4]
    --exclude <GLOB>         Exclude paths matching glob pattern
    --include-hidden         Include hidden files/directories
    --follow-symlinks        Follow symbolic links during traversal
```

## Engine Details

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

### Envmap

Maps the system environment — a privacy-first port of the MIF Environment
Mapper. Read-only and safe. Discovers OS metadata, system packages (Homebrew),
language packages (pip, npm, gems), and installed applications.

All output is redacted by default (usernames, paths, tokens, emails, IPs,
UUIDs, AWS keys). Pass `--redact false` to disable.

```bash
xmac envmap                              # full environment map (redacted)
xmac envmap --redact false               # show raw paths/usernames
xmac envmap --apps --system false        # only enumerate installed apps
```

### Depth

Filesystem integrity checks:
- World-writable files, SUID/SGID binaries
- Broken symlinks (relative symlinks resolved correctly)
- Missing dylib dependencies (via `otool -L`)

```bash
xmac depth --permissions --symlinks --dylibs /usr/local/bin
```

### Diagnostics

Runs built-in diagnostics for detected package managers (Homebrew, MacPorts,
Nix, Cargo, pip, npm). Included in `scan` by default.

## Build & Test

```bash
cargo build --release
cargo test
```

## Contributing

Found a bug? Have an idea? Contributions welcome.

1. Fork and branch: `git checkout -b fix/my-fix`
2. Verify: `cargo build --release && cargo test` (zero warnings, all pass)
3. Commit with a clear message
4. Open a PR against `main`

### Adding a new engine

1. Create `src/engines/<name>/` and implement the `Engine` trait
2. Register in `src/engines/mod.rs`
3. Wire into `src/cli/args.rs` and `src/main.rs`
4. Add tests in `tests/integration_tests.rs`

## License

MIT
