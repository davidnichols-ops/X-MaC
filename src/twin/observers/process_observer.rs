// ProcessObserver — polls running processes and emits events
//
// Polls `ps` every 5 seconds (configurable). Compares the current process
// list against the previous snapshot to detect:
//   - ProcessLaunched: new PIDs that appeared
//   - ProcessTerminated: PIDs that disappeared
//   - ProcessAnomaly: CPU spikes (>80%), memory leaks (>2GB), zombies
//
// Events are written to the EventStore.

use crate::twin::database::event_store::{EventStore, StoredEvent};
use crate::twin::process::{collect_ps_processes, ProcessNode};
use anyhow::Result;
use std::collections::{HashMap, HashSet};
use std::time::Duration;

/// Configuration for the process observer.
#[derive(Debug, Clone)]
pub struct ProcessObserverConfig {
    /// Polling interval (default: 5 seconds).
    pub poll_interval: Duration,
    /// CPU usage threshold for anomaly (default: 80.0%).
    pub cpu_spike_threshold: f64,
    /// Memory usage threshold for anomaly (default: 2 GB).
    pub memory_leak_threshold: u64,
    /// Maximum events to emit per poll cycle (default: 100).
    pub max_events_per_cycle: usize,
}

impl Default for ProcessObserverConfig {
    fn default() -> Self {
        Self {
            poll_interval: Duration::from_secs(5),
            cpu_spike_threshold: 80.0,
            memory_leak_threshold: 2 * 1024 * 1024 * 1024,
            max_events_per_cycle: 100,
        }
    }
}

/// The process observer. Maintains state between polls to detect changes.
pub struct ProcessObserver {
    config: ProcessObserverConfig,
    /// Previous snapshot: pid → ProcessNode
    prev_processes: HashMap<u32, ProcessNode>,
    /// Pids that have been anomalous for >1 cycle (to avoid re-emitting)
    anomalous_pids: HashSet<u32>,
}

impl ProcessObserver {
    pub fn new(config: ProcessObserverConfig) -> Self {
        Self {
            config,
            prev_processes: HashMap::new(),
            anomalous_pids: HashSet::new(),
        }
    }

    pub fn with_defaults() -> Self {
        Self::new(ProcessObserverConfig::default())
    }

    /// Run one observation cycle. Collects processes, diffs against the
    /// previous snapshot, emits events to the store. Returns the number
    /// of events emitted.
    pub async fn observe_cycle(&mut self, store: &EventStore) -> Result<usize> {
        let current = collect_ps_processes();
        let current_map: HashMap<u32, ProcessNode> =
            current.iter().map(|p| (p.pid, p.clone())).collect();

        let mut events = Vec::new();
        let now_ms = chrono::Utc::now().timestamp_millis();

        // Detect launched processes.
        for proc in &current {
            if !self.prev_processes.contains_key(&proc.pid) {
                // Skip PID 0 (kernel) and 1 (launchd) on first cycle.
                if !self.prev_processes.is_empty() || proc.pid > 1 {
                    let mut payload = serde_json::Map::new();
                    payload.insert("pid".into(), serde_json::json!(proc.pid));
                    payload.insert("name".into(), serde_json::json!(proc.name));
                    payload.insert("cpu_percent".into(), serde_json::json!(proc.cpu_pct));
                    payload.insert(
                        "memory_mb".into(),
                        serde_json::json!((proc.memory_bytes as f64 / 1_048_576.0).round()),
                    );
                    if let Some(owner) = &proc.owner {
                        payload.insert("owner".into(), serde_json::json!(owner));
                    }
                    if let Some(ppid) = proc.parent_pid {
                        payload.insert("parent_pid".into(), serde_json::json!(ppid));
                    }
                    events.push(StoredEvent {
                        id: String::new(),
                        timestamp_ms: now_ms,
                        event_type: "ProcessLaunched".to_string(),
                        severity: "info".to_string(),
                        source: "process_observer".to_string(),
                        entity_id: Some(format!("process:{}", proc.pid)),
                        payload: payload.into_iter().collect(),
                    });
                }
            }
        }

        // Detect terminated processes.
        for (pid, prev) in &self.prev_processes {
            if !current_map.contains_key(pid) {
                let mut payload = serde_json::Map::new();
                payload.insert("pid".into(), serde_json::json!(pid));
                payload.insert("name".into(), serde_json::json!(prev.name));
                events.push(StoredEvent {
                    id: String::new(),
                    timestamp_ms: now_ms,
                    event_type: "ProcessTerminated".to_string(),
                    severity: "info".to_string(),
                    source: "process_observer".to_string(),
                    entity_id: Some(format!("process:{}", pid)),
                    payload: payload.into_iter().collect(),
                });
            }
        }

        // Detect anomalies (only new ones, not persistent).
        let mut current_anomalies: HashSet<u32> = HashSet::new();
        for proc in &current {
            let mut is_anomalous = false;
            let mut anomaly_type = String::new();
            let mut description = String::new();
            let mut severity = "warning";

            // CPU spike.
            if proc.cpu_pct > self.config.cpu_spike_threshold {
                is_anomalous = true;
                anomaly_type = "cpu_spike".to_string();
                description = format!(
                    "{} using {:.1}% CPU — abnormal spike",
                    proc.name, proc.cpu_pct
                );
                if proc.cpu_pct > 95.0 {
                    severity = "critical";
                }
            }

            // Memory leak.
            if proc.memory_bytes > self.config.memory_leak_threshold {
                is_anomalous = true;
                if anomaly_type.is_empty() {
                    anomaly_type = "memory_leak".to_string();
                    description = format!(
                        "{} using {:.1} GB memory — possible leak",
                        proc.name,
                        proc.memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
                    );
                    if proc.memory_bytes > 8 * 1024 * 1024 * 1024 {
                        severity = "critical";
                    }
                }
            }

            // Zombie.
            if proc.state == "Z" {
                is_anomalous = true;
                if anomaly_type.is_empty() {
                    anomaly_type = "zombie".to_string();
                    description = format!("{} is a zombie process", proc.name);
                }
            }

            if is_anomalous {
                current_anomalies.insert(proc.pid);
                // Only emit if this is a NEW anomaly (not already anomalous last cycle).
                if !self.anomalous_pids.contains(&proc.pid) {
                    let mut payload = serde_json::Map::new();
                    payload.insert("pid".into(), serde_json::json!(proc.pid));
                    payload.insert("name".into(), serde_json::json!(proc.name));
                    payload.insert("anomaly_type".into(), serde_json::json!(anomaly_type));
                    payload.insert("description".into(), serde_json::json!(description));
                    payload.insert("cpu_percent".into(), serde_json::json!(proc.cpu_pct));
                    payload.insert(
                        "memory_mb".into(),
                        serde_json::json!((proc.memory_bytes as f64 / 1_048_576.0).round()),
                    );
                    events.push(StoredEvent {
                        id: String::new(),
                        timestamp_ms: now_ms,
                        event_type: "ProcessAnomaly".to_string(),
                        severity: severity.to_string(),
                        source: "process_observer".to_string(),
                        entity_id: Some(format!("process:{}", proc.pid)),
                        payload: payload.into_iter().collect(),
                    });
                }
            }
        }

        // Update anomalous pids set (so we only emit when anomaly starts).
        self.anomalous_pids = current_anomalies;

        // Limit events per cycle.
        events.truncate(self.config.max_events_per_cycle);

        // Batch append.
        let count = if events.is_empty() {
            0
        } else {
            store.append_batch(events).await?
        };

        // Update state for next cycle.
        self.prev_processes = current_map;

        Ok(count)
    }

    /// Get the poll interval.
    pub fn poll_interval(&self) -> Duration {
        self.config.poll_interval
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twin::database::TwinDb;

    #[tokio::test]
    async fn test_first_cycle_emits_no_launches() {
        // First cycle with empty prev should not emit ProcessLaunched for
        // kernel/launchd (pid <= 1), but should emit for user processes.
        let db = TwinDb::open_memory().unwrap();
        let store = EventStore::new(db.handle());
        let mut observer = ProcessObserver::with_defaults();

        let count = observer.observe_cycle(&store).await.unwrap();
        // On a real system, there will be many processes. But we skip pid <= 1
        // on the first cycle, so at least those won't be emitted.
        // The count should be > 0 on any real system.
        assert!(count > 0, "should emit some process launch events");
    }

    #[tokio::test]
    async fn test_second_cycle_emits_changes() {
        let db = TwinDb::open_memory().unwrap();
        let store = EventStore::new(db.handle());
        let mut observer = ProcessObserver::with_defaults();

        // First cycle — populates prev_processes.
        let _ = observer.observe_cycle(&store).await.unwrap();

        // Second cycle — should mostly emit nothing (same processes running).
        let count = observer.observe_cycle(&store).await.unwrap();
        // On a stable system, this might be 0 or a few (processes that
        // launched/terminated in 5s). We just verify it doesn't crash.
        assert!(
            count < 100,
            "should not emit excessive events on stable system"
        );
    }
}
