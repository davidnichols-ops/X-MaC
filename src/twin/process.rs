use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Process Intelligence System — observes and models every running process.
///
/// Covers: process trees, CPU/GPU/memory/disk/network tracking, energy
/// impact, anomaly detection, process fingerprints, bottleneck prediction,
/// workload balancing, and behavioral learning.
///
/// Maps to Digital Twin operations 121-160.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessIntelligenceGraph {
    pub process_tree: Vec<ProcessNode>,
    pub top_cpu_consumers: Vec<ProcessInfo>,
    pub top_memory_consumers: Vec<ProcessInfo>,
    pub anomalies: Vec<ProcessAnomaly>,
    pub workload_fingerprints: HashMap<u32, String>,
    pub bottleneck_predictions: Vec<String>,
    pub total_processes: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessNode {
    pub pid: u32,
    pub name: String,
    pub parent_pid: Option<u32>,
    pub children: Vec<u32>,
    pub cpu_pct: f64,
    pub memory_bytes: u64,
    pub disk_read_bytes: Option<u64>,
    pub disk_write_bytes: Option<u64>,
    pub network_bytes: Option<u64>,
    pub energy_impact: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAnomaly {
    pub pid: u32,
    pub name: String,
    pub anomaly_type: String,
    pub description: String,
    pub severity: String,
}

impl ProcessIntelligenceGraph {
    /// Collect the process intelligence graph.
    pub fn collect() -> Self {
        // TODO: implement full process intelligence (ops 121-160)
        // Start with what optimize/telemetry already provides
        Self {
            process_tree: Vec::new(),
            top_cpu_consumers: Vec::new(),
            top_memory_consumers: Vec::new(),
            anomalies: Vec::new(),
            workload_fingerprints: HashMap::new(),
            bottleneck_predictions: Vec::new(),
            total_processes: 0,
        }
    }
}
