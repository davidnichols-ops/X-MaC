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
                // Phase 5 ops 132-134: installer file detection
                CleanRule {
                    name: "dmg_files",
                    pattern: "**/*.dmg",
                    description: "Disk image files (installers)",
                    category: Category::DmgFile,
                },
                CleanRule {
                    name: "pkg_files",
                    pattern: "**/*.pkg",
                    description: "Package installer files",
                    category: Category::PkgFile,
                },
                CleanRule {
                    name: "old_installers",
                    pattern: "**/*.{dmg,pkg,mpkg}",
                    description: "Old installer files (DMG/PKG)",
                    category: Category::OldInstaller,
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
    ".pyc", ".pyo", ".o", ".obj", ".a", ".so", ".class", ".swp", ".swo",
];

/// Rotated / archived log file extensions.
pub const ROTATED_LOG_EXTENSIONS: &[&str] = &[".gz", ".bz2", ".xz", ".zst"];

/// Editor / system temp file patterns.
pub const TEMP_FILE_NAMES: &[&str] = &[".DS_Store"];

/// Editor swap file patterns.
pub const SWAP_FILE_PATTERNS: &[&str] = &[".swp", ".swo"];

/// Installer file extensions that can be cleaned up (op 132-134).
pub const INSTALLER_EXTENSIONS: &[&str] = &[".dmg", ".pkg", ".mpkg"];

/// Whether a file path looks like an installer (DMG / PKG / mpkg) — op 133/134.
#[allow(dead_code)]
pub fn is_installer_file(path: &std::path::Path) -> bool {
    path.extension()
        .map(|ext| {
            let ext = ext.to_string_lossy().to_lowercase();
            INSTALLER_EXTENSIONS.iter().any(|e| *e == format!(".{ext}"))
        })
        .unwrap_or(false)
}

/// Whether a file path is a DMG disk image — op 133.
#[allow(dead_code)]
pub fn is_dmg_file(path: &std::path::Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("dmg"))
        .unwrap_or(false)
}

/// Whether a file path is a PKG installer — op 134.
#[allow(dead_code)]
pub fn is_pkg_file(path: &std::path::Path) -> bool {
    path.extension()
        .map(|ext| ext.eq_ignore_ascii_case("pkg") || ext.eq_ignore_ascii_case("mpkg"))
        .unwrap_or(false)
}

impl CleanRules {
    pub fn cache_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        if cfg!(target_os = "macos") {
            vec![
                home.join("Library/Caches"),
                PathBuf::from("/Library/Caches"),
            ]
        } else {
            // Linux: XDG cache + system caches
            let xdg_cache = std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".cache"));
            vec![xdg_cache, PathBuf::from("/var/cache")]
        }
    }

    pub fn xcode_paths(&self) -> Vec<PathBuf> {
        if cfg!(target_os = "macos") {
            let home = MacosUtils::home_dir();
            vec![
                home.join("Library/Developer/Xcode/DerivedData"),
                home.join("Library/Developer/Xcode/Archives"),
                home.join("Library/Developer/Xcode/iOS DeviceSupport"),
            ]
        } else {
            vec![] // Xcode doesn't exist on Linux
        }
    }

    /// Package-manager cache directories. These are regenerated on demand and
    /// safe to clear.
    pub fn pkg_cache_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        let mut paths = vec![
            home.join(".npm/_cacache"),
            home.join(".npm/_npx"),
            home.join(".npm/_logs"),
            home.join(".cargo/registry/cache"),
            home.join(".cargo/registry/src"),
            home.join(".cargo/registry/index"),
            home.join(".gradle/caches"),
            home.join(".m2/repository"),
        ];
        if cfg!(target_os = "macos") {
            paths.push(home.join("Library/Caches/pip"));
            paths.push(home.join("Library/Caches/Homebrew"));
            paths.push(home.join("Library/Caches/go-build"));
            paths.push(home.join(".cache"));
            // Docker Desktop on macOS stores data inside the VM, but the
            // Docker CLI config and build cache are on the host.
            paths.push(home.join("Library/Containers/com.docker.docker/Data/vms/0/data"));
            paths.push(home.join(".docker/build"));
            paths.push(home.join(".docker/cache"));
        } else {
            // Linux: XDG cache home covers pip, go-build, etc.
            let xdg_cache = std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".cache"));
            paths.push(xdg_cache.clone());
            paths.push(xdg_cache.join("pip"));
            paths.push(xdg_cache.join("go-build"));
            paths.push(xdg_cache.join("Homebrew"));
            // Linux system package manager caches
            paths.push(PathBuf::from("/var/cache/apt/archives"));
            paths.push(PathBuf::from("/var/cache/dnf"));
            paths.push(PathBuf::from("/var/cache/pacman/pkg"));
            paths.push(PathBuf::from("/var/cache/zypp"));
            // Docker on Linux — build cache and image layers
            paths.push(home.join(".docker/build"));
            paths.push(home.join(".docker/cache"));
            // Root-owned Docker data (will only be scanned if running as root)
            paths.push(PathBuf::from("/var/lib/docker/overlay2"));
            paths.push(PathBuf::from("/var/lib/docker/buildkit"));
        }
        // Docker build cache (cross-platform, Docker uses this path on all OSes)
        paths.push(home.join(".local/share/docker/overlay2"));
        paths
    }

    /// Docker image and container cache paths. Docker Desktop and Docker Engine
    /// store images, containers, build caches, and volumes in well-known
    /// directories. These can grow to many GB and are safe to prune via
    /// `docker system prune`.
    pub fn docker_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        let mut paths = Vec::new();

        if cfg!(target_os = "macos") {
            // Docker Desktop on macOS stores everything inside the VM, but
            // the Docker CLI config and some caches are on the host.
            paths.push(home.join("Library/Containers/com.docker.docker/Data"));
            paths.push(home.join(".docker/build"));
            paths.push(home.join(".docker/cache"));
        } else {
            // Linux: Docker Engine stores data under /var/lib/docker
            paths.push(PathBuf::from("/var/lib/docker/overlay2"));
            paths.push(PathBuf::from("/var/lib/docker/buildkit"));
            paths.push(PathBuf::from("/var/lib/docker/volumes"));
            paths.push(PathBuf::from("/var/lib/docker/image"));
            // Rootless Docker
            paths.push(home.join(".local/share/docker/overlay2"));
            paths.push(home.join(".local/share/docker/buildkit"));
            paths.push(home.join(".local/share/docker/volumes"));
        }
        paths
    }

    /// System temp directories (resolved through firmlinks on macOS).
    pub fn temp_paths(&self) -> Vec<PathBuf> {
        if cfg!(target_os = "macos") {
            vec![
                MacosUtils::resolve_firmlink(&PathBuf::from("/tmp")),
                MacosUtils::resolve_firmlink(&PathBuf::from("/var/tmp")),
            ]
        } else {
            vec![PathBuf::from("/tmp"), PathBuf::from("/var/tmp")]
        }
    }

    /// User log directory.
    pub fn user_log_paths(&self) -> Vec<PathBuf> {
        if cfg!(target_os = "macos") {
            vec![MacosUtils::home_dir().join("Library/Logs")]
        } else {
            vec![MacosUtils::home_dir().join(".local/share/logs")]
        }
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
    /// to avoid breaking installed tooling and to protect backup volumes.
    pub fn sweep_skip_dirs(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        let mut paths = vec![
            home.join(".cargo/bin"),
            home.join(".cursor"),
            home.join(".windsurf"),
            home.join(".vscode"),
            home.join(".rustup"),
            home.join(".local"),
        ];
        if cfg!(target_os = "macos") {
            paths.push(PathBuf::from("/System"));
        }
        paths.push(PathBuf::from("/usr"));
        // Always skip Time Machine / backup volumes during sweeps.
        paths.extend(crate::util::backup::backup_volumes());
        paths
    }

    // ---- Browser cache paths --------------------------------------------

    /// Browser cache directories. On macOS: Safari, Chrome, Firefox, Edge,
    /// Brave, Arc. On Linux: Chrome, Firefox, Brave, Edge (XDG paths).
    pub fn browser_cache_paths(&self) -> Vec<(&'static str, PathBuf)> {
        let home = MacosUtils::home_dir();
        if cfg!(target_os = "macos") {
            vec![
                ("Safari", home.join("Library/Caches/com.apple.Safari")),
                (
                    "Safari Containers",
                    home.join(
                        "Library/Containers/com.apple.Safari/Data/Library/Caches/com.apple.Safari",
                    ),
                ),
                ("Chrome", home.join("Library/Caches/Google/Chrome")),
                ("Firefox", home.join("Library/Caches/Firefox")),
                ("Edge", home.join("Library/Caches/Microsoft Edge")),
                ("Brave", home.join("Library/Caches/BraveSoftware")),
                ("Arc", home.join("Library/Caches/com.thebrowser.Browser")),
            ]
        } else {
            let xdg_cache = std::env::var("XDG_CACHE_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".cache"));
            let xdg_config = std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".config"));
            vec![
                ("Chrome", xdg_cache.join("google-chrome")),
                ("Firefox", xdg_cache.join("mozilla/firefox")),
                ("Edge", xdg_cache.join("microsoft-edge")),
                ("Brave", xdg_cache.join("BraveSoftware/Brave-Browser")),
                ("Chromium", xdg_cache.join("chromium")),
                // Firefox stores profiles under .mozilla/firefox/*/cache2
                ("Firefox Profiles", xdg_config.join("mozilla/firefox")),
            ]
        }
    }

    // ---- Mail attachment paths -------------------------------------------

    /// Mail attachment and download directories (macOS only — Linux mail
    /// clients use varied locations).
    pub fn mail_paths(&self) -> Vec<PathBuf> {
        if cfg!(target_os = "macos") {
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
        } else {
            vec![]
        }
    }

    // ---- iOS backup paths ------------------------------------------------

    /// iOS device backup directory (Finder/iTunes backups — macOS only).
    pub fn ios_backup_paths(&self) -> Vec<PathBuf> {
        if cfg!(target_os = "macos") {
            let home = MacosUtils::home_dir();
            vec![home.join("Library/Application Support/MobileSync/Backup")]
        } else {
            vec![]
        }
    }

    // ---- Trash paths -----------------------------------------------------

    /// Trash directories on all mounted volumes.
    pub fn trash_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        let mut paths = if cfg!(target_os = "macos") {
            vec![home.join(".Trash")]
        } else {
            // Linux: XDG trash spec
            let xdg_data = std::env::var("XDG_DATA_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join(".local/share"));
            vec![xdg_data.join("Trash")]
        };
        // External/volume trashes (macOS: /Volumes, Linux: /media, /mnt)
        let volume_roots = if cfg!(target_os = "macos") {
            vec!["/Volumes"]
        } else {
            vec!["/media", "/mnt"]
        };
        for root in volume_roots {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let trash_name = if cfg!(target_os = "macos") {
                        ".Trashes"
                    } else {
                        ".Trash-1000"
                    };
                    let trash = entry.path().join(trash_name);
                    if trash.exists() {
                        paths.push(trash);
                    }
                }
            }
        }
        paths
    }

    // ---- Document version paths ------------------------------------------

    /// Document revision stores on volumes (macOS APFS only).
    pub fn document_version_paths(&self) -> Vec<PathBuf> {
        if cfg!(target_os = "macos") {
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
        } else {
            vec![]
        }
    }

    // ---- Application directories (for language file scanning) -----------

    /// Directories containing .app bundles to scan for language files.
    /// On Linux, returns empty (no .app bundles).
    pub fn app_dirs(&self) -> Vec<PathBuf> {
        if cfg!(target_os = "macos") {
            let home = MacosUtils::home_dir();
            vec![PathBuf::from("/Applications"), home.join("Applications")]
        } else {
            vec![]
        }
    }

    /// Language codes to preserve when scanning for removable .lproj dirs.
    /// Base.lproj is always preserved (it's not language-specific).
    pub fn keep_language_codes(&self) -> &[&'static str] {
        &["en", "English", "Base", "base"]
    }

    // ---- Installer file detection (op 132-134) -------------------------

    /// Directories to scan for leftover installer files (DMG/PKG). On macOS
    /// this includes the user's Downloads folder and Desktop; on Linux the
    /// XDG downloads directory.
    #[allow(dead_code)]
    pub fn installer_scan_dirs(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        if cfg!(target_os = "macos") {
            vec![
                home.join("Downloads"),
                home.join("Desktop"),
                home.join("Documents"),
            ]
        } else {
            let xdg_download = std::env::var("XDG_DOWNLOAD_DIR")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home.join("Downloads"));
            vec![xdg_download]
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    #[test]
    fn default_rules_include_dmg_and_pkg() {
        let rules = CleanRules::default();
        let names: Vec<_> = rules.rules.iter().map(|r| r.name).collect();
        assert!(names.contains(&"dmg_files"));
        assert!(names.contains(&"pkg_files"));
        assert!(names.contains(&"old_installers"));
    }

    #[test]
    fn is_dmg_file_detects_dmg_extension() {
        assert!(is_dmg_file(Path::new("/Users/x/Downloads/Setup.dmg")));
        assert!(is_dmg_file(Path::new("/Users/x/Setup.DMG")));
        assert!(!is_dmg_file(Path::new("/Users/x/Setup.pkg")));
    }

    #[test]
    fn is_pkg_file_detects_pkg_and_mpkg() {
        assert!(is_pkg_file(Path::new("/Users/x/Installer.pkg")));
        assert!(is_pkg_file(Path::new("/Users/x/Installer.mpkg")));
        assert!(!is_pkg_file(Path::new("/Users/x/Installer.dmg")));
    }

    #[test]
    fn is_installer_file_covers_all_extensions() {
        assert!(is_installer_file(Path::new("a.dmg")));
        assert!(is_installer_file(Path::new("a.pkg")));
        assert!(is_installer_file(Path::new("a.mpkg")));
        assert!(!is_installer_file(Path::new("a.txt")));
    }

    #[test]
    fn installer_scan_dirs_returns_non_empty() {
        let rules = CleanRules::default();
        let dirs = rules.installer_scan_dirs();
        assert!(!dirs.is_empty());
    }
}
