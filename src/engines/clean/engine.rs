use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use walkdir::WalkDir;

use crate::cli::args::CleanArgs;
use crate::util::disk::physical_size;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

use super::scanner::CleanScanner;
use super::rules::{CleanRules, BUILD_ARTIFACT_DIRS, BUILD_ARTIFACT_FILE_PATTERNS, ROTATED_LOG_EXTENSIONS, TEMP_FILE_NAMES, SWAP_FILE_PATTERNS};

pub struct CleanEngine {
    args: CleanArgs,
    rules: CleanRules,
}

impl CleanEngine {
    pub fn new(args: CleanArgs) -> Self {
        Self {
            rules: CleanRules::default(),
            args,
        }
    }

    async fn scan_caches(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let min_age = humantime::parse_duration(&self.args.min_age).unwrap_or_else(|_| std::time::Duration::from_secs(30 * 24 * 60 * 60));
        let min_size = byte_unit::Byte::from_str(&self.args.min_size).map(|b| b.get_bytes() as u64).unwrap_or(1024 * 1024);

        let mut all_paths = self.rules.cache_paths();
        for user_path in &self.args.paths {
            all_paths.push(user_path.clone());
        }

        for path in all_paths {
            if !path.exists() {
                continue;
            }

            let entries = CleanScanner::scan_directory(&path, &ctx.config);
            for entry in entries {
                items += 1;
                let entry_path = entry.path();

                if !CleanScanner::is_older_than(entry_path, min_age) {
                    continue;
                }

                let size = if entry.file_type().is_dir() {
                    CleanScanner::dir_size(entry_path)
                } else {
                    CleanScanner::file_size(entry_path)
                };

                if size < min_size {
                    continue;
                }

                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Low,
                        Category::Cache,
                        Target::Path(entry_path.to_path_buf()),
                        "Cache file detected",
                        format!("Found cache file older than {} with size {}", self.args.min_age, crate::util::disk::format_bytes(size)),
                    )
                    .with_size(size)
                    .with_hint("Consider clearing this cache to free disk space".to_string()),
                );
            }
        }

        (findings, items)
    }

    async fn scan_xcode(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for path in self.rules.xcode_paths() {
            if !path.exists() {
                continue;
            }
            items += 1;

            if let Ok(dir_size) = tokio::task::spawn_blocking({
                let path = path.clone();
                move || CleanScanner::dir_size(&path)
            }).await {
                if dir_size > 0 {
                    findings.push(
                        Finding::new(
                            EngineId::Clean,
                            Severity::Medium,
                            Category::XcodeArtifact,
                            Target::Path(path.clone()),
                            "Xcode artifact directory",
                            format!("Found Xcode {} with size {}", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), crate::util::disk::format_bytes(dir_size)),
                        )
                        .with_size(dir_size)
                        .with_hint("Run 'xcodebuild clean' or delete DerivedData to free space".to_string()),
                    );
                }
            }
        }

        (findings, items)
    }

    async fn scan_orphans(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let app_support = crate::util::macos::MacosUtils::get_application_support_dir();

        if !app_support.exists() {
            return (findings, items);
        }

        // Build a cache of installed app bundle IDs + names once.
        let installed_apps = Self::collect_installed_apps();

        if let Ok(entries) = std::fs::read_dir(&app_support) {
            for entry in entries.flatten() {
                let dir_path = entry.path();
                if !dir_path.is_dir() {
                    continue;
                }
                items += 1;

                let dir_name = dir_path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let is_orphan = !Self::is_app_installed(&dir_name, &installed_apps);

                if is_orphan {
                    let size = CleanScanner::dir_size(&dir_path);
                    if size > 0 {
                        findings.push(
                            Finding::new(
                                EngineId::Clean,
                                Severity::Low,
                                Category::OrphanFile,
                                Target::Path(dir_path.clone()),
                                "Orphan application support directory",
                                format!("Found orphaned app support directory: {}", dir_name),
                            )
                            .with_size(size)
                            .with_hint("This application may have been uninstalled. Verify before deleting.".to_string()),
                        );
                    }
                }
            }
        }

        (findings, items)
    }

    /// Collect installed app bundle identifiers and names from the standard
    /// Applications directories. Returns a set of lowercase strings that can
    /// be matched against Application Support directory names.
    fn collect_installed_apps() -> std::collections::HashSet<String> {
        use std::process::Command;
        let mut set = std::collections::HashSet::new();
        let home = crate::util::macos::MacosUtils::home_dir();
        let app_dirs = [
            PathBuf::from("/Applications"),
            PathBuf::from("/System/Applications"),
            home.join("Applications"),
        ];

        for app_dir in &app_dirs {
            if !app_dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(app_dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_dir() || !path.to_string_lossy().ends_with(".app") {
                        continue;
                    }
                    // Add the bundle name (without .app).
                    if let Some(name) = path.file_stem() {
                        set.insert(name.to_string_lossy().to_lowercase());
                    }
                    // Add the bundle identifier via defaults read.
                    let info_plist = path.join("Contents/Info.plist");
                    if info_plist.exists() {
                        if let Ok(output) = Command::new("defaults")
                            .args(["read", &info_plist.to_string_lossy(), "CFBundleIdentifier"])
                            .output()
                        {
                            if output.status.success() {
                                let bundle_id = String::from_utf8_lossy(&output.stdout).trim().to_lowercase();
                                if !bundle_id.is_empty() {
                                    set.insert(bundle_id);
                                }
                            }
                        }
                    }
                }
            }
        }
        set
    }

    fn is_app_installed(app_name: &str, installed: &std::collections::HashSet<String>) -> bool {
        let name_lower = app_name.to_lowercase();
        // Direct match against bundle name or bundle ID.
        if installed.contains(&name_lower) {
            return true;
        }
        // Apple system services (com.apple.*) are always considered installed
        // — they're part of the OS and the .app may not be in /Applications.
        if name_lower.starts_with("com.apple.") {
            return true;
        }
        false
    }

    async fn scan_duplicates(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let mut hash_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        let search_paths = vec![
            crate::util::macos::MacosUtils::home_dir().join("Downloads"),
            crate::util::macos::MacosUtils::home_dir().join("Documents"),
        ];

        for search_path in search_paths {
            if !search_path.exists() {
                continue;
            }

            let entries = WalkDir::new(&search_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file());

            for entry in entries {
                items += 1;
                let path = entry.path();
                if let Ok(hash) = Self::compute_file_hash(path).await {
                    hash_map.entry(hash).or_default().push(path.to_path_buf());
                }
            }
        }

        for (hash, paths) in hash_map {
            if paths.len() > 1 {
                let total_size: u64 = paths.iter()
                    .filter_map(|p| std::fs::metadata(p).ok())
                    .map(physical_size)
                    .sum();

                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Medium,
                        Category::DuplicateFile,
                        Target::Path(paths[0].clone()),
                        "Duplicate files detected",
                        format!("Found {} duplicate files with hash {}. Total size: {}", paths.len(), hash.chars().take(8).collect::<String>(), crate::util::disk::format_bytes(total_size)),
                    )
                    .with_size(total_size)
                    .with_metadata("duplicate_paths".to_string(), serde_json::json!(paths.iter().map(|p| p.to_string_lossy().to_string()).collect::<Vec<_>>()))
                    .with_hint("Review and remove duplicate files to save space".to_string()),
                );
            }
        }

        (findings, items)
    }

    async fn compute_file_hash(path: &std::path::Path) -> Result<String, std::io::Error> {
        use tokio::io::AsyncReadExt;

        let mut file = tokio::fs::File::open(path).await?;
        let mut hasher = blake3::Hasher::new();
        let mut buffer = vec![0u8; 8192];

        loop {
            let bytes_read = file.read(&mut buffer).await?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        Ok(hasher.finalize().to_hex().to_string())
    }

    // ---- New scan methods: package-manager caches ------------------------

    /// Detect package-manager cache directories (npm, pip, cargo, Homebrew,
    /// go, gradle, maven). These are regenerated on demand and safe to clear.
    async fn scan_pkg_caches(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for path in self.rules.pkg_cache_paths() {
            if !path.exists() {
                continue;
            }
            items += 1;

            let size = if path.is_dir() {
                CleanScanner::dir_size(&path)
            } else {
                CleanScanner::file_size(&path)
            };

            if size == 0 {
                continue;
            }

            let name = path.file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| path.to_string_lossy().to_string());

            findings.push(
                Finding::new(
                    EngineId::Clean,
                    Severity::Low,
                    Category::PackageManagerCache,
                    Target::Path(path.clone()),
                    "Package-manager cache detected",
                    format!("Found package-manager cache '{}' with size {}", name, crate::util::disk::format_bytes(size)),
                )
                .with_size(size)
                .with_hint("Safe to clear — regenerated on next install/build".to_string()),
            );
        }

        (findings, items)
    }

    // ---- New scan methods: temp files ------------------------------------

    /// Detect temp files: contents of /tmp and /var/tmp, .DS_Store files,
    /// and editor swap files (.swp/.swo) under the home directory.
    async fn scan_temp_files(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        // 1. System temp directories — report aggregate size.
        for temp_path in self.rules.temp_paths() {
            if !temp_path.exists() {
                continue;
            }
            items += 1;
            let size = CleanScanner::dir_size(&temp_path);
            if size > 0 {
                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Low,
                        Category::TempFile,
                        Target::Path(temp_path.clone()),
                        "System temp directory",
                        format!("Temp directory '{}' contains {} of data", temp_path.to_string_lossy(), crate::util::disk::format_bytes(size)),
                    )
                    .with_size(size)
                    .with_hint("Clear with: rm -rf /tmp/* /var/tmp/* (macOS recreates these on reboot)".to_string()),
                );
            }
        }

        // 2. .DS_Store and swap files under home — use a bounded walk that
        //    prunes skip dirs to avoid traversing editor extensions etc.
        let home = self.rules.home_search_root();
        let skip_dirs: Vec<PathBuf> = self.rules.sweep_skip_dirs();

        let ds_store_findings = tokio::task::spawn_blocking({
            let home = home.clone();
            let skip_dirs = skip_dirs.clone();
            move || Self::sweep_named_files(&home, TEMP_FILE_NAMES, &skip_dirs, "DS_Store")
        })
        .await
        .unwrap_or_default();

        items += ds_store_findings.len() as u64;
        let ds_size: u64 = ds_store_findings.iter().filter_map(|f| f.size_bytes).sum();
        if !ds_store_findings.is_empty() {
            findings.push(
                Finding::new(
                    EngineId::Clean,
                    Severity::Low,
                    Category::TempFile,
                    Target::Path(home.join(".DS_Store")),
                    "DS_Store files detected",
                    format!("Found {} .DS_Store files under home (total {})", ds_store_findings.len(), crate::util::disk::format_bytes(ds_size)),
                )
                .with_size(ds_size)
                .with_metadata("file_count", serde_json::json!(ds_store_findings.len()))
                .with_hint("Clear with: find ~ -name .DS_Store -delete".to_string()),
            );
        }

        // 3. Swap files (.swp/.swo).
        let swap_findings = tokio::task::spawn_blocking({
            let home = home.clone();
            let skip_dirs = skip_dirs.clone();
            move || Self::sweep_named_files(&home, SWAP_FILE_PATTERNS, &skip_dirs, "swap")
        })
        .await
        .unwrap_or_default();

        items += swap_findings.len() as u64;
        let swap_size: u64 = swap_findings.iter().filter_map(|f| f.size_bytes).sum();
        if !swap_findings.is_empty() {
            findings.push(
                Finding::new(
                    EngineId::Clean,
                    Severity::Low,
                    Category::TempFile,
                    Target::Path(home.join("*.swp")),
                    "Editor swap files detected",
                    format!("Found {} swap files (.swp/.swo) under home (total {})", swap_findings.len(), crate::util::disk::format_bytes(swap_size)),
                )
                .with_size(swap_size)
                .with_metadata("file_count", serde_json::json!(swap_findings.len()))
                .with_hint("Clear with: find ~ -name '*.swp' -o -name '*.swo' -delete".to_string()),
            );
        }

        (findings, items)
    }

    // ---- New scan methods: build artifacts -------------------------------

    /// Detect build-artifact directories (node_modules, target, __pycache__,
    /// dist, build, etc.) and compiled artifact files (.pyc, .o, .so, etc.)
    /// under the home directory. Skips editor extensions and toolchain dirs
    /// to avoid breaking installed software.
    async fn scan_build_artifacts(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        let home = self.rules.home_search_root();
        let skip_dirs = self.rules.sweep_skip_dirs();

        // 1. Build-artifact directories (node_modules, target, __pycache__, …)
        let dir_findings = tokio::task::spawn_blocking({
            let home = home.clone();
            let skip_dirs = skip_dirs.clone();
            move || Self::sweep_build_artifact_dirs(&home, &skip_dirs)
        })
        .await
        .unwrap_or_default();

        items += dir_findings.0;
        findings.extend(dir_findings.1);

        // 2. Compiled artifact files (.pyc, .o, .so, .class, etc.)
        let file_findings = tokio::task::spawn_blocking({
            let home = home.clone();
            let skip_dirs = skip_dirs.clone();
            move || Self::sweep_build_artifact_files(&home, &skip_dirs)
        })
        .await
        .unwrap_or_default();

        items += file_findings.0;
        findings.extend(file_findings.1);

        (findings, items)
    }

    // ---- New scan methods: rotated logs ----------------------------------

    /// Detect rotated / archived log files (*.gz, *.bz2, *.xz, *.zst, *.0)
    /// in system and user log directories. Active log files are never flagged.
    async fn scan_rotated_logs(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        let mut all_log_dirs = self.rules.system_log_paths();
        all_log_dirs.extend(self.rules.user_log_paths());

        for log_dir in all_log_dirs {
            if !log_dir.exists() {
                continue;
            }

            let entries = WalkDir::new(&log_dir)
                .max_depth(1)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file());

            for entry in entries {
                items += 1;
                let path = entry.path();
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();

                let is_rotated = ROTATED_LOG_EXTENSIONS.iter().any(|ext| name.ends_with(ext))
                    || name.ends_with(".0")
                    || name.ends_with(".1")
                    || name.ends_with(".2")
                    || name.ends_with(".3")
                    || name.ends_with(".4")
                    || name.ends_with(".5")
                    || name.ends_with(".6")
                    || name.ends_with(".7")
                    || name.ends_with(".8")
                    || name.ends_with(".9");

                if !is_rotated {
                    continue;
                }

                let size = CleanScanner::file_size(path);
                if size == 0 {
                    continue;
                }

                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Low,
                        Category::Log,
                        Target::Path(path.to_path_buf()),
                        "Rotated log file detected",
                        format!("Found rotated log '{}' with size {}", name, crate::util::disk::format_bytes(size)),
                    )
                    .with_size(size)
                    .with_hint("Safe to delete — these are archived logs, not active".to_string()),
                );
            }
        }

        (findings, items)
    }

    // ---- Sweep helpers (run on spawn_blocking) ---------------------------

    /// Walk `root` (pruning `skip_dirs`) and collect findings for files whose
    /// name ends with any of the given `patterns`. Returns one Finding per
    /// file so the caller can aggregate if desired.
    fn sweep_named_files(
        root: &std::path::Path,
        patterns: &[&str],
        skip_dirs: &[PathBuf],
        label: &str,
    ) -> Vec<Finding> {
        let mut results = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| !Self::is_in_skip_dir(e.path(), skip_dirs))
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let name = entry.file_name().to_string_lossy().to_string();
            if patterns.iter().any(|p| name.ends_with(p) || name == **p) {
                let size = entry.metadata().map(physical_size).unwrap_or(0);
                results.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Low,
                        Category::TempFile,
                        Target::Path(entry.path().to_path_buf()),
                        format!("{} file detected", label),
                        format!("Found {} file: {}", label, entry.path().to_string_lossy()),
                    )
                    .with_size(size),
                );
            }
        }

        results
    }

    /// Walk `root` (pruning `skip_dirs`) and find build-artifact directories
    /// (node_modules, target, __pycache__, etc.). When a match is found the
    /// directory is pruned (we don't descend into it) to avoid wasted work.
    /// Returns (items_scanned, findings).
    fn sweep_build_artifact_dirs(root: &std::path::Path, skip_dirs: &[PathBuf]) -> (u64, Vec<Finding>) {
        let mut items = 0u64;
        let mut findings = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                // Prune skip dirs.
                if Self::is_in_skip_dir(e.path(), skip_dirs) {
                    return false;
                }
                // Prune build-artifact dirs themselves — we report them but
                // don't descend into them.
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    if BUILD_ARTIFACT_DIRS.contains(&name.as_ref()) {
                        return false;
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_dir())
        {
            let name = entry.file_name().to_string_lossy();
            if BUILD_ARTIFACT_DIRS.contains(&name.as_ref()) {
                items += 1;
                let path = entry.path();
                let size = CleanScanner::dir_size(path);
                if size > 0 {
                    findings.push(
                        Finding::new(
                            EngineId::Clean,
                            Severity::Medium,
                            Category::BuildArtifact,
                            Target::Path(path.to_path_buf()),
                            format!("Build artifact directory: {}", name),
                            format!("Found '{}' directory with size {} — regenerated on next build", name, crate::util::disk::format_bytes(size)),
                        )
                        .with_size(size)
                        .with_hint(format!("Safe to delete: rm -rf '{}'", path.to_string_lossy())),
                    );
                }
            }
        }

        (items, findings)
    }

    /// Walk `root` (pruning `skip_dirs` and build-artifact dirs) and find
    /// compiled artifact files (.pyc, .o, .so, .class, etc.). Returns
    /// (items_scanned, findings).
    fn sweep_build_artifact_files(root: &std::path::Path, skip_dirs: &[PathBuf]) -> (u64, Vec<Finding>) {
        let mut items = 0u64;
        let mut findings = Vec::new();

        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e| {
                if Self::is_in_skip_dir(e.path(), skip_dirs) {
                    return false;
                }
                if e.file_type().is_dir() {
                    let name = e.file_name().to_string_lossy();
                    if BUILD_ARTIFACT_DIRS.contains(&name.as_ref()) {
                        return false;
                    }
                }
                true
            })
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let name = entry.file_name().to_string_lossy();
            if BUILD_ARTIFACT_FILE_PATTERNS.iter().any(|p| name.ends_with(p)) {
                items += 1;
                let size = entry.metadata().map(physical_size).unwrap_or(0);
                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Low,
                        Category::BuildArtifact,
                        Target::Path(entry.path().to_path_buf()),
                        format!("Build artifact file: {}", name),
                        format!("Found compiled artifact '{}' ({}). Regenerated on next build.", entry.path().to_string_lossy(), crate::util::disk::format_bytes(size)),
                    )
                    .with_size(size)
                    .with_hint("Safe to delete — regenerated by the compiler/build tool".to_string()),
                );
            }
        }

        (items, findings)
    }

    /// Check whether `path` is inside any of the `skip_dirs`.
    fn is_in_skip_dir(path: &std::path::Path, skip_dirs: &[PathBuf]) -> bool {
        let path_str = path.to_string_lossy();
        for skip in skip_dirs {
            let skip_str = skip.to_string_lossy();
            if path_str == skip_str || path_str.starts_with(&*format!("{}/", skip_str)) {
                return true;
            }
        }
        false
    }

    // ---- Browser caches --------------------------------------------------

    async fn scan_browser_caches(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for (name, path) in self.rules.browser_cache_paths() {
            if !path.exists() {
                continue;
            }
            items += 1;
            let size = CleanScanner::dir_size(&path);
            if size > 0 {
                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Low,
                        Category::BrowserCache,
                        Target::Path(path.clone()),
                        format!("{} browser cache", name),
                        format!("Browser cache for {} occupies {}", name, crate::util::disk::format_bytes(size)),
                    )
                    .with_size(size)
                    .with_hint("Safe to clear — browser will rebuild cache on next use".to_string()),
                );
            }
        }

        (findings, items)
    }

    // ---- Mail attachments ------------------------------------------------

    async fn scan_mail(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for path in self.rules.mail_paths() {
            if !path.exists() {
                continue;
            }
            items += 1;
            let size = CleanScanner::dir_size(&path);
            if size > 0 {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());
                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Medium,
                        Category::MailAttachment,
                        Target::Path(path.clone()),
                        "Mail attachments/downloads detected",
                        format!("Mail data directory '{}' occupies {}", name, crate::util::disk::format_bytes(size)),
                    )
                    .with_size(size)
                    .with_hint("Attachments may be re-downloaded from server. Review before deleting.".to_string()),
                );
            }
        }

        (findings, items)
    }

    // ---- iOS backups -----------------------------------------------------

    async fn scan_ios_backups(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for backup_root in self.rules.ios_backup_paths() {
            if !backup_root.exists() {
                continue;
            }
            items += 1;

            // Each subdirectory is a device backup. List them with sizes so
            // the user can decide which old backups to remove.
            if let Ok(entries) = std::fs::read_dir(&backup_root) {
                for entry in entries.flatten() {
                    let dir = entry.path();
                    if !dir.is_dir() {
                        continue;
                    }
                    items += 1;
                    let size = CleanScanner::dir_size(&dir);
                    if size > 0 {
                        // Try to read the backup status from Status.plist
                        let backup_name = dir.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        findings.push(
                            Finding::new(
                                EngineId::Clean,
                                Severity::Medium,
                                Category::IosBackup,
                                Target::Path(dir.clone()),
                                "iOS device backup detected",
                                format!("Backup '{}' occupies {}", backup_name, crate::util::disk::format_bytes(size)),
                            )
                            .with_size(size)
                            .with_hint("Old iOS backups can be removed if you no longer need them. Keep the most recent per device.".to_string()),
                        );
                    }
                }
            }
        }

        (findings, items)
    }

    // ---- Language files --------------------------------------------------

    async fn scan_language_files(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let keep = self.rules.keep_language_codes();

        for app_dir in self.rules.app_dirs() {
            if !app_dir.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(&app_dir) {
                for entry in entries.flatten() {
                    let app_path = entry.path();
                    if !app_path.is_dir() || !app_path.to_string_lossy().ends_with(".app") {
                        continue;
                    }

                    let resources = app_path.join("Contents/Resources");
                    if !resources.exists() {
                        continue;
                    }

                    // Collect removable .lproj directories for this app
                    let mut app_lproj_size: u64 = 0;
                    let mut app_lproj_count = 0u64;

                    if let Ok(res_entries) = std::fs::read_dir(&resources) {
                        for res_entry in res_entries.flatten() {
                            let lproj_path = res_entry.path();
                            if !lproj_path.is_dir() {
                                continue;
                            }
                            let name = lproj_path.file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            if !name.ends_with(".lproj") {
                                continue;
                            }
                            items += 1;
                            let lang_code = name.trim_end_matches(".lproj");
                            if keep.contains(&lang_code) {
                                continue;
                            }
                            app_lproj_count += 1;
                            app_lproj_size += CleanScanner::dir_size(&lproj_path);
                        }
                    }

                    if app_lproj_count > 0 && app_lproj_size > 0 {
                        let app_name = app_path.file_stem()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();
                        findings.push(
                            Finding::new(
                                EngineId::Clean,
                                Severity::Low,
                                Category::LanguageFile,
                                Target::Path(app_path.join("Contents/Resources")),
                                format!("Removable language files: {}", app_name),
                                format!("Found {} non-English .lproj directories ({} total) in {}", app_lproj_count, crate::util::disk::format_bytes(app_lproj_size), app_name),
                            )
                            .with_size(app_lproj_size)
                            .with_metadata("lproj_count", serde_json::json!(app_lproj_count))
                            .with_hint("Safe to remove non-English .lproj dirs. Will be restored on app update.".to_string()),
                        );
                    }
                }
            }
        }

        (findings, items)
    }

    // ---- Trash bins ------------------------------------------------------

    async fn scan_trash(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for trash_path in self.rules.trash_paths() {
            if !trash_path.exists() {
                continue;
            }
            items += 1;
            let size = CleanScanner::dir_size(&trash_path);
            if size > 0 {
                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Medium,
                        Category::TrashBin,
                        Target::Path(trash_path.clone()),
                        "Trash bin with content",
                        format!("Trash at '{}' contains {} of data", trash_path.to_string_lossy(), crate::util::disk::format_bytes(size)),
                    )
                    .with_size(size)
                    .with_hint("Empty trash to reclaim space. Use 'rm -rf' for locked files.".to_string()),
                );
            }
        }

        (findings, items)
    }

    // ---- Large files -----------------------------------------------------

    async fn scan_large_files(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let min_large = byte_unit::Byte::from_str(&self.args.min_large_size)
            .map(|b| b.get_bytes() as u64)
            .unwrap_or(100 * 1024 * 1024);

        let search_paths = if self.args.paths.is_empty() {
            vec![crate::util::macos::MacosUtils::home_dir()]
        } else {
            self.args.paths.clone()
        };

        for search_path in search_paths {
            if !search_path.exists() {
                continue;
            }

            let entries = WalkDir::new(&search_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file());

            for entry in entries {
                items += 1;
                let size = entry.metadata().map(physical_size).unwrap_or(0);
                if size >= min_large {
                    let path = entry.path();
                    let name = entry.file_name().to_string_lossy().to_string();
                    findings.push(
                        Finding::new(
                            EngineId::Clean,
                            Severity::Low,
                            Category::LargeFile,
                            Target::Path(path.to_path_buf()),
                            format!("Large file: {} ({})", name, crate::util::disk::format_bytes(size)),
                            format!("File '{}' is {} — exceeds threshold of {}", path.to_string_lossy(), crate::util::disk::format_bytes(size), crate::util::disk::format_bytes(min_large)),
                        )
                        .with_size(size)
                        .with_hint("Review and delete if no longer needed".to_string()),
                    );
                }
            }
        }

        (findings, items)
    }

    // ---- Document versions -----------------------------------------------

    async fn scan_document_versions(&self, _ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for path in self.rules.document_version_paths() {
            if !path.exists() {
                continue;
            }
            items += 1;
            let size = CleanScanner::dir_size(&path);
            if size > 0 {
                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Low,
                        Category::DocumentVersion,
                        Target::Path(path.clone()),
                        "Document version store detected",
                        format!("Document revisions at '{}' occupy {}", path.to_string_lossy(), crate::util::disk::format_bytes(size)),
                    )
                    .with_size(size)
                    .with_hint("Removing frees space but loses document revision history. macOS recreates this on demand.".to_string()),
                );
            }
        }

        (findings, items)
    }
}

#[async_trait]
impl Engine for CleanEngine {
    fn id(&self) -> EngineId {
        EngineId::Clean
    }

    fn name(&self) -> &'static str {
        "Clean Engine"
    }

    fn description(&self) -> &'static str {
        "Detects bloat: caches, logs, Xcode artifacts, build artifacts, temp files, and orphan files"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        let (cache_findings, cache_items) = self.scan_caches(&ctx).await;
        items_scanned += cache_items;
        findings_count += cache_findings.len() as u64;
        for finding in cache_findings {
            ctx.emit(finding).await;
        }

        if self.args.xcode {
            let (xcode_findings, xcode_items) = self.scan_xcode(&ctx).await;
            items_scanned += xcode_items;
            findings_count += xcode_findings.len() as u64;
            for finding in xcode_findings {
                ctx.emit(finding).await;
            }
        }

        let (orphan_findings, orphan_items) = self.scan_orphans(&ctx).await;
        items_scanned += orphan_items;
        findings_count += orphan_findings.len() as u64;
        for finding in orphan_findings {
            ctx.emit(finding).await;
        }

        if self.args.dedup {
            let (dup_findings, dup_items) = self.scan_duplicates(&ctx).await;
            items_scanned += dup_items;
            findings_count += dup_findings.len() as u64;
            for finding in dup_findings {
                ctx.emit(finding).await;
            }
        }

        if self.args.pkg_caches {
            let (pkg_findings, pkg_items) = self.scan_pkg_caches(&ctx).await;
            items_scanned += pkg_items;
            findings_count += pkg_findings.len() as u64;
            for finding in pkg_findings {
                ctx.emit(finding).await;
            }
        }

        if self.args.temp {
            let (temp_findings, temp_items) = self.scan_temp_files(&ctx).await;
            items_scanned += temp_items;
            findings_count += temp_findings.len() as u64;
            for finding in temp_findings {
                ctx.emit(finding).await;
            }
        }

        if self.args.build_artifacts {
            let (build_findings, build_items) = self.scan_build_artifacts(&ctx).await;
            items_scanned += build_items;
            findings_count += build_findings.len() as u64;
            for finding in build_findings {
                ctx.emit(finding).await;
            }
        }

        let (log_findings, log_items) = self.scan_rotated_logs(&ctx).await;
        items_scanned += log_items;
        findings_count += log_findings.len() as u64;
        for finding in log_findings {
            ctx.emit(finding).await;
        }

        if self.args.browser {
            let (f, i) = self.scan_browser_caches(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
            for finding in f { ctx.emit(finding).await; }
        }

        if self.args.mail {
            let (f, i) = self.scan_mail(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
            for finding in f { ctx.emit(finding).await; }
        }

        if self.args.ios_backups {
            let (f, i) = self.scan_ios_backups(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
            for finding in f { ctx.emit(finding).await; }
        }

        if self.args.languages {
            let (f, i) = self.scan_language_files(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
            for finding in f { ctx.emit(finding).await; }
        }

        if self.args.trash {
            let (f, i) = self.scan_trash(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
            for finding in f { ctx.emit(finding).await; }
        }

        if self.args.large_files {
            let (f, i) = self.scan_large_files(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
            for finding in f { ctx.emit(finding).await; }
        }

        // Document versions — always scanned (low cost, few paths)
        let (dv_findings, dv_items) = self.scan_document_versions(&ctx).await;
        items_scanned += dv_items;
        findings_count += dv_findings.len() as u64;
        for finding in dv_findings { ctx.emit(finding).await; }

        Ok(EngineStats {
            engine: self.id(),
            duration: start.elapsed(),
            items_scanned,
            findings_count,
            errors_count: 0,
        })
    }
}

impl Default for CleanEngine {
    fn default() -> Self {
        Self::new(Self::default_args())
    }
}

impl CleanEngine {
    /// Returns the default CleanArgs — used by `quick`, `purge`, and other
    /// composite commands that need to construct a CleanEngine with specific
    /// overrides.
    pub fn default_args() -> CleanArgs {
        CleanArgs {
            min_age: "30d".to_string(),
            min_size: "1M".to_string(),
            dedup: false,
            xcode: true,
            pkg_caches: true,
            temp: true,
            build_artifacts: true,
            browser: true,
            mail: true,
            ios_backups: true,
            languages: true,
            trash: true,
            large_files: true,
            min_large_size: "100M".to_string(),
            paths: Vec::new(),
        }
    }
}
