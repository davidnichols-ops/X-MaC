// Graph Integrity Checker — detects orphan nodes, duplicate nodes, invalid
// relationships, stale state, and graph entries that no longer correspond to
// filesystem reality.
//
// This is the "self-healing" layer of the Digital Twin. A graph that drifts
// from reality is worse than no graph at all — it produces false confidence.
// The integrity checker runs after every twin collection and reports exactly
// what is broken so the system can repair or discard it.

#![allow(dead_code)]

use super::knowledge_graph::{EdgeType, KnowledgeGraph, NodeType};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════
//  Integrity Report
// ═══════════════════════════════════════════════════════════════════════

/// Result of a graph integrity check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityReport {
    pub checked_at_ms: u64,
    pub total_nodes: usize,
    pub total_edges: usize,
    pub issues: Vec<IntegrityIssue>,
    pub summary: IntegritySummary,
    pub is_clean: bool,
}

/// A single integrity issue found in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityIssue {
    pub issue_type: IssueType,
    pub severity: IssueSeverity,
    pub node_id: Option<String>,
    pub edge_source: Option<String>,
    pub edge_target: Option<String>,
    pub description: String,
    pub suggested_fix: String,
}

/// Type of integrity issue.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum IssueType {
    /// Node has no edges connecting it to the rest of the graph.
    OrphanNode,
    /// Two or more nodes represent the same real-world entity.
    DuplicateNode,
    /// Edge references a node that does not exist.
    DanglingEdge,
    /// Edge connects a node to itself.
    SelfLoop,
    /// File/directory node points to a path that no longer exists on disk.
    StaleFileNode,
    /// Node has not been updated within the expected freshness window.
    StaleNode,
    /// Edge type is semantically invalid for the connected node types.
    InvalidEdgeType,
    /// Required property is missing from a node.
    MissingProperty,
}

/// Severity of an integrity issue.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum IssueSeverity {
    Info,
    Warning,
    Error,
    Critical,
}

/// Summary counts by issue type and severity.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IntegritySummary {
    pub orphan_nodes: usize,
    pub duplicate_nodes: usize,
    pub dangling_edges: usize,
    pub self_loops: usize,
    pub stale_file_nodes: usize,
    pub stale_nodes: usize,
    pub invalid_edge_types: usize,
    pub missing_properties: usize,
    pub total_errors: usize,
    pub total_warnings: usize,
    pub total_critical: usize,
}

// ═══════════════════════════════════════════════════════════════════════
//  Integrity Checker
// ═══════════════════════════════════════════════════════════════════════

/// Configuration for the integrity checker.
#[derive(Debug, Clone)]
pub struct IntegrityConfig {
    /// Max age (ms) before a node is considered stale.
    pub stale_threshold_ms: u64,
    /// Whether to verify file paths against the filesystem.
    pub verify_filesystem: bool,
    /// Whether to check for duplicate nodes.
    pub check_duplicates: bool,
    /// Whether to check for orphan nodes.
    pub check_orphans: bool,
}

impl Default for IntegrityConfig {
    fn default() -> Self {
        Self {
            // 7 days — if a node hasn't been updated in a week, it's stale.
            stale_threshold_ms: 7 * 24 * 60 * 60 * 1000,
            verify_filesystem: true,
            check_duplicates: true,
            check_orphans: true,
        }
    }
}

/// Check the integrity of a knowledge graph.
pub fn check_graph(graph: &KnowledgeGraph) -> IntegrityReport {
    check_graph_with_config(graph, &IntegrityConfig::default())
}

/// Check graph integrity with custom configuration.
pub fn check_graph_with_config(
    graph: &KnowledgeGraph,
    config: &IntegrityConfig,
) -> IntegrityReport {
    let mut issues = Vec::new();
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // ── Build node ID set for edge validation ──
    let node_ids: HashSet<&str> = graph.nodes.iter().map(|n| n.id.as_str()).collect();

    // ── Check for dangling edges and self-loops ──
    for edge in &graph.edges {
        if edge.source == edge.target {
            issues.push(IntegrityIssue {
                issue_type: IssueType::SelfLoop,
                severity: IssueSeverity::Warning,
                node_id: None,
                edge_source: Some(edge.source.clone()),
                edge_target: Some(edge.target.clone()),
                description: format!(
                    "Edge of type {:?} connects node '{}' to itself",
                    edge.edge_type, edge.source
                ),
                suggested_fix: "Remove self-referential edge or redirect to correct target."
                    .to_string(),
            });
        }

        if !node_ids.contains(edge.source.as_str()) {
            issues.push(IntegrityIssue {
                issue_type: IssueType::DanglingEdge,
                severity: IssueSeverity::Error,
                node_id: Some(edge.source.clone()),
                edge_source: Some(edge.source.clone()),
                edge_target: Some(edge.target.clone()),
                description: format!("Edge source '{}' does not exist as a node", edge.source),
                suggested_fix: "Remove the edge or create the missing source node.".to_string(),
            });
        }

        if !node_ids.contains(edge.target.as_str()) {
            issues.push(IntegrityIssue {
                issue_type: IssueType::DanglingEdge,
                severity: IssueSeverity::Error,
                node_id: Some(edge.target.clone()),
                edge_source: Some(edge.source.clone()),
                edge_target: Some(edge.target.clone()),
                description: format!("Edge target '{}' does not exist as a node", edge.target),
                suggested_fix: "Remove the edge or create the missing target node.".to_string(),
            });
        }
    }

    // ── Check for orphan nodes (no edges at all) ──
    if config.check_orphans {
        let connected_nodes: HashSet<&str> = graph
            .edges
            .iter()
            .flat_map(|e| [e.source.as_str(), e.target.as_str()])
            .collect();

        for node in &graph.nodes {
            // Hardware root nodes are allowed to be orphans — they are the
            // top of the hierarchy and may not have incoming edges.
            if node.node_type == NodeType::Hardware {
                continue;
            }
            if !connected_nodes.contains(node.id.as_str()) {
                issues.push(IntegrityIssue {
                    issue_type: IssueType::OrphanNode,
                    severity: IssueSeverity::Warning,
                    node_id: Some(node.id.clone()),
                    edge_source: None,
                    edge_target: None,
                    description: format!(
                        "Node '{}' ({:?}) has no edges — it is disconnected from the graph",
                        node.id, node.node_type
                    ),
                    suggested_fix:
                        "Add edges to connect this node, or remove it if it is no longer relevant."
                            .to_string(),
                });
            }
        }
    }

    // ── Check for duplicate nodes ──
    if config.check_duplicates {
        // Group by (node_type, label) — same type + same label = potential dup.
        let mut seen: HashMap<(NodeType, String), Vec<&str>> = HashMap::new();
        for node in &graph.nodes {
            let key = (node.node_type.clone(), node.label.clone());
            seen.entry(key).or_default().push(node.id.as_str());
        }
        for ((node_type, label), ids) in &seen {
            if ids.len() > 1 {
                issues.push(IntegrityIssue {
                    issue_type: IssueType::DuplicateNode,
                    severity: IssueSeverity::Warning,
                    node_id: Some(ids.join(", ")),
                    edge_source: None,
                    edge_target: None,
                    description: format!(
                        "{} duplicate nodes of type {:?} with label '{}': [{}]",
                        ids.len(),
                        node_type,
                        label,
                        ids.join(", ")
                    ),
                    suggested_fix:
                        "Merge duplicate nodes into one, keeping the most recently updated."
                            .to_string(),
                });
            }
        }
    }

    // ── Check for stale file nodes (path no longer exists) ──
    if config.verify_filesystem {
        for node in &graph.nodes {
            if matches!(node.node_type, NodeType::File | NodeType::Directory) {
                if let Some(path_str) = node.properties.get("path").and_then(|v| v.as_str()) {
                    let path = Path::new(path_str);
                    if !path.exists() {
                        issues.push(IntegrityIssue {
                            issue_type: IssueType::StaleFileNode,
                            severity: IssueSeverity::Error,
                            node_id: Some(node.id.clone()),
                            edge_source: None,
                            edge_target: None,
                            description: format!(
                                "File/directory node '{}' references path that no longer exists: {}",
                                node.id, path_str
                            ),
                            suggested_fix:
                                "Remove the stale node or re-scan the filesystem to update it."
                                    .to_string(),
                        });
                    }
                }
            }
        }
    }

    // ── Check for stale nodes (not updated within threshold) ──
    for node in &graph.nodes {
        if let Some(updated_ms) = node
            .properties
            .get("updated_at_ms")
            .and_then(|v| v.as_u64())
        {
            if now_ms > updated_ms && (now_ms - updated_ms) > config.stale_threshold_ms {
                let age_days = (now_ms - updated_ms) / (24 * 60 * 60 * 1000);
                issues.push(IntegrityIssue {
                    issue_type: IssueType::StaleNode,
                    severity: IssueSeverity::Info,
                    node_id: Some(node.id.clone()),
                    edge_source: None,
                    edge_target: None,
                    description: format!(
                        "Node '{}' has not been updated in {} days (threshold: {} days)",
                        node.id,
                        age_days,
                        config.stale_threshold_ms / (24 * 60 * 60 * 1000)
                    ),
                    suggested_fix: "Re-scan the relevant subsystem to refresh this node."
                        .to_string(),
                });
            }
        }
    }

    // ── Check for invalid edge types ──
    for edge in &graph.edges {
        if let (Some(src), Some(tgt)) = (
            graph.nodes.iter().find(|n| n.id == edge.source),
            graph.nodes.iter().find(|n| n.id == edge.target),
        ) {
            if !is_valid_edge_type(&edge.edge_type, &src.node_type, &tgt.node_type) {
                issues.push(IntegrityIssue {
                    issue_type: IssueType::InvalidEdgeType,
                    severity: IssueSeverity::Warning,
                    node_id: None,
                    edge_source: Some(edge.source.clone()),
                    edge_target: Some(edge.target.clone()),
                    description: format!(
                        "Edge type {:?} is semantically invalid between {:?} -> {:?}",
                        edge.edge_type, src.node_type, tgt.node_type
                    ),
                    suggested_fix:
                        "Remove the edge or use a correct edge type for these node types."
                            .to_string(),
                });
            }
        }
    }

    // ── Check for missing required properties ──
    for node in &graph.nodes {
        let missing = missing_required_properties(&node.node_type, &node.properties);
        for prop in missing {
            issues.push(IntegrityIssue {
                issue_type: IssueType::MissingProperty,
                severity: IssueSeverity::Info,
                node_id: Some(node.id.clone()),
                edge_source: None,
                edge_target: None,
                description: format!(
                    "Node '{}' ({:?}) is missing required property '{}'",
                    node.id, node.node_type, prop
                ),
                suggested_fix: format!("Populate the '{}' property during twin collection.", prop),
            });
        }
    }

    // ── Build summary ──
    let summary = build_summary(&issues);
    let is_clean = issues
        .iter()
        .all(|i| i.severity != IssueSeverity::Error && i.severity != IssueSeverity::Critical);

    IntegrityReport {
        checked_at_ms: now_ms,
        total_nodes: graph.nodes.len(),
        total_edges: graph.edges.len(),
        issues,
        summary,
        is_clean,
    }
}

/// Determine if an edge type is semantically valid between two node types.
fn is_valid_edge_type(edge: &EdgeType, src: &NodeType, tgt: &NodeType) -> bool {
    use EdgeType::*;
    use NodeType::*;
    match (edge, src, tgt) {
        // Contains: Directory -> File/Directory, Application -> File
        (Contains, Directory, File) => true,
        (Contains, Directory, Directory) => true,
        (Contains, Application, File) => true,
        (Contains, Application, Directory) => true,
        (Contains, _, _) => false,

        // ParentOf: Process -> Process
        (ParentOf, Process, Process) => true,
        (ParentOf, _, _) => false,

        // Spawns: Process -> Process
        (Spawns, Process, Process) => true,
        (Spawns, _, _) => false,

        // Uses: Process -> File, Process -> MemoryRegion
        (Uses, Process, File) => true,
        (Uses, Process, MemoryRegion) => true,
        (Uses, _, _) => false,

        // Creates: Application -> File, Application -> CacheEntry
        (Creates, Application, File) => true,
        (Creates, Application, CacheEntry) => true,
        (Creates, Application, Directory) => true,
        (Creates, _, _) => false,

        // Consumes: Process -> Cpu, Process -> Memory, Process -> Storage
        (Consumes, Process, Cpu) => true,
        (Consumes, Process, Memory) => true,
        (Consumes, Process, Storage) => true,
        (Consumes, _, _) => false,

        // DependsOn: Application -> Framework, Application -> Dylib
        (DependsOn, Application, Framework) => true,
        (DependsOn, Application, Dylib) => true,
        (DependsOn, Framework, Framework) => true,
        (DependsOn, Framework, Dylib) => true,
        (DependsOn, _, _) => false,

        // AccessedBy: File -> Process (note: reversed direction)
        (AccessedBy, File, Process) => true,
        (AccessedBy, _, _) => false,

        // Causes: Event -> anything
        (Causes, Event, _) => true,
        (Causes, _, _) => false,

        // LaunchesAt: Application -> LaunchAgent/LoginItem
        (LaunchesAt, Application, LaunchAgent) => true,
        (LaunchesAt, Application, LaunchDaemon) => true,
        (LaunchesAt, Application, LoginItem) => true,
        (LaunchesAt, _, _) => false,

        // HasPermission: Application -> User (or self-referential for app perms)
        (HasPermission, Application, User) => true,
        (HasPermission, _, _) => false,

        // ConnectedTo: Hardware -> Hardware
        (ConnectedTo, Hardware, Hardware) => true,
        (ConnectedTo, Hardware, Cpu) => true,
        (ConnectedTo, Hardware, Gpu) => true,
        (ConnectedTo, Hardware, NeuralEngine) => true,
        (ConnectedTo, Hardware, Memory) => true,
        (ConnectedTo, Hardware, Storage) => true,
        (ConnectedTo, Hardware, Battery) => true,
        (ConnectedTo, Hardware, Thermal) => true,
        (ConnectedTo, Hardware, Network) => true,
        (ConnectedTo, _, _) => false,

        // OwnedBy: CacheEntry -> Application, File -> Application
        (OwnedBy, CacheEntry, Application) => true,
        (OwnedBy, File, Application) => true,
        (OwnedBy, _, _) => false,
    }
}

/// Return the list of required properties that are missing for a node type.
fn missing_required_properties(
    node_type: &NodeType,
    props: &HashMap<String, serde_json::Value>,
) -> Vec<&'static str> {
    let required: &[&str] = match node_type {
        NodeType::File => &["path"],
        NodeType::Directory => &["path"],
        NodeType::Application => &["bundle_id"],
        NodeType::Process => &["pid"],
        NodeType::CacheEntry => &["path"],
        _ => &[],
    };
    required
        .iter()
        .filter(|r| !props.contains_key(**r))
        .copied()
        .collect()
}

/// Build summary counts from issues.
fn build_summary(issues: &[IntegrityIssue]) -> IntegritySummary {
    let mut s = IntegritySummary::default();
    for issue in issues {
        match issue.issue_type {
            IssueType::OrphanNode => s.orphan_nodes += 1,
            IssueType::DuplicateNode => s.duplicate_nodes += 1,
            IssueType::DanglingEdge => s.dangling_edges += 1,
            IssueType::SelfLoop => s.self_loops += 1,
            IssueType::StaleFileNode => s.stale_file_nodes += 1,
            IssueType::StaleNode => s.stale_nodes += 1,
            IssueType::InvalidEdgeType => s.invalid_edge_types += 1,
            IssueType::MissingProperty => s.missing_properties += 1,
        }
        match issue.severity {
            IssueSeverity::Info => {}
            IssueSeverity::Warning => s.total_warnings += 1,
            IssueSeverity::Error => s.total_errors += 1,
            IssueSeverity::Critical => s.total_critical += 1,
        }
    }
    s
}

/// Format an integrity report as a human-readable string.
pub fn format_report(report: &IntegrityReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "Graph Integrity Report ({} nodes, {} edges)\n",
        report.total_nodes, report.total_edges
    ));
    out.push_str(&format!(
        "Status: {}\n\n",
        if report.is_clean {
            "CLEAN — no errors or critical issues"
        } else {
            "ISSUES DETECTED"
        }
    ));

    let s = &report.summary;
    if s.orphan_nodes > 0 || s.duplicate_nodes > 0 || s.dangling_edges > 0 || s.stale_file_nodes > 0
    {
        out.push_str("Issues by type:\n");
        if s.orphan_nodes > 0 {
            out.push_str(&format!("  Orphan nodes:       {}\n", s.orphan_nodes));
        }
        if s.duplicate_nodes > 0 {
            out.push_str(&format!("  Duplicate nodes:    {}\n", s.duplicate_nodes));
        }
        if s.dangling_edges > 0 {
            out.push_str(&format!("  Dangling edges:     {}\n", s.dangling_edges));
        }
        if s.self_loops > 0 {
            out.push_str(&format!("  Self-loops:         {}\n", s.self_loops));
        }
        if s.stale_file_nodes > 0 {
            out.push_str(&format!("  Stale file nodes:   {}\n", s.stale_file_nodes));
        }
        if s.stale_nodes > 0 {
            out.push_str(&format!("  Stale nodes:        {}\n", s.stale_nodes));
        }
        if s.invalid_edge_types > 0 {
            out.push_str(&format!("  Invalid edge types: {}\n", s.invalid_edge_types));
        }
        if s.missing_properties > 0 {
            out.push_str(&format!("  Missing properties: {}\n", s.missing_properties));
        }
        out.push_str(&format!(
            "\nSeverity: {} errors, {} warnings, {} critical\n",
            s.total_errors, s.total_warnings, s.total_critical
        ));
        out.push('\n');
    }

    // Show up to 20 issues in detail.
    if !report.issues.is_empty() {
        out.push_str("Detailed issues (showing up to 20):\n");
        for issue in report.issues.iter().take(20) {
            out.push_str(&format!(
                "  [{:?}] {:?}: {} → {}\n",
                issue.severity, issue.issue_type, issue.description, issue.suggested_fix
            ));
        }
        if report.issues.len() > 20 {
            out.push_str(&format!(
                "  ... and {} more issues not shown.\n",
                report.issues.len() - 20
            ));
        }
    }

    out
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twin::knowledge_graph::{GraphEdge, GraphNode, KnowledgeGraph};
    use std::collections::HashMap;

    fn make_graph(nodes: Vec<GraphNode>, edges: Vec<GraphEdge>) -> KnowledgeGraph {
        KnowledgeGraph {
            nodes,
            edges,
            generated_at_ms: 0,
            node_count: 0,
            edge_count: 0,
        }
    }

    fn make_node(id: &str, node_type: NodeType, label: &str) -> GraphNode {
        GraphNode {
            id: id.to_string(),
            node_type,
            label: label.to_string(),
            properties: HashMap::new(),
            health_score: None,
            category: None,
        }
    }

    fn make_edge(src: &str, tgt: &str, edge_type: EdgeType) -> GraphEdge {
        GraphEdge {
            source: src.to_string(),
            target: tgt.to_string(),
            edge_type,
            properties: HashMap::new(),
        }
    }

    #[test]
    fn test_clean_graph_no_issues() {
        let graph = make_graph(
            vec![
                make_node("hw:root", NodeType::Hardware, "MacBook"),
                make_node("hw:cpu", NodeType::Cpu, "8 cores"),
            ],
            vec![make_edge("hw:root", "hw:cpu", EdgeType::ConnectedTo)],
        );
        let report = check_graph(&graph);
        assert!(
            report.is_clean,
            "Expected clean graph, got: {:?}",
            report.issues
        );
    }

    #[test]
    fn test_orphan_node_detected() {
        let graph = make_graph(
            vec![
                make_node("hw:root", NodeType::Hardware, "MacBook"),
                make_node("orphan:1", NodeType::Application, "GhostApp"),
            ],
            vec![],
        );
        let report = check_graph(&graph);
        assert!(report.summary.orphan_nodes >= 1);
        assert!(report
            .issues
            .iter()
            .any(|i| i.issue_type == IssueType::OrphanNode));
    }

    #[test]
    fn test_dangling_edge_detected() {
        let graph = make_graph(
            vec![make_node("hw:root", NodeType::Hardware, "MacBook")],
            vec![make_edge("hw:root", "missing:node", EdgeType::ConnectedTo)],
        );
        let report = check_graph(&graph);
        assert!(report.summary.dangling_edges >= 1);
        assert!(!report.is_clean);
    }

    #[test]
    fn test_self_loop_detected() {
        let graph = make_graph(
            vec![make_node("proc:1", NodeType::Process, "bash")],
            vec![make_edge("proc:1", "proc:1", EdgeType::ParentOf)],
        );
        let report = check_graph(&graph);
        assert!(report.summary.self_loops >= 1);
    }

    #[test]
    fn test_duplicate_nodes_detected() {
        let graph = make_graph(
            vec![
                make_node("app:slack:1", NodeType::Application, "Slack"),
                make_node("app:slack:2", NodeType::Application, "Slack"),
            ],
            vec![],
        );
        let report = check_graph(&graph);
        assert!(report.summary.duplicate_nodes >= 1);
    }

    #[test]
    fn test_invalid_edge_type_detected() {
        // Contains edge from Process to File is invalid (should be Directory -> File)
        let graph = make_graph(
            vec![
                make_node("proc:1", NodeType::Process, "bash"),
                make_node("file:1", NodeType::File, "test.txt"),
            ],
            vec![make_edge("proc:1", "file:1", EdgeType::Contains)],
        );
        let report = check_graph(&graph);
        assert!(report.summary.invalid_edge_types >= 1);
    }

    #[test]
    fn test_missing_property_detected() {
        let mut node = make_node("file:1", NodeType::File, "test.txt");
        // No "path" property set
        let graph = make_graph(vec![node.clone()], vec![]);
        let report = check_graph(&graph);
        assert!(report.summary.missing_properties >= 1);

        // Now add the path property
        node.properties
            .insert("path".to_string(), serde_json::json!("/tmp/test.txt"));
        let graph2 = make_graph(vec![node], vec![]);
        let report2 = check_graph(&graph2);
        assert_eq!(report2.summary.missing_properties, 0);
    }

    #[test]
    fn test_hardware_root_not_flagged_as_orphan() {
        let graph = make_graph(
            vec![make_node("hw:root", NodeType::Hardware, "MacBook")],
            vec![],
        );
        let report = check_graph(&graph);
        assert_eq!(report.summary.orphan_nodes, 0);
    }

    #[test]
    fn test_format_report_output() {
        let graph = make_graph(
            vec![
                make_node("hw:root", NodeType::Hardware, "MacBook"),
                make_node("orphan:1", NodeType::Application, "GhostApp"),
            ],
            vec![],
        );
        let report = check_graph(&graph);
        let formatted = format_report(&report);
        assert!(formatted.contains("Graph Integrity Report"));
        assert!(formatted.contains("Orphan nodes"));
    }

    #[test]
    fn test_valid_edge_types() {
        // Contains: Directory -> File is valid
        let graph = make_graph(
            vec![
                make_node("dir:1", NodeType::Directory, "/tmp"),
                make_node("file:1", NodeType::File, "test.txt"),
            ],
            vec![make_edge("dir:1", "file:1", EdgeType::Contains)],
        );
        let report = check_graph(&graph);
        assert_eq!(
            report.summary.invalid_edge_types, 0,
            "Directory -> File Contains should be valid"
        );
    }

    #[test]
    fn test_stale_node_detected() {
        let mut node = make_node("app:old", NodeType::Application, "OldApp");
        // Set updated_at to 30 days ago
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let thirty_days_ago = now - 30 * 24 * 60 * 60 * 1000;
        node.properties.insert(
            "updated_at_ms".to_string(),
            serde_json::json!(thirty_days_ago),
        );

        let graph = make_graph(vec![node], vec![]);
        let report = check_graph(&graph);
        assert!(report.summary.stale_nodes >= 1);
    }
}
