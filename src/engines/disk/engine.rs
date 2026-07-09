use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use walkdir::WalkDir;

use crate::cli::args::DiskArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

/// The disk engine analyzes disk usage and emits findings for the top
/// largest directories and files under the given paths. Unlike the clean
/// engine, these findings are informational (Severity::Info) — they help
/// the user understand where disk space is being used.
pub struct DiskEngine {
    args: DiskArgs,
}

impl DiskEngine {
    pub fn new(args: DiskArgs) -> Self {
        Self { args }
    }

    fn dir_size(path: &std::path::Path) -> u64 {
        WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum()
    }
}

#[async_trait]
impl Engine for DiskEngine {
    fn id(&self) -> EngineId {
        EngineId::All
    }

    fn name(&self) -> &'static str {
        "Disk Engine"
    }

    fn description(&self) -> &'static str {
        "Analyzes disk usage — top directories and files by size"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        let min_size = byte_unit::Byte::from_str(&self.args.min_size)
            .map(|b| b.get_bytes() as u64)
            .unwrap_or(100 * 1024 * 1024);

        let search_paths = if self.args.paths.is_empty() {
            vec![crate::util::macos::MacosUtils::home_dir()]
        } else {
            self.args.paths.clone()
        };

        for search_path in &search_paths {
            if !search_path.exists() {
                continue;
            }

            // Collect immediate children with sizes, sorted descending
            let mut entries: Vec<(PathBuf, u64, bool)> = Vec::new();

            if let Ok(dir_entries) = std::fs::read_dir(search_path) {
                for entry in dir_entries.flatten() {
                    let path = entry.path();
                    let is_dir = path.is_dir();
                    let size = if is_dir {
                        Self::dir_size(&path)
                    } else {
                        entry.metadata().map(|m| m.len()).unwrap_or(0)
                    };
                    items_scanned += 1;
                    if size >= min_size {
                        entries.push((path, size, is_dir));
                    }
                }
            }

            entries.sort_by(|a, b| b.1.cmp(&a.1));
            entries.truncate(self.args.top);

            for (path, size, is_dir) in entries {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());

                let title = if is_dir {
                    format!("Disk usage: {} ({} dir)", name, crate::util::disk::format_bytes(size))
                } else {
                    format!("Disk usage: {} ({} file)", name, crate::util::disk::format_bytes(size))
                };

                let finding = Finding::new(
                    EngineId::All,
                    Severity::Info,
                    Category::SystemInfo,
                    Target::Path(path.clone()),
                    title,
                    format!("{} '{}' in {} — {}", if is_dir { "Directory" } else { "File" }, name, search_path.display(), crate::util::disk::format_bytes(size)),
                )
                .with_size(size)
                .with_hint(if is_dir {
                    "Large directory — drill down with: xmac disk <path>".to_string()
                } else {
                    "Large file — review and delete if no longer needed".to_string()
                });

                ctx.emit(finding).await;
                findings_count += 1;
            }

            // Also find the top N largest individual files recursively
            let mut large_files: Vec<(PathBuf, u64)> = Vec::new();

            let walker = WalkDir::new(search_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file());

            for entry in walker {
                items_scanned += 1;
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                if size >= min_size {
                    large_files.push((entry.path().to_path_buf(), size));
                }
            }

            large_files.sort_by(|a, b| b.1.cmp(&a.1));
            large_files.truncate(self.args.top);

            for (path, size) in large_files {
                let name = path.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());

                let finding = Finding::new(
                    EngineId::All,
                    Severity::Info,
                    Category::LargeFile,
                    Target::Path(path.clone()),
                    format!("Large file: {} ({})", name, crate::util::disk::format_bytes(size)),
                    format!("File '{}' — {}", path.display(), crate::util::disk::format_bytes(size)),
                )
                .with_size(size)
                .with_hint("Review and delete if no longer needed".to_string());

                ctx.emit(finding).await;
                findings_count += 1;
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

impl Default for DiskEngine {
    fn default() -> Self {
        Self::new(DiskArgs {
            top: 20,
            min_size: "100M".to_string(),
            paths: Vec::new(),
        })
    }
}
