use clap::{Parser, Subcommand, Args, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "xmac",
    bin_name = "xmac",
    version,
    about = "macOS cleaner, optimizer, and system scanner — the CLI equivalent of CleanMyMac / OnyX",
    long_about = LONG_ABOUT,
    arg_required_else_help = true,
    after_help = AFTER_HELP,
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

static LONG_ABOUT: &str = "\
xmac is a macOS system sanitizer and discovery tool.

It scans your system to detect bloat, reclaimable disk space, conflicts,
runtime environments, and filesystem integrity issues — then generates
reviewable remediation scripts. All scan operations are read-only.

  xmac quick         # one-shot: clean scan + maintenance + disk breakdown
  xmac clean         # find reclaimable space (caches, builds, browser, trash…)
  xmac maintain      # flush DNS, reindex Spotlight, purge RAM, run periodic…
  xmac disk          # what's taking up space? (top dirs & files by size)
  xmac scan          # full system scan (clean + conflict + map + envmap)
  xmac doctor        # alias for `scan` (like `brew doctor`)

All operations are read-only. Use --fix-script to generate a remediation
shell script you can review and run.";

static AFTER_HELP: &str = "\
EXAMPLES:
  xmac quick                          # quick health check + cleanup scan
  xmac clean                          # find all reclaimable disk space
  xmac clean --dedup ~/Downloads      # also find duplicate files
  xmac maintain                       # run safe maintenance tasks
  xmac disk                           # show what's using your disk space
  xmac disk ~/Projects --top 30       # top 30 entries in a specific dir
  xmac --fix-script ./fixes.sh clean  # generate a remediation script
  xmac scan --include-depth           # full scan + filesystem integrity

  # JSON output for scripting:
  xmac --format json clean | jq .

  # More info: xmac help <command>";

#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    /// Output format: report (human-readable summary), json (one finding per
    /// line), or json-pretty (indented array).
    #[arg(short, long, value_enum, default_value = "report", global = true)]
    pub format: OutputFormat,

    /// Write output to a file instead of stdout.
    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,

    /// Increase verbosity (-v info, -vv debug, -vvv trace).
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    /// Suppress progress output.
    #[arg(short, long, global = true)]
    pub quiet: bool,

    /// Number of concurrent workers.
    #[arg(long, default_value = "4", global = true)]
    pub concurrency: usize,

    /// Exclude paths matching glob pattern (repeatable).
    #[arg(long, value_name = "GLOB", global = true)]
    pub exclude: Vec<String>,

    /// Include hidden files/directories in scans.
    #[arg(long, global = true)]
    pub include_hidden: bool,

    /// Follow symbolic links during traversal.
    #[arg(long, global = true)]
    pub follow_symlinks: bool,

    /// Cache directory for scan results.
    #[arg(long, global = true)]
    pub cache_dir: Option<PathBuf>,

    /// After the scan completes, write a reviewable remediation shell script
    /// to this path. Destructive commands are commented out — review and
    /// uncomment before running.
    #[arg(long, value_name = "PATH", global = true)]
    pub fix_script: Option<PathBuf>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// One-shot cleanup: scans for reclaimable space, runs safe maintenance
    /// tasks, and shows a disk usage breakdown. The fastest way to check
    /// your system health.
    Quick(QuickArgs),

    /// Full system scan: caches, conflicts, environment mapping, and
    /// package-manager diagnostics. The recommended command for thorough
    /// system checks. (Alias: `doctor`)
    Scan(ScanArgs),

    /// Alias for `scan` — familiar to Homebrew users.
    Doctor(ScanArgs),

    /// Run all engines including depth (filesystem integrity). Use `--skip`
    /// to exclude specific engines.
    All(AllArgs),

    /// Find reclaimable disk space: caches, browser data, build artifacts,
    /// trash, old iOS backups, language files, large files, and more.
    /// All read-only — use --fix-script to generate cleanup commands.
    Clean(CleanArgs),

    /// Detect PATH conflicts, environment variable conflicts, and port usage.
    Conflict(ConflictArgs),

    /// Map Python/Node.js environments and container runtimes.
    Map(MapArgs),

    /// Map the system environment: OS, packages, and installed apps.
    /// Privacy-first (redacts usernames, paths, tokens by default).
    Envmap(EnvmapArgs),

    /// Check filesystem integrity: permissions, symlinks, dylib dependencies.
    Depth(DepthArgs),

    /// Run macOS maintenance: flush DNS, reindex Spotlight, rebuild
    /// LaunchServices, run periodic scripts, purge RAM, clear Quick Look.
    /// Safe tasks run automatically; sudo tasks are emitted as findings.
    Maintain(MaintainArgs),

    /// Show disk usage breakdown — top directories and files by size.
    /// Like CleanMyMac's Space Lens or Cleaner One Pro's Disk Map.
    Disk(DiskArgs),

    /// Install xmac to a directory on your PATH so it runs from anywhere.
    Install(InstallArgs),
}

impl Commands {
    pub fn engine_id(&self) -> crate::core::types::EngineId {
        match self {
            Commands::Quick(_) => crate::core::types::EngineId::All,
            Commands::Scan(_) => crate::core::types::EngineId::All,
            Commands::Doctor(_) => crate::core::types::EngineId::All,
            Commands::Clean(_) => crate::core::types::EngineId::Clean,
            Commands::Conflict(_) => crate::core::types::EngineId::Conflict,
            Commands::Map(_) => crate::core::types::EngineId::Map,
            Commands::Envmap(_) => crate::core::types::EngineId::Envmap,
            Commands::Depth(_) => crate::core::types::EngineId::Depth,
            Commands::All(_) => crate::core::types::EngineId::All,
            Commands::Maintain(_) => crate::core::types::EngineId::All,
            Commands::Disk(_) => crate::core::types::EngineId::All,
            Commands::Install(_) => crate::core::types::EngineId::All,
        }
    }
}

/// Arguments for the `quick` command — one-shot cleanup + maintenance + disk.
#[derive(Args, Debug, Clone)]
pub struct QuickArgs {
    /// Also find duplicate files (BLAKE3 hash). Off by default since it's
    /// slower.
    #[arg(long)]
    pub dedup: bool,

    /// Skip maintenance tasks (only scan for reclaimable space + show disk).
    #[arg(long)]
    pub no_maintain: bool,

    /// Skip disk usage breakdown (only scan + maintain).
    #[arg(long)]
    pub no_disk: bool,

    /// Directory to analyze for disk usage (defaults to home).
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

/// Arguments for the `scan` command — the recommended default.
#[derive(Args, Debug, Clone)]
pub struct ScanArgs {
    /// Skip specific engines. Available: clean, conflict, map, envmap, depth, diag.
    #[arg(long, value_enum)]
    pub skip: Vec<ScanEngineIdArg>,

    /// Include the depth engine (filesystem integrity). Off by default
    /// because it can be noisy on large Homebrew installations.
    #[arg(long)]
    pub include_depth: bool,

    /// Include the envmap engine (environment mapping). On by default.
    #[arg(long, default_value = "true")]
    pub envmap: bool,

    /// Include package-manager diagnostics (brew doctor, etc.). On by default.
    #[arg(long, default_value = "true")]
    pub diagnostics: bool,
}

/// Engine IDs selectable from the `scan` command's `--skip` flag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ScanEngineIdArg {
    Clean,
    Conflict,
    Map,
    Envmap,
    Depth,
    Diag,
}

/// Arguments for the `install` command.
#[derive(Args, Debug, Clone)]
pub struct InstallArgs {
    /// Directory to install into. Must be on your PATH. Defaults to
    /// /opt/homebrew/bin on Apple Silicon, /usr/local/bin on Intel.
    #[arg(value_name = "DIR")]
    pub dir: Option<PathBuf>,

    /// Force overwrite an existing symlink.
    #[arg(long)]
    pub force: bool,
}

#[derive(Args, Debug, Clone)]
pub struct CleanArgs {
    /// Only flag files older than this (e.g. 7d, 30d, 90d).
    #[arg(long, default_value = "30d")]
    pub min_age: String,

    /// Minimum file size to flag (e.g. 1M, 100M, 1G).
    #[arg(long, default_value = "1M")]
    pub min_size: String,

    /// Also find duplicate files via BLAKE3 hashing (slower).
    #[arg(long)]
    pub dedup: bool,

    /// Detect Xcode artifacts (DerivedData, Archives, iOS DeviceSupport).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub xcode: bool,

    /// Detect package-manager caches (npm, pip, cargo, Homebrew, go, etc.).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub pkg_caches: bool,

    /// Detect temp files (.DS_Store, editor swap files, /tmp contents).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub temp: bool,

    /// Detect build artifacts (node_modules, target, __pycache__, dist, .pyc, .o).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub build_artifacts: bool,

    /// Detect browser caches (Safari, Chrome, Firefox, Edge, Brave, Arc).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub browser: bool,

    /// Detect mail attachments and downloads.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub mail: bool,

    /// Detect old iOS device backups.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub ios_backups: bool,

    /// Detect removable language files (.lproj) in applications.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub languages: bool,

    /// Detect trash bins on all volumes.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub trash: bool,

    /// Detect large files (>= --min-large-size, default 100M).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub large_files: bool,

    /// Minimum size for large file detection (e.g. 100M, 500M, 1G).
    #[arg(long, default_value = "100M")]
    pub min_large_size: String,

    /// Directory to scan (defaults to home directory).
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct ConflictArgs {
    #[arg(long, default_value = "true")]
    pub path: bool,

    #[arg(long, default_value = "true")]
    pub env: bool,

    #[arg(long, default_value = "true")]
    pub ports: bool,

    #[arg(long, value_delimiter = ',', default_value = "3000,5000,8000,8080,9000")]
    pub port_list: Vec<u16>,
}

#[derive(Args, Debug, Clone)]
pub struct MapArgs {
    #[arg(long, default_value = "true")]
    pub python: bool,

    #[arg(long, default_value = "true")]
    pub nodejs: bool,

    #[arg(long, default_value = "true")]
    pub containers: bool,

    #[arg(long, default_value = "true")]
    pub disk_usage: bool,

    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

/// Arguments for the `envmap` command — environment mapping.
///
/// Mirrors the MIF Environment Mapper: discovers OS metadata, system packages
/// (Homebrew formulae + casks), language packages (pip, pipx, npm, gems), and
/// installed applications from `/Applications` and `~/Applications`. All
/// output is privacy-filtered when `--redact` is on (the default).
#[derive(Args, Debug, Clone)]
pub struct EnvmapArgs {
    /// Collect OS / system metadata (platform, kernel, hostname, arch).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub system: bool,

    /// Discover system-level packages (Homebrew formulae + casks on macOS;
    /// dpkg/rpm/pacman on Linux).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub system_packages: bool,

    /// Discover language-runtime packages (Python pip + pipx, npm global,
    /// Ruby gems).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub language_packages: bool,

    /// Enumerate installed applications from `/Applications` and
    /// `~/Applications` (bundle name + version via Info.plist).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub apps: bool,

    /// Additional application directories to scan for `.app` bundles
    /// (in addition to the defaults).
    #[arg(long, value_name = "DIR")]
    pub app_dirs: Vec<PathBuf>,

    /// Privacy-first redaction. When true (the default), usernames, home
    /// directory paths, emails, tokens/keys, IPs, UUIDs, and AWS keys are
    /// replaced with `[REDACTED_*]` placeholders in all output. Pass
    /// `--redact false` to disable.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub redact: bool,

    /// Also redact the system hostname from output. Off by default; pass
    /// `--redact-hostnames true` to enable.
    #[arg(long, default_value = "false", action = clap::ArgAction::Set)]
    pub redact_hostnames: bool,
}

impl Default for EnvmapArgs {
    fn default() -> Self {
        Self {
            system: true,
            system_packages: true,
            language_packages: true,
            apps: true,
            app_dirs: Vec::new(),
            redact: true,
            redact_hostnames: false,
        }
    }
}

#[derive(Args, Debug, Clone)]
pub struct DepthArgs {
    #[arg(long, default_value = "true")]
    pub permissions: bool,

    #[arg(long, default_value = "true")]
    pub symlinks: bool,

    #[arg(long, default_value = "true")]
    pub dylibs: bool,

    #[arg(value_name = "PATH", default_values = ["/usr/local/bin", "/opt/homebrew"])]
    pub paths: Vec<PathBuf>,
}

#[derive(Args, Debug, Clone)]
pub struct AllArgs {
    #[arg(long, value_enum)]
    pub skip: Vec<EngineIdArg>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum EngineIdArg {
    Clean,
    Conflict,
    Map,
    Envmap,
    Depth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Json,
    JsonPretty,
    Report,
}

/// Arguments for the `maintain` command — system maintenance tasks.
#[derive(Args, Debug, Clone)]
pub struct MaintainArgs {
    /// Flush DNS cache (dscacheutil + mDNSResponder).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub dns: bool,

    /// Reindex Spotlight search.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub spotlight: bool,

    /// Rebuild LaunchServices database (fixes "Open With" menu).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub launchservices: bool,

    /// Run periodic maintenance scripts (daily, weekly, monthly).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub periodic: bool,

    /// Repair disk permissions (runs `sudo diskutil repairPermissions /`).
    #[arg(long, default_value = "false", action = clap::ArgAction::Set)]
    pub repair_permissions: bool,

    /// Purge inactive RAM (runs `purge` command).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub purge_ram: bool,

    /// Rebuild dyld shared cache.
    #[arg(long, default_value = "false", action = clap::ArgAction::Set)]
    pub dyld: bool,

    /// Clear Quick Look thumbnail cache.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub quicklook: bool,
}

/// Arguments for the `disk` command — disk usage visualization.
#[derive(Args, Debug, Clone)]
pub struct DiskArgs {
    /// Number of top entries to show per directory level.
    #[arg(long, default_value = "20")]
    pub top: usize,

    /// Minimum size to display (smaller entries are aggregated).
    #[arg(long, default_value = "100M")]
    pub min_size: String,

    /// Directory to analyze (defaults to home).
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

