// Explanation & Evidence Layer — every AI-generated recommendation must come
// with a human-readable explanation and the underlying graph/filesystem
// evidence that justifies it.
//
// This module implements three X-MAC-TWIN tasks:
//   [136] Graph-to-user explanation path
//   [142] Confidence threshold for AI recommendations
//   [143] "Why?" explanation for every AI recommendation
//   [144] "Show me the evidence" mode

#![allow(dead_code)]

use super::knowledge_graph::{KnowledgeGraph, NodeType};
use super::model::DigitalTwin;
use serde::{Deserialize, Serialize};
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════
//  Confidence Thresholds
// ═══════════════════════════════════════════════════════════════════════

/// Confidence levels that determine how a recommendation is presented.
///
/// - High:   can be auto-applied (with user's prior approval)
/// - Medium: shown as a suggestion with explanation
/// - Low:    shown as information only, never auto-applied
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ConfidenceLevel {
    /// < 0.4 — information only, no action suggested
    Low,
    /// 0.4 – 0.7 — suggestion with explanation
    Medium,
    /// > 0.7 — strong recommendation, can be auto-applied with approval
    High,
}

impl ConfidenceLevel {
    /// Classify a raw confidence score (0.0–1.0) into a level.
    pub fn from_score(score: f64) -> Self {
        if score < 0.4 {
            Self::Low
        } else if score < 0.7 {
            Self::Medium
        } else {
            Self::High
        }
    }

    /// Whether this confidence level permits automatic action.
    pub fn permits_auto_action(self) -> bool {
        matches!(self, Self::High)
    }

    /// Human-readable label.
    pub fn label(self) -> &'static str {
        match self {
            Self::Low => "low confidence",
            Self::Medium => "medium confidence",
            Self::High => "high confidence",
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Evidence
// ═══════════════════════════════════════════════════════════════════════

/// A single piece of evidence supporting a recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// What kind of evidence this is.
    pub kind: EvidenceKind,
    /// Human-readable description of what the evidence shows.
    pub description: String,
    /// The concrete data point (e.g. "2.3 GB", "47 duplicates", "180 days old").
    pub data_point: String,
    /// The source node/edge/file that produced this evidence.
    pub source: String,
}

/// Type of evidence.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// Filesystem metric (size, age, access time, duplicate count).
    FilesystemMetric,
    /// Graph relationship (node A is connected to node B).
    GraphRelationship,
    /// Process behavior (CPU usage, memory consumption, anomalies).
    ProcessBehavior,
    /// Hardware constraint (disk full, thermal throttling, low battery).
    HardwareConstraint,
    /// Historical pattern (this has happened before, this is a trend).
    HistoricalPattern,
    /// Safety rule (this action is blocked/allowed by a safety rule).
    SafetyRule,
    /// Comparison against similar systems or baselines.
    ComparisonBaseline,
}

// ═══════════════════════════════════════════════════════════════════════
//  Explanation
// ═══════════════════════════════════════════════════════════════════════

/// A complete explanation for a recommendation or finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Explanation {
    /// The recommendation being explained (e.g. "Delete 47 duplicate files").
    pub recommendation: String,
    /// Why this recommendation was made — the causal chain.
    pub why: String,
    /// Confidence score (0.0–1.0).
    pub confidence: f64,
    /// Classified confidence level.
    pub confidence_level: ConfidenceLevel,
    /// How this confidence level affects presentation.
    pub presentation: PresentationAdvice,
    /// The evidence supporting this recommendation.
    pub evidence: Vec<Evidence>,
    /// What the user should consider before acting.
    pub caveats: Vec<String>,
    /// What will happen if the action is taken (predicted outcome).
    pub predicted_outcome: String,
    /// Whether the action is reversible.
    pub is_reversible: bool,
}

/// How a recommendation should be presented based on confidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PresentationAdvice {
    /// "auto_apply", "suggest", or "inform"
    pub mode: String,
    /// Whether to show the "Why?" button.
    pub show_why: bool,
    /// Whether to show the "Show me the evidence" button.
    pub show_evidence: bool,
    /// Whether to require explicit confirmation.
    pub requires_confirmation: bool,
}

impl PresentationAdvice {
    pub fn from_confidence(level: ConfidenceLevel) -> Self {
        match level {
            ConfidenceLevel::High => Self {
                mode: "auto_apply".to_string(),
                show_why: true,
                show_evidence: true,
                requires_confirmation: true, // still confirm even for high confidence
            },
            ConfidenceLevel::Medium => Self {
                mode: "suggest".to_string(),
                show_why: true,
                show_evidence: true,
                requires_confirmation: true,
            },
            ConfidenceLevel::Low => Self {
                mode: "inform".to_string(),
                show_why: true,
                show_evidence: true,
                requires_confirmation: false, // low confidence = no action to confirm
            },
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Evidence Inspector — "Show me the evidence" mode
// ═══════════════════════════════════════════════════════════════════════

/// Detailed inspection of a single entity in the graph.
/// This is what the user sees when they click "Show me the evidence".
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityInspection {
    /// The entity being inspected.
    pub entity_id: String,
    pub entity_type: String,
    pub label: String,
    /// All properties of this entity.
    pub properties: Vec<(String, String)>,
    /// All relationships (edges) connected to this entity.
    pub relationships: Vec<RelationshipDetail>,
    /// Related files on disk (if applicable).
    pub filesystem_details: Option<FilesystemDetail>,
    /// Health score for this entity (if computed).
    pub health_score: Option<f64>,
    /// Whether this entity currently exists on the filesystem.
    pub exists_on_disk: Option<bool>,
}

/// A relationship connected to an entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelationshipDetail {
    pub direction: String, // "outgoing" or "incoming"
    pub edge_type: String,
    pub connected_to: String,
    pub connected_label: String,
    pub connected_type: String,
}

/// Filesystem details for a file/directory entity.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemDetail {
    pub path: String,
    pub size_bytes: Option<u64>,
    pub modified_at: Option<String>,
    pub accessed_at: Option<String>,
    pub exists: bool,
}

// ═══════════════════════════════════════════════════════════════════════
//  Explanation Builder
// ═══════════════════════════════════════════════════════════════════════

/// Build an explanation for a recommendation from twin evidence.
pub struct ExplanationBuilder<'a> {
    twin: &'a DigitalTwin,
    graph: Option<&'a KnowledgeGraph>,
}

impl<'a> ExplanationBuilder<'a> {
    pub fn new(twin: &'a DigitalTwin) -> Self {
        Self { twin, graph: None }
    }

    pub fn with_graph(mut self, graph: &'a KnowledgeGraph) -> Self {
        self.graph = Some(graph);
        self
    }

    /// Explain a storage cleanup recommendation (e.g. "Delete these duplicate files").
    pub fn explain_cleanup(&self, recommendation: &str) -> Explanation {
        let mut evidence = Vec::new();

        // Gather filesystem evidence.
        let fs = &self.twin.filesystem;
        let dup_size: u64 = fs
            .duplicate_clusters
            .iter()
            .map(|c| c.total_size_bytes)
            .sum();
        if !fs.duplicate_clusters.is_empty() {
            evidence.push(Evidence {
                kind: EvidenceKind::FilesystemMetric,
                description: format!(
                    "{} duplicate clusters detected consuming {} of storage",
                    fs.duplicate_clusters.len(),
                    format_bytes(dup_size)
                ),
                data_point: format_bytes(dup_size),
                source: "filesystem_graph.duplicate_clusters".to_string(),
            });
        }

        // Detect storage leaks and runaway folders via methods.
        let storage_leaks = fs.detect_storage_leaks();
        let leaks_size: u64 = storage_leaks.iter().map(|l| l.size_bytes).sum();
        if !storage_leaks.is_empty() {
            evidence.push(Evidence {
                kind: EvidenceKind::FilesystemMetric,
                description: format!(
                    "Storage leaks detected: {} in {} locations",
                    format_bytes(leaks_size),
                    storage_leaks.len()
                ),
                data_point: format_bytes(leaks_size),
                source: "filesystem_graph.storage_leaks".to_string(),
            });
        }

        let runaway_folders = fs.detect_runaway_folders();
        if !runaway_folders.is_empty() {
            evidence.push(Evidence {
                kind: EvidenceKind::FilesystemMetric,
                description: format!(
                    "{} runaway folders growing abnormally fast",
                    runaway_folders.len()
                ),
                data_point: format!("{} folders", runaway_folders.len()),
                source: "filesystem_graph.runaway_folders".to_string(),
            });
        }

        // Disk space evidence.
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        if total_disk > 0 {
            let free_pct = (1.0 - fs.total_size_bytes as f64 / total_disk as f64) * 100.0;
            evidence.push(Evidence {
                kind: EvidenceKind::HardwareConstraint,
                description: format!(
                    "Disk is {:.0}% full ({} used of {})",
                    100.0 - free_pct,
                    format_bytes(fs.total_size_bytes),
                    format_bytes(total_disk)
                ),
                data_point: format!("{:.0}% full", 100.0 - free_pct),
                source: "hardware.storage".to_string(),
            });
        }

        let confidence = compute_confidence(&evidence);
        let level = ConfidenceLevel::from_score(confidence);
        let reclaimable: u64 = dup_size + leaks_size;
        let why = if evidence.is_empty() {
            "No specific evidence found for this recommendation.".to_string()
        } else {
            format!(
                "This recommendation is based on {} evidence point(s): {}",
                evidence.len(),
                evidence
                    .iter()
                    .map(|e| e.description.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        };

        Explanation {
            recommendation: recommendation.to_string(),
            why,
            confidence,
            confidence_level: level,
            presentation: PresentationAdvice::from_confidence(level),
            evidence,
            caveats: vec![
                "Always review the file list before confirming deletion.".to_string(),
                "Files in active project directories may be needed.".to_string(),
            ],
            predicted_outcome: format!(
                "Approximately {} of storage will be reclaimed.",
                format_bytes(reclaimable)
            ),
            is_reversible: true, // trash-first cleanup
        }
    }

    /// Explain a process management recommendation (e.g. "Quit this app").
    pub fn explain_process_action(&self, recommendation: &str, pid: u32) -> Explanation {
        let mut evidence = Vec::new();

        // Find the process in the twin.
        if let Some(proc) = self
            .twin
            .processes
            .process_tree
            .iter()
            .find(|p| p.pid == pid)
        {
            if proc.cpu_pct > 50.0 {
                evidence.push(Evidence {
                    kind: EvidenceKind::ProcessBehavior,
                    description: format!(
                        "{} (pid {}) is using {:.1}% CPU",
                        proc.name, proc.pid, proc.cpu_pct
                    ),
                    data_point: format!("{:.1}% CPU", proc.cpu_pct),
                    source: format!("process.{}", proc.pid),
                });
            }
            if proc.memory_bytes > 1024 * 1024 * 1024 {
                evidence.push(Evidence {
                    kind: EvidenceKind::ProcessBehavior,
                    description: format!(
                        "{} (pid {}) is using {} of memory",
                        proc.name,
                        proc.pid,
                        format_bytes(proc.memory_bytes)
                    ),
                    data_point: format_bytes(proc.memory_bytes),
                    source: format!("process.{}", proc.pid),
                });
            }
        }

        // Check for anomalies.
        let anomalies: Vec<_> = self
            .twin
            .processes
            .anomalies
            .iter()
            .filter(|a| a.pid == pid)
            .collect();
        for anomaly in &anomalies {
            evidence.push(Evidence {
                kind: EvidenceKind::ProcessBehavior,
                description: format!(
                    "Anomaly detected: {} — {}",
                    anomaly.anomaly_type, anomaly.description
                ),
                data_point: anomaly.anomaly_type.clone(),
                source: format!("process_anomaly.{}", anomaly.pid),
            });
        }

        // Memory pressure context.
        if self.twin.memory.pressure_level >= 2 {
            evidence.push(Evidence {
                kind: EvidenceKind::HardwareConstraint,
                description: format!(
                    "System memory pressure is elevated (level {}, {:.0}% used)",
                    self.twin.memory.pressure_level,
                    self.twin.memory.utilization * 100.0
                ),
                data_point: format!("pressure level {}", self.twin.memory.pressure_level),
                source: "memory.pressure".to_string(),
            });
        }

        let confidence = compute_confidence(&evidence);
        let level = ConfidenceLevel::from_score(confidence);
        let why = if evidence.is_empty() {
            format!("No specific evidence found for process pid {}.", pid)
        } else {
            format!(
                "Process {} is recommended for action because: {}",
                pid,
                evidence
                    .iter()
                    .map(|e| e.description.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            )
        };

        Explanation {
            recommendation: recommendation.to_string(),
            why,
            confidence,
            confidence_level: level,
            presentation: PresentationAdvice::from_confidence(level),
            evidence,
            caveats: vec![
                "Quitting an app may cause unsaved work to be lost.".to_string(),
                "Some processes will automatically restart.".to_string(),
            ],
            predicted_outcome:
                "The process will be terminated, freeing its CPU and memory allocations."
                    .to_string(),
            is_reversible: true,
        }
    }

    /// Explain a general system health recommendation.
    pub fn explain_system_health(&self, recommendation: &str) -> Explanation {
        let mut evidence = Vec::new();

        // Health score evidence.
        evidence.push(Evidence {
            kind: EvidenceKind::ComparisonBaseline,
            description: format!("System health score is {:.1}/100", self.twin.health_score),
            data_point: format!("{:.1}/100", self.twin.health_score),
            source: "twin.health_score".to_string(),
        });

        // Memory evidence.
        if self.twin.memory.utilization > 0.8 {
            evidence.push(Evidence {
                kind: EvidenceKind::HardwareConstraint,
                description: format!(
                    "Memory utilization is {:.0}% with {:.1} GB swap in use",
                    self.twin.memory.utilization * 100.0,
                    self.twin.memory.swap_used_bytes as f64 / 1_073_741_824.0
                ),
                data_point: format!("{:.0}% memory", self.twin.memory.utilization * 100.0),
                source: "memory.utilization".to_string(),
            });
        }

        // Thermal evidence.
        let thermal = &self.twin.hardware.thermal;
        if thermal.thermal_pressure != "Nominal" && !thermal.thermal_pressure.is_empty() {
            evidence.push(Evidence {
                kind: EvidenceKind::HardwareConstraint,
                description: format!(
                    "Thermal pressure is {} (CPU temp {:?}°C)",
                    thermal.thermal_pressure, thermal.cpu_temp_c
                ),
                data_point: thermal.thermal_pressure.clone(),
                source: "hardware.thermal".to_string(),
            });
        }

        // Process anomalies.
        let anomaly_count = self.twin.processes.anomalies.len();
        if anomaly_count > 0 {
            evidence.push(Evidence {
                kind: EvidenceKind::ProcessBehavior,
                description: format!("{} process anomalies detected", anomaly_count),
                data_point: format!("{} anomalies", anomaly_count),
                source: "processes.anomalies".to_string(),
            });
        }

        let confidence = compute_confidence(&evidence);
        let level = ConfidenceLevel::from_score(confidence);

        Explanation {
            recommendation: recommendation.to_string(),
            why: format!(
                "System health assessment based on {} evidence point(s) across memory, thermal, and process dimensions.",
                evidence.len()
            ),
            confidence,
            confidence_level: level,
            presentation: PresentationAdvice::from_confidence(level),
            evidence,
            caveats: vec!["System health is a snapshot — conditions may change rapidly.".to_string()],
            predicted_outcome: "Following recommendations should improve the overall health score.".to_string(),
            is_reversible: true,
        }
    }

    /// Inspect a specific entity in the graph — "Show me the evidence" mode.
    pub fn inspect_entity(&self, entity_id: &str) -> Option<EntityInspection> {
        let graph = self.graph?;

        let node = graph.nodes.iter().find(|n| n.id == entity_id)?;
        let mut relationships = Vec::new();

        // Find all edges connected to this node.
        for edge in &graph.edges {
            if edge.source == entity_id {
                if let Some(target) = graph.nodes.iter().find(|n| n.id == edge.target) {
                    relationships.push(RelationshipDetail {
                        direction: "outgoing".to_string(),
                        edge_type: format!("{:?}", edge.edge_type),
                        connected_to: target.id.clone(),
                        connected_label: target.label.clone(),
                        connected_type: format!("{:?}", target.node_type),
                    });
                }
            }
            if edge.target == entity_id {
                if let Some(source) = graph.nodes.iter().find(|n| n.id == edge.source) {
                    relationships.push(RelationshipDetail {
                        direction: "incoming".to_string(),
                        edge_type: format!("{:?}", edge.edge_type),
                        connected_to: source.id.clone(),
                        connected_label: source.label.clone(),
                        connected_type: format!("{:?}", source.node_type),
                    });
                }
            }
        }

        // Filesystem details for file/directory nodes.
        let filesystem_details = if matches!(node.node_type, NodeType::File | NodeType::Directory) {
            node.properties
                .get("path")
                .and_then(|v| v.as_str())
                .map(|path| {
                    let p = Path::new(path);
                    let exists = p.exists();
                    let size_bytes = if exists && p.is_file() {
                        std::fs::metadata(p).ok().map(|m| m.len())
                    } else {
                        None
                    };
                    let modified_at = std::fs::metadata(p)
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .and_then(|t| {
                            t.duration_since(std::time::UNIX_EPOCH).ok().map(|d| {
                                chrono::DateTime::<chrono::Utc>::from_timestamp(
                                    d.as_secs() as i64,
                                    0,
                                )
                                .map(|dt| dt.format("%Y-%m-%d %H:%M").to_string())
                                .unwrap_or_default()
                            })
                        });
                    FilesystemDetail {
                        path: path.to_string(),
                        size_bytes,
                        modified_at,
                        accessed_at: None,
                        exists,
                    }
                })
        } else {
            None
        };

        let exists_on_disk = filesystem_details.as_ref().map(|f| f.exists);

        // Convert properties to sorted vec.
        let mut properties: Vec<(String, String)> = node
            .properties
            .iter()
            .map(|(k, v)| (k.clone(), v.to_string()))
            .collect();
        properties.sort_by(|a, b| a.0.cmp(&b.0));

        Some(EntityInspection {
            entity_id: entity_id.to_string(),
            entity_type: format!("{:?}", node.node_type),
            label: node.label.clone(),
            properties,
            relationships,
            filesystem_details,
            health_score: node.health_score,
            exists_on_disk,
        })
    }
}

/// Compute a confidence score from the amount and strength of evidence.
fn compute_confidence(evidence: &[Evidence]) -> f64 {
    if evidence.is_empty() {
        return 0.3; // No evidence = low confidence default
    }
    // Base confidence from evidence count.
    let count_factor = (evidence.len() as f64 / 5.0).min(1.0) * 0.4;
    // Boost from evidence diversity (different kinds).
    let mut kinds: Vec<&EvidenceKind> = evidence.iter().map(|e| &e.kind).collect();
    kinds.dedup();
    let diversity_factor = (kinds.len() as f64 / 4.0).min(1.0) * 0.3;
    // Base trust.
    let base = 0.3;
    (base + count_factor + diversity_factor).min(0.95)
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

/// Format an explanation for terminal output.
pub fn format_explanation(explanation: &Explanation) -> String {
    let mut out = String::new();
    out.push_str(&format!("Recommendation: {}\n", explanation.recommendation));
    out.push_str(&format!("Why: {}\n", explanation.why));
    out.push_str(&format!(
        "Confidence: {:.0}% ({}) — {}\n",
        explanation.confidence * 100.0,
        explanation.confidence_level.label(),
        match explanation.confidence_level {
            ConfidenceLevel::High => "can be auto-applied with confirmation",
            ConfidenceLevel::Medium => "shown as suggestion",
            ConfidenceLevel::Low => "information only, no action suggested",
        }
    ));
    out.push_str(&format!(
        "Presentation: {} (confirmation required: {})\n",
        explanation.presentation.mode, explanation.presentation.requires_confirmation
    ));

    if !explanation.evidence.is_empty() {
        out.push_str("\nEvidence:\n");
        for (i, ev) in explanation.evidence.iter().enumerate() {
            out.push_str(&format!(
                "  {}. [{:?}] {}\n     Data: {}\n     Source: {}\n",
                i + 1,
                ev.kind,
                ev.description,
                ev.data_point,
                ev.source
            ));
        }
    }

    if !explanation.caveats.is_empty() {
        out.push_str("\nCaveats:\n");
        for c in &explanation.caveats {
            out.push_str(&format!("  ! {}\n", c));
        }
    }

    out.push_str(&format!(
        "\nPredicted outcome: {}\n",
        explanation.predicted_outcome
    ));
    out.push_str(&format!(
        "Reversible: {}\n",
        if explanation.is_reversible {
            "yes"
        } else {
            "no"
        }
    ));

    out
}

/// Format an entity inspection for terminal output.
pub fn format_inspection(inspection: &EntityInspection) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Entity: {} ({})\n",
        inspection.label, inspection.entity_type
    ));
    out.push_str(&format!("ID: {}\n\n", inspection.entity_id));

    if !inspection.properties.is_empty() {
        out.push_str("Properties:\n");
        for (k, v) in &inspection.properties {
            out.push_str(&format!("  {}: {}\n", k, v));
        }
        out.push('\n');
    }

    if !inspection.relationships.is_empty() {
        out.push_str("Relationships:\n");
        for rel in &inspection.relationships {
            out.push_str(&format!(
                "  [{}] {:?} → {} ({})\n",
                rel.direction, rel.edge_type, rel.connected_label, rel.connected_type
            ));
        }
        out.push('\n');
    }

    if let Some(fs) = &inspection.filesystem_details {
        out.push_str("Filesystem:\n");
        out.push_str(&format!("  Path: {}\n", fs.path));
        out.push_str(&format!("  Exists: {}\n", fs.exists));
        if let Some(size) = fs.size_bytes {
            out.push_str(&format!("  Size: {}\n", format_bytes(size)));
        }
        if let Some(modified) = &fs.modified_at {
            out.push_str(&format!("  Modified: {}\n", modified));
        }
    }

    if let Some(score) = inspection.health_score {
        out.push_str(&format!("\nHealth score: {:.1}\n", score));
    }

    out
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_confidence_levels() {
        assert_eq!(ConfidenceLevel::from_score(0.1), ConfidenceLevel::Low);
        assert_eq!(ConfidenceLevel::from_score(0.5), ConfidenceLevel::Medium);
        assert_eq!(ConfidenceLevel::from_score(0.8), ConfidenceLevel::High);
    }

    #[test]
    fn test_only_high_confidence_permits_auto_action() {
        assert!(!ConfidenceLevel::Low.permits_auto_action());
        assert!(!ConfidenceLevel::Medium.permits_auto_action());
        assert!(ConfidenceLevel::High.permits_auto_action());
    }

    #[test]
    fn test_presentation_advice() {
        let high = PresentationAdvice::from_confidence(ConfidenceLevel::High);
        assert_eq!(high.mode, "auto_apply");
        assert!(high.show_why);
        assert!(high.show_evidence);
        assert!(high.requires_confirmation);

        let low = PresentationAdvice::from_confidence(ConfidenceLevel::Low);
        assert_eq!(low.mode, "inform");
        assert!(!low.requires_confirmation);
    }

    #[test]
    fn test_compute_confidence_no_evidence() {
        let score = compute_confidence(&[]);
        assert!(score < 0.4);
    }

    #[test]
    fn test_compute_confidence_with_evidence() {
        let evidence = vec![
            Evidence {
                kind: EvidenceKind::FilesystemMetric,
                description: "test".to_string(),
                data_point: "1".to_string(),
                source: "test".to_string(),
            },
            Evidence {
                kind: EvidenceKind::ProcessBehavior,
                description: "test2".to_string(),
                data_point: "2".to_string(),
                source: "test2".to_string(),
            },
        ];
        let score = compute_confidence(&evidence);
        assert!(score > 0.4);
    }

    #[test]
    fn test_format_explanation() {
        let explanation = Explanation {
            recommendation: "Delete duplicates".to_string(),
            why: "Because they waste space".to_string(),
            confidence: 0.85,
            confidence_level: ConfidenceLevel::High,
            presentation: PresentationAdvice::from_confidence(ConfidenceLevel::High),
            evidence: vec![Evidence {
                kind: EvidenceKind::FilesystemMetric,
                description: "47 duplicates".to_string(),
                data_point: "47 files".to_string(),
                source: "fs_graph".to_string(),
            }],
            caveats: vec!["Review before deleting".to_string()],
            predicted_outcome: "2.3 GB reclaimed".to_string(),
            is_reversible: true,
        };
        let formatted = format_explanation(&explanation);
        assert!(formatted.contains("Recommendation: Delete duplicates"));
        assert!(formatted.contains("Evidence:"));
        assert!(formatted.contains("Reversible: yes"));
    }
}
