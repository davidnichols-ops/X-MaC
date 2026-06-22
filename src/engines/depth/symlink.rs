use std::path::PathBuf;
use walkdir::WalkDir;

use crate::core::context::ScanContext;
use crate::core::types::{Category, EngineId, Finding, Severity, Target};

pub struct SymlinkScanner {
    paths: Vec<PathBuf>,
}

impl SymlinkScanner {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    pub async fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();

        let target_paths = if self.paths.is_empty() {
            vec![
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/opt/homebrew/bin"),
                PathBuf::from("/opt/homebrew/lib"),
            ]
        } else {
            self.paths.clone()
        };

        for target_path in target_paths {
            if !target_path.exists() {
                continue;
            }

            let entries = WalkDir::new(&target_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok());

            for entry in entries {
                let path = entry.path().to_path_buf();

                if Self::is_broken_symlink(&path) {
                    let target = Self::resolve_symlink(&path);

                    findings.push(
                        Finding::new(
                            EngineId::Depth,
                            Severity::Medium,
                            Category::BrokenSymlink,
                            Target::Path(path.clone()),
                            "Broken symlink detected",
                            format!(
                                "Symlink points to non-existent target: {}",
                                target.as_ref().map(|t| t.to_string_lossy().to_string()).unwrap_or_else(|| "unknown".to_string())
                            ),
                        )
                        .with_metadata(
                            "target".to_string(),
                            serde_json::json!(target.as_ref().map(|t| t.to_string_lossy().to_string())),
                        )
                        .with_hint("Consider removing this broken symlink".to_string()),
                    );
                }
            }
        }

        Ok(findings)
    }

    pub fn is_broken_symlink(path: &PathBuf) -> bool {
        use std::fs;

        match fs::symlink_metadata(path) {
            Ok(metadata) => {
                if metadata.file_type().is_symlink() {
                    fs::read_link(path)
                        .map(|target| {
                            // read_link returns the raw target. If it's
                            // relative, it must be resolved against the
                            // symlink's parent directory — NOT the CWD.
                            let resolved = if target.is_absolute() {
                                target
                            } else {
                                path.parent()
                                    .unwrap_or(std::path::Path::new(""))
                                    .join(target)
                            };
                            !resolved.exists()
                        })
                        .unwrap_or(true)
                } else {
                    false
                }
            }
            Err(_) => false,
        }
    }

    pub fn resolve_symlink(path: &PathBuf) -> Option<PathBuf> {
        std::fs::read_link(path).ok()
    }
}

impl Default for SymlinkScanner {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
