// ObserverRunner — orchestrates multiple observers concurrently
//
// Spawns the process observer (poll-based) and filesystem observer
// (event-based) as concurrent tasks. The runner continues until:
//   - The duration elapses (if specified)
//   - A shutdown signal is received (Ctrl-C)
//
// Usage:
//   let runner = ObserverRunner::new(db_handle);
//   runner.run_for(Duration::from_secs(60)).await?;

use crate::twin::database::event_store::EventStore;
use crate::twin::database::TwinDbHandle;
use crate::twin::observers::filesystem_observer::{FilesystemObserver, FilesystemObserverConfig};
use crate::twin::observers::process_observer::{ProcessObserver, ProcessObserverConfig};
use anyhow::Result;
use serde::Serialize;
use std::time::Duration;
use tokio::time::interval;

pub struct ObserverRunner {
    db: TwinDbHandle,
    process_config: ProcessObserverConfig,
    filesystem_config: FilesystemObserverConfig,
}

impl ObserverRunner {
    pub fn new(db: TwinDbHandle) -> Self {
        Self {
            db,
            process_config: ProcessObserverConfig::default(),
            filesystem_config: FilesystemObserverConfig::default(),
        }
    }

    pub fn with_process_config(mut self, config: ProcessObserverConfig) -> Self {
        self.process_config = config;
        self
    }

    pub fn with_filesystem_config(mut self, config: FilesystemObserverConfig) -> Self {
        self.filesystem_config = config;
        self
    }

    /// Run observers for a fixed duration, then stop.
    /// Prints progress to stderr.
    pub async fn run_for(&self, duration: Duration) -> Result<ObserverStats> {
        let store = EventStore::new(self.db.clone());
        let mut process_observer = ProcessObserver::new(self.process_config.clone());
        let fs_observer = FilesystemObserver::new(self.filesystem_config.clone())?;

        let mut stats = ObserverStats::default();
        let mut tick = interval(Duration::from_secs(1));
        let mut fs_drain_tick = interval(Duration::from_millis(500));

        let start = std::time::Instant::now();

        loop {
            tokio::select! {
                _ = tick.tick() => {
                    let elapsed = start.elapsed();
                    if elapsed >= duration {
                        eprintln!("\nObserver run complete: {}s elapsed.", elapsed.as_secs());
                        break;
                    }
                    // Progress output.
                    eprint!(
                        "\rObserving... {}s / {}s (events: {})",
                        elapsed.as_secs(),
                        duration.as_secs(),
                        stats.total_events
                    );
                }
                _ = fs_drain_tick.tick() => {
                    // Drain filesystem events.
                    let count = fs_observer.drain_and_store(&store).await.unwrap_or(0);
                    stats.fs_events += count;
                    stats.total_events += count;
                }
            }

            // Check if it's time for a process poll.
            // We use a simple approach: poll every N seconds based on config.
            // Since tick fires every 1s, we count cycles.
            // For simplicity, we poll on every tick where elapsed % poll_interval == 0.
            let elapsed_secs = start.elapsed().as_secs();
            let poll_secs = process_observer.poll_interval().as_secs();
            if poll_secs > 0
                && elapsed_secs > 0
                && elapsed_secs.is_multiple_of(poll_secs)
                && stats.last_process_poll_secs < elapsed_secs
            {
                let count = process_observer.observe_cycle(&store).await.unwrap_or(0);
                stats.process_events += count;
                stats.total_events += count;
                stats.last_process_poll_secs = elapsed_secs;
                stats.poll_cycles += 1;
            }
        }

        // Final drain of filesystem events.
        let final_count = fs_observer.drain_and_store(&store).await.unwrap_or(0);
        stats.fs_events += final_count;
        stats.total_events += final_count;

        eprintln!();
        Ok(stats)
    }

    /// Run observers indefinitely until Ctrl-C.
    pub async fn run_until_signal(&self) -> Result<ObserverStats> {
        let store = EventStore::new(self.db.clone());
        let mut process_observer = ProcessObserver::new(self.process_config.clone());
        let fs_observer = FilesystemObserver::new(self.filesystem_config.clone())?;

        let mut stats = ObserverStats::default();
        let mut tick = interval(Duration::from_secs(1));
        let mut fs_drain_tick = interval(Duration::from_millis(500));

        let start = std::time::Instant::now();
        let mut shutdown = false;

        // Set up Ctrl-C handler.
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::mpsc::channel::<()>(1);
        tokio::spawn(async move {
            let _ = tokio::signal::ctrl_c().await;
            let _ = shutdown_tx.send(()).await;
        });

        eprintln!("Observing... (Ctrl-C to stop)");

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    shutdown = true;
                }
                _ = tick.tick() => {
                    if shutdown {
                        eprintln!("\nShutting down observers...");
                        break;
                    }
                    let elapsed = start.elapsed();
                    eprint!(
                        "\rObserving... {}s (events: {})",
                        elapsed.as_secs(),
                        stats.total_events
                    );
                }
                _ = fs_drain_tick.tick() => {
                    let count = fs_observer.drain_and_store(&store).await.unwrap_or(0);
                    stats.fs_events += count;
                    stats.total_events += count;
                }
            }

            // Process poll.
            let elapsed_secs = start.elapsed().as_secs();
            let poll_secs = process_observer.poll_interval().as_secs();
            if poll_secs > 0
                && elapsed_secs > 0
                && elapsed_secs.is_multiple_of(poll_secs)
                && stats.last_process_poll_secs < elapsed_secs
            {
                let count = process_observer.observe_cycle(&store).await.unwrap_or(0);
                stats.process_events += count;
                stats.total_events += count;
                stats.last_process_poll_secs = elapsed_secs;
                stats.poll_cycles += 1;
            }
        }

        // Final drain.
        let final_count = fs_observer.drain_and_store(&store).await.unwrap_or(0);
        stats.fs_events += final_count;
        stats.total_events += final_count;

        eprintln!();
        Ok(stats)
    }
}

/// Statistics from an observer run.
#[derive(Debug, Clone, Default, Serialize)]
pub struct ObserverStats {
    pub total_events: usize,
    pub process_events: usize,
    pub fs_events: usize,
    pub poll_cycles: u64,
    last_process_poll_secs: u64,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twin::database::TwinDb;

    #[tokio::test]
    async fn test_run_for_6_seconds() {
        let db = TwinDb::open_memory().unwrap();
        let runner = ObserverRunner::new(db.handle());

        // Run for 6 seconds with a 1s poll interval to ensure at least one poll.
        let tmp = std::env::temp_dir().join("xmac_runner_test");
        std::fs::create_dir_all(&tmp).unwrap();

        let runner = runner
            .with_process_config(ProcessObserverConfig {
                poll_interval: Duration::from_secs(1),
                ..Default::default()
            })
            .with_filesystem_config(FilesystemObserverConfig {
                watch_paths: vec![tmp.clone()],
                exclude_paths: vec![],
                ..Default::default()
            });

        let stats = runner.run_for(Duration::from_secs(6)).await.unwrap();
        assert!(stats.total_events > 0, "should emit some process events");
        assert!(stats.poll_cycles >= 1, "should have at least 1 poll cycle");

        let _ = std::fs::remove_dir_all(&tmp);
    }
}
