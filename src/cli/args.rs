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
    /// Check filesystem integrity: permissions, symlinks, dylib dependencies.
    Depth(DepthArgs),
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
            Commands::Depth(_) => crate::core::types::EngineId::Depth,
            Commands::All(_) => crate::core::types::EngineId::All,
            Commands::Install(_) => crate::core::types::EngineId::All,
        }
    }
}

/// Arguments for the `scan` command — the recommended default.
#[derive(Args, Debug, Clone)]
pub struct ScanArgs {
    /// Skip specific engines. Available: clean, conflict, map, depth, diag.
    #[arg(long, value_enum)]
    pub skip: Vec<ScanEngineIdArg>,

    /// Include the depth engine (filesystem integrity). Off by default
    /// because it can be noisy on large Homebrew installations.
    #[arg(long)]
    pub include_depth: bool,

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
    Depth,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Json,
    JsonPretty,
    Report,
}

