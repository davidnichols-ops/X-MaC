// Canonical Digital Twin State Model — defines the semantic meaning of every
// node and edge in the digital twin graph.
//
// [133] A node represents something real. Every edge has clear semantic
// meaning. This module is the authoritative reference for what the graph
// represents and how to interpret it.
//
// This is both documentation and a runtime validator. The state model
// defines:
//   1. What each NodeType represents in the real world
//   2. What each EdgeType means semantically
//   3. What properties each node type must have
//   4. What the valid relationships are between node types

#![allow(dead_code)]

use super::knowledge_graph::{EdgeType, NodeType};
use serde::{Deserialize, Serialize};

// ═══════════════════════════════════════════════════════════════════════
//  Node Semantics
// ═══════════════════════════════════════════════════════════════════════

/// Semantic definition of a node type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSemantics {
    pub node_type: String,
    /// What this node represents in the real world.
    pub represents: String,
    /// Required properties for this node type.
    pub required_properties: Vec<String>,
    /// Optional properties that may be present.
    pub optional_properties: Vec<String>,
    /// Whether this node corresponds to a real filesystem path.
    pub has_filesystem_presence: bool,
    /// How often this node should be refreshed (in seconds).
    pub freshness_seconds: u64,
    /// Example node ID format.
    pub id_format: String,
}

/// Get the semantic definition for a node type.
pub fn node_semantics(node_type: &NodeType) -> NodeSemantics {
    match node_type {
        NodeType::Hardware => NodeSemantics {
            node_type: "hardware".to_string(),
            represents: "The physical Mac — the top-level hardware entity. One per machine."
                .to_string(),
            required_properties: vec!["model_identifier".to_string()],
            optional_properties: vec!["fingerprint".to_string(), "soc_generation".to_string()],
            has_filesystem_presence: false,
            freshness_seconds: 3600,
            id_format: "hw:root".to_string(),
        },
        NodeType::Cpu => NodeSemantics {
            node_type: "cpu".to_string(),
            represents: "The CPU — core count, architecture, current utilization.".to_string(),
            required_properties: vec!["total_cores".to_string()],
            optional_properties: vec![
                "performance_cores".to_string(),
                "efficiency_cores".to_string(),
            ],
            has_filesystem_presence: false,
            freshness_seconds: 60,
            id_format: "hw:cpu".to_string(),
        },
        NodeType::Gpu => NodeSemantics {
            node_type: "gpu".to_string(),
            represents: "The GPU — core count, memory, current utilization.".to_string(),
            required_properties: vec![],
            optional_properties: vec!["core_count".to_string(), "metal_version".to_string()],
            has_filesystem_presence: false,
            freshness_seconds: 60,
            id_format: "hw:gpu".to_string(),
        },
        NodeType::NeuralEngine => NodeSemantics {
            node_type: "neural_engine".to_string(),
            represents: "The Apple Neural Engine (ANE) — dedicated ML accelerator.".to_string(),
            required_properties: vec![],
            optional_properties: vec!["utilization".to_string()],
            has_filesystem_presence: false,
            freshness_seconds: 60,
            id_format: "hw:ane".to_string(),
        },
        NodeType::Memory => NodeSemantics {
            node_type: "memory".to_string(),
            represents: "System memory (RAM) — total, used, pressure, swap.".to_string(),
            required_properties: vec!["total_bytes".to_string()],
            optional_properties: vec![
                "used_bytes".to_string(),
                "pressure_level".to_string(),
                "swap_used_bytes".to_string(),
            ],
            has_filesystem_presence: false,
            freshness_seconds: 10,
            id_format: "hw:memory".to_string(),
        },
        NodeType::Storage => NodeSemantics {
            node_type: "storage".to_string(),
            represents: "A storage device — SSD/HDD, capacity, used, health.".to_string(),
            required_properties: vec!["capacity_bytes".to_string()],
            optional_properties: vec![
                "used_bytes".to_string(),
                "device_name".to_string(),
                "smart_status".to_string(),
            ],
            has_filesystem_presence: false,
            freshness_seconds: 300,
            id_format: "hw:storage:{n}".to_string(),
        },
        NodeType::Battery => NodeSemantics {
            node_type: "battery".to_string(),
            represents: "Battery — charge, health, cycle count, time remaining.".to_string(),
            required_properties: vec![],
            optional_properties: vec![
                "charge_pct".to_string(),
                "health_pct".to_string(),
                "cycle_count".to_string(),
            ],
            has_filesystem_presence: false,
            freshness_seconds: 60,
            id_format: "hw:battery".to_string(),
        },
        NodeType::Thermal => NodeSemantics {
            node_type: "thermal".to_string(),
            represents: "Thermal state — CPU/GPU temperature, thermal pressure.".to_string(),
            required_properties: vec![],
            optional_properties: vec!["cpu_temp_c".to_string(), "thermal_pressure".to_string()],
            has_filesystem_presence: false,
            freshness_seconds: 30,
            id_format: "hw:thermal".to_string(),
        },
        NodeType::Network => NodeSemantics {
            node_type: "network".to_string(),
            represents: "Network interface — active connections, bandwidth.".to_string(),
            required_properties: vec![],
            optional_properties: vec!["interface".to_string(), "ip_address".to_string()],
            has_filesystem_presence: false,
            freshness_seconds: 30,
            id_format: "hw:network:{iface}".to_string(),
        },
        NodeType::Application => NodeSemantics {
            node_type: "application".to_string(),
            represents: "An installed macOS application (.app bundle).".to_string(),
            required_properties: vec!["bundle_id".to_string()],
            optional_properties: vec![
                "version".to_string(),
                "path".to_string(),
                "last_launched".to_string(),
            ],
            has_filesystem_presence: true,
            freshness_seconds: 3600,
            id_format: "app:{bundle_id}".to_string(),
        },
        NodeType::Process => NodeSemantics {
            node_type: "process".to_string(),
            represents: "A running process — PID, CPU, memory, parent.".to_string(),
            required_properties: vec!["pid".to_string()],
            optional_properties: vec![
                "cpu_pct".to_string(),
                "memory_bytes".to_string(),
                "parent_pid".to_string(),
            ],
            has_filesystem_presence: false,
            freshness_seconds: 5,
            id_format: "proc:{pid}".to_string(),
        },
        NodeType::File => NodeSemantics {
            node_type: "file".to_string(),
            represents: "A file on disk — path, size, type, owner.".to_string(),
            required_properties: vec!["path".to_string()],
            optional_properties: vec![
                "size_bytes".to_string(),
                "mime_type".to_string(),
                "owner".to_string(),
                "modified_at".to_string(),
            ],
            has_filesystem_presence: true,
            freshness_seconds: 300,
            id_format: "file:{hash_of_path}".to_string(),
        },
        NodeType::Directory => NodeSemantics {
            node_type: "directory".to_string(),
            represents: "A directory on disk — path, size, file count.".to_string(),
            required_properties: vec!["path".to_string()],
            optional_properties: vec!["size_bytes".to_string(), "file_count".to_string()],
            has_filesystem_presence: true,
            freshness_seconds: 300,
            id_format: "dir:{hash_of_path}".to_string(),
        },
        NodeType::CacheEntry => NodeSemantics {
            node_type: "cache_entry".to_string(),
            represents: "A cache file or directory owned by an application.".to_string(),
            required_properties: vec!["path".to_string()],
            optional_properties: vec!["size_bytes".to_string(), "app_bundle_id".to_string()],
            has_filesystem_presence: true,
            freshness_seconds: 600,
            id_format: "cache:{hash_of_path}".to_string(),
        },
        NodeType::MemoryRegion => NodeSemantics {
            node_type: "memory_region".to_string(),
            represents: "A memory region allocated by a process.".to_string(),
            required_properties: vec!["pid".to_string()],
            optional_properties: vec!["size_bytes".to_string(), "region_type".to_string()],
            has_filesystem_presence: false,
            freshness_seconds: 10,
            id_format: "mem:{pid}:{region_id}".to_string(),
        },
        NodeType::NetworkConnection => NodeSemantics {
            node_type: "network_connection".to_string(),
            represents: "An active network connection (TCP/UDP).".to_string(),
            required_properties: vec![],
            optional_properties: vec![
                "local_addr".to_string(),
                "remote_addr".to_string(),
                "protocol".to_string(),
            ],
            has_filesystem_presence: false,
            freshness_seconds: 10,
            id_format: "net:{local}:{remote}".to_string(),
        },
        NodeType::Dependency => NodeSemantics {
            node_type: "dependency".to_string(),
            represents: "A dependency relationship between software components.".to_string(),
            required_properties: vec![],
            optional_properties: vec!["version".to_string()],
            has_filesystem_presence: false,
            freshness_seconds: 3600,
            id_format: "dep:{name}".to_string(),
        },
        NodeType::Framework => NodeSemantics {
            node_type: "framework".to_string(),
            represents: "A system or third-party framework bundle.".to_string(),
            required_properties: vec!["path".to_string()],
            optional_properties: vec!["version".to_string(), "bundle_id".to_string()],
            has_filesystem_presence: true,
            freshness_seconds: 3600,
            id_format: "fw:{bundle_id}".to_string(),
        },
        NodeType::Dylib => NodeSemantics {
            node_type: "dylib".to_string(),
            represents: "A dynamic library loaded by a process.".to_string(),
            required_properties: vec!["path".to_string()],
            optional_properties: vec!["version".to_string()],
            has_filesystem_presence: true,
            freshness_seconds: 3600,
            id_format: "dylib:{hash_of_path}".to_string(),
        },
        NodeType::LaunchAgent => NodeSemantics {
            node_type: "launch_agent".to_string(),
            represents: "A user-level launch agent (LaunchAgents plist).".to_string(),
            required_properties: vec!["label".to_string()],
            optional_properties: vec!["path".to_string(), "run_at_load".to_string()],
            has_filesystem_presence: true,
            freshness_seconds: 3600,
            id_format: "agent:{label}".to_string(),
        },
        NodeType::LaunchDaemon => NodeSemantics {
            node_type: "launch_daemon".to_string(),
            represents: "A system-level launch daemon (LaunchDaemons plist).".to_string(),
            required_properties: vec!["label".to_string()],
            optional_properties: vec!["path".to_string(), "run_at_load".to_string()],
            has_filesystem_presence: true,
            freshness_seconds: 3600,
            id_format: "daemon:{label}".to_string(),
        },
        NodeType::LoginItem => NodeSemantics {
            node_type: "login_item".to_string(),
            represents: "A login item that starts at user login.".to_string(),
            required_properties: vec!["name".to_string()],
            optional_properties: vec!["path".to_string(), "hidden".to_string()],
            has_filesystem_presence: true,
            freshness_seconds: 3600,
            id_format: "login:{name}".to_string(),
        },
        NodeType::User => NodeSemantics {
            node_type: "user".to_string(),
            represents: "A user account on the system.".to_string(),
            required_properties: vec!["username".to_string()],
            optional_properties: vec!["uid".to_string(), "home_dir".to_string()],
            has_filesystem_presence: false,
            freshness_seconds: 3600,
            id_format: "user:{username}".to_string(),
        },
        NodeType::Event => NodeSemantics {
            node_type: "event".to_string(),
            represents: "A discrete event that occurred on the system.".to_string(),
            required_properties: vec!["event_type".to_string(), "timestamp".to_string()],
            optional_properties: vec!["severity".to_string(), "source".to_string()],
            has_filesystem_presence: false,
            freshness_seconds: 0, // Events are immutable
            id_format: "evt:{uuid}".to_string(),
        },
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Edge Semantics
// ═══════════════════════════════════════════════════════════════════════

/// Semantic definition of an edge type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EdgeSemantics {
    pub edge_type: String,
    /// What this edge represents.
    pub represents: String,
    /// Valid source node types.
    pub valid_sources: Vec<String>,
    /// Valid target node types.
    pub valid_targets: Vec<String>,
    /// Whether this edge is directional.
    pub directional: bool,
}

/// Get the semantic definition for an edge type.
pub fn edge_semantics(edge_type: &EdgeType) -> EdgeSemantics {
    use EdgeType::*;
    match edge_type {
        Contains => EdgeSemantics {
            edge_type: "contains".to_string(),
            represents: "A directory or application contains a file or subdirectory.".to_string(),
            valid_sources: vec!["directory".to_string(), "application".to_string()],
            valid_targets: vec!["file".to_string(), "directory".to_string()],
            directional: true,
        },
        ParentOf => EdgeSemantics {
            edge_type: "parent_of".to_string(),
            represents: "A process is the parent of another process.".to_string(),
            valid_sources: vec!["process".to_string()],
            valid_targets: vec!["process".to_string()],
            directional: true,
        },
        Spawns => EdgeSemantics {
            edge_type: "spawns".to_string(),
            represents: "A process spawns a child process.".to_string(),
            valid_sources: vec!["process".to_string()],
            valid_targets: vec!["process".to_string()],
            directional: true,
        },
        Uses => EdgeSemantics {
            edge_type: "uses".to_string(),
            represents: "A process uses a resource (file, memory region).".to_string(),
            valid_sources: vec!["process".to_string()],
            valid_targets: vec!["file".to_string(), "memory_region".to_string()],
            directional: true,
        },
        Creates => EdgeSemantics {
            edge_type: "creates".to_string(),
            represents: "An application creates a file, cache, or directory.".to_string(),
            valid_sources: vec!["application".to_string()],
            valid_targets: vec![
                "file".to_string(),
                "cache_entry".to_string(),
                "directory".to_string(),
            ],
            directional: true,
        },
        Consumes => EdgeSemantics {
            edge_type: "consumes".to_string(),
            represents: "A process consumes CPU, memory, or storage resources.".to_string(),
            valid_sources: vec!["process".to_string()],
            valid_targets: vec![
                "cpu".to_string(),
                "memory".to_string(),
                "storage".to_string(),
            ],
            directional: true,
        },
        DependsOn => EdgeSemantics {
            edge_type: "depends_on".to_string(),
            represents: "An application or framework depends on another framework or dylib."
                .to_string(),
            valid_sources: vec!["application".to_string(), "framework".to_string()],
            valid_targets: vec!["framework".to_string(), "dylib".to_string()],
            directional: true,
        },
        AccessedBy => EdgeSemantics {
            edge_type: "accessed_by".to_string(),
            represents: "A file is accessed by a process.".to_string(),
            valid_sources: vec!["file".to_string()],
            valid_targets: vec!["process".to_string()],
            directional: true,
        },
        Causes => EdgeSemantics {
            edge_type: "causes".to_string(),
            represents: "An event causes a state change in a node.".to_string(),
            valid_sources: vec!["event".to_string()],
            valid_targets: vec!["any".to_string()],
            directional: true,
        },
        LaunchesAt => EdgeSemantics {
            edge_type: "launches_at".to_string(),
            represents: "An application launches at startup via a launch agent/daemon/login item."
                .to_string(),
            valid_sources: vec!["application".to_string()],
            valid_targets: vec![
                "launch_agent".to_string(),
                "launch_daemon".to_string(),
                "login_item".to_string(),
            ],
            directional: true,
        },
        HasPermission => EdgeSemantics {
            edge_type: "has_permission".to_string(),
            represents: "An application has a permission for a user.".to_string(),
            valid_sources: vec!["application".to_string()],
            valid_targets: vec!["user".to_string()],
            directional: true,
        },
        ConnectedTo => EdgeSemantics {
            edge_type: "connected_to".to_string(),
            represents: "A hardware component is connected to another hardware component."
                .to_string(),
            valid_sources: vec!["hardware".to_string()],
            valid_targets: vec![
                "hardware".to_string(),
                "cpu".to_string(),
                "gpu".to_string(),
                "neural_engine".to_string(),
                "memory".to_string(),
                "storage".to_string(),
                "battery".to_string(),
                "thermal".to_string(),
                "network".to_string(),
            ],
            directional: true,
        },
        OwnedBy => EdgeSemantics {
            edge_type: "owned_by".to_string(),
            represents: "A cache entry or file is owned by an application.".to_string(),
            valid_sources: vec!["cache_entry".to_string(), "file".to_string()],
            valid_targets: vec!["application".to_string()],
            directional: true,
        },
    }
}

/// Generate a human-readable state model reference document.
pub fn state_model_document() -> String {
    let mut out = String::new();
    out.push_str("X-MaC Digital Twin — Canonical State Model\n");
    out.push_str("═══════════════════════════════════════════════════════════════════════════\n\n");
    out.push_str(
        "Every node represents something real. Every edge has clear semantic meaning.\n\n",
    );

    out.push_str("NODE TYPES\n");
    out.push_str("──────────\n");
    let node_types = [
        NodeType::Hardware,
        NodeType::Cpu,
        NodeType::Gpu,
        NodeType::NeuralEngine,
        NodeType::Memory,
        NodeType::Storage,
        NodeType::Battery,
        NodeType::Thermal,
        NodeType::Network,
        NodeType::Application,
        NodeType::Process,
        NodeType::File,
        NodeType::Directory,
        NodeType::CacheEntry,
        NodeType::MemoryRegion,
        NodeType::NetworkConnection,
        NodeType::Dependency,
        NodeType::Framework,
        NodeType::Dylib,
        NodeType::LaunchAgent,
        NodeType::LaunchDaemon,
        NodeType::LoginItem,
        NodeType::User,
        NodeType::Event,
    ];
    for nt in &node_types {
        let s = node_semantics(nt);
        out.push_str(&format!("\n  {} ({})\n", s.node_type, s.id_format));
        out.push_str(&format!("    Represents: {}\n", s.represents));
        if !s.required_properties.is_empty() {
            out.push_str(&format!(
                "    Required: {}\n",
                s.required_properties.join(", ")
            ));
        }
        out.push_str(&format!("    Freshness: {}s\n", s.freshness_seconds));
    }

    out.push_str("\n\nEDGE TYPES\n");
    out.push_str("──────────\n");
    let edge_types = [
        EdgeType::Contains,
        EdgeType::ParentOf,
        EdgeType::Spawns,
        EdgeType::Uses,
        EdgeType::Creates,
        EdgeType::Consumes,
        EdgeType::DependsOn,
        EdgeType::AccessedBy,
        EdgeType::Causes,
        EdgeType::LaunchesAt,
        EdgeType::HasPermission,
        EdgeType::ConnectedTo,
        EdgeType::OwnedBy,
    ];
    for et in &edge_types {
        let s = edge_semantics(et);
        out.push_str(&format!(
            "\n  {} ({})\n",
            s.edge_type,
            if s.directional {
                "directional"
            } else {
                "undirectional"
            }
        ));
        out.push_str(&format!("    Represents: {}\n", s.represents));
        out.push_str(&format!("    Source: {}\n", s.valid_sources.join(" | ")));
        out.push_str(&format!("    Target: {}\n", s.valid_targets.join(" | ")));
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
    fn test_all_node_types_have_semantics() {
        let node_types = [
            NodeType::Hardware,
            NodeType::Cpu,
            NodeType::Gpu,
            NodeType::NeuralEngine,
            NodeType::Memory,
            NodeType::Storage,
            NodeType::Battery,
            NodeType::Thermal,
            NodeType::Network,
            NodeType::Application,
            NodeType::Process,
            NodeType::File,
            NodeType::Directory,
            NodeType::CacheEntry,
            NodeType::MemoryRegion,
            NodeType::NetworkConnection,
            NodeType::Dependency,
            NodeType::Framework,
            NodeType::Dylib,
            NodeType::LaunchAgent,
            NodeType::LaunchDaemon,
            NodeType::LoginItem,
            NodeType::User,
            NodeType::Event,
        ];
        for nt in &node_types {
            let s = node_semantics(nt);
            assert!(
                !s.represents.is_empty(),
                "Node type {:?} should have semantics",
                nt
            );
            assert!(!s.node_type.is_empty());
            assert!(!s.id_format.is_empty());
        }
    }

    #[test]
    fn test_all_edge_types_have_semantics() {
        let edge_types = [
            EdgeType::Contains,
            EdgeType::ParentOf,
            EdgeType::Spawns,
            EdgeType::Uses,
            EdgeType::Creates,
            EdgeType::Consumes,
            EdgeType::DependsOn,
            EdgeType::AccessedBy,
            EdgeType::Causes,
            EdgeType::LaunchesAt,
            EdgeType::HasPermission,
            EdgeType::ConnectedTo,
            EdgeType::OwnedBy,
        ];
        for et in &edge_types {
            let s = edge_semantics(et);
            assert!(
                !s.represents.is_empty(),
                "Edge type {:?} should have semantics",
                et
            );
            assert!(!s.valid_sources.is_empty());
            assert!(!s.valid_targets.is_empty());
        }
    }

    #[test]
    fn test_filesystem_nodes_have_path_property() {
        // File, Directory, CacheEntry, Framework, Dylib all require 'path'.
        let path_types = [
            NodeType::File,
            NodeType::Directory,
            NodeType::CacheEntry,
            NodeType::Framework,
            NodeType::Dylib,
        ];
        for nt in &path_types {
            let s = node_semantics(nt);
            assert!(
                s.has_filesystem_presence,
                "Node type {:?} should have filesystem presence",
                nt
            );
            assert!(
                s.required_properties.contains(&"path".to_string()),
                "Node type {:?} should require 'path' property",
                nt
            );
        }
        // Application has filesystem presence but requires 'bundle_id' (path is optional).
        let app = node_semantics(&NodeType::Application);
        assert!(app.has_filesystem_presence);
        assert!(app.required_properties.contains(&"bundle_id".to_string()));
    }

    #[test]
    fn test_process_node_requires_pid() {
        let s = node_semantics(&NodeType::Process);
        assert!(s.required_properties.contains(&"pid".to_string()));
    }

    #[test]
    fn test_state_model_document_is_comprehensive() {
        let doc = state_model_document();
        assert!(doc.contains("NODE TYPES"));
        assert!(doc.contains("EDGE TYPES"));
        assert!(doc.contains("Represents:"));
        // Should mention all node types
        for nt in &[
            "hardware",
            "cpu",
            "memory",
            "application",
            "process",
            "file",
            "event",
        ] {
            assert!(
                doc.contains(nt),
                "Document should mention node type '{}'",
                nt
            );
        }
    }
}
