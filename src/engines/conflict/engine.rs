use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::cli::args::ConflictArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{EngineId, EngineStats};

use super::env::EnvConflictScanner;
use super::path::PathConflictScanner;
use super::port::PortConflictScanner;

pub struct ConflictEngine {
    args: ConflictArgs,
}

impl ConflictEngine {
    pub fn new(args: ConflictArgs) -> Self {
        Self { args }
    }
}

#[async_trait]
impl Engine for ConflictEngine {
    fn id(&self) -> EngineId {
        EngineId::Conflict
    }

    fn name(&self) -> &'static str {
        "Conflict Engine"
    }

    fn description(&self) -> &'static str {
        "Detects conflicts: PATH, environment variables, and ports"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    /// op 76: Identify conflicting software — scan for PATH, environment,
    /// and port conflicts that indicate overlapping or conflicting
    /// software installations.
    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        if self.args.path {
            let scanner = PathConflictScanner::new();
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

        if self.args.env {
            let scanner = EnvConflictScanner::new();
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

        if self.args.ports {
            let scanner = PortConflictScanner::new(self.args.port_list.clone());
            let findings = scanner
                .scan(&ctx)
                .await
                .map_err(|e| EngineError::ScanFailed(e.to_string()))?;
            items_scanned += self.args.port_list.len() as u64;
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

impl Default for ConflictEngine {
    fn default() -> Self {
        Self::new(ConflictArgs {
            path: true,
            env: true,
            ports: true,
            port_list: vec![3000, 5000, 8000, 8080, 9000],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::super::env::EnvConflictScanner;
    use super::super::path::PathConflictScanner;
    use super::super::port::PortConflictScanner;
    use super::*;
    use crate::cli::args::ConflictArgs;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_conflict_engine_new_preserves_args() {
        let args = ConflictArgs {
            path: true,
            env: false,
            ports: false,
            port_list: vec![1234],
        };
        let engine = ConflictEngine::new(args);
        assert_eq!(engine.id(), EngineId::Conflict);
        assert_eq!(engine.name(), "Conflict Engine");
        assert!(!engine.description().is_empty());
    }

    #[test]
    fn test_conflict_engine_default_values() {
        let engine = ConflictEngine::default();
        assert_eq!(engine.id(), EngineId::Conflict);
        assert_eq!(engine.name(), "Conflict Engine");
        assert!(engine.description().contains("PATH"));
    }

    #[test]
    fn test_path_scanner_new_and_default() {
        let scanner = PathConflictScanner::new();
        let _ = scanner;
        let default = PathConflictScanner::default();
        let _ = default;
    }

    #[test]
    fn test_env_scanner_new_and_default() {
        let _ = EnvConflictScanner::new();
        let _ = EnvConflictScanner::default();
    }

    #[test]
    fn test_port_scanner_new_and_default() {
        let scanner = PortConflictScanner::new(vec![80, 443]);
        let _ = scanner;
        let default = PortConflictScanner::default();
        let _ = default;
    }

    #[test]
    fn test_parse_shell_config_bash_export() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join(".bashrc");
        fs::write(
            &cfg,
            "export PATH=/usr/local/bin:/usr/bin\nexport FOO=\"bar baz\"\n# a comment\n",
        )
        .unwrap();
        let vars = EnvConflictScanner::parse_shell_config(&cfg, "bash");
        assert_eq!(
            vars.get("PATH"),
            Some(&"/usr/local/bin:/usr/bin".to_string())
        );
        assert_eq!(vars.get("FOO"), Some(&"bar baz".to_string()));
    }

    #[test]
    fn test_parse_shell_config_zsh_without_export() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join(".zshrc");
        fs::write(&cfg, "PATH=/opt/homebrew/bin:$PATH\nEDITOR='vim'\n").unwrap();
        let vars = EnvConflictScanner::parse_shell_config(&cfg, "zsh");
        assert_eq!(
            vars.get("PATH"),
            Some(&"/opt/homebrew/bin:$PATH".to_string())
        );
        assert_eq!(vars.get("EDITOR"), Some(&"vim".to_string()));
    }

    #[test]
    fn test_parse_shell_config_fish_set_x() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("config.fish");
        fs::write(
            &cfg,
            "set -x PATH /usr/local/bin /usr/bin\nset -gx EDITOR vim\n",
        )
        .unwrap();
        let vars = EnvConflictScanner::parse_shell_config(&cfg, "fish");
        assert_eq!(
            vars.get("PATH"),
            Some(&"/usr/local/bin /usr/bin".to_string())
        );
        assert_eq!(vars.get("EDITOR"), Some(&"vim".to_string()));
    }

    #[test]
    fn test_parse_shell_config_nonexistent_returns_empty() {
        let missing = PathBuf::from("/this/does/not/exist/.bashrc");
        let vars = EnvConflictScanner::parse_shell_config(&missing, "bash");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_parse_shell_config_unknown_shell_returns_empty() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join("config");
        fs::write(&cfg, "PATH=/usr/bin\n").unwrap();
        let vars = EnvConflictScanner::parse_shell_config(&cfg, "powershell");
        assert!(vars.is_empty());
    }

    #[test]
    fn test_parse_shell_config_strips_quotes() {
        let dir = TempDir::new().unwrap();
        let cfg = dir.path().join(".bashrc");
        fs::write(&cfg, "export X='single'\nexport Y=\"double\"\n").unwrap();
        let vars = EnvConflictScanner::parse_shell_config(&cfg, "bash");
        assert_eq!(vars.get("X"), Some(&"single".to_string()));
        assert_eq!(vars.get("Y"), Some(&"double".to_string()));
    }
}
