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
