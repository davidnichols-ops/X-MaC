use std::path::PathBuf;
use std::process::Command;
use walkdir::WalkDir;

use crate::core::context::ScanContext;
use crate::core::types::{Category, EngineId, Finding, Severity, Target};
use crate::util::macos::MacosUtils;
use crate::util::disk;

pub struct PythonScanner {
    include_disk_usage: bool,
    user_paths: Vec<PathBuf>,
}

impl PythonScanner {
    pub fn new(include_disk_usage: bool, user_paths: Vec<PathBuf>) -> Self {
        Self { include_disk_usage, user_paths }
    }

    pub async fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        let search_paths = self.get_search_paths();

        for search_path in search_paths {
            if !search_path.exists() {
                continue;
            }

            let entries = WalkDir::new(&search_path)
                .max_depth(3)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir());

            for entry in entries {
                let dir_path = entry.path().to_path_buf();

                if let Some(env_type) = Self::detect_env_type(&dir_path) {
                    let version = Self::get_python_version(&dir_path).unwrap_or_else(|| "unknown".to_string());
                    let size = if self.include_disk_usage {
                        disk::dir_size(&dir_path)
                    } else {
                        0
                    };

                    findings.push(
                        Finding::new(
                            EngineId::Map,
                            Severity::Info,
                            Category::PythonEnv,
                            Target::Path(dir_path.clone()),
                            format!("Python {} environment ({})", version, env_type),
                            format!("Found {} environment at {}", env_type, dir_path.display()),
                        )
                        .with_size(size)
                        .with_metadata("env_type".to_string(), serde_json::json!(env_type))
                        .with_metadata("python_version".to_string(), serde_json::json!(version)),
                    );
                }
            }
        }

        if let Some(global_python) = Self::find_global_python() {
            findings.push(
                Finding::new(
                    EngineId::Map,
                    Severity::Info,
                    Category::PythonEnv,
                    Target::Path(global_python.clone()),
                    "Global Python installation",
                    format!("Found Python at {}", global_python.display()),
                )
                .with_metadata("is_global".to_string(), serde_json::json!(true)),
            );
        }

        Ok(findings)
    }

    fn get_search_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        let mut paths = vec![
            home.join(".pyenv/versions"),
            home.join("Library/Caches/pypoetry"),
            home.join("Library/Application Support/pypoetry"),
            home.join(".local/share/uv"),
            home.join("Projects"),
            home.join("dev"),
        ];
        paths.extend(self.user_paths.iter().cloned());
        paths
    }

    pub fn detect_env_type(path: &PathBuf) -> Option<&'static str> {
        if path.join("pyvenv.cfg").exists() {
            Some("venv")
        } else if path.join("conda-meta").exists() {
            Some("conda")
        } else if path.join("poetry.lock").exists() {
            Some("poetry")
        } else if path.join("Pipfile").exists() {
            Some("pipenv")
        } else if path.join("uv.lock").exists() {
            Some("uv")
        } else if path.join(".venv").exists() || path.file_name().map(|n| n.to_string_lossy().contains("venv")).unwrap_or(false) {
            Some("venv")
        } else {
            None
        }
    }

    pub fn get_python_version(path: &PathBuf) -> Option<String> {
        let python_bin = path.join("bin/python");

        if python_bin.exists() {
            let output = Command::new(&python_bin)
                .arg("--version")
                .output()
                .ok()?;

            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if version.starts_with("Python ") {
                return Some(version.trim_start_matches("Python ").to_string());
            }
        }

        let pyvenv_cfg = path.join("pyvenv.cfg");
        if pyvenv_cfg.exists() {
            if let Ok(content) = std::fs::read_to_string(&pyvenv_cfg) {
                for line in content.lines() {
                    if line.starts_with("version = ") {
                        return Some(line.trim_start_matches("version = ").to_string());
                    }
                }
            }
        }

        None
    }

    fn find_global_python() -> Option<PathBuf> {
        let candidates = vec![
            PathBuf::from("/usr/bin/python3"),
            PathBuf::from("/usr/local/bin/python3"),
            PathBuf::from("/opt/homebrew/bin/python3"),
        ];

        for candidate in candidates {
            if candidate.exists() {
                return Some(candidate);
            }
        }

        None
    }
}

impl Default for PythonScanner {
    fn default() -> Self {
        Self::new(true, Vec::new())
    }
}
