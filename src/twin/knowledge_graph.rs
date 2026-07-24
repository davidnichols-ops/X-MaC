// X-MaC Knowledge Graph — unified graph abstraction over the Digital Twin
//
// Every object (app, file, process, memory region, network connection) is a
// first-class entity with relationships, history, health, and explanations.
// All GUI views and MCP tools query this graph instead of maintaining their
// own models.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::model::DigitalTwin;

// ═══════════════════════════════════════════════════════════════════════
//  Graph Core
// ═══════════════════════════════════════════════════════════════════════

/// The unified knowledge graph — a single queryable model of the entire Mac.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub generated_at_ms: u64,
    pub node_count: usize,
    pub edge_count: usize,
}

/// A first-class entity in the graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub node_type: NodeType,
    pub label: String,
    pub properties: HashMap<String, serde_json::Value>,
    pub health_score: Option<f64>,
    pub category: Option<String>,
}

/// Relationship between two entities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: EdgeType,
    pub properties: HashMap<String, serde_json::Value>,
}

/// Entity types in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum NodeType {
    Hardware,
    Cpu,
    Gpu,
    NeuralEngine,
    Memory,
    Storage,
    Battery,
    Thermal,
    Network,
    Application,
    Process,
    File,
    Directory,
    CacheEntry,
    MemoryRegion,
    NetworkConnection,
    Dependency,
    Framework,
    Dylib,
    LaunchAgent,
    LaunchDaemon,
    LoginItem,
    User,
    Event,
}

/// Relationship types in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum EdgeType {
    /// Process uses a resource (CPU, memory, file)
    Uses,
    /// Application creates a file/cache
    Creates,
    /// Process consumes memory/CPU/energy
    Consumes,
    /// App/framework depends on another
    DependsOn,
    /// Process parent-child relationship
    ParentOf,
    /// Directory contains file
    Contains,
    /// File accessed by process
    AccessedBy,
    /// Event causes state change
    Causes,
    /// App launches at startup
    LaunchesAt,
    /// App has permission
    HasPermission,
    /// Hardware component connected to
    ConnectedTo,
    /// Cache entry belongs to app
    OwnedBy,
    /// Process spawns child
    Spawns,
}

// ═══════════════════════════════════════════════════════════════════════
//  Graph Queries
// ═══════════════════════════════════════════════════════════════════════

/// Query parameters for graph traversal.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQuery {
    /// Start node ID (or null for full graph)
    pub start_node: Option<String>,
    /// Filter by node type
    pub node_type: Option<NodeType>,
    /// Filter by edge type
    pub edge_type: Option<EdgeType>,
    /// Max traversal depth from start node
    pub max_depth: Option<usize>,
    /// Limit number of results
    pub limit: Option<usize>,
}

/// Result of a graph query.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphQueryResult {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub depth_reached: usize,
}

#[allow(dead_code)]
impl KnowledgeGraph {
    /// Build the knowledge graph from a Digital Twin snapshot.
    pub fn from_twin(twin: &DigitalTwin) -> Self {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        // ── Hardware nodes ──
        let hw_id = "hw:root".to_string();
        nodes.push(GraphNode {
            id: hw_id.clone(),
            node_type: NodeType::Hardware,
            label: twin.hardware.model_identifier.clone(),
            properties: {
                let mut p = HashMap::new();
                p.insert(
                    "soc_generation".into(),
                    serde_json::json!(twin.hardware.soc_generation),
                );
                p.insert(
                    "fingerprint".into(),
                    serde_json::json!(twin.hardware.fingerprint),
                );
                p
            },
            health_score: Some(twin.health_score),
            category: Some("hardware".into()),
        });

        // CPU
        let cpu_id = "hw:cpu".to_string();
        nodes.push(GraphNode {
            id: cpu_id.clone(),
            node_type: NodeType::Cpu,
            label: format!(
                "{} cores ({}P + {}E)",
                twin.hardware.cpu_cores.total_logical_cores,
                twin.hardware.cpu_cores.performance_cores,
                twin.hardware.cpu_cores.efficiency_cores
            ),
            properties: {
                let mut p = HashMap::new();
                p.insert(
                    "total_cores".into(),
                    serde_json::json!(twin.hardware.cpu_cores.total_logical_cores),
                );
                p.insert(
                    "performance_cores".into(),
                    serde_json::json!(twin.hardware.cpu_cores.performance_cores),
                );
                p.insert(
                    "efficiency_cores".into(),
                    serde_json::json!(twin.hardware.cpu_cores.efficiency_cores),
                );
                p
            },
            health_score: None,
            category: Some("cpu".into()),
        });
        edges.push(GraphEdge {
            source: hw_id.clone(),
            target: cpu_id.clone(),
            edge_type: EdgeType::ConnectedTo,
            properties: HashMap::new(),
        });

        // GPU
        let gpu_id = "hw:gpu".to_string();
        nodes.push(GraphNode {
            id: gpu_id.clone(),
            node_type: NodeType::Gpu,
            label: format!("GPU ({} cores)", twin.hardware.gpu.core_count),
            properties: {
                let mut p = HashMap::new();
                p.insert(
                    "core_count".into(),
                    serde_json::json!(twin.hardware.gpu.core_count),
                );
                if let Some(util) = twin.hardware.gpu.utilization_pct {
                    p.insert("utilization_pct".into(), serde_json::json!(util));
                }
                p
            },
            health_score: None,
            category: Some("gpu".into()),
        });
        edges.push(GraphEdge {
            source: hw_id.clone(),
            target: gpu_id.clone(),
            edge_type: EdgeType::ConnectedTo,
            properties: HashMap::new(),
        });

        // Neural Engine
        let ane_id = "hw:ane".to_string();
        nodes.push(GraphNode {
            id: ane_id.clone(),
            node_type: NodeType::NeuralEngine,
            label: format!(
                "Neural Engine ({} cores)",
                twin.hardware.neural_engine.core_count
            ),
            properties: {
                let mut p = HashMap::new();
                p.insert(
                    "core_count".into(),
                    serde_json::json!(twin.hardware.neural_engine.core_count),
                );
                p
            },
            health_score: None,
            category: Some("neural_engine".into()),
        });
        edges.push(GraphEdge {
            source: hw_id.clone(),
            target: ane_id.clone(),
            edge_type: EdgeType::ConnectedTo,
            properties: HashMap::new(),
        });

        // Memory
        let mem_id = "hw:memory".to_string();
        nodes.push(GraphNode {
            id: mem_id.clone(),
            node_type: NodeType::Memory,
            label: format_bytes(twin.hardware.memory.total_bytes),
            properties: {
                let mut p = HashMap::new();
                p.insert(
                    "total_bytes".into(),
                    serde_json::json!(twin.hardware.memory.total_bytes),
                );
                p.insert(
                    "used_bytes".into(),
                    serde_json::json!(twin.memory.used_bytes),
                );
                p.insert(
                    "available_bytes".into(),
                    serde_json::json!(twin.memory.available_bytes),
                );
                p.insert(
                    "utilization".into(),
                    serde_json::json!(twin.memory.utilization),
                );
                p.insert(
                    "pressure_level".into(),
                    serde_json::json!(twin.memory.pressure_level),
                );
                p
            },
            health_score: Some(1.0 - twin.memory.utilization),
            category: Some("memory".into()),
        });
        edges.push(GraphEdge {
            source: hw_id.clone(),
            target: mem_id.clone(),
            edge_type: EdgeType::ConnectedTo,
            properties: HashMap::new(),
        });

        // Storage
        let storage_id = "hw:storage".to_string();
        nodes.push(GraphNode {
            id: storage_id.clone(),
            node_type: NodeType::Storage,
            label: format!(
                "{} files, {}",
                twin.filesystem.total_files,
                format_bytes(twin.filesystem.total_size_bytes)
            ),
            properties: {
                let mut p = HashMap::new();
                p.insert(
                    "total_files".into(),
                    serde_json::json!(twin.filesystem.total_files),
                );
                p.insert(
                    "total_size_bytes".into(),
                    serde_json::json!(twin.filesystem.total_size_bytes),
                );
                p.insert(
                    "duplicate_clusters".into(),
                    serde_json::json!(twin.filesystem.duplicate_clusters.len()),
                );
                if let Some(forecast) = twin.filesystem.exhaustion_forecast_days {
                    p.insert(
                        "exhaustion_forecast_days".into(),
                        serde_json::json!(forecast),
                    );
                }
                p
            },
            health_score: None,
            category: Some("storage".into()),
        });
        edges.push(GraphEdge {
            source: hw_id.clone(),
            target: storage_id.clone(),
            edge_type: EdgeType::ConnectedTo,
            properties: HashMap::new(),
        });

        // Battery
        if let Some(bat) = &twin.hardware.battery {
            let bat_id = "hw:battery".to_string();
            nodes.push(GraphNode {
                id: bat_id.clone(),
                node_type: NodeType::Battery,
                label: format!("Battery ({} cycles)", bat.cycle_count),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("cycle_count".into(), serde_json::json!(bat.cycle_count));
                    p.insert("chemistry".into(), serde_json::json!(bat.chemistry));
                    p
                },
                health_score: None,
                category: Some("battery".into()),
            });
            edges.push(GraphEdge {
                source: hw_id.clone(),
                target: bat_id,
                edge_type: EdgeType::ConnectedTo,
                properties: HashMap::new(),
            });
        }

        // ── Application nodes ──
        for app in &twin.software_genome.applications {
            let app_id = format!("app:{}", app.bundle_id);
            nodes.push(GraphNode {
                id: app_id.clone(),
                node_type: NodeType::Application,
                label: app.name.clone(),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("bundle_id".into(), serde_json::json!(app.bundle_id));
                    p.insert("version".into(), serde_json::json!(app.version));
                    p.insert("path".into(), serde_json::json!(app.path));
                    p.insert("size_bytes".into(), serde_json::json!(app.size_bytes));
                    p.insert("developer".into(), serde_json::json!(app.developer));
                    p.insert("is_signed".into(), serde_json::json!(app.is_signed));
                    p
                },
                health_score: None,
                category: Some("application".into()),
            });
            // App uses memory
            edges.push(GraphEdge {
                source: app_id.clone(),
                target: mem_id.clone(),
                edge_type: EdgeType::Uses,
                properties: HashMap::new(),
            });
            // App uses storage
            edges.push(GraphEdge {
                source: app_id.clone(),
                target: storage_id.clone(),
                edge_type: EdgeType::Creates,
                properties: {
                    let mut p = HashMap::new();
                    p.insert("size_bytes".into(), serde_json::json!(app.size_bytes));
                    p
                },
            });
        }

        // ── Process nodes ──
        for proc in &twin.processes.process_tree {
            let proc_id = format!("proc:{}", proc.pid);
            nodes.push(GraphNode {
                id: proc_id.clone(),
                node_type: NodeType::Process,
                label: proc.name.clone(),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("pid".into(), serde_json::json!(proc.pid));
                    p.insert("cpu_pct".into(), serde_json::json!(proc.cpu_pct));
                    p.insert("memory_bytes".into(), serde_json::json!(proc.memory_bytes));
                    p.insert("state".into(), serde_json::json!(proc.state));
                    if let Some(energy) = proc.energy_impact {
                        p.insert("energy_impact".into(), serde_json::json!(energy));
                    }
                    p
                },
                health_score: Some(if proc.cpu_pct > 80.0 {
                    0.3
                } else if proc.cpu_pct > 50.0 {
                    0.6
                } else {
                    0.9
                }),
                category: Some("process".into()),
            });
            // Process consumes memory
            edges.push(GraphEdge {
                source: proc_id.clone(),
                target: mem_id.clone(),
                edge_type: EdgeType::Consumes,
                properties: {
                    let mut p = HashMap::new();
                    p.insert("memory_bytes".into(), serde_json::json!(proc.memory_bytes));
                    p
                },
            });
            // Process uses CPU
            edges.push(GraphEdge {
                source: proc_id.clone(),
                target: cpu_id.clone(),
                edge_type: EdgeType::Uses,
                properties: {
                    let mut p = HashMap::new();
                    p.insert("cpu_pct".into(), serde_json::json!(proc.cpu_pct));
                    p
                },
            });
            // Parent-child relationships
            if let Some(parent_pid) = proc.parent_pid {
                edges.push(GraphEdge {
                    source: format!("proc:{}", parent_pid),
                    target: proc_id.clone(),
                    edge_type: EdgeType::ParentOf,
                    properties: HashMap::new(),
                });
            }
        }

        // ── Memory leak candidates ──
        for leak in &twin.memory.leak_candidates {
            let leak_id = format!("leak:{}", leak.pid);
            nodes.push(GraphNode {
                id: leak_id.clone(),
                node_type: NodeType::MemoryRegion,
                label: format!(
                    "Leak: {} (+{}/min)",
                    leak.name,
                    format_bytes(leak.growth_rate_bytes_per_min as u64)
                ),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("pid".into(), serde_json::json!(leak.pid));
                    p.insert("name".into(), serde_json::json!(leak.name));
                    p.insert(
                        "growth_rate".into(),
                        serde_json::json!(leak.growth_rate_bytes_per_min),
                    );
                    p.insert(
                        "duration_mins".into(),
                        serde_json::json!(leak.duration_mins),
                    );
                    p
                },
                health_score: Some(0.1),
                category: Some("memory_leak".into()),
            });
            edges.push(GraphEdge {
                source: leak_id,
                target: mem_id.clone(),
                edge_type: EdgeType::Consumes,
                properties: HashMap::new(),
            });
        }

        // ── Duplicate clusters ──
        for cluster in &twin.filesystem.duplicate_clusters {
            let dup_id = format!("dup:{}", &cluster.hash[..16.min(cluster.hash.len())]);
            nodes.push(GraphNode {
                id: dup_id.clone(),
                node_type: NodeType::File,
                label: format!(
                    "{} duplicate files ({})",
                    cluster.files.len(),
                    format_bytes(cluster.total_size_bytes)
                ),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("file_count".into(), serde_json::json!(cluster.files.len()));
                    p.insert(
                        "total_size_bytes".into(),
                        serde_json::json!(cluster.total_size_bytes),
                    );
                    p.insert("hash".into(), serde_json::json!(cluster.hash));
                    p
                },
                health_score: Some(0.5),
                category: Some("duplicate".into()),
            });
            edges.push(GraphEdge {
                source: dup_id,
                target: storage_id.clone(),
                edge_type: EdgeType::OwnedBy,
                properties: HashMap::new(),
            });
        }

        // ── App intelligence nodes ──
        for app in &twin.applications.apps {
            let ai_id = format!("ai_app:{}", app.bundle_id);
            nodes.push(GraphNode {
                id: ai_id.clone(),
                node_type: NodeType::Application,
                label: app.name.clone(),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("bundle_id".into(), serde_json::json!(app.bundle_id));
                    p.insert("version".into(), serde_json::json!(app.version));
                    p.insert("is_unused".into(), serde_json::json!(app.is_unused));
                    p.insert("is_suspicious".into(), serde_json::json!(app.is_suspicious));
                    p.insert(
                        "preferences_corrupted".into(),
                        serde_json::json!(app.preferences_corrupted),
                    );
                    if let Some(crash) = app.crash_probability {
                        p.insert("crash_probability".into(), serde_json::json!(crash));
                    }
                    p
                },
                health_score: Some(app.health_score),
                category: Some(if app.is_suspicious {
                    "suspicious_app"
                } else if app.is_unused {
                    "unused_app"
                } else {
                    "app_intelligence"
                })
                .map(|s| s.to_string()),
            });
        }

        // ── Energy consumers ──
        for consumer in &twin.energy.energy_consumers {
            let energy_id = format!("energy:{}", consumer.name.replace(' ', "_"));
            nodes.push(GraphNode {
                id: energy_id.clone(),
                node_type: NodeType::Process,
                label: format!("{} ({})", consumer.name, consumer.energy_impact as i64),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("name".into(), serde_json::json!(consumer.name));
                    p.insert(
                        "energy_impact".into(),
                        serde_json::json!(consumer.energy_impact),
                    );
                    p.insert("category".into(), serde_json::json!(consumer.category));
                    p
                },
                health_score: Some(if consumer.energy_impact > 1000.0 {
                    0.3
                } else if consumer.energy_impact > 500.0 {
                    0.6
                } else {
                    0.9
                }),
                category: Some("energy_consumer".into()),
            });
        }

        // ── Framework nodes ──
        for fw in &twin.software_genome.frameworks {
            let fw_id = format!("fw:{}", fw.name);
            nodes.push(GraphNode {
                id: fw_id.clone(),
                node_type: NodeType::Framework,
                label: format!("{} {}", fw.name, fw.version.as_deref().unwrap_or("")),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("path".into(), serde_json::json!(fw.path));
                    p.insert("size_bytes".into(), serde_json::json!(fw.size_bytes));
                    p
                },
                health_score: None,
                category: Some("framework".into()),
            });
            edges.push(GraphEdge {
                source: fw_id,
                target: storage_id.clone(),
                edge_type: EdgeType::OwnedBy,
                properties: HashMap::new(),
            });
        }

        // ── Launch agents ──
        for agent in &twin.software_genome.launch_agents {
            let agent_id = format!("agent:{}", agent.name);
            nodes.push(GraphNode {
                id: agent_id.clone(),
                node_type: NodeType::LaunchAgent,
                label: agent.name.clone(),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("path".into(), serde_json::json!(agent.path));
                    p
                },
                health_score: None,
                category: Some("launch_agent".into()),
            });
            edges.push(GraphEdge {
                source: agent_id,
                target: hw_id.clone(),
                edge_type: EdgeType::LaunchesAt,
                properties: HashMap::new(),
            });
        }

        // ── Login items ──
        for item in &twin.software_genome.login_items {
            let item_id = format!("login:{}", item.name);
            nodes.push(GraphNode {
                id: item_id.clone(),
                node_type: NodeType::LoginItem,
                label: item.name.clone(),
                properties: {
                    let mut p = HashMap::new();
                    p.insert("path".into(), serde_json::json!(item.path));
                    p
                },
                health_score: None,
                category: Some("login_item".into()),
            });
            edges.push(GraphEdge {
                source: item_id,
                target: hw_id.clone(),
                edge_type: EdgeType::LaunchesAt,
                properties: HashMap::new(),
            });
        }

        let node_count = nodes.len();
        let edge_count = edges.len();

        Self {
            nodes,
            edges,
            generated_at_ms: twin.timestamp_ms,
            node_count,
            edge_count,
        }
    }

    /// Query the graph with optional filters.
    pub fn query(&self, q: &GraphQuery) -> GraphQueryResult {
        let mut filtered_nodes: Vec<GraphNode> = self
            .nodes
            .iter()
            .filter(|n| q.node_type.as_ref().is_none_or(|t| &n.node_type == t))
            .cloned()
            .collect();

        if let Some(limit) = q.limit {
            filtered_nodes.truncate(limit);
        }

        let filtered_ids: std::collections::HashSet<String> =
            filtered_nodes.iter().map(|n| n.id.clone()).collect();

        let filtered_edges: Vec<GraphEdge> = self
            .edges
            .iter()
            .filter(|e| filtered_ids.contains(&e.source) && filtered_ids.contains(&e.target))
            .filter(|e| q.edge_type.as_ref().is_none_or(|t| &e.edge_type == t))
            .cloned()
            .collect();

        GraphQueryResult {
            nodes: filtered_nodes,
            edges: filtered_edges,
            depth_reached: 1,
        }
    }

    /// Get a node by ID.
    pub fn get_node(&self, id: &str) -> Option<&GraphNode> {
        self.nodes.iter().find(|n| n.id == id)
    }

    /// Get all neighbors of a node.
    pub fn get_neighbors(&self, node_id: &str) -> Vec<&GraphEdge> {
        self.edges
            .iter()
            .filter(|e| e.source == node_id || e.target == node_id)
            .collect()
    }

    /// Get subgraph rooted at a node, up to max_depth hops.
    pub fn get_subgraph(&self, node_id: &str, max_depth: usize) -> GraphQueryResult {
        let mut visited: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut current_level: Vec<String> = vec![node_id.to_string()];
        let mut depth_reached = 0;

        for depth in 0..max_depth {
            if current_level.is_empty() {
                break;
            }
            depth_reached = depth + 1;
            let mut next_level: Vec<String> = Vec::new();
            for id in &current_level {
                if visited.insert(id.clone()) {
                    for edge in &self.edges {
                        if edge.source == *id && !visited.contains(&edge.target) {
                            next_level.push(edge.target.clone());
                        }
                        if edge.target == *id && !visited.contains(&edge.source) {
                            next_level.push(edge.source.clone());
                        }
                    }
                }
            }
            current_level = next_level;
        }

        let nodes: Vec<GraphNode> = self
            .nodes
            .iter()
            .filter(|n| visited.contains(&n.id))
            .cloned()
            .collect();

        let edges: Vec<GraphEdge> = self
            .edges
            .iter()
            .filter(|e| visited.contains(&e.source) && visited.contains(&e.target))
            .cloned()
            .collect();

        GraphQueryResult {
            nodes,
            edges,
            depth_reached,
        }
    }

    /// Get nodes by category.
    pub fn get_by_category(&self, category: &str) -> Vec<&GraphNode> {
        self.nodes
            .iter()
            .filter(|n| n.category.as_deref() == Some(category))
            .collect()
    }

    /// Get statistics summary.
    pub fn stats(&self) -> GraphStats {
        let mut by_type: HashMap<String, usize> = HashMap::new();
        for node in &self.nodes {
            let key = format!("{:?}", node.node_type).to_lowercase();
            *by_type.entry(key).or_default() += 1;
        }
        let mut by_edge: HashMap<String, usize> = HashMap::new();
        for edge in &self.edges {
            let key = format!("{:?}", edge.edge_type).to_lowercase();
            *by_edge.entry(key).or_default() += 1;
        }
        GraphStats {
            total_nodes: self.node_count,
            total_edges: self.edge_count,
            nodes_by_type: by_type,
            edges_by_type: by_edge,
        }
    }
}

/// Graph statistics summary.
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphStats {
    pub total_nodes: usize,
    pub total_edges: usize,
    pub nodes_by_type: HashMap<String, usize>,
    pub edges_by_type: HashMap<String, usize>,
}

// ═══════════════════════════════════════════════════════════════════════
//  Helpers
// ═══════════════════════════════════════════════════════════════════════

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

#[allow(dead_code)]
fn json<T: Serialize>(value: T) -> serde_json::Value {
    serde_json::to_value(value).unwrap_or(serde_json::Value::Null)
}
