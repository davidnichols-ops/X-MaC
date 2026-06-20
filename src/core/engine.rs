use async_trait::async_trait;
use std::sync::Arc;

use crate::core::context::ScanContext;
use crate::core::error::EngineError;
use crate::core::types::{EngineId, EngineStats};

#[async_trait]
#[allow(dead_code)]
pub trait Engine: Send + Sync {
    fn id(&self) -> EngineId;

    fn name(&self) -> &'static str;

    fn description(&self) -> &'static str;

    async fn validate(&self, ctx: &ScanContext) -> std::result::Result<(), EngineError>;

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError>;

    async fn run(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        self.validate(&ctx).await?;
        self.scan(ctx).await
    }
}
