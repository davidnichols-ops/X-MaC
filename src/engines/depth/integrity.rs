use std::path::PathBuf;
use walkdir::WalkDir;

use crate::core::context::ScanContext;
use crate::core::types::{Category, EngineId, Finding, Severity, Target};

pub struct IntegrityScanner {
    paths: Vec<PathBuf>,
}

impl IntegrityScanner {
    pub fn new(paths: Vec<PathBuf>) -> Self {
        Self { paths }
    }

    pub async fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();

        let target_paths = if self.paths.is_empty() {
            vec![
                PathBuf::from("/usr/local/bin"),
                PathBuf::from("/usr/local/lib"),
                PathBuf::from("/opt/homebrew"),
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

                if let Some(finding) = Self::check_permissions(&path) {
                    findings.push(finding);
                }
            }
        }

        Ok(findings)
    }

    pub fn check_permissions(path: &PathBuf) -> Option<Finding> {
        use std::os::unix::fs::PermissionsExt;

        let metadata = match std::fs::metadata(path) {
            Ok(m) => m,
            Err(_) => return None,
        };

        let mode = metadata.permissions().mode();

        if mode & 0o002 != 0 {
            return Some(
                Finding::new(
                    EngineId::Depth,
                    Severity::High,
                    Category::PermissionIssue,
                    Target::Path(path.clone()),
                    "World-writable file or directory",
                    format!("{} is world-writable (permissions: {:o})", path.display(), mode),
                )
                .with_hint("World-writable files can be a security risk. Consider removing world-write permission.".to_string()),
            );
        }

        if mode & 0o4000 != 0 {
            return Some(
                Finding::new(
                    EngineId::Depth,
                    Severity::High,
                    Category::PermissionIssue,
                    Target::Path(path.clone()),
                    "SUID binary detected",
                    format!("{} has SUID bit set (permissions: {:o})", path.display(), mode),
                )
                .with_metadata("suid".to_string(), serde_json::json!(true))
                .with_hint("SUID binaries can be a security risk if not properly maintained.".to_string()),
            );
        }

        if mode & 0o2000 != 0 {
            return Some(
                Finding::new(
                    EngineId::Depth,
                    Severity::High,
                    Category::PermissionIssue,
                    Target::Path(path.clone()),
                    "SGID binary detected",
                    format!("{} has SGID bit set (permissions: {:o})", path.display(), mode),
                )
                .with_metadata("sgid".to_string(), serde_json::json!(true))
                .with_hint("SGID binaries can be a security risk if not properly maintained.".to_string()),
            );
        }

        None
    }
}

impl Default for IntegrityScanner {
    fn default() -> Self {
        Self::new(Vec::new())
    }
}
