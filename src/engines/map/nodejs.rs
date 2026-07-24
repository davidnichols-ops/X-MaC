use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::core::context::ScanContext;
use crate::core::types::{Category, EngineId, Finding, Severity, Target};
use crate::util::disk;
use crate::util::macos::MacosUtils;

pub struct NodejsScanner {
    include_disk_usage: bool,
    user_paths: Vec<PathBuf>,
}

impl NodejsScanner {
    pub fn new(include_disk_usage: bool, user_paths: Vec<PathBuf>) -> Self {
        Self {
            include_disk_usage,
            user_paths,
        }
    }

    /// op 294: Manage npm caches — scan for Node.js projects, nvm
    /// installations, and global Node.js installs, reporting package
    /// managers and node_modules disk usage.
    pub async fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        let search_paths = self.get_search_paths();

        for search_path in &search_paths {
            if !search_path.exists() {
                continue;
            }

            let entries = WalkDir::new(search_path)
                .max_depth(3)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir());

            for entry in entries {
                let dir_path = entry.path().to_path_buf();

                if dir_path.join("package.json").exists() {
                    let package_manager = Self::detect_package_manager(&dir_path);
                    let node_version =
                        Self::get_node_version(&dir_path).unwrap_or_else(|| "unknown".to_string());
                    let size = if self.include_disk_usage {
                        disk::dir_size(&dir_path.join("node_modules"))
                    } else {
                        0
                    };

                    let package_name =
                        Self::get_package_name(&dir_path).unwrap_or_else(|| "unknown".to_string());

                    findings.push(
                        Finding::new(
                            EngineId::Map,
                            Severity::Info,
                            Category::NodeEnv,
                            Target::Path(dir_path.clone()),
                            format!(
                                "Node.js project: {} ({})",
                                package_name,
                                package_manager.unwrap_or("npm")
                            ),
                            format!(
                                "Found Node.js project using {}",
                                package_manager.unwrap_or("npm")
                            ),
                        )
                        .with_size(size)
                        .with_metadata(
                            "package_manager".to_string(),
                            serde_json::json!(package_manager),
                        )
                        .with_metadata("node_version".to_string(), serde_json::json!(node_version))
                        .with_metadata("package_name".to_string(), serde_json::json!(package_name)),
                    );
                }
            }
        }

        if let Some(nvm_versions) = Self::scan_nvm() {
            for (version, path) in nvm_versions {
                findings.push(
                    Finding::new(
                        EngineId::Map,
                        Severity::Info,
                        Category::NodeEnv,
                        Target::Path(path.clone()),
                        format!("nvm Node.js version: {}", version),
                        format!("Found Node.js {} installed via nvm", version),
                    )
                    .with_metadata("version_manager".to_string(), serde_json::json!("nvm")),
                );
            }
        }

        if let Some(global_node) = Self::find_global_node() {
            findings.push(
                Finding::new(
                    EngineId::Map,
                    Severity::Info,
                    Category::NodeEnv,
                    Target::Path(global_node.clone()),
                    "Global Node.js installation",
                    format!("Found Node.js at {}", global_node.display()),
                )
                .with_metadata("is_global".to_string(), serde_json::json!(true)),
            );
        }

        Ok(findings)
    }

    fn get_search_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        let mut paths = vec![
            home.join("Projects"),
            home.join("dev"),
            home.join("Documents"),
            home.join("node_modules"),
        ];
        paths.extend(self.user_paths.iter().cloned());
        paths
    }

    pub fn detect_package_manager(path: &Path) -> Option<&'static str> {
        if path.join("pnpm-lock.yaml").exists() {
            Some("pnpm")
        } else if path.join("yarn.lock").exists() {
            Some("yarn")
        } else if path.join("package-lock.json").exists() {
            Some("npm")
        } else {
            None
        }
    }

    fn get_node_version(path: &Path) -> Option<String> {
        let nvmrc = path.join(".nvmrc");
        if nvmrc.exists() {
            if let Ok(version) = std::fs::read_to_string(&nvmrc) {
                return Some(version.trim().to_string());
            }
        }

        let node_version = path.join(".node-version");
        if node_version.exists() {
            if let Ok(version) = std::fs::read_to_string(&node_version) {
                return Some(version.trim().to_string());
            }
        }

        None
    }

    fn get_package_name(path: &Path) -> Option<String> {
        let package_json = path.join("package.json");
        if package_json.exists() {
            if let Ok(content) = std::fs::read_to_string(&package_json) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(name) = json.get("name").and_then(|v| v.as_str()) {
                        return Some(name.to_string());
                    }
                }
            }
        }
        None
    }

    fn scan_nvm() -> Option<Vec<(String, PathBuf)>> {
        let home = MacosUtils::home_dir();
        let nvm_dir = home.join(".nvm/versions/node");

        if !nvm_dir.exists() {
            return None;
        }

        let mut versions = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&nvm_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(dir_name) = path.file_name() {
                        let version = dir_name.to_string_lossy().to_string();
                        if version.starts_with("v") {
                            versions.push((version, path));
                        }
                    }
                }
            }
        }

        if versions.is_empty() {
            None
        } else {
            Some(versions)
        }
    }

    fn find_global_node() -> Option<PathBuf> {
        let candidates = vec![
            PathBuf::from("/usr/local/bin/node"),
            PathBuf::from("/opt/homebrew/bin/node"),
        ];

        candidates.into_iter().find(|candidate| candidate.exists())
    }
}

impl Default for NodejsScanner {
    fn default() -> Self {
        Self::new(true, Vec::new())
    }
}
