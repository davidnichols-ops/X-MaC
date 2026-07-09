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

    // ---- Browser cache paths --------------------------------------------

    /// Browser cache directories for Safari, Chrome, Firefox, Edge, Brave,
    /// and Arc. Each entry is (browser_name, cache_path).
    pub fn browser_cache_paths(&self) -> Vec<(&'static str, PathBuf)> {
        let home = MacosUtils::home_dir();
        vec![
            ("Safari", home.join("Library/Caches/com.apple.Safari")),
            ("Safari Containers", home.join("Library/Containers/com.apple.Safari/Data/Library/Caches/com.apple.Safari")),
            ("Chrome", home.join("Library/Caches/Google/Chrome")),
            ("Firefox", home.join("Library/Caches/Firefox")),
            ("Edge", home.join("Library/Caches/Microsoft Edge")),
            ("Brave", home.join("Library/Caches/BraveSoftware")),
            ("Arc", home.join("Library/Caches/com.thebrowser.Browser")),
        ]
    }

    // ---- Mail attachment paths -------------------------------------------

    /// Mail attachment and download directories.
    pub fn mail_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        vec![
            home.join("Library/Containers/com.apple.mail/Data/Library/Mail Downloads"),
            home.join("Library/Mail/V2/MailData"),
            home.join("Library/Mail/V3/MailData"),
            home.join("Library/Mail/V4/MailData"),
            home.join("Library/Mail/V5/MailData"),
            home.join("Library/Mail/V6/MailData"),
            home.join("Library/Mail/V7/MailData"),
            home.join("Library/Mail/V8/MailData"),
        ]
    }

    // ---- iOS backup paths ------------------------------------------------

    /// iOS device backup directory (Finder/iTunes backups).
    pub fn ios_backup_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        vec![home.join("Library/Application Support/MobileSync/Backup")]
    }

    // ---- Trash paths -----------------------------------------------------

    /// Trash directories on all mounted volumes.
    pub fn trash_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![MacosUtils::home_dir().join(".Trash")];
        // External/volume trashes
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let trash = entry.path().join(".Trashes");
                if trash.exists() {
                    paths.push(trash);
                }
            }
        }
        paths
    }

    // ---- Document version paths ------------------------------------------

    /// Document revision stores on volumes.
    pub fn document_version_paths(&self) -> Vec<PathBuf> {
        let mut paths = vec![PathBuf::from("/.DocumentRevisions-V100")];
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let rev = entry.path().join(".DocumentRevisions-V100");
                if rev.exists() {
                    paths.push(rev);
                }
            }
        }
        paths
    }

    // ---- Application directories (for language file scanning) -----------

    /// Directories containing .app bundles to scan for language files.
    pub fn app_dirs(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        vec![
            PathBuf::from("/Applications"),
            home.join("Applications"),
        ]
    }

    /// Language codes to preserve when scanning for removable .lproj dirs.
    /// Base.lproj is always preserved (it's not language-specific).
    pub fn keep_language_codes(&self) -> &[&'static str] {
        &["en", "English", "Base", "base"]
    }
}
