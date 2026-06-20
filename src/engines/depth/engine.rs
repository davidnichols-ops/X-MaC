use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use walkdir::WalkDir;

use crate::cli::args::DepthArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{EngineId, EngineStats};

use super::integrity::IntegrityScanner;
use super::symlink::SymlinkScanner;

pub struct DepthEngine {
    args: DepthArgs,
}

impl DepthEngine {
    pub fn new(args: DepthArgs) -> Self {
        Self { args }
    }
}

#[async_trait]
impl Engine for DepthEngine {
    fn id(&self) -> EngineId {
        EngineId::Depth
    }

    fn name(&self) -> &'static str {
        "Depth Engine"
    }

    fn description(&self) -> &'static str {
        "Filesystem integrity: permissions, symlinks, dylibs"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        if self.args.permissions {
            let scanner = IntegrityScanner::new(self.args.paths.clone());
            let findings = scanner.scan(&ctx).await.map_err(|e| EngineError::ScanFailed(e.to_string()))?;
            items_scanned += findings.len() as u64;
            findings_count += findings.len() as u64;
            for finding in findings {
                ctx.emit(finding).await;
            }
        }

        if self.args.symlinks {
            let scanner = SymlinkScanner::new(self.args.paths.clone());
            let findings = scanner.scan(&ctx).await.map_err(|e| EngineError::ScanFailed(e.to_string()))?;
            items_scanned += findings.len() as u64;
            findings_count += findings.len() as u64;
            for finding in findings {
                ctx.emit(finding).await;
            }
        }

        if self.args.dylibs {
            let findings = self.scan_dylibs(&ctx).await;
            items_scanned += findings.len() as u64;
            findings_count += findings.len() as u64;
            for finding in findings {
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

impl DepthEngine {
    async fn scan_dylibs(&self, ctx: &ScanContext) -> Vec<crate::core::types::Finding> {
        let mut findings = Vec::new();

        let target_paths = vec![
            PathBuf::from("/usr/local/lib"),
            PathBuf::from("/opt/homebrew/lib"),
        ];

        for target_path in target_paths {
            if !target_path.exists() {
                continue;
            }

            let entries = WalkDir::new(&target_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| {
                    e.path()
                        .extension()
                        .map(|ext| ext == "dylib")
                        .unwrap_or(false)
                });

            for entry in entries {
                let path = entry.path().to_path_buf();

                if let Some(finding) = Self::check_dylib(&path) {
                    findings.push(finding);
                }
            }
        }

        findings
    }

    fn check_dylib(path: &PathBuf) -> Option<crate::core::types::Finding> {
        use std::process::Command;

        let output = Command::new("otool")
            .args(["-L", path.to_str().unwrap_or("")])
            .output();

        match output {
            Ok(out) => {
                let output_str = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = output_str.lines().collect();

                if lines.len() > 1 {
                    for line in lines.iter().skip(1) {
                        let dep = line.trim();
                        if !dep.starts_with("/usr/lib") && !dep.starts_with("/System") {
                            if !PathBuf::from(dep).exists() {
                                return Some(
                                    crate::core::types::Finding::new(
                                        EngineId::Depth,
                                        crate::core::types::Severity::Medium,
                                        crate::core::types::Category::MissingDylib,
                                        crate::core::types::Target::Path(path.clone()),
                                        "Missing dylib dependency",
                                        format!("{} depends on missing library: {}", path.display(), dep),
                                    )
                                    .with_hint("Reinstall the package or rebuild the library".to_string()),
                                );
                            }
                        }
                    }
                }
                None
            }
            Err(_) => None,
        }
    }
}

impl Default for DepthEngine {
    fn default() -> Self {
        Self::new(DepthArgs {
            permissions: true,
            symlinks: true,
            dylibs: true,
            paths: vec![
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/opt/homebrew"),
            ],
        })
    }
}
