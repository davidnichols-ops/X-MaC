use clap::{Parser, Subcommand, Args, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(
    name = "x-mac",
    version,
    about = "macOS system sanitizer & discovery tool",
    long_about = "X-MaC scans your macOS system to detect bloat, conflicts,\n\
                  runtime environments, and filesystem integrity issues.\n\
                  All operations are read-only - no system state is modified.",
    arg_required_else_help = true
)]
pub struct Cli {
    #[command(flatten)]
    pub global: GlobalArgs,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Args, Debug, Clone)]
pub struct GlobalArgs {
    #[arg(short, long, value_enum, default_value = "json", global = true)]
    pub format: OutputFormat,

    #[arg(short, long, global = true)]
    pub output: Option<PathBuf>,

    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    pub verbose: u8,

    #[arg(short, long, global = true)]
    pub quiet: bool,

    #[arg(long, default_value = "4", global = true)]
    pub concurrency: usize,

    #[arg(long, value_name = "GLOB", global = true)]
    pub exclude: Vec<String>,

    #[arg(long, global = true)]
    pub include_hidden: bool,

    #[arg(long, global = true)]
    pub follow_symlinks: bool,

    #[arg(long, global = true)]
    pub cache_dir: Option<PathBuf>,

    /// After the scan completes, write a reviewable remediation shell script
    /// to this path. The script is safe-by-default: destructive commands are
    /// commented out and require manual review. Requires `--format report`
    /// (or any format that buffers findings).
    #[arg(long, value_name = "PATH", global = true)]
    pub fix_script: Option<PathBuf>,
}

#[derive(Subcommand, Debug, Clone)]
pub enum Commands {
    /// Run a safe, comprehensive scan: caches, conflicts, environment mapping,
    /// filesystem integrity, and package-manager diagnostics. This is the
    /// recommended default command for everyday use.
    Scan(ScanArgs),
    /// Run everything (all engines). Equivalent to the old `all` command.
    All(AllArgs),
    /// Detect caches, Xcode artifacts, orphan files, and duplicates.
    Clean(CleanArgs),
    /// Detect PATH conflicts, environment variable conflicts, and port usage.
    Conflict(ConflictArgs),
    /// Map Python/Node.js environments and container runtimes.
    Map(MapArgs),
    /// Map the system environment: OS, system/language packages, and
    /// installed applications. Privacy-first (redacts usernames, paths,
    /// tokens, emails by default). Read-only.
    Envmap(EnvmapArgs),
    /// Check filesystem integrity: permissions, symlinks, dylib dependencies.
    Depth(DepthArgs),
    /// Run system maintenance tasks: flush DNS, reindex Spotlight, rebuild
    /// LaunchServices, run periodic scripts, repair permissions, purge RAM.
    Maintain(MaintainArgs),
    /// Show disk usage breakdown — top folders and files by size.
    Disk(DiskArgs),
    /// Install xmac to a directory on your PATH so it runs from anywhere.
    Install(InstallArgs),
}

impl Commands {
    pub fn engine_id(&self) -> crate::core::types::EngineId {
        match self {
            Commands::Scan(_) => crate::core::types::EngineId::All,
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
    #[arg(long, default_value = "30d")]
    pub min_age: String,

    #[arg(long, default_value = "1M")]
    pub min_size: String,

    #[arg(long)]
    pub dedup: bool,

    #[arg(long, default_value = "true")]
    pub xcode: bool,

    /// Detect package-manager caches (npm, pip, cargo, Homebrew, go, gradle, maven).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub pkg_caches: bool,

    /// Detect temp files (/tmp, /var/tmp, .DS_Store, editor swap files).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub temp: bool,

    /// Detect build artifacts (node_modules, target, __pycache__, dist, .pyc, .o, etc.).
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

    /// Detect large files (>= 100 MB by default, use --min-large-size to change).
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub large_files: bool,

    /// Minimum size for large file detection (default: 100M).
    #[arg(long, default_value = "100M")]
    pub min_large_size: String,

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

