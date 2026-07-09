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
| `scan` | **Recommended.** Runs clean + conflict + map + envmap + package-manager diagnostics. Depth is opt-in with `--include-depth`. |
| `all` | Run all engines including depth (use `--skip` to exclude specific engines) |
| `clean` | Detect caches, Xcode artifacts, orphan files, and duplicates |
| `conflict` | Detect PATH conflicts, environment variable conflicts, and port usage |
| `map` | Map Python/Node.js environments and container runtimes |
| `envmap` | Map the system environment: OS, system/language packages, and installed applications. Privacy-first (redacts usernames, paths, tokens, emails by default). Read-only. |
| `depth` | Check filesystem integrity: permissions, symlinks, dylib dependencies |
| `maintain` | Run system maintenance: flush DNS, reindex Spotlight, rebuild LaunchServices, run periodic scripts, purge RAM, clear Quick Look cache |
| `disk` | Show disk usage breakdown — top directories and files by size |
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
--skip <ENGINE>         Skip an engine: clean, conflict, map, envmap, depth, diag
--include-depth         Include the depth engine (off by default)
--envmap <BOOL>         Run the envmap engine (environment mapping) [default: true]
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

Detects reclaimable disk space — a comprehensive CLI equivalent of CleanMyMac / OnyX / Cleaner One Pro:
- Cache files (aged + size-filtered)
- Xcode DerivedData, Archives, iOS DeviceSupport
- Orphaned Application Support directories (matched by app bundle name and bundle ID)
- Package-manager caches (npm, pip, cargo, Homebrew, go, gradle, maven)
- Temp files (`/tmp`, `/var/tmp`, `.DS_Store`, editor swap files)
- Build artifacts (`node_modules`, `target`, `__pycache__`, `dist`, `build`, `.pyc`, `.o`, etc.)
- Rotated log files (`*.gz`, `*.bz2`, `*.0`, etc. in `/var/log` and `~/Library/Logs`)
- Browser caches (Safari, Chrome, Firefox, Edge, Brave, Arc)
- Mail attachments and downloads
- Old iOS device backups (`~/Library/Application Support/MobileSync/Backup/`)
- Removable language files (`.lproj` in `/Applications`, preserves English + Base)
- Trash bins on all mounted volumes
- Large files (>= 100 MB by default, configurable with `--min-large-size`)
- Document version stores (`.DocumentRevisions-V100`)
- Duplicate files via BLAKE3 hashing (opt-in with `--dedup`)

Build-artifact and temp-file sweeps are scoped to the home directory and skip
editor extensions (`.cursor`, `.windsurf`, `.vscode`), toolchain binaries
(`.cargo/bin`, `.rustup`), and system paths (`/System`, `/usr`) to avoid
breaking installed software.

```bash
xmac clean --min-age 30d --min-size 1M --dedup ~/Downloads

# Disable specific clean categories
xmac clean --build-artifacts false --temp false

# Only scan browser caches and trash
xmac clean --xcode false --temp false --build-artifacts false --pkg-caches false --languages false --large-files false --mail false --ios-backups false

# Find large files >= 500MB
xmac clean --min-large-size 500M --xcode false --temp false --build-artifacts false --pkg-caches false --languages false --mail false --ios-backups false --trash false
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

### Envmap

Maps the system environment — a privacy-first port of the MIF Environment
Mapper. Read-only and safe (included in `scan` by default). It discovers:

- **OS / system metadata** — platform, kernel version, hostname, architecture.
- **System packages** — Homebrew formulae + casks on macOS; `dpkg`/`rpm`/`pacman`
  on Linux (first source that yields output wins).
- **Language packages** — Python (`pip freeze` + `pipx list`), Node.js
  (`npm list -g`), Ruby (`gem list --local`).
- **Installed applications** — `.app` bundles under `/Applications` and
  `~/Applications`, with bundle name + version read from each bundle's
  `Contents/Info.plist` (via the `plist` crate; no CoreFoundation linkage).

Every string that flows into a finding (title, description, metadata) is run
through a redactor when `--redact` is on (the default). The redactor ports
MIF's `SENSITIVE_PATTERNS` and scrubs:

- macOS / Linux user home paths (`/Users/<name>`, `/home/<name>`)
- SSH / GnuPG key paths
- Passwords, tokens, API keys, secrets (`name=value` / `name: value` forms)
- Email addresses
- IPv4 addresses
- UUIDs
- AWS access key IDs (`AKIA…`)
- Passwords embedded in connection strings (`://user:pw@host`)
- The system hostname (when `--redact-hostnames true` is passed)

```bash
# Full environment map (all sources, redaction on)
xmac envmap

# Only enumerate installed applications, write a report
xmac --format report envmap --system false --system-packages false --language-packages false

# Disable privacy redaction (show raw paths/usernames)
xmac envmap --redact false

# Also scrub the hostname, and scan an extra Applications dir
xmac envmap --redact-hostnames true --app-dirs /Volumes/External/Applications
```

Options:
```
--system <BOOL>              Collect OS / system metadata [default: true]
--system-packages <BOOL>     Discover system packages (Homebrew/dpkg/rpm/pacman) [default: true]
--language-packages <BOOL>   Discover language packages (pip, pipx, npm, gems) [default: true]
--apps <BOOL>                Enumerate .app bundles from /Applications + ~/Applications [default: true]
--app-dirs <DIR>             Additional application directories to scan
--redact <BOOL>              Privacy-first redaction [default: true]
--redact-hostnames <BOOL>    Also redact the system hostname [default: false]
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

### Maintain

Runs macOS system maintenance tasks — the CLI equivalent of OnyX's maintenance
module. Tasks that are safe and don't require `sudo` run automatically; tasks
that require `sudo` (repair permissions, dyld rebuild) are emitted as findings
with the command in the remediation hint for review via `--fix-script`.

Tasks:
- **Flush DNS cache** — `dscacheutil -flushcache` + `killall -HUP mDNSResponder`
- **Reindex Spotlight** — `mdutil -E /`
- **Rebuild LaunchServices** — `lsregister -kill -r` (fixes "Open With" menu)
- **Run periodic scripts** — `periodic daily/weekly/monthly`
- **Purge inactive RAM** — `purge`
- **Clear Quick Look cache** — `qlmanage -r cache`
- **Repair disk permissions** (opt-in) — `sudo diskutil repairPermissions /`
- **Rebuild dyld shared cache** (opt-in) — `sudo update_dyld_shared_cache`

```bash
# Run all safe maintenance tasks
xmac maintain

# Only flush DNS and purge RAM
xmac maintain --spotlight false --launchservices false --periodic false --quicklook false

# Include sudo-requiring tasks (emitted as reviewable findings)
xmac maintain --repair-permissions true --dyld true

# Generate a fix script with the maintenance commands
xmac --fix-script ./maintenance.sh maintain
```

### Disk

Shows a disk usage breakdown — the CLI equivalent of CleanMyMac's Space Lens
or Cleaner One Pro's Disk Map. Lists the top directories and files by size
under the given path (defaults to home).

```bash
# Top 20 entries in home dir >= 100MB
xmac --format report disk

# Top 50 entries in a specific directory >= 1GB
xmac --format report disk --top 50 --min-size 1G /Applications

# Analyze a specific project directory
xmac disk ~/Projects
```

## Build

```bash
cargo build --release
```

## Testing

```bash
cargo test
```

## Contributing

Found a bug? Have an idea for a new engine or feature? Contributions are welcome.

### Submitting fixes

1. Fork the repo and create a branch: `git checkout -b fix/my-fix`
2. Make your changes — keep diffs minimal and follow existing code style
3. Verify: `cargo build --release && cargo test` (zero warnings, all tests pass)
4. Commit with a clear message describing what changed and why
5. Open a pull request against `main`

### Adding a new engine or add-on

X-MaC is built around a simple engine trait (`src/core/engine.rs`). To add a new scanner:

1. Create a new module under `src/engines/<your_engine>/`
2. Implement the `Engine` trait (`id`, `name`, `description`, `validate`, `scan`)
3. Emit findings via `ctx.emit(finding).await`
4. Register the engine in `src/engines/mod.rs`
5. Wire it into the CLI in `src/cli/args.rs` and `src/main.rs`
6. Add tests in `tests/integration_tests.rs`

### Reporting issues

Open an issue on [GitHub](https://github.com/davidnichols-ops/X-MaC/issues) with:
- What you expected
- What actually happened
- The command you ran and its output (use `--format report` for the structured summary)
- Your macOS version and architecture (`sw_vers` and `uname -m`)

## License

MIT
