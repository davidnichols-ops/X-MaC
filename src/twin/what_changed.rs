// WhatChanged — temporal diff query: "What changed between A and B?"
//
// This is the first proof that the Digital Twin is alive. It compares the
// event log between two timestamps and produces a human-readable summary
// of what happened on the Mac.
//
// Output categories:
//   - new applications installed
//   - applications removed
//   - storage growth (bytes added/removed)
//   - process changes (new/terminated/anomalous)
//   - major file changes (large files created/deleted)
//   - configuration changes
//   - security alerts

use crate::twin::database::event_store::EventStore;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Summary of changes between two timestamps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangeReport {
    pub start_ms: i64,
    pub end_ms: i64,
    pub total_events: usize,
    pub new_applications: Vec<AppChange>,
    pub removed_applications: Vec<AppChange>,
    pub storage_growth: StorageGrowth,
    pub process_changes: ProcessChanges,
    pub major_file_changes: Vec<FileChange>,
    pub configuration_changes: Vec<ConfigChange>,
    pub security_alerts: Vec<SecurityAlert>,
    pub timeline: Vec<TimelineEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppChange {
    pub name: String,
    pub bundle_id: Option<String>,
    pub timestamp_ms: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageGrowth {
    pub bytes_added: u64,
    pub bytes_removed: u64,
    pub net_change: i64,
    pub files_created: u64,
    pub files_deleted: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessChanges {
    pub new_processes: u64,
    pub terminated_processes: u64,
    pub anomalies: u64,
    pub top_cpu_consumers: Vec<String>,
    pub top_memory_consumers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileChange {
    pub path: String,
    pub size_bytes: u64,
    pub change_type: String, // "created" | "deleted" | "grown"
    pub timestamp_ms: i64,
    pub entity_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChange {
    pub description: String,
    pub timestamp_ms: i64,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityAlert {
    pub description: String,
    pub severity: String,
    pub timestamp_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimelineEntry {
    pub timestamp_ms: i64,
    pub event_type: String,
    pub description: String,
    pub severity: String,
}

/// Compute a change report from events between `start_ms` and `end_ms`.
pub async fn what_changed_between(
    store: &EventStore,
    start_ms: i64,
    end_ms: i64,
) -> anyhow::Result<ChangeReport> {
    let events = store.get_events_between(start_ms, end_ms).await?;

    let mut new_apps = Vec::new();
    let mut removed_apps = Vec::new();
    let mut bytes_added: u64 = 0;
    let mut bytes_removed: u64 = 0;
    let mut files_created: u64 = 0;
    let mut files_deleted: u64 = 0;
    let mut new_processes: u64 = 0;
    let mut terminated_processes: u64 = 0;
    let mut anomalies: u64 = 0;
    let mut major_files = Vec::new();
    let mut config_changes = Vec::new();
    let mut security_alerts = Vec::new();
    let mut timeline = Vec::new();
    let mut cpu_consumers: HashMap<String, u64> = HashMap::new();
    let mut mem_consumers: HashMap<String, u64> = HashMap::new();

    for event in &events {
        // Timeline entry (limit to notable events).
        if is_notable(&event.event_type, &event.severity) {
            timeline.push(TimelineEntry {
                timestamp_ms: event.timestamp_ms,
                event_type: event.event_type.clone(),
                description: format!(
                    "{}: {}",
                    event.event_type,
                    event
                        .payload
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                ),
                severity: event.severity.clone(),
            });
        }

        match event.event_type.as_str() {
            "AppInstalled" => {
                new_apps.push(AppChange {
                    name: event
                        .payload
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    bundle_id: event
                        .payload
                        .get("bundle_id")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    timestamp_ms: event.timestamp_ms,
                    source: event.source.clone(),
                });
            }
            "AppRemoved" => {
                removed_apps.push(AppChange {
                    name: event
                        .payload
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    bundle_id: event
                        .payload
                        .get("bundle_id")
                        .and_then(|v| v.as_str())
                        .map(String::from),
                    timestamp_ms: event.timestamp_ms,
                    source: event.source.clone(),
                });
            }
            "FileCreated" => {
                files_created += 1;
                let size = event
                    .payload
                    .get("size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                bytes_added += size;
                if size > 100 * 1024 * 1024 {
                    // > 100MB is "major"
                    major_files.push(FileChange {
                        path: event
                            .payload
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        size_bytes: size,
                        change_type: "created".to_string(),
                        timestamp_ms: event.timestamp_ms,
                        entity_id: event.entity_id.clone(),
                    });
                }
            }
            "FileDeleted" => {
                files_deleted += 1;
                let size = event
                    .payload
                    .get("size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                bytes_removed += size;
                if size > 100 * 1024 * 1024 {
                    major_files.push(FileChange {
                        path: event
                            .payload
                            .get("path")
                            .and_then(|v| v.as_str())
                            .unwrap_or("")
                            .to_string(),
                        size_bytes: size,
                        change_type: "deleted".to_string(),
                        timestamp_ms: event.timestamp_ms,
                        entity_id: event.entity_id.clone(),
                    });
                }
            }
            "LargeFileDetected" => {
                let size = event
                    .payload
                    .get("size")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                major_files.push(FileChange {
                    path: event
                        .payload
                        .get("path")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    size_bytes: size,
                    change_type: "grown".to_string(),
                    timestamp_ms: event.timestamp_ms,
                    entity_id: event.entity_id.clone(),
                });
            }
            "ProcessLaunched" => {
                new_processes += 1;
                let name = event
                    .payload
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                if let Some(cpu) = event.payload.get("cpu_percent").and_then(|v| v.as_f64()) {
                    *cpu_consumers.entry(name.to_string()).or_insert(0) += cpu as u64;
                }
                if let Some(mem) = event.payload.get("memory_mb").and_then(|v| v.as_f64()) {
                    *mem_consumers.entry(name.to_string()).or_insert(0) += mem as u64;
                }
            }
            "ProcessTerminated" => {
                terminated_processes += 1;
            }
            "ProcessAnomaly" => {
                anomalies += 1;
            }
            "StorageGrowth" | "StorageWarning" => {
                let added = event
                    .payload
                    .get("bytes_added")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let removed = event
                    .payload
                    .get("bytes_removed")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                bytes_added += added;
                bytes_removed += removed;
            }
            "SecurityAlert" => {
                security_alerts.push(SecurityAlert {
                    description: event
                        .payload
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    severity: event.severity.clone(),
                    timestamp_ms: event.timestamp_ms,
                });
            }
            "UserAction" | "OptimizationApplied" => {
                config_changes.push(ConfigChange {
                    description: event
                        .payload
                        .get("description")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string(),
                    timestamp_ms: event.timestamp_ms,
                    source: event.source.clone(),
                });
            }
            _ => {}
        }
    }

    // Top consumers.
    let mut top_cpu: Vec<(String, u64)> = cpu_consumers.into_iter().collect();
    top_cpu.sort_by_key(|b| std::cmp::Reverse(b.1));
    let mut top_mem: Vec<(String, u64)> = mem_consumers.into_iter().collect();
    top_mem.sort_by_key(|b| std::cmp::Reverse(b.1));

    Ok(ChangeReport {
        start_ms,
        end_ms,
        total_events: events.len(),
        new_applications: new_apps,
        removed_applications: removed_apps,
        storage_growth: StorageGrowth {
            bytes_added,
            bytes_removed,
            net_change: bytes_added as i64 - bytes_removed as i64,
            files_created,
            files_deleted,
        },
        process_changes: ProcessChanges {
            new_processes,
            terminated_processes,
            anomalies,
            top_cpu_consumers: top_cpu.into_iter().take(5).map(|(n, _)| n).collect(),
            top_memory_consumers: top_mem.into_iter().take(5).map(|(n, _)| n).collect(),
        },
        major_file_changes: major_files,
        configuration_changes: config_changes,
        security_alerts,
        timeline,
    })
}

fn is_notable(event_type: &str, severity: &str) -> bool {
    matches!(severity, "warning" | "error" | "critical")
        || matches!(
            event_type,
            "AppInstalled"
                | "AppRemoved"
                | "AppCrash"
                | "StorageWarning"
                | "MemoryPressureChanged"
                | "MemoryLeakDetected"
                | "ThermalWarning"
                | "SecurityAlert"
                | "ProcessAnomaly"
                | "OptimizationApplied"
        )
}

/// Format a ChangeReport as a human-readable summary.
pub fn format_report(report: &ChangeReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "What changed ({} → {})\n",
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(report.start_ms)
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| report.start_ms.to_string()),
        chrono::DateTime::<chrono::Utc>::from_timestamp_millis(report.end_ms)
            .map(|d| d.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| report.end_ms.to_string()),
    ));
    out.push_str(&format!("Total events: {}\n\n", report.total_events));

    if !report.new_applications.is_empty() {
        out.push_str("New applications:\n");
        for app in &report.new_applications {
            out.push_str(&format!("  + {} ({})\n", app.name, app.source));
        }
        out.push('\n');
    }

    if !report.removed_applications.is_empty() {
        out.push_str("Removed applications:\n");
        for app in &report.removed_applications {
            out.push_str(&format!("  - {} ({})\n", app.name, app.source));
        }
        out.push('\n');
    }

    let sg = &report.storage_growth;
    if sg.files_created > 0 || sg.files_deleted > 0 {
        out.push_str("Storage:\n");
        out.push_str(&format!("  Files created: {}\n", sg.files_created));
        out.push_str(&format!("  Files deleted: {}\n", sg.files_deleted));
        out.push_str(&format!(
            "  Bytes added: {}\n",
            format_bytes(sg.bytes_added)
        ));
        out.push_str(&format!(
            "  Bytes removed: {}\n",
            format_bytes(sg.bytes_removed)
        ));
        out.push_str(&format!(
            "  Net change: {}\n",
            format_bytes_signed(sg.net_change)
        ));
        out.push('\n');
    }

    let pc = &report.process_changes;
    if pc.new_processes > 0 || pc.anomalies > 0 {
        out.push_str("Processes:\n");
        out.push_str(&format!("  Launched: {}\n", pc.new_processes));
        out.push_str(&format!("  Terminated: {}\n", pc.terminated_processes));
        out.push_str(&format!("  Anomalies: {}\n", pc.anomalies));
        if !pc.top_cpu_consumers.is_empty() {
            out.push_str("  Top CPU: ");
            out.push_str(&pc.top_cpu_consumers.join(", "));
            out.push('\n');
        }
        if !pc.top_memory_consumers.is_empty() {
            out.push_str("  Top Memory: ");
            out.push_str(&pc.top_memory_consumers.join(", "));
            out.push('\n');
        }
        out.push('\n');
    }

    if !report.major_file_changes.is_empty() {
        out.push_str("Major file changes (>100MB):\n");
        for fc in report.major_file_changes.iter().take(20) {
            out.push_str(&format!(
                "  {} {} ({})\n",
                match fc.change_type.as_str() {
                    "created" => "+",
                    "deleted" => "-",
                    _ => "~",
                },
                fc.path,
                format_bytes(fc.size_bytes)
            ));
        }
        out.push('\n');
    }

    if !report.security_alerts.is_empty() {
        out.push_str("Security alerts:\n");
        for alert in &report.security_alerts {
            out.push_str(&format!("  [{}] {}\n", alert.severity, alert.description));
        }
        out.push('\n');
    }

    if !report.configuration_changes.is_empty() {
        out.push_str("Configuration / optimization changes:\n");
        for cc in &report.configuration_changes {
            out.push_str(&format!("  - {} ({})\n", cc.description, cc.source));
        }
        out.push('\n');
    }

    if !report.timeline.is_empty() {
        out.push_str("Timeline (notable events):\n");
        for entry in report.timeline.iter().take(50) {
            let time = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(entry.timestamp_ms)
                .map(|d| d.format("%H:%M:%S").to_string())
                .unwrap_or_else(|| entry.timestamp_ms.to_string());
            out.push_str(&format!(
                "  {} [{}] {}\n",
                time, entry.severity, entry.description
            ));
        }
    }

    out
}

fn format_bytes(bytes: u64) -> String {
    if bytes >= 1024 * 1024 * 1024 {
        format!("{:.1} GB", bytes as f64 / (1024.0 * 1024.0 * 1024.0))
    } else if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{} B", bytes)
    }
}

fn format_bytes_signed(bytes: i64) -> String {
    if bytes >= 0 {
        format!("+{}", format_bytes(bytes as u64))
    } else {
        format!("-{}", format_bytes(bytes.unsigned_abs()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twin::database::event_store::StoredEvent;
    use crate::twin::database::TwinDb;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_what_changed_empty() {
        let db = TwinDb::open_memory().unwrap();
        let store = EventStore::new(db.handle());
        let report = what_changed_between(&store, 0, 10000).await.unwrap();
        assert_eq!(report.total_events, 0);
    }

    #[tokio::test]
    async fn test_what_changed_with_events() {
        let db = TwinDb::open_memory().unwrap();
        let store = EventStore::new(db.handle());

        // App installed.
        let mut payload = HashMap::new();
        payload.insert("name".to_string(), serde_json::json!("Slack"));
        payload.insert(
            "bundle_id".to_string(),
            serde_json::json!("com.tinyspeck.slackmacgap"),
        );
        store
            .append_event(StoredEvent {
                id: String::new(),
                timestamp_ms: 1000,
                event_type: "AppInstalled".to_string(),
                severity: "info".to_string(),
                source: "filesystem_observer".to_string(),
                entity_id: Some("app:slack".to_string()),
                payload,
            })
            .await
            .unwrap();

        // Large file created.
        let mut payload = HashMap::new();
        payload.insert("path".to_string(), serde_json::json!("/tmp/big.bin"));
        payload.insert("size".to_string(), serde_json::json!(500_000_000u64));
        store
            .append_event(StoredEvent {
                id: String::new(),
                timestamp_ms: 2000,
                event_type: "FileCreated".to_string(),
                severity: "info".to_string(),
                source: "filesystem_observer".to_string(),
                entity_id: Some("file:big".to_string()),
                payload,
            })
            .await
            .unwrap();

        // Security alert.
        let mut payload = HashMap::new();
        payload.insert(
            "description".to_string(),
            serde_json::json!("Suspicious login item detected"),
        );
        store
            .append_event(StoredEvent {
                id: String::new(),
                timestamp_ms: 3000,
                event_type: "SecurityAlert".to_string(),
                severity: "warning".to_string(),
                source: "security_observer".to_string(),
                entity_id: None,
                payload,
            })
            .await
            .unwrap();

        let report = what_changed_between(&store, 0, 10000).await.unwrap();
        assert_eq!(report.total_events, 3);
        assert_eq!(report.new_applications.len(), 1);
        assert_eq!(report.new_applications[0].name, "Slack");
        assert_eq!(report.major_file_changes.len(), 1);
        assert_eq!(report.security_alerts.len(), 1);
        assert!(report.storage_growth.bytes_added >= 500_000_000);
    }

    #[tokio::test]
    async fn test_format_report() {
        let db = TwinDb::open_memory().unwrap();
        let store = EventStore::new(db.handle());

        let mut payload = HashMap::new();
        payload.insert("name".to_string(), serde_json::json!("Docker"));
        store
            .append_event(StoredEvent {
                id: String::new(),
                timestamp_ms: 1000,
                event_type: "AppInstalled".to_string(),
                severity: "info".to_string(),
                source: "test".to_string(),
                entity_id: Some("app:docker".to_string()),
                payload,
            })
            .await
            .unwrap();

        let report = what_changed_between(&store, 0, 10000).await.unwrap();
        let text = format_report(&report);
        assert!(text.contains("Docker"));
        assert!(text.contains("New applications"));
    }
}
