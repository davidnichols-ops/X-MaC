// X-MaC Event Stream — continuous observer and timeline
//
// Tracks state changes on the Mac: process launches, app installs, file
// creation, memory pressure, storage growth, crashes, battery changes.
// Maintains a replayable timeline — "Git for your Mac."

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ═══════════════════════════════════════════════════════════════════════
//  Event Core
// ═══════════════════════════════════════════════════════════════════════

/// A single event in the Mac's timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwinEvent {
    pub id: String,
    pub timestamp_ms: u64,
    pub event_type: EventType,
    pub entity_id: String,
    pub entity_label: String,
    pub description: String,
    pub severity: EventSeverity,
    pub category: String,
    pub metadata: HashMap<String, serde_json::Value>,
}

/// Type of system event.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventType {
    ProcessLaunched,
    ProcessTerminated,
    ProcessAnomaly,
    AppInstalled,
    AppRemoved,
    AppCrash,
    FileCreated,
    FileDeleted,
    LargeFileDetected,
    DuplicateDetected,
    OrphanDetected,
    MemoryPressureChanged,
    MemoryLeakDetected,
    StorageGrowth,
    StorageWarning,
    BatteryDrain,
    BatteryDegradation,
    ThermalWarning,
    NetworkChange,
    SystemBoot,
    SystemSleep,
    SystemWake,
    SecurityAlert,
    OptimizationApplied,
    UserAction,
    Recommendation,
}

/// Event severity level.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EventSeverity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

/// The event stream — a chronological log of everything that happened.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EventStream {
    pub events: Vec<TwinEvent>,
    pub first_event_ms: u64,
    pub last_event_ms: u64,
    pub total_events: usize,
}

/// Query parameters for the event stream.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventQuery {
    /// Filter by event type
    pub event_type: Option<EventType>,
    /// Filter by severity (minimum)
    pub min_severity: Option<EventSeverity>,
    /// Start timestamp (inclusive)
    pub since_ms: Option<u64>,
    /// End timestamp (inclusive)
    pub until_ms: Option<u64>,
    /// Category filter
    pub category: Option<String>,
    /// Limit number of results
    pub limit: Option<usize>,
}

#[allow(dead_code)]
impl EventStream {
    /// Create a new empty event stream.
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a new event.
    pub fn record(&mut self, event: TwinEvent) {
        if self.events.is_empty() {
            self.first_event_ms = event.timestamp_ms;
        }
        self.last_event_ms = event.timestamp_ms;
        self.events.push(event);
        self.total_events = self.events.len();
    }

    /// Query events with filters.
    pub fn query(&self, q: &EventQuery) -> Vec<&TwinEvent> {
        let mut results: Vec<&TwinEvent> = self
            .events
            .iter()
            .filter(|e| q.event_type.as_ref().is_none_or(|t| &e.event_type == t))
            .filter(|e| q.category.as_ref().is_none_or(|c| &e.category == c))
            .filter(|e| q.since_ms.is_none_or(|s| e.timestamp_ms >= s))
            .filter(|e| q.until_ms.is_none_or(|u| e.timestamp_ms <= u))
            .filter(|e| {
                q.min_severity
                    .as_ref()
                    .is_none_or(|min| severity_rank(&e.severity) >= severity_rank(min))
            })
            .collect();

        // Sort by timestamp descending (newest first)
        results.sort_by_key(|b| std::cmp::Reverse(b.timestamp_ms));

        if let Some(limit) = q.limit {
            results.truncate(limit);
        }
        results
    }

    /// Generate events from a Digital Twin snapshot by comparing with a
    /// previous snapshot (diff).
    pub fn diff_snapshots(
        &mut self,
        prev: &super::model::DigitalTwin,
        curr: &super::model::DigitalTwin,
    ) {
        let ts = curr.timestamp_ms;

        // Memory pressure change
        if prev.memory.pressure_level != curr.memory.pressure_level {
            self.record(TwinEvent {
                id: format!("evt:memory_pressure:{}", ts),
                timestamp_ms: ts,
                event_type: EventType::MemoryPressureChanged,
                entity_id: "hw:memory".into(),
                entity_label: "System Memory".into(),
                description: format!(
                    "Memory pressure changed from {} to {}",
                    prev.memory.pressure_level, curr.memory.pressure_level
                ),
                severity: if curr.memory.pressure_level >= 3 {
                    EventSeverity::Critical
                } else if curr.memory.pressure_level >= 2 {
                    EventSeverity::High
                } else if curr.memory.pressure_level >= 1 {
                    EventSeverity::Medium
                } else {
                    EventSeverity::Info
                },
                category: "memory".into(),
                metadata: {
                    let mut m = HashMap::new();
                    m.insert(
                        "prev_level".into(),
                        serde_json::json!(prev.memory.pressure_level),
                    );
                    m.insert(
                        "curr_level".into(),
                        serde_json::json!(curr.memory.pressure_level),
                    );
                    m.insert(
                        "utilization".into(),
                        serde_json::json!(curr.memory.utilization),
                    );
                    m
                },
            });
        }

        // New memory leaks
        let prev_leaks: std::collections::HashSet<u32> =
            prev.memory.leak_candidates.iter().map(|l| l.pid).collect();
        for leak in &curr.memory.leak_candidates {
            if !prev_leaks.contains(&leak.pid) {
                self.record(TwinEvent {
                    id: format!("evt:leak:{}:{}", leak.pid, ts),
                    timestamp_ms: ts,
                    event_type: EventType::MemoryLeakDetected,
                    entity_id: format!("proc:{}", leak.pid),
                    entity_label: leak.name.clone(),
                    description: format!(
                        "Memory leak detected: {} growing {}/min for {}m",
                        leak.name,
                        format_bytes(leak.growth_rate_bytes_per_min as u64),
                        leak.duration_mins
                    ),
                    severity: EventSeverity::High,
                    category: "memory".into(),
                    metadata: HashMap::new(),
                });
            }
        }

        // Storage growth
        if curr.filesystem.total_size_bytes > prev.filesystem.total_size_bytes {
            let growth = curr.filesystem.total_size_bytes - prev.filesystem.total_size_bytes;
            if growth > 1024 * 1024 * 100 {
                // > 100MB
                self.record(TwinEvent {
                    id: format!("evt:storage_growth:{}", ts),
                    timestamp_ms: ts,
                    event_type: EventType::StorageGrowth,
                    entity_id: "hw:storage".into(),
                    entity_label: "Macintosh HD".into(),
                    description: format!("Storage increased by {}", format_bytes(growth)),
                    severity: if growth > 1024 * 1024 * 1024 {
                        EventSeverity::High
                    } else {
                        EventSeverity::Medium
                    },
                    category: "storage".into(),
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("growth_bytes".into(), serde_json::json!(growth));
                        m.insert(
                            "prev_total".into(),
                            serde_json::json!(prev.filesystem.total_size_bytes),
                        );
                        m.insert(
                            "curr_total".into(),
                            serde_json::json!(curr.filesystem.total_size_bytes),
                        );
                        m
                    },
                });
            }
        }

        // New duplicate clusters
        let prev_dups: std::collections::HashSet<String> = prev
            .filesystem
            .duplicate_clusters
            .iter()
            .map(|c| c.hash.clone())
            .collect();
        for cluster in &curr.filesystem.duplicate_clusters {
            if !prev_dups.contains(&cluster.hash) {
                self.record(TwinEvent {
                    id: format!(
                        "evt:dup:{}:{}",
                        &cluster.hash[..16.min(cluster.hash.len())],
                        ts
                    ),
                    timestamp_ms: ts,
                    event_type: EventType::DuplicateDetected,
                    entity_id: format!("dup:{}", &cluster.hash[..16.min(cluster.hash.len())]),
                    entity_label: format!("{} duplicate files", cluster.files.len()),
                    description: format!(
                        "Found {} duplicate files totaling {}",
                        cluster.files.len(),
                        format_bytes(cluster.total_size_bytes)
                    ),
                    severity: EventSeverity::Medium,
                    category: "filesystem".into(),
                    metadata: HashMap::new(),
                });
            }
        }

        // New process anomalies
        let prev_anomalies: std::collections::HashSet<String> = prev
            .processes
            .anomalies
            .iter()
            .map(|a| format!("{}-{}", a.pid, a.anomaly_type))
            .collect();
        for anomaly in &curr.processes.anomalies {
            let key = format!("{}-{}", anomaly.pid, anomaly.anomaly_type);
            if !prev_anomalies.contains(&key) {
                self.record(TwinEvent {
                    id: format!("evt:anomaly:{}:{}", key, ts),
                    timestamp_ms: ts,
                    event_type: EventType::ProcessAnomaly,
                    entity_id: format!("proc:{}", anomaly.pid),
                    entity_label: anomaly.name.clone(),
                    description: anomaly.description.clone(),
                    severity: match anomaly.severity.to_lowercase().as_str() {
                        "critical" => EventSeverity::Critical,
                        "high" => EventSeverity::High,
                        "medium" => EventSeverity::Medium,
                        "low" => EventSeverity::Low,
                        _ => EventSeverity::Info,
                    },
                    category: "process".into(),
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert(
                            "anomaly_type".into(),
                            serde_json::json!(anomaly.anomaly_type),
                        );
                        m
                    },
                });
            }
        }

        // Health score change
        if (prev.health_score - curr.health_score).abs() > 5.0 {
            self.record(TwinEvent {
                id: format!("evt:health:{}", ts),
                timestamp_ms: ts,
                event_type: EventType::SystemBoot, // reuse as "system state change"
                entity_id: "system".into(),
                entity_label: "System Health".into(),
                description: format!(
                    "System health {} from {:.0} to {:.0}",
                    if curr.health_score > prev.health_score {
                        "improved"
                    } else {
                        "declined"
                    },
                    prev.health_score,
                    curr.health_score
                ),
                severity: if curr.health_score < 50.0 {
                    EventSeverity::High
                } else if curr.health_score < 70.0 {
                    EventSeverity::Medium
                } else {
                    EventSeverity::Info
                },
                category: "system".into(),
                metadata: {
                    let mut m = HashMap::new();
                    m.insert("prev_score".into(), serde_json::json!(prev.health_score));
                    m.insert("curr_score".into(), serde_json::json!(curr.health_score));
                    m
                },
            });
        }

        // New suspicious apps
        let prev_suspicious: std::collections::HashSet<String> =
            prev.applications.suspicious_apps.iter().cloned().collect();
        for app in &curr.applications.suspicious_apps {
            if !prev_suspicious.contains(app) {
                self.record(TwinEvent {
                    id: format!("evt:suspicious:{}:{}", app, ts),
                    timestamp_ms: ts,
                    event_type: EventType::SecurityAlert,
                    entity_id: format!("app:{}", app),
                    entity_label: app.clone(),
                    description: format!("Suspicious application detected: {}", app),
                    severity: EventSeverity::High,
                    category: "security".into(),
                    metadata: HashMap::new(),
                });
            }
        }
    }

    /// Generate events from a single Digital Twin snapshot (no diff).
    /// This creates events for all current anomalies, leaks, etc.
    #[allow(clippy::wrong_self_convention)]
    pub fn from_twin(&mut self, twin: &super::model::DigitalTwin) {
        let ts = twin.timestamp_ms;

        // Process anomalies
        for anomaly in &twin.processes.anomalies {
            self.record(TwinEvent {
                id: format!(
                    "evt:anomaly:{}-{}:{}",
                    anomaly.pid, anomaly.anomaly_type, ts
                ),
                timestamp_ms: ts,
                event_type: EventType::ProcessAnomaly,
                entity_id: format!("proc:{}", anomaly.pid),
                entity_label: anomaly.name.clone(),
                description: anomaly.description.clone(),
                severity: match anomaly.severity.to_lowercase().as_str() {
                    "critical" => EventSeverity::Critical,
                    "high" => EventSeverity::High,
                    "medium" => EventSeverity::Medium,
                    "low" => EventSeverity::Low,
                    _ => EventSeverity::Info,
                },
                category: "process".into(),
                metadata: {
                    let mut m = HashMap::new();
                    m.insert(
                        "anomaly_type".into(),
                        serde_json::json!(anomaly.anomaly_type),
                    );
                    m
                },
            });
        }

        // Memory leaks
        for leak in &twin.memory.leak_candidates {
            self.record(TwinEvent {
                id: format!("evt:leak:{}:{}", leak.pid, ts),
                timestamp_ms: ts,
                event_type: EventType::MemoryLeakDetected,
                entity_id: format!("proc:{}", leak.pid),
                entity_label: leak.name.clone(),
                description: format!(
                    "Memory leak: {} growing {}/min",
                    leak.name,
                    format_bytes(leak.growth_rate_bytes_per_min as u64)
                ),
                severity: EventSeverity::High,
                category: "memory".into(),
                metadata: HashMap::new(),
            });
        }

        // Duplicate clusters
        for cluster in &twin.filesystem.duplicate_clusters {
            self.record(TwinEvent {
                id: format!(
                    "evt:dup:{}:{}",
                    &cluster.hash[..16.min(cluster.hash.len())],
                    ts
                ),
                timestamp_ms: ts,
                event_type: EventType::DuplicateDetected,
                entity_id: format!("dup:{}", &cluster.hash[..16.min(cluster.hash.len())]),
                entity_label: format!("{} duplicates", cluster.files.len()),
                description: format!(
                    "{} duplicate files totaling {}",
                    cluster.files.len(),
                    format_bytes(cluster.total_size_bytes)
                ),
                severity: EventSeverity::Medium,
                category: "filesystem".into(),
                metadata: HashMap::new(),
            });
        }

        // Suspicious apps
        for app in &twin.applications.suspicious_apps {
            self.record(TwinEvent {
                id: format!("evt:suspicious:{}:{}", app, ts),
                timestamp_ms: ts,
                event_type: EventType::SecurityAlert,
                entity_id: format!("app:{}", app),
                entity_label: app.clone(),
                description: format!("Suspicious application: {}", app),
                severity: EventSeverity::High,
                category: "security".into(),
                metadata: HashMap::new(),
            });
        }

        // Storage warning
        if let Some(forecast) = twin.filesystem.exhaustion_forecast_days {
            if forecast < 30 {
                self.record(TwinEvent {
                    id: format!("evt:storage_warning:{}", ts),
                    timestamp_ms: ts,
                    event_type: EventType::StorageWarning,
                    entity_id: "hw:storage".into(),
                    entity_label: "Macintosh HD".into(),
                    description: format!("Storage will be full in ~{} days", forecast),
                    severity: if forecast < 7 {
                        EventSeverity::Critical
                    } else {
                        EventSeverity::High
                    },
                    category: "storage".into(),
                    metadata: {
                        let mut m = HashMap::new();
                        m.insert("forecast_days".into(), serde_json::json!(forecast));
                        m
                    },
                });
            }
        }

        // Energy offenders
        for consumer in twin
            .energy
            .energy_consumers
            .iter()
            .filter(|c| c.energy_impact > 1000.0)
        {
            self.record(TwinEvent {
                id: format!("evt:energy:{}:{}", consumer.name.replace(' ', "_"), ts),
                timestamp_ms: ts,
                event_type: EventType::BatteryDrain,
                entity_id: format!("energy:{}", consumer.name.replace(' ', "_")),
                entity_label: consumer.name.clone(),
                description: format!(
                    "High energy consumer: {} ({})",
                    consumer.name, consumer.energy_impact as i64
                ),
                severity: EventSeverity::Medium,
                category: "energy".into(),
                metadata: {
                    let mut m = HashMap::new();
                    m.insert(
                        "energy_impact".into(),
                        serde_json::json!(consumer.energy_impact),
                    );
                    m
                },
            });
        }
    }

    /// Get a timeline summary (grouped by hour).
    pub fn timeline_summary(&self) -> Vec<TimelineBucket> {
        if self.events.is_empty() {
            return Vec::new();
        }

        let mut buckets: HashMap<u64, TimelineBucket> = HashMap::new();
        for event in &self.events {
            let hour = event.timestamp_ms / 3_600_000; // group by hour
            let bucket = buckets.entry(hour).or_insert_with(|| TimelineBucket {
                hour_ms: hour * 3_600_000,
                events: Vec::new(),
                critical_count: 0,
                high_count: 0,
                medium_count: 0,
                low_count: 0,
                info_count: 0,
            });
            bucket.events.push(event.id.clone());
            match event.severity {
                EventSeverity::Critical => bucket.critical_count += 1,
                EventSeverity::High => bucket.high_count += 1,
                EventSeverity::Medium => bucket.medium_count += 1,
                EventSeverity::Low => bucket.low_count += 1,
                EventSeverity::Info => bucket.info_count += 1,
            }
        }

        let mut result: Vec<TimelineBucket> = buckets.into_values().collect();
        result.sort_by_key(|b| b.hour_ms);
        result
    }
}

/// A bucket of events grouped by time period.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineBucket {
    pub hour_ms: u64,
    pub events: Vec<String>,
    pub critical_count: usize,
    pub high_count: usize,
    pub medium_count: usize,
    pub low_count: usize,
    pub info_count: usize,
}

#[allow(dead_code)]
fn severity_rank(s: &EventSeverity) -> u8 {
    match s {
        EventSeverity::Info => 0,
        EventSeverity::Low => 1,
        EventSeverity::Medium => 2,
        EventSeverity::High => 3,
        EventSeverity::Critical => 4,
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{} {}", bytes, UNITS[unit])
    } else {
        format!("{:.1} {}", size, UNITS[unit])
    }
}
