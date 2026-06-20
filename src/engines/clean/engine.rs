use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use walkdir::WalkDir;

use crate::cli::args::CleanArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

use super::scanner::CleanScanner;
use super::rules::CleanRules;

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

                let is_orphan = !Self::is_app_installed(&dir_name);

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
                            .with_hint("This application may have been uninstalled. You can safely delete this directory.".to_string()),
                        );
                    }
                }
            }
        }

        (findings, items)
    }

    fn is_app_installed(app_name: &str) -> bool {
        let applications = [
            "/Applications",
            "/System/Applications",
        ];

        for app_dir in &applications {
            let app_path = PathBuf::from(app_dir).join(format!("{}.app", app_name));
            if app_path.exists() {
                return true;
            }
        }

        let home = crate::util::macos::MacosUtils::home_dir();
        let user_apps = home.join("Applications");
        let user_app_path = user_apps.join(format!("{}.app", app_name));
        user_app_path.exists()
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
                    .map(|m| m.len())
                    .sum();

                findings.push(
                    Finding::new(
                        EngineId::Clean,
                        Severity::Medium,
                        Category::DuplicateFile,
                        Target::Path(paths[0].clone()),
                        "Duplicate files detected",
                        format!("Found {} duplicate files with hash {}. Total size: {}", paths.len(), &hash[..8], crate::util::disk::format_bytes(total_size)),
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
        "Detects bloat: caches, logs, Xcode artifacts, and orphan files"
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
        Self::new(CleanArgs {
            min_age: "30d".to_string(),
            min_size: "1M".to_string(),
            dedup: false,
            xcode: true,
            paths: Vec::new(),
        })
    }
}
