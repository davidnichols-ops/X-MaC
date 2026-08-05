use std::sync::atomic::{AtomicBool, Ordering};
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
    /// Set to true when SIGINT / SIGTERM is received. Engines should poll
    /// this via `ctx.is_cancelled()` and abort cleanly. See MAOS #151.
    pub cancelled: Arc<AtomicBool>,
}

impl ScanContext {
    pub async fn new(cli: &Cli, tx: mpsc::Sender<Finding>) -> anyhow::Result<Self> {
        let config = ScanConfig {
            concurrency: cli.global.concurrency,
            include_hidden: cli.global.include_hidden,
            follow_symlinks: cli.global.follow_symlinks,
            exclude_patterns: cli.global.exclude.clone(),
            cache_dir: cli.global.cache_dir.clone(),
            resource_mode: cli.global.resource_mode.clone(),
        };

        let progress = Arc::new(ProgressReporter::new(cli.global.quiet));
        let macos = Arc::new(MacosUtils::new());

        Ok(Self {
            config,
            tx,
            progress,
            macos,
            cancelled: Arc::new(AtomicBool::new(false)),
        })
    }

    pub async fn emit(&self, finding: Finding) {
        if let Err(e) = self.tx.send(finding).await {
            tracing::error!("Failed to send finding: {}", e);
        }
    }

    /// Returns true if the scan has been cancelled (SIGINT / SIGTERM).
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::Relaxed)
    }

    /// Request cancellation. Engines polling `is_cancelled()` will see it
    /// on their next check and unwind cleanly.
    #[allow(dead_code)]
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::Relaxed);
    }
}

impl Clone for ScanContext {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            tx: self.tx.clone(),
            progress: Arc::clone(&self.progress),
            macos: Arc::clone(&self.macos),
            cancelled: Arc::clone(&self.cancelled),
        }
    }
}
