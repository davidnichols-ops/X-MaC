use std::sync::Arc;
use tokio::sync::mpsc;

use crate::cli::args::Cli;
use crate::core::types::{Finding, ScanConfig};
use crate::util::macos::MacosUtils;
use crate::util::progress::ProgressReporter;

pub struct ScanContext {
    pub config: ScanConfig,
    pub tx: mpsc::Sender<Finding>,
    pub progress: Arc<ProgressReporter>,
    pub macos: Arc<MacosUtils>,
}

impl ScanContext {
    pub async fn new(cli: &Cli, tx: mpsc::Sender<Finding>) -> anyhow::Result<Self> {
        let config = ScanConfig {
            concurrency: cli.global.concurrency,
            include_hidden: cli.global.include_hidden,
            follow_symlinks: cli.global.follow_symlinks,
            exclude_patterns: cli.global.exclude.clone(),
            cache_dir: cli.global.cache_dir.clone(),
        };

        let progress = Arc::new(ProgressReporter::new(cli.global.quiet));
        let macos = Arc::new(MacosUtils::new());

        Ok(Self {
            config,
            tx,
            progress,
            macos,
        })
    }

    pub async fn emit(&self, finding: Finding) {
        if let Err(e) = self.tx.send(finding).await {
            tracing::error!("Failed to send finding: {}", e);
        }
    }
}

impl Clone for ScanContext {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            tx: self.tx.clone(),
            progress: Arc::clone(&self.progress),
            macos: Arc::clone(&self.macos),
        }
    }
}
