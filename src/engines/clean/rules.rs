use std::path::PathBuf;

use crate::core::types::Category;
use crate::util::macos::MacosUtils;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CleanRule {
    pub name: &'static str,
    pub pattern: &'static str,
    pub description: &'static str,
    pub category: Category,
}

#[allow(dead_code)]
pub struct CleanRules {
    pub rules: Vec<CleanRule>,
}

impl Default for CleanRules {
    fn default() -> Self {
        Self {
            rules: vec![
                CleanRule {
                    name: "xcode_derived_data",
                    pattern: "Library/Developer/Xcode/DerivedData/*",
                    description: "Xcode build artifacts",
                    category: Category::XcodeArtifact,
                },
                CleanRule {
                    name: "xcode_archives",
                    pattern: "Library/Developer/Xcode/Archives/*",
                    description: "Xcode archives",
                    category: Category::XcodeArtifact,
                },
                CleanRule {
                    name: "xcode_device_support",
                    pattern: "Library/Developer/Xcode/iOS DeviceSupport/*",
                    description: "iOS device support files",
                    category: Category::XcodeArtifact,
                },
                CleanRule {
                    name: "home_caches",
                    pattern: "Library/Caches/*",
                    description: "User cache files",
                    category: Category::Cache,
                },
                CleanRule {
                    name: "system_caches",
                    pattern: "/Library/Caches/*",
                    description: "System cache files",
                    category: Category::Cache,
                },
                CleanRule {
                    name: "home_logs",
                    pattern: "Library/Logs/*",
                    description: "User log files",
                    category: Category::Log,
                },
                CleanRule {
                    name: "system_logs",
                    pattern: "/var/log/*",
                    description: "System log files",
                    category: Category::Log,
                },
            ],
        }
    }
}

/// Directory names that are build artifacts (safe to delete; regenerated on
/// next build/install).
pub const BUILD_ARTIFACT_DIRS: &[&str] = &[
    "node_modules",
    "target",
    "__pycache__",
    ".next",
    "dist",
    "build",
    ".gradle",
    ".maven",
    ".nuxt",
    ".turbo",
    ".svelte-kit",
    "Pods",
];

/// File extensions / names that are build artifacts.
pub const BUILD_ARTIFACT_FILE_PATTERNS: &[&str] = &[
    ".pyc",
    ".pyo",
    ".o",
    ".obj",
    ".a",
    ".so",
    ".class",
    ".swp",
    ".swo",
];

/// Rotated / archived log file extensions.
pub const ROTATED_LOG_EXTENSIONS: &[&str] = &[
    ".gz",
    ".bz2",
    ".xz",
    ".zst",
];

/// Editor / system temp file patterns.
pub const TEMP_FILE_NAMES: &[&str] = &[".DS_Store"];

/// Editor swap file patterns.
pub const SWAP_FILE_PATTERNS: &[&str] = &[".swp", ".swo"];

impl CleanRules {
    pub fn cache_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        vec![home.join("Library/Caches"), PathBuf::from("/Library/Caches")]
    }

    pub fn xcode_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        vec![
            home.join("Library/Developer/Xcode/DerivedData"),
            home.join("Library/Developer/Xcode/Archives"),
            home.join("Library/Developer/Xcode/iOS DeviceSupport"),
        ]
    }

    /// Package-manager cache directories. These are regenerated on demand and
    /// safe to clear.
    pub fn pkg_cache_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        vec![
            home.join(".npm/_cacache"),
            home.join(".npm/_npx"),
            home.join(".npm/_logs"),
            home.join(".cache"),
            home.join(".cargo/registry/cache"),
            home.join(".cargo/registry/src"),
            home.join(".cargo/registry/index"),
            home.join("Library/Caches/pip"),
            home.join("Library/Caches/Homebrew"),
            home.join("Library/Caches/go-build"),
            home.join(".gradle/caches"),
            home.join(".m2/repository"),
        ]
    }

    /// System temp directories (resolved through firmlinks).
    pub fn temp_paths(&self) -> Vec<PathBuf> {
        vec![
            MacosUtils::resolve_firmlink(&PathBuf::from("/tmp")),
            MacosUtils::resolve_firmlink(&PathBuf::from("/var/tmp")),
        ]
    }

    /// User log directory.
    pub fn user_log_paths(&self) -> Vec<PathBuf> {
        vec![MacosUtils::home_dir().join("Library/Logs")]
    }

    /// System log directory (for rotated log detection only — active logs
    /// are never flagged).
    pub fn system_log_paths(&self) -> Vec<PathBuf> {
        vec![PathBuf::from("/var/log")]
    }

    /// Home directory — used as the search root for build artifacts and
    /// .DS_Store / swap file sweeps.
    pub fn home_search_root(&self) -> PathBuf {
        MacosUtils::home_dir()
    }

    /// Directories to skip when sweeping for build artifacts / temp files,
    /// to avoid breaking installed tooling.
    pub fn sweep_skip_dirs(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        vec![
            home.join(".cargo/bin"),
            home.join(".cursor"),
            home.join(".windsurf"),
            home.join(".vscode"),
            home.join(".rustup"),
            home.join(".local"),
            PathBuf::from("/System"),
            PathBuf::from("/usr"),
        ]
    }
}
