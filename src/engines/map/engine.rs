use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::cli::args::MapArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{EngineId, EngineStats};

use super::containers::ContainerScanner;
use super::nodejs::NodejsScanner;
use super::python::PythonScanner;

pub struct MapEngine {
    args: MapArgs,
}

impl MapEngine {
    pub fn new(args: MapArgs) -> Self {
        Self { args }
    }
}

#[async_trait]
impl Engine for MapEngine {
    fn id(&self) -> EngineId {
        EngineId::Map
    }

    fn name(&self) -> &'static str {
        "Map Engine"
    }

    fn description(&self) -> &'static str {
        "Maps runtime environments: Python, Node.js, containers"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        if self.args.python {
            let scanner = PythonScanner::new(self.args.disk_usage, self.args.paths.clone());
            let findings = scanner
                .scan(&ctx)
                .await
                .map_err(|e| EngineError::ScanFailed(e.to_string()))?;
            items_scanned += findings.len() as u64;
            findings_count += findings.len() as u64;
            for finding in findings {
                ctx.emit(finding).await;
            }
        }

        if self.args.nodejs {
            let scanner = NodejsScanner::new(self.args.disk_usage, self.args.paths.clone());
            let findings = scanner
                .scan(&ctx)
                .await
                .map_err(|e| EngineError::ScanFailed(e.to_string()))?;
            items_scanned += findings.len() as u64;
            findings_count += findings.len() as u64;
            for finding in findings {
                ctx.emit(finding).await;
            }
        }

        if self.args.containers {
            let scanner = ContainerScanner::new();
            let findings = scanner
                .scan(&ctx)
                .await
                .map_err(|e| EngineError::ScanFailed(e.to_string()))?;
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

impl Default for MapEngine {
    fn default() -> Self {
        Self::new(MapArgs {
            python: true,
            nodejs: true,
            containers: true,
            disk_usage: true,
            paths: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::nodejs::NodejsScanner;
    use super::super::python::PythonScanner;
    use super::*;
    use crate::cli::args::MapArgs;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_map_engine_new_preserves_args() {
        let args = MapArgs {
            python: false,
            nodejs: true,
            containers: false,
            disk_usage: false,
            paths: vec![PathBuf::from("/tmp")],
        };
        let engine = MapEngine::new(args);
        assert_eq!(engine.id(), EngineId::Map);
        assert_eq!(engine.name(), "Map Engine");
        assert!(!engine.description().is_empty());
    }

    #[test]
    fn test_map_engine_default_values() {
        let engine = MapEngine::default();
        assert_eq!(engine.id(), EngineId::Map);
        assert_eq!(engine.name(), "Map Engine");
        assert!(engine.description().contains("Python"));
        assert!(engine.description().contains("Node.js"));
    }

    #[test]
    fn test_python_scanner_default_constructs() {
        let scanner = PythonScanner::default();
        // Should construct without panic; scan is async so we only verify
        // the public detection helpers here.
        let _ = scanner;
    }

    #[test]
    fn test_detect_env_type_venv_via_pyvenv_cfg() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pyvenv.cfg"), "home = /usr/bin\n").unwrap();
        assert_eq!(PythonScanner::detect_env_type(dir.path()), Some("venv"));
    }

    #[test]
    fn test_detect_env_type_conda() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join("conda-meta")).unwrap();
        assert_eq!(PythonScanner::detect_env_type(dir.path()), Some("conda"));
    }

    #[test]
    fn test_detect_env_type_poetry() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("poetry.lock"), "").unwrap();
        assert_eq!(PythonScanner::detect_env_type(dir.path()), Some("poetry"));
    }

    #[test]
    fn test_detect_env_type_pipenv() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("Pipfile"), "").unwrap();
        assert_eq!(PythonScanner::detect_env_type(dir.path()), Some("pipenv"));
    }

    #[test]
    fn test_detect_env_type_uv() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("uv.lock"), "").unwrap();
        assert_eq!(PythonScanner::detect_env_type(dir.path()), Some("uv"));
    }

    #[test]
    fn test_detect_env_type_none_for_empty_dir() {
        let dir = TempDir::new().unwrap();
        assert_eq!(PythonScanner::detect_env_type(dir.path()), None);
    }

    #[test]
    fn test_detect_env_type_venv_via_dotvenv_dir() {
        let dir = TempDir::new().unwrap();
        fs::create_dir(dir.path().join(".venv")).unwrap();
        assert_eq!(PythonScanner::detect_env_type(dir.path()), Some("venv"));
    }

    #[test]
    fn test_get_python_version_from_pyvenv_cfg() {
        let dir = TempDir::new().unwrap();
        fs::write(
            dir.path().join("pyvenv.cfg"),
            "home = /usr/bin\nversion = 3.11.4\n",
        )
        .unwrap();
        assert_eq!(
            PythonScanner::get_python_version(dir.path()),
            Some("3.11.4".to_string())
        );
    }

    #[test]
    fn test_get_python_version_none_when_no_indicators() {
        let dir = TempDir::new().unwrap();
        assert_eq!(PythonScanner::get_python_version(dir.path()), None);
    }

    #[test]
    fn test_nodejs_scanner_default_constructs() {
        let _ = NodejsScanner::default();
    }

    #[test]
    fn test_detect_package_manager_pnpm() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        assert_eq!(
            NodejsScanner::detect_package_manager(dir.path()),
            Some("pnpm")
        );
    }

    #[test]
    fn test_detect_package_manager_yarn() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("yarn.lock"), "").unwrap();
        assert_eq!(
            NodejsScanner::detect_package_manager(dir.path()),
            Some("yarn")
        );
    }

    #[test]
    fn test_detect_package_manager_npm() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package-lock.json"), "").unwrap();
        assert_eq!(
            NodejsScanner::detect_package_manager(dir.path()),
            Some("npm")
        );
    }

    #[test]
    fn test_detect_package_manager_none() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("package.json"), "{}").unwrap();
        assert_eq!(NodejsScanner::detect_package_manager(dir.path()), None);
    }

    #[test]
    fn test_detect_package_manager_pnpm_takes_priority() {
        // pnpm is checked first in detect_package_manager.
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("pnpm-lock.yaml"), "").unwrap();
        fs::write(dir.path().join("package-lock.json"), "").unwrap();
        assert_eq!(
            NodejsScanner::detect_package_manager(dir.path()),
            Some("pnpm")
        );
    }
}
