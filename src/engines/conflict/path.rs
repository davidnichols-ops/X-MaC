use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use crate::core::context::ScanContext;
use crate::core::types::{Category, EngineId, Finding, Severity, Target};

pub struct PathConflictScanner;

impl PathConflictScanner {
    pub fn new() -> Self {
        Self
    }

    pub async fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        let mut binary_map: HashMap<String, Vec<PathBuf>> = HashMap::new();

        let path_dirs = Self::parse_path();

        for dir in &path_dirs {
            if !dir.exists() {
                continue;
            }

            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }

                    if Self::is_executable(&path) {
                        let binary_name = path.file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_default();

                        binary_map.entry(binary_name).or_default().push(path);
                    }
                }
            }
        }

        for (binary_name, paths) in binary_map {
            if paths.len() > 1 {
                let path_strings: Vec<String> = paths.iter()
                    .map(|p| p.to_string_lossy().to_string())
                    .collect();

                findings.push(
                    Finding::new(
                        EngineId::Conflict,
                        Severity::Medium,
                        Category::PathConflict,
                        Target::Path(paths[0].clone()),
                        format!("Duplicate binary: {}", binary_name),
                        format!("Found {} at: {}", binary_name, path_strings.join(", ")),
                    )
                    .with_metadata("paths".to_string(), serde_json::json!(path_strings))
                    .with_hint(format!("The first occurrence in PATH will be used. Consider removing duplicates.")),
                );
            }
        }

        Ok(findings)
    }

    fn parse_path() -> Vec<PathBuf> {
        std::env::var("PATH")
            .unwrap_or_default()
            .split(':')
            .map(PathBuf::from)
            .collect()
    }

    fn is_executable(path: &PathBuf) -> bool {
        use std::os::unix::fs::PermissionsExt;

        if let Ok(metadata) = std::fs::metadata(path) {
            let mode = metadata.permissions().mode();
            mode & 0o111 != 0
        } else {
            false
        }
    }

}

impl Default for PathConflictScanner {
    fn default() -> Self {
        Self::new()
    }
}
