use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::telemetry::{ProcessTelemetry, SystemTelemetry, TelemetryBuffer, TelemetrySnapshot};

// ═══════════════════════════════════════════════════════════════════════
//  Memory Graph Types
// ═══════════════════════════════════════════════════════════════════════

/// Node types in the memory graph.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryNodeType {
    Process,
    HardwareConsumer,
    SwapFile,
    CompressorPool,
}

/// Edge types in the memory graph.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryEdgeType {
    /// Process owns/allocated memory (process → hardware consumer)
    Accesses,
    /// Process is a parent of another process
    ParentOf,
    /// Process depends on another (IPC/XPC) — inferred from same-user grouping
    DependsOn,
    /// Hardware consumer backs memory to swap
    Backs,
    /// Memory region is compressed in the compressor pool
    CompressedIn,
    /// System-level: hardware consumer participates in the compressor
    ContributesTo,
}

/// A node in the memory graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGraphNode {
    pub id: u32,
    pub node_type: MemoryNodeType,
    pub label: String,
    /// 24-dimensional feature vector for processes, 8 for hardware, 6 for swap/compressor
    pub features: Vec<f32>,
}

/// An edge in the memory graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGraphEdge {
    pub source: u32,
    pub target: u32,
    pub edge_type: MemoryEdgeType,
}

/// The complete memory graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryGraph {
    pub nodes: Vec<MemoryGraphNode>,
    pub edges: Vec<MemoryGraphEdge>,
    pub num_process_features: usize,
    pub num_hardware_features: usize,
    pub num_swap_features: usize,
    pub num_compressor_features: usize,
    pub collected_at: u64,
    /// Predicted pressure level (0=normal, 1=warn, 2=critical) in 60s
    pub predicted_pressure_60s: Option<f32>,
    /// Confidence of the prediction (0.0 - 1.0)
    pub prediction_confidence: Option<f32>,
    /// Time-to-threshold in seconds (when free memory will reach critical)
    pub time_to_critical_secs: Option<f64>,
}

// ═══════════════════════════════════════════════════════════════════════
//  Graph Builder
// ═══════════════════════════════════════════════════════════════════════

/// Builds a heterogeneous memory graph from telemetry snapshots.
pub struct GraphBuilder {
    /// Whether to include system processes in the graph
    include_system: bool,
    /// Maximum number of process nodes (sorted by RSS)
    max_processes: usize,
    /// Telemetry buffer for computing trends
    buffer: TelemetryBuffer,
}

#[allow(dead_code)]
impl GraphBuilder {
    pub fn new(max_processes: usize, include_system: bool, buffer_size: usize) -> Self {
        Self {
            include_system,
            max_processes,
            buffer: TelemetryBuffer::new(buffer_size),
        }
    }

    /// Add a telemetry snapshot to the buffer and build the graph.
    pub fn build(&mut self, snapshot: &TelemetrySnapshot) -> MemoryGraph {
        self.buffer.push(snapshot.clone());
        self.build_graph()
    }

    /// Build a graph from the current buffer state without adding a snapshot.
    pub fn build_from_buffer(&self) -> MemoryGraph {
        self.build_graph()
    }

    fn build_graph(&self) -> MemoryGraph {
        let snapshot = match self.buffer.latest() {
            Some(s) => s,
            None => {
                return MemoryGraph {
                    nodes: Vec::new(),
                    edges: Vec::new(),
                    num_process_features: PROCESS_FEATURE_DIM,
                    num_hardware_features: HARDWARE_FEATURE_DIM,
                    num_swap_features: SWAP_FEATURE_DIM,
                    num_compressor_features: COMPRESSOR_FEATURE_DIM,
                    collected_at: 0,
                    predicted_pressure_60s: None,
                    prediction_confidence: None,
                    time_to_critical_secs: None,
                };
            }
        };

        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut next_id: u32 = 0;

        // ── Process nodes ──────────────────────────────────────────────
        let mut process_ids: HashMap<u32, u32> = HashMap::new();

        // Filter and sort processes by RSS (descending)
        let mut procs: Vec<&ProcessTelemetry> = snapshot
            .processes
            .iter()
            .filter(|p| self.include_system || !p.is_system)
            .filter(|p| p.resident_size > 0)
            .collect();
        procs.sort_by_key(|b| std::cmp::Reverse(b.resident_size));
        procs.truncate(self.max_processes);

        for proc_tel in &procs {
            let id = next_id;
            next_id += 1;
            process_ids.insert(proc_tel.pid, id);

            let features = self.extract_process_features(proc_tel, &snapshot.system);
            nodes.push(MemoryGraphNode {
                id,
                node_type: MemoryNodeType::Process,
                label: proc_tel.name.clone(),
                features,
            });
        }

        // ── Hardware consumer nodes ──────────────────────────────────
        let cpu_id = next_id;
        next_id += 1;
        nodes.push(MemoryGraphNode {
            id: cpu_id,
            node_type: MemoryNodeType::HardwareConsumer,
            label: "CPU".to_string(),
            features: self.extract_cpu_features(&snapshot.system),
        });

        let gpu_id = next_id;
        next_id += 1;
        nodes.push(MemoryGraphNode {
            id: gpu_id,
            node_type: MemoryNodeType::HardwareConsumer,
            label: "GPU".to_string(),
            features: self.extract_gpu_features(&snapshot.system),
        });

        let ane_id = next_id;
        next_id += 1;
        nodes.push(MemoryGraphNode {
            id: ane_id,
            node_type: MemoryNodeType::HardwareConsumer,
            label: "ANE".to_string(),
            features: self.extract_ane_features(&snapshot.system),
        });

        // ── Swap file node ────────────────────────────────────────────
        let swap_id = next_id;
        next_id += 1;
        nodes.push(MemoryGraphNode {
            id: swap_id,
            node_type: MemoryNodeType::SwapFile,
            label: "swapfile".to_string(),
            features: self.extract_swap_features(&snapshot.system),
        });

        // ── Compressor pool node ───────────────────────────────────────
        let compressor_id = next_id;
        next_id += 1;
        nodes.push(MemoryGraphNode {
            id: compressor_id,
            node_type: MemoryNodeType::CompressorPool,
            label: "compressor".to_string(),
            features: self.extract_compressor_features(&snapshot.system),
        });

        // ── Edges ─────────────────────────────────────────────────────

        // Process → CPU (accesses)
        for (&pid, &node_id) in &process_ids {
            edges.push(MemoryGraphEdge {
                source: node_id,
                target: cpu_id,
                edge_type: MemoryEdgeType::Accesses,
            });
            // Process → GPU (if it has graphics footprint)
            if let Some(proc_tel) = snapshot.processes.iter().find(|p| p.pid == pid) {
                if proc_tel.graphics_footprint > 0 {
                    edges.push(MemoryGraphEdge {
                        source: node_id,
                        target: gpu_id,
                        edge_type: MemoryEdgeType::Accesses,
                    });
                }
                if proc_tel.neural_footprint > 0 {
                    edges.push(MemoryGraphEdge {
                        source: node_id,
                        target: ane_id,
                        edge_type: MemoryEdgeType::Accesses,
                    });
                }
            }
        }

        // Process → Process (parent_of)
        for proc_tel in &procs {
            if let Some(&child_id) = process_ids.get(&proc_tel.pid) {
                if let Some(&parent_id) = process_ids.get(&proc_tel.ppid) {
                    if child_id != parent_id {
                        edges.push(MemoryGraphEdge {
                            source: parent_id,
                            target: child_id,
                            edge_type: MemoryEdgeType::ParentOf,
                        });
                    }
                }
            }
        }

        // Process → Process (depends_on) — group by parent for same-user apps
        // For now, connect processes with the same ppid (siblings) as weak dependencies
        let mut siblings: HashMap<u32, Vec<u32>> = HashMap::new();
        for proc_tel in &procs {
            if let Some(&id) = process_ids.get(&proc_tel.pid) {
                siblings.entry(proc_tel.ppid).or_default().push(id);
            }
        }
        for group in siblings.values() {
            if group.len() > 1 && group.len() <= 5 {
                // Connect siblings with depends_on edges (limited to avoid explosion)
                for i in 0..group.len() {
                    for j in (i + 1)..group.len() {
                        edges.push(MemoryGraphEdge {
                            source: group[i],
                            target: group[j],
                            edge_type: MemoryEdgeType::DependsOn,
                        });
                    }
                }
            }
        }

        // CPU → Compressor (contributes_to)
        edges.push(MemoryGraphEdge {
            source: cpu_id,
            target: compressor_id,
            edge_type: MemoryEdgeType::ContributesTo,
        });

        // Compressor → Swap (compressed_in / backs)
        if snapshot.system.swap_used_bytes > 0 {
            edges.push(MemoryGraphEdge {
                source: compressor_id,
                target: swap_id,
                edge_type: MemoryEdgeType::Backs,
            });
        }

        // Process → Compressor (compressed_in) — for processes with compressed memory
        for proc_tel in &procs {
            if proc_tel.compressed_bytes > 0 {
                if let Some(&id) = process_ids.get(&proc_tel.pid) {
                    edges.push(MemoryGraphEdge {
                        source: id,
                        target: compressor_id,
                        edge_type: MemoryEdgeType::CompressedIn,
                    });
                }
            }
        }

        // ── Predictions (heuristic, replaced by GNN later) ────────────
        let _ = next_id;
        let predicted_pressure = self.predict_pressure(&snapshot.system);
        let time_to_critical = self.buffer.time_to_threshold(
            snapshot.system.total_bytes / 10, // critical = 10% free
        );

        MemoryGraph {
            nodes,
            edges,
            num_process_features: PROCESS_FEATURE_DIM,
            num_hardware_features: HARDWARE_FEATURE_DIM,
            num_swap_features: SWAP_FEATURE_DIM,
            num_compressor_features: COMPRESSOR_FEATURE_DIM,
            collected_at: snapshot.system.timestamp_ms,
            predicted_pressure_60s: Some(predicted_pressure.0),
            prediction_confidence: Some(predicted_pressure.1),
            time_to_critical_secs: time_to_critical,
        }
    }

    // ── Feature extraction ─────────────────────────────────────────────

    /// Extract 24-dimensional process features.
    fn extract_process_features(&self, proc: &ProcessTelemetry, sys: &SystemTelemetry) -> Vec<f32> {
        let total = sys.total_bytes.max(1) as f64;
        let rss_rate = self.buffer.process_rss_rate(proc.pid);
        let rss_rate_norm = (rss_rate / (total * 0.01)).tanh() as f32; // normalize: 1% of total/sec

        vec![
            // 0: log(RSS) normalized
            (proc.resident_size as f64).ln_1p() as f32 / 20.0,
            // 1: RSS / total memory
            (proc.resident_size as f64 / total) as f32,
            // 2: virtual size / total memory
            (proc.virtual_size as f64 / total) as f32,
            // 3: phys_footprint / total
            (proc.phys_footprint as f64 / total) as f32,
            // 4: compressed / RSS
            if proc.resident_size > 0 {
                (proc.compressed_bytes as f64 / proc.resident_size as f64) as f32
            } else {
                0.0
            },
            // 5: internal / RSS
            if proc.resident_size > 0 {
                (proc.internal_bytes as f64 / proc.resident_size as f64) as f32
            } else {
                0.0
            },
            // 6: external / RSS
            if proc.resident_size > 0 {
                (proc.external_bytes as f64 / proc.resident_size as f64) as f32
            } else {
                0.0
            },
            // 7: reusable / RSS
            if proc.resident_size > 0 {
                (proc.reusable_bytes as f64 / proc.resident_size as f64) as f32
            } else {
                0.0
            },
            // 8: purgeable volatile / RSS
            if proc.resident_size > 0 {
                (proc.purgeable_volatile as f64 / proc.resident_size as f64) as f32
            } else {
                0.0
            },
            // 9: graphics footprint / total
            (proc.graphics_footprint as f64 / total) as f32,
            // 10: neural footprint / total
            (proc.neural_footprint as f64 / total) as f32,
            // 11: network footprint / total
            (proc.network_footprint as f64 / total) as f32,
            // 12: media footprint / total
            (proc.media_footprint as f64 / total) as f32,
            // 13: RSS growth rate (normalized, tanh-bounded)
            rss_rate_norm,
            // 14: page faults (log normalized)
            (proc.page_faults as f64).ln_1p() as f32 / 15.0,
            // 15: pageins (log normalized)
            (proc.pageins as f64).ln_1p() as f32 / 15.0,
            // 16: cow faults (log normalized)
            (proc.cow_faults as f64).ln_1p() as f32 / 15.0,
            // 17: thread count (log normalized)
            (proc.thread_count as f64).ln_1p() as f32 / 8.0,
            // 18: priority (normalized: -20 to 20 → 0 to 1)
            (proc.priority as f32 + 20.0) / 40.0,
            // 19: is_foreground
            if proc.is_foreground { 1.0 } else { 0.0 },
            // 20: is_system
            if proc.is_system { 1.0 } else { 0.0 },
            // 21: pid (normalized: 0 to ~100000 → 0 to 1)
            (proc.pid as f32) / 100000.0,
            // 22: has compressed memory
            if proc.compressed_bytes > 0 { 1.0 } else { 0.0 },
            // 23: has purgeable memory
            if proc.purgeable_volatile > 0 {
                1.0
            } else {
                0.0
            },
        ]
    }

    /// Extract 8-dimensional CPU hardware features.
    fn extract_cpu_features(&self, sys: &SystemTelemetry) -> Vec<f32> {
        let total = sys.total_bytes.max(1) as f64;
        vec![
            // 0: wired / total
            (sys.wired_bytes as f64 / total) as f32,
            // 1: active / total
            (sys.active_bytes as f64 / total) as f32,
            // 2: free / total
            (sys.free_bytes as f64 / total) as f32,
            // 3: utilization
            sys.utilization() as f32,
            // 4: pressure level (1=normal, 2=warn, 4=critical → 0, 0.5, 1.0)
            match sys.pressure_level {
                4 => 1.0,
                2 => 0.5,
                _ => 0.0,
            },
            // 5: page fault rate (log normalized)
            (sys.faults as f64).ln_1p() as f32 / 20.0,
            // 6: cow fault rate (log normalized)
            (sys.cow_faults as f64).ln_1p() as f32 / 20.0,
            // 7: free memory rate (bytes/sec, normalized)
            (self.buffer.free_memory_rate() / (total * 0.01)).tanh() as f32,
        ]
    }

    /// Extract 8-dimensional GPU hardware features.
    fn extract_gpu_features(&self, sys: &SystemTelemetry) -> Vec<f32> {
        let total = sys.total_bytes.max(1) as f64;
        let gpu_limit = sys.gpu_working_set_limit.max(1) as f64;
        vec![
            // 0: GPU working set limit / total
            (gpu_limit / total) as f32,
            // 1: GPU wired / total
            (sys.gpu_wired_bytes as f64 / total) as f32,
            // 2: GPU wired / GPU limit
            if gpu_limit > 0.0 {
                (sys.gpu_wired_bytes as f64 / gpu_limit) as f32
            } else {
                0.0
            },
            // 3: has GPU limit
            if sys.gpu_working_set_limit > 0 {
                1.0
            } else {
                0.0
            },
            // 4: speculative / total (GPU often uses speculative pages)
            (sys.speculative_bytes as f64 / total) as f32,
            // 5: purgeable / total
            (sys.purgeable_bytes as f64 / total) as f32,
            // 6: pressure level
            match sys.pressure_level {
                4 => 1.0,
                2 => 0.5,
                _ => 0.0,
            },
            // 7: utilization
            sys.utilization() as f32,
        ]
    }

    /// Extract 8-dimensional ANE hardware features.
    fn extract_ane_features(&self, sys: &SystemTelemetry) -> Vec<f32> {
        let total = sys.total_bytes.max(1) as f64;
        vec![
            // 0: ANE not directly observable — use neural footprint proxy
            0.0,
            // 1: total / total (reference)
            1.0,
            // 2: pressure level
            match sys.pressure_level {
                4 => 1.0,
                2 => 0.5,
                _ => 0.0,
            },
            // 3: utilization
            sys.utilization() as f32,
            // 4: free / total
            (sys.free_bytes as f64 / total) as f32,
            // 5: wired / total
            (sys.wired_bytes as f64 / total) as f32,
            // 6: compressor / total
            (sys.compressor_bytes_used as f64 / total) as f32,
            // 7: swap / total
            (sys.swap_used_bytes as f64 / total) as f32,
        ]
    }

    /// Extract 6-dimensional swap features.
    fn extract_swap_features(&self, sys: &SystemTelemetry) -> Vec<f32> {
        let total = sys.swap_total_bytes.max(1) as f64;
        vec![
            // 0: swap used / swap total
            (sys.swap_used_bytes as f64 / total) as f32,
            // 1: swap total / system total
            (sys.swap_total_bytes as f64 / sys.total_bytes.max(1) as f64) as f32,
            // 2: swapins (log normalized)
            (sys.swapins as f64).ln_1p() as f32 / 15.0,
            // 3: swapouts (log normalized)
            (sys.swapouts as f64).ln_1p() as f32 / 15.0,
            // 4: has swap
            if sys.swap_total_bytes > 0 { 1.0 } else { 0.0 },
            // 5: swap pressure (used / total)
            if sys.swap_total_bytes > 0 {
                (sys.swap_used_bytes as f64 / sys.swap_total_bytes as f64) as f32
            } else {
                0.0
            },
        ]
    }

    /// Extract 6-dimensional compressor features.
    fn extract_compressor_features(&self, sys: &SystemTelemetry) -> Vec<f32> {
        let total = sys.total_bytes.max(1) as f64;
        let pool = sys.compressor_pool_size.max(1) as f64;
        vec![
            // 0: compressor used / pool size
            (sys.compressor_bytes_used as f64 / pool) as f32,
            // 1: compressor used / total
            (sys.compressor_bytes_used as f64 / total) as f32,
            // 2: compression ratio
            sys.compression_ratio() as f32,
            // 3: compressions (log normalized)
            (sys.compressions as f64).ln_1p() as f32 / 20.0,
            // 4: decompressions (log normalized)
            (sys.decompressions as f64).ln_1p() as f32 / 20.0,
            // 5: pool size / total
            (sys.compressor_pool_size as f64 / total) as f32,
        ]
    }

    // ── Heuristic pressure prediction (replaced by GNN in Phase 2) ─────

    /// Predict pressure level in 60 seconds based on current trends.
    /// Returns (pressure_0_to_2, confidence_0_to_1).
    fn predict_pressure(&self, sys: &SystemTelemetry) -> (f32, f32) {
        let _utilization = sys.utilization();
        let free_rate = self.buffer.free_memory_rate();
        let total = sys.total_bytes as f64;

        // Current pressure
        let current_pressure = match sys.pressure_level {
            4 => 2.0,
            2 => 1.0,
            _ => 0.0,
        };

        // Predicted utilization in 60s
        let predicted_free = sys.free_bytes as f64 + free_rate * 60.0;
        let predicted_free_ratio = (predicted_free / total).max(0.0);
        let predicted_utilization = 1.0 - predicted_free_ratio;

        let predicted_pressure = if predicted_utilization > 0.90 {
            2.0
        } else if predicted_utilization > 0.75 {
            1.0
        } else {
            0.0
        };

        // Confidence: higher with more history and stronger trend
        let confidence = if self.buffer.len() >= 3 {
            0.5 + (free_rate.abs() / (total * 0.01)).min(0.5)
        } else {
            0.3
        };

        // Blend current and predicted
        let blended = current_pressure * 0.4 + predicted_pressure * 0.6;
        (blended as f32, confidence.min(1.0) as f32)
    }
}

// Feature dimensions
const PROCESS_FEATURE_DIM: usize = 24;
const HARDWARE_FEATURE_DIM: usize = 8;
const SWAP_FEATURE_DIM: usize = 6;
const COMPRESSOR_FEATURE_DIM: usize = 6;

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn build_graph_from_snapshot() {
        let mut builder = GraphBuilder::new(50, true, 10);
        let snapshot = TelemetrySnapshot::collect();
        let graph = builder.build(&snapshot);

        // Should have process nodes + 3 hardware + swap + compressor
        assert!(graph.nodes.len() >= 5, "should have at least 5 nodes");
        assert!(!graph.edges.is_empty(), "should have edges");

        // Check node types
        let has_process = graph
            .nodes
            .iter()
            .any(|n| n.node_type == MemoryNodeType::Process);
        let has_hardware = graph
            .nodes
            .iter()
            .any(|n| n.node_type == MemoryNodeType::HardwareConsumer);
        assert!(has_process);
        assert!(has_hardware);
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn process_features_are_24_dim() {
        let builder = GraphBuilder::new(50, true, 10);
        let snapshot = TelemetrySnapshot::collect();
        let proc = snapshot
            .processes
            .iter()
            .find(|p| !p.is_system)
            .unwrap_or(&snapshot.processes[0]);
        let features = builder.extract_process_features(proc, &snapshot.system);
        assert_eq!(features.len(), 24);
        // All features should be finite
        for f in &features {
            assert!(f.is_finite(), "feature should be finite");
        }
    }

    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn hardware_features_are_8_dim() {
        let builder = GraphBuilder::new(50, true, 10);
        let sys = SystemTelemetry::collect();
        let cpu = builder.extract_cpu_features(&sys);
        let gpu = builder.extract_gpu_features(&sys);
        let ane = builder.extract_ane_features(&sys);
        assert_eq!(cpu.len(), 8);
        assert_eq!(gpu.len(), 8);
        assert_eq!(ane.len(), 8);
    }

    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn swap_and_compressor_features() {
        let builder = GraphBuilder::new(50, true, 10);
        let sys = SystemTelemetry::collect();
        let swap = builder.extract_swap_features(&sys);
        let comp = builder.extract_compressor_features(&sys);
        assert_eq!(swap.len(), 6);
        assert_eq!(comp.len(), 6);
    }

    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn graph_has_predictions() {
        let mut builder = GraphBuilder::new(50, true, 10);
        let snapshot = TelemetrySnapshot::collect();
        let graph = builder.build(&snapshot);
        assert!(graph.predicted_pressure_60s.is_some());
        assert!(graph.prediction_confidence.is_some());
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn graph_with_trends() {
        let mut builder = GraphBuilder::new(50, true, 10);
        // Collect multiple snapshots to build trends
        for _ in 0..3 {
            let snap = TelemetrySnapshot::collect();
            let _graph = builder.build(&snap);
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let graph = builder.build_from_buffer();
        assert!(graph.nodes.len() > 5);
    }

    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn graph_serializes_to_json() {
        let mut builder = GraphBuilder::new(20, true, 5);
        let snapshot = TelemetrySnapshot::collect();
        let graph = builder.build(&snapshot);
        let json = serde_json::to_string(&graph).unwrap();
        assert!(json.contains("process"));
        assert!(json.contains("hardware_consumer"));
        assert!(json.contains("compressor_pool"));
    }
}
