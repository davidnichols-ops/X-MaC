use clap::{Args, Parser, Subcommand, ValueEnum};
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

    /// Resource mode controls scan parallelism and CPU strain.
    /// eco = sequential (lowest CPU), balanced = limited parallelism,
    /// turbo = full parallelism (fastest, higher CPU).
    #[arg(long, default_value = "balanced", global = true)]
    pub resource_mode: String,

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

    /// Extract a file system graph for GNN training and inference. Emits
    /// nodes (files/dirs with features) and edges (parent-child, symlink,
    /// dependency) as JSON. The graph can be consumed by a Graph Neural
    /// Network for intelligent cleanup scoring.
    Graph(GraphArgs),

    /// Execute a reviewed cleanup plan: move selected categories to Trash
    /// with full safety validation, undo metadata, and verification. This is
    /// the only subcommand that modifies the filesystem.
    Purge(PurgeArgs),

    /// Install xmac to a directory on your PATH so it runs from anywhere.
    Install(InstallArgs),

    /// Boost RAM: show memory usage, purge inactive memory, and optionally
    /// kill memory-hungry processes. On macOS runs `purge`; on Linux drops
    /// kernel caches. Shows before/after comparison.
    RamBoost(RamBoostArgs),

    /// Optimize memory: collect telemetry, build a memory graph, and predict
    /// pressure. In observe mode (default), no actions are taken — the engine
    /// reports the current state and predicted pressure trajectory.
    Optimize(OptimizeArgs),

    /// View or edit configuration. Supports `init`, `show`, `profile`,
    /// `set`, and `path` subcommands.
    Config(ConfigArgs),

    /// Run the background daemon for smart scheduling, telemetry collection,
    /// and proactive optimization. Manages a PID file for single-instance
    /// enforcement and supports graceful shutdown via SIGTERM/SIGINT.
    Daemon(DaemonArgs),

    /// Zen Mode — one-click comprehensive optimization with preview. Runs
    /// a clean scan, memory optimization, and safe maintenance in one pass,
    /// then shows a summary of what was done and what was reclaimed.
    Zen(ZenArgs),

    /// AI Advisor — analyzes system state and produces natural-language
    /// recommendations. Explains what's happening and what to do in plain
    /// English, with severity-ranked actionable suggestions.
    Advisor(AdvisorArgs),

    /// View cleanup and scan history with trends and savings reports.
    History(HistoryArgs),

    /// Generate shell completion scripts. Source them in your shell config
    /// for tab-completion of xmac commands and flags.
    /// Example: `xmac completions --shell zsh > ~/.zsh/completions/_xmac`
    Completions(CompletionsArgs),

    /// Digital Twin — collect a complete system snapshot and query the
    /// reasoning engine. Use subcommands to ask questions, predict problems,
    /// simulate actions, or get recommendations.
    Twin(TwinArgs),

    /// Run the MCP (Model Context Protocol) server. Exposes the Digital Twin
    /// as a structured environment for AI agents (Claude, GPT, local models).
    /// The server reads JSON-RPC from stdin and writes to stdout.
    Mcp,

    /// Safety — classify file paths by risk level and list loaded safety
    /// rules. Every file targeted for cleanup traces to a named rule with
    /// a rating (safe/review/protected).
    Safety(SafetyArgs),

    /// Find duplicate files via BLAKE3 hashing and perceptual image
    /// fingerprinting. A standalone version of `clean --dedup` focused
    /// only on duplicate detection.
    Dedup(DedupArgs),
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
            Commands::Graph(_) => crate::core::types::EngineId::All,
            Commands::Purge(_) => crate::core::types::EngineId::Clean,
            Commands::Install(_) => crate::core::types::EngineId::All,
            Commands::RamBoost(_) => crate::core::types::EngineId::All,
            Commands::Optimize(_) => crate::core::types::EngineId::All,
            Commands::Config(_) => crate::core::types::EngineId::All,
            Commands::Daemon(_) => crate::core::types::EngineId::All,
            Commands::Zen(_) => crate::core::types::EngineId::All,
            Commands::Advisor(_) => crate::core::types::EngineId::All,
            Commands::History(_) => crate::core::types::EngineId::All,
            Commands::Completions(_) => crate::core::types::EngineId::All,
            Commands::Twin(_) => crate::core::types::EngineId::All,
            Commands::Mcp => crate::core::types::EngineId::All,
            Commands::Safety(_) => crate::core::types::EngineId::All,
            Commands::Dedup(_) => crate::core::types::EngineId::Duplicate,
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

    /// Detect Docker image and build caches. Safe to prune via `docker system prune`.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub docker: bool,

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

    /// Detect orphaned application support directories (left by uninstalled apps).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub orphans: bool,

    /// Scan ONLY the specified categories (skip all others).
    /// Accepts a comma-separated list of category names:
    /// cache, orphan_file, package_manager_cache, build_artifact, temp_file,
    /// xcode_artifact, browser_cache, log, trash_bin, large_file, mail_attachment,
    /// ios_backup, language_file, document_version, duplicate_file, docker.
    #[arg(long, value_delimiter = ',')]
    pub only: Vec<String>,

    /// Resource usage mode controls scan parallelism and CPU strain.
    /// eco = sequential (lowest CPU), balanced = limited parallelism, turbo = full parallelism.
    #[arg(long, default_value = "balanced")]
    pub resource_mode: String,

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

    #[arg(
        long,
        value_delimiter = ',',
        default_value = "3000,5000,8000,8080,9000"
    )]
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
    Csv,
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

/// Arguments for the `graph` command — file system graph extraction.
#[derive(Args, Debug, Clone)]
pub struct GraphArgs {
    /// Maximum directory depth to walk (safety limit).
    #[arg(long, default_value = "15")]
    pub max_depth: usize,

    /// Maximum number of nodes to extract (safety limit).
    #[arg(long, default_value = "50000")]
    pub max_nodes: usize,

    /// Directory to write graph JSON files to. If not specified, graph data
    /// is emitted as metadata in findings (use --format json to see it).
    #[arg(long, value_name = "DIR")]
    pub output_graph: Option<PathBuf>,

    /// Directory to extract graph from (defaults to home directory).
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,
}

/// Arguments for the `purge` command — transactional cleanup.
#[derive(Args, Debug, Clone)]
pub struct PurgeArgs {
    /// Show the cleanup plan without executing any destructive actions.
    #[arg(long)]
    pub dry_run: bool,

    /// Override the default policy and allow high-risk categories such as
    /// orphaned app support and large files. Each category still requires
    /// an explicit `--category` selection.
    #[arg(long)]
    pub force_review: bool,

    /// Only purge specific categories. Repeatable. Use with --dry-run first.
    #[arg(long, value_enum)]
    pub category: Vec<PurgeCategoryArg>,

    /// Minimum file age to include in the purge (e.g. 30d, 90d).
    #[arg(long, default_value = "30d")]
    pub min_age: String,

    /// Minimum file size to include in the purge (e.g. 1M, 100M).
    #[arg(long, default_value = "1M")]
    pub min_size: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum PurgeCategoryArg {
    Cache,
    TempFile,
    BuildArtifact,
    PackageManagerCache,
    BrowserCache,
    Log,
    TrashBin,
    XcodeArtifact,
    OrphanFile,
    LargeFile,
    DuplicateFile,
    MailAttachment,
    IosBackup,
    LanguageFile,
    DocumentVersion,
    UniversalBinary,
}

/// Arguments for the `ram-boost` command — RAM optimizer and memory cleaner.
#[derive(Args, Debug, Clone)]
pub struct RamBoostArgs {
    /// Show memory report only — don't purge or kill anything.
    #[arg(long, default_value = "false")]
    pub report_only: bool,

    /// Purge inactive/compressed memory (macOS: `purge`, Linux: drop_caches).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub purge: bool,

    /// Kill the top N memory-hungry processes (use with caution).
    #[arg(long, default_value = "0")]
    pub kill_top: usize,

    /// Kill processes by name (e.g. --kill-name "Slack,Spotify").
    /// The process is sent SIGTERM first, then SIGKILL after 5s if still alive.
    #[arg(long)]
    pub kill_name: Option<String>,

    /// Force kill (SIGKILL) instead of graceful SIGTERM.
    #[arg(long, default_value = "false")]
    pub force: bool,

    /// Minimum RSS in MB for a process to be considered for killing.
    #[arg(long, default_value = "500")]
    pub min_rss_mb: u64,

    /// Don't kill system processes (kernel_task, launchd, WindowServer, etc.).
    /// Enabled by default — disable with --allow-system-kill (dangerous).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub protect_system: bool,

    /// Use osascript to run purge with admin privileges (shows macOS password dialog).
    #[arg(long, default_value = "false")]
    pub privileged: bool,
}

/// Arguments for the `optimize` command — memory telemetry, graph building,
/// and pressure prediction. Phase 1 (observe) collects data and predicts;
/// future phases will add action recommendation and execution.
#[derive(Args, Debug, Clone)]
pub struct OptimizeArgs {
    /// Observation mode: collect telemetry and predict pressure without
    /// taking any actions. This is the default and safest mode.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub observe: bool,

    /// Maximum number of process nodes to include in the memory graph
    /// (sorted by RSS, descending). Larger graphs capture more detail
    /// but increase inference latency.
    #[arg(long, default_value = "50")]
    pub max_processes: usize,

    /// Exclude system processes (kernel_task, launchd, WindowServer, etc.)
    /// from the memory graph. Enabled by default for cleaner output.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub exclude_system: bool,

    /// Number of telemetry snapshots to keep in the ring buffer for
    /// trend analysis. More snapshots = better trend detection but
    /// more memory usage.
    #[arg(long, default_value = "288")]
    pub buffer_size: usize,

    /// Number of top memory-consuming processes to emit as findings.
    #[arg(long, default_value = "10")]
    pub top_n: usize,

    /// Write the memory graph as JSON to the specified file.
    #[arg(long)]
    pub output_graph: Option<std::path::PathBuf>,

    /// Continuous mode: collect snapshots every N seconds and emit
    /// findings for each. Set to 0 for a single snapshot (default).
    #[arg(long, default_value = "0")]
    pub interval_secs: u64,
}

// ═══════════════════════════════════════════════════════════════════════
//  Config command
// ═══════════════════════════════════════════════════════════════════════

/// Arguments for the `config` command.
#[derive(Args, Debug, Clone)]
pub struct ConfigArgs {
    #[command(subcommand)]
    pub action: ConfigAction,
}

#[derive(Subcommand, Debug, Clone)]
pub enum ConfigAction {
    /// Create a default config file at ~/.config/xmac/config.toml.
    Init,
    /// Print the current config as TOML.
    Show,
    /// Print the config file path.
    Path,
    /// List available optimization profiles.
    Profiles,
    /// Set the active optimization profile.
    SetProfile { name: String },
    /// Set a specific config key (dotted path, e.g. `clean.min_age_days`).
    Set { key: String, value: String },
    /// Get a specific config value by dotted path.
    Get { key: String },
}

// ═══════════════════════════════════════════════════════════════════════
//  Daemon command
// ═══════════════════════════════════════════════════════════════════════

/// Arguments for the `daemon` command.
#[derive(Args, Debug, Clone)]
pub struct DaemonArgs {
    /// Run once and exit (useful for testing or cron integration).
    #[arg(long)]
    pub once: bool,

    /// Override the check interval (seconds).
    #[arg(long)]
    pub interval: Option<u64>,

    /// Enable verbose daemon logging.
    #[arg(long)]
    pub daemon_verbose: bool,

    /// Check if the daemon is currently running.
    #[arg(long)]
    pub status: bool,

    /// Stop a running daemon (sends SIGTERM via PID file).
    #[arg(long)]
    pub stop: bool,
}

// ═══════════════════════════════════════════════════════════════════════
//  Zen Mode command
// ═══════════════════════════════════════════════════════════════════════

/// Arguments for the `zen` command — one-click comprehensive optimization.
#[derive(Args, Debug, Clone)]
pub struct ZenArgs {
    /// Preview what would be done without taking any actions.
    #[arg(long)]
    pub dry_run: bool,

    /// Skip the memory optimization step.
    #[arg(long)]
    pub no_memory: bool,

    /// Skip the maintenance step.
    #[arg(long)]
    pub no_maintain: bool,

    /// Skip the disk cleanup step.
    #[arg(long)]
    pub no_clean: bool,

    /// Use a specific optimization profile for this run.
    #[arg(long)]
    pub profile: Option<String>,

    /// Actually execute cleanup (without this, zen mode is preview-only).
    #[arg(long)]
    pub execute: bool,
}

// ═══════════════════════════════════════════════════════════════════════
//  Advisor command
// ═══════════════════════════════════════════════════════════════════════

/// Arguments for the `advisor` command — AI-powered recommendations.
#[derive(Args, Debug, Clone)]
pub struct AdvisorArgs {
    /// Only show recommendations at or above this severity (info, low, medium, high, critical).
    #[arg(long, default_value = "info")]
    pub min_severity: String,

    /// Maximum number of recommendations to show.
    #[arg(long, default_value = "20")]
    pub top: usize,

    /// Output format: text (default) or json.
    #[arg(long, default_value = "text")]
    pub advisor_format: String,

    /// Include system health score (0-100) in the output.
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub health_score: bool,
}

// ═══════════════════════════════════════════════════════════════════════
//  History command
// ═══════════════════════════════════════════════════════════════════════

/// Arguments for the `history` command.
#[derive(Args, Debug, Clone)]
pub struct HistoryArgs {
    /// Show the last N entries.
    #[arg(long, default_value = "20")]
    pub last: usize,

    /// Show a summary of total savings over time.
    #[arg(long)]
    pub summary: bool,

    /// Export history as JSON to a file.
    #[arg(long)]
    pub export: Option<std::path::PathBuf>,

    /// Clear all history.
    #[arg(long)]
    pub clear: bool,
}

// ═══════════════════════════════════════════════════════════════════════
//  Completions command
// ═══════════════════════════════════════════════════════════════════════

/// Arguments for the `completions` command.
#[derive(Args, Debug, Clone)]
pub struct CompletionsArgs {
    /// Which shell to generate completions for.
    #[arg(long)]
    pub shell: ShellArg,
}

/// Supported shells for completion generation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum ShellArg {
    Bash,
    Zsh,
    Fish,
    Elvish,
    PowerShell,
}

// ═══════════════════════════════════════════════════════════════════════
//  Twin command
// ═══════════════════════════════════════════════════════════════════════

/// Arguments for the `twin` command — Digital Twin snapshot and reasoning.
#[derive(Args, Debug, Clone)]
pub struct TwinArgs {
    /// Twin subcommand: collect, ask, predict, simulate, recommend, query,
    /// benchmark, monitor. Defaults to `collect` (full snapshot).
    #[arg(long, default_value = "collect")]
    pub action: TwinAction,

    /// Question to ask the reasoning engine (used with --action ask).
    #[arg(long, value_name = "QUESTION")]
    pub question: Option<String>,

    /// Action to simulate (used with --action simulate).
    #[arg(long, value_name = "ACTION_DESC")]
    pub simulate: Option<String>,

    /// Query dimension: health, memory, disk, battery, process, trust
    /// (used with --action query).
    #[arg(long, value_name = "DIMENSION")]
    pub query: Option<String>,

    /// Start time for `what-changed` query (ISO 8601 or relative like "7d", "24h").
    #[arg(long, value_name = "TIMESTAMP")]
    pub since: Option<String>,

    /// End time for `what-changed` query (defaults to now).
    #[arg(long, value_name = "TIMESTAMP")]
    pub until: Option<String>,

    /// Duration to run observers (used with --action observe).
    /// Format: "60s", "5m", "1h". Defaults to "60s".
    #[arg(long, value_name = "DURATION", default_value = "60s")]
    pub duration: String,
}

/// Sub-actions for the `twin` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum TwinAction {
    /// Collect a full Digital Twin snapshot (default).
    Collect,
    /// Ask the reasoning engine a question.
    Ask,
    /// Predict future problems.
    Predict,
    /// Simulate an action and assess risk.
    Simulate,
    /// Get optimization recommendations.
    Recommend,
    /// Query a specific dimension.
    Query,
    /// Generate anonymized benchmark data.
    Benchmark,
    /// Produce a continuous monitoring plan.
    Monitor,
    /// Initialize the persistent twin database (SQLite event store).
    InitDb,
    /// Query "what changed?" between two timestamps (--since, --until).
    WhatChanged,
    /// Run compaction on the event store (prune raw events older than 7 days).
    Compact,
    /// Run observers for a duration (--duration). Feeds events into the store.
    Observe,
}

// ═══════════════════════════════════════════════════════════════════════
//  Safety command
// ═══════════════════════════════════════════════════════════════════════

/// Arguments for the `safety` command.
#[derive(Args, Debug, Clone)]
pub struct SafetyArgs {
    /// Safety action: classify a path, list all rules, or preview cleanup.
    #[arg(long, default_value = "list")]
    pub action: SafetyAction,

    /// Path to classify (used with --action classify).
    #[arg(long, value_name = "PATH")]
    pub path: Option<String>,

    /// Cleanup profile to preview (used with --action preview).
    #[arg(long, value_name = "PROFILE", default_value = "all")]
    pub profile: String,
}

/// Actions for the `safety` command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum SafetyAction {
    /// Classify a single path (--path required).
    Classify,
    /// List all loaded safety rules.
    List,
    /// Preview cleanup for a profile (--profile).
    Preview,
}

// ═══════════════════════════════════════════════════════════════════════
//  Dedup command
// ═══════════════════════════════════════════════════════════════════════

/// Arguments for the `dedup` command — find duplicate files.
#[derive(Args, Debug, Clone)]
pub struct DedupArgs {
    /// Directories to scan for duplicates. Defaults to ~/Downloads,
    /// ~/Documents, ~/Desktop.
    #[arg(value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Minimum file size to consider (e.g. 1K, 1M, 100M). Default: 1K.
    #[arg(long, default_value = "1K")]
    pub min_size: String,

    /// Also detect similar (not just identical) images using perceptual
    /// hashing.
    #[arg(long)]
    pub similar: bool,
}
