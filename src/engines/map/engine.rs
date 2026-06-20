use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::cli::args::MapArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{EngineId, EngineStats};

use super::python::PythonScanner;
use super::nodejs::NodejsScanner;
use super::containers::ContainerScanner;

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
            let findings = scanner.scan(&ctx).await.map_err(|e| EngineError::ScanFailed(e.to_string()))?;
            items_scanned += findings.len() as u64;
            findings_count += findings.len() as u64;
            for finding in findings {
                ctx.emit(finding).await;
            }
        }

        if self.args.nodejs {
            let scanner = NodejsScanner::new(self.args.disk_usage, self.args.paths.clone());
            let findings = scanner.scan(&ctx).await.map_err(|e| EngineError::ScanFailed(e.to_string()))?;
            items_scanned += findings.len() as u64;
            findings_count += findings.len() as u64;
            for finding in findings {
                ctx.emit(finding).await;
            }
        }

        if self.args.containers {
            let scanner = ContainerScanner::new();
            let findings = scanner.scan(&ctx).await.map_err(|e| EngineError::ScanFailed(e.to_string()))?;
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
