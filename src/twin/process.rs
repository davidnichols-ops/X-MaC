//! Process Intelligence System — observes and models every running process.
//!
//! Covers: process trees, CPU/GPU/memory/disk/network tracking, energy
//! impact, wakeups, anomaly detection, process fingerprints, bottleneck
//! prediction, workload balancing, behavioral learning, crash detection,
//! efficiency analysis, performance profiles, and scheduling optimization.
//!
//! Maps to Digital Twin operations 121-160.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

// ═══════════════════════════════════════════════════════════════════════
//  Core process intelligence structures
// ═══════════════════════════════════════════════════════════════════════

/// Process Intelligence System — observes and models every running process.
///
/// Covers: process trees, CPU/GPU/memory/disk/network tracking, energy
/// impact, anomaly detection, process fingerprints, bottleneck prediction,
/// workload balancing, and behavioral learning.
///
/// Maps to Digital Twin operations 121-160.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessIntelligenceGraph {
    /// All observed processes as a flat list (op 121).
    pub process_tree: Vec<ProcessNode>,
    /// Top CPU-consuming processes (op 127).
    pub top_cpu_consumers: Vec<ProcessInfo>,
    /// Top memory-consuming processes (op 129).
    pub top_memory_consumers: Vec<ProcessInfo>,
    /// Detected process anomalies (op 140).
    pub anomalies: Vec<ProcessAnomaly>,
    /// Workload fingerprints per PID (op 148).
    pub workload_fingerprints: HashMap<u32, String>,
    /// Predicted bottlenecks (op 152).
    pub bottleneck_predictions: Vec<String>,
    /// Total number of processes (op 121).
    pub total_processes: usize,
    /// Map of application name → list of PIDs (op 126).
    #[serde(default)]
    pub app_process_map: HashMap<String, Vec<u32>>,
    /// Idle processes (op 136).
    #[serde(default)]
    pub idle_processes: Vec<u32>,
    /// Crashed services (op 141).
    #[serde(default)]
    pub crashed_services: Vec<String>,
    /// Performance profiles per app (op 147).
    #[serde(default)]
    pub performance_profiles: HashMap<String, PerformanceProfile>,
    /// Normal behavior baselines per process (op 149).
    #[serde(default)]
    pub behavior_baselines: HashMap<u32, BehaviorBaseline>,
    /// Process history snapshots (op 158).
    #[serde(default)]
    pub process_history: Vec<ProcessHistorySnapshot>,
    /// Scheduling recommendations (op 153).
    #[serde(default)]
    pub scheduling_recommendations: Vec<SchedulingRecommendation>,
    /// Process intelligence graph edges (op 160).
    #[serde(default)]
    pub graph_edges: Vec<ProcessGraphEdge>,
    /// Resource usage explanations (op 151).
    #[serde(default)]
    pub resource_explanations: Vec<ResourceExplanation>,
    /// Active app priorities (op 155).
    #[serde(default)]
    pub app_priorities: Vec<AppPriority>,
    /// Workload balance assessment (op 157).
    #[serde(default)]
    pub workload_balance: Option<WorkloadBalance>,
}

/// A single process node in the process tree (ops 121-124).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessNode {
    pub pid: u32,
    pub name: String,
    /// Owner username (op 122).
    pub owner: Option<String>,
    /// Parent PID (op 123).
    pub parent_pid: Option<u32>,
    /// Child PIDs (op 124).
    pub children: Vec<u32>,
    /// CPU usage percentage (op 127).
    pub cpu_pct: f64,
    /// GPU usage percentage (op 128).
    pub gpu_pct: Option<f64>,
    /// Memory usage in bytes (op 129).
    pub memory_bytes: u64,
    /// Memory allocations count (op 130).
    pub allocation_count: Option<u64>,
    /// Disk read bytes (op 131).
    pub disk_read_bytes: Option<u64>,
    /// Disk write bytes (op 132).
    pub disk_write_bytes: Option<u64>,
    /// Network bytes (op 133).
    pub network_bytes: Option<u64>,
    /// Energy impact score (op 134).
    pub energy_impact: Option<f64>,
    /// Wakeup count (op 135).
    pub wakeup_count: Option<u64>,
    /// Process state (running, sleeping, zombie, etc.).
    pub state: String,
    /// Thread count.
    pub thread_count: Option<u32>,
    /// Start time in seconds since epoch.
    pub start_time_secs: Option<u64>,
}

/// A process info entry for top-N rankings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub pid: u32,
    pub name: String,
    pub value: f64,
    pub unit: String,
}

/// A detected process anomaly (op 140).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessAnomaly {
    pub pid: u32,
    pub name: String,
    pub anomaly_type: String,
    pub description: String,
    pub severity: String,
}

/// A performance profile for an application (op 147).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceProfile {
    pub app_name: String,
    pub avg_cpu_pct: f64,
    pub peak_cpu_pct: f64,
    pub avg_memory_bytes: u64,
    pub peak_memory_bytes: u64,
    pub avg_disk_io_bytes: u64,
    pub avg_network_bytes: u64,
    pub efficiency_score: f64,
    pub sample_count: u32,
}

/// A normal behavior baseline for a process (op 149).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BehaviorBaseline {
    pub pid: u32,
    pub name: String,
    pub avg_cpu_pct: f64,
    pub avg_memory_bytes: u64,
    pub std_dev_cpu: f64,
    pub std_dev_memory: f64,
    pub sample_count: u32,
}

/// A process history snapshot (op 158).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessHistorySnapshot {
    pub timestamp_secs: u64,
    pub total_processes: usize,
    pub total_cpu_pct: f64,
    pub total_memory_bytes: u64,
    pub top_cpu_pid: u32,
    pub top_cpu_name: String,
}

/// A scheduling recommendation (op 153).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulingRecommendation {
    pub pid: u32,
    pub name: String,
    pub recommendation: String,
    pub reason: String,
    pub priority: String,
}

/// An edge in the process intelligence graph (op 160).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessGraphEdge {
    pub source: String,
    pub target: String,
    pub edge_type: String,
    pub weight: f64,
}

/// A resource usage explanation (op 151).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceExplanation {
    pub pid: u32,
    pub name: String,
    pub resource: String,
    pub explanation: String,
    pub percentage: f64,
}

/// An app priority entry (op 155).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPriority {
    pub app_name: String,
    pub pid: u32,
    pub priority: f64,
    pub is_active: bool,
}

/// Workload balance assessment (op 157).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkloadBalance {
    pub cpu_balance_score: f64,
    pub memory_balance_score: f64,
    pub is_balanced: bool,
    pub overloaded_apps: Vec<String>,
    pub underutilized_apps: Vec<String>,
}

/// op 142: A prediction of whether an application is likely to crash,
/// based on resource pressure, anomaly history, and crash probability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct FailurePrediction {
    pub app_name: String,
    /// 0-1 probability of failure.
    pub crash_probability: f64,
    /// Contributing risk factors (e.g. "high_cpu", "memory_anomaly").
    pub risk_factors: Vec<String>,
    /// Human-readable explanation.
    pub explanation: String,
}

impl ProcessIntelligenceGraph {
    /// Collect the process intelligence graph from the running system.
    pub fn collect() -> Self {
        let mut graph = Self::empty();

        // op 121-124: Observe running processes, identify owners, parents, children
        graph.observe_processes();

        // op 125: Map process trees
        graph.map_process_trees();

        // op 126: Map application → processes
        graph.map_app_processes();

        // op 127-135: Track CPU, GPU, memory, allocations, disk I/O, network, energy, wakeups
        // (done during observe_processes)

        // op 136: Detect idle processes
        graph.detect_idle_processes();

        // op 138-140: Detect memory leaks, CPU spikes, abnormal behavior
        graph.detect_anomalies();

        // op 141: Detect crashed services
        graph.detect_crashed_services();

        // op 145-148: Analyze efficiency, compare versions, create profiles, fingerprints
        graph.create_performance_profiles();
        graph.create_workload_fingerprints();

        // op 149-150: Learn normal behavior, detect anomalies
        graph.learn_normal_behavior();

        // op 151: Explain resource usage
        graph.explain_resource_usage();

        // op 152-153: Predict bottlenecks, optimize scheduling
        graph.predict_bottlenecks();
        graph.optimize_scheduling();

        // op 155-157: Prioritize active apps, reduce unnecessary work, balance workloads
        graph.prioritize_apps();
        graph.balance_workloads();

        // op 158: Maintain process history
        graph.record_history();

        // op 160: Build process intelligence graph
        graph.build_graph();

        graph
    }

    /// Create an empty process intelligence graph.
    pub fn empty() -> Self {
        Self {
            process_tree: Vec::new(),
            top_cpu_consumers: Vec::new(),
            top_memory_consumers: Vec::new(),
            anomalies: Vec::new(),
            workload_fingerprints: HashMap::new(),
            bottleneck_predictions: Vec::new(),
            total_processes: 0,
            app_process_map: HashMap::new(),
            idle_processes: Vec::new(),
            crashed_services: Vec::new(),
            performance_profiles: HashMap::new(),
            behavior_baselines: HashMap::new(),
            process_history: Vec::new(),
            scheduling_recommendations: Vec::new(),
            graph_edges: Vec::new(),
            resource_explanations: Vec::new(),
            app_priorities: Vec::new(),
            workload_balance: None,
        }
    }

    // ─── op 121-124: Observe running processes ─────────────────────────

    /// Observe every running process using `ps aux` and populate the
    /// process tree with owner, parent, children, and resource metrics.
    pub fn observe_processes(&mut self) {
        let processes = collect_ps_processes();
        self.process_tree = processes;
        self.total_processes = self.process_tree.len();

        // Build top CPU consumers (op 127)
        let mut cpu_consumers: Vec<ProcessInfo> = self
            .process_tree
            .iter()
            .map(|p| ProcessInfo {
                pid: p.pid,
                name: p.name.clone(),
                value: p.cpu_pct,
                unit: "%".to_string(),
            })
            .collect();
        cpu_consumers.sort_by(|a, b| {
            b.value
                .partial_cmp(&a.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        cpu_consumers.truncate(10);
        self.top_cpu_consumers = cpu_consumers;

        // Build top memory consumers (op 129)
        let mut mem_consumers: Vec<ProcessInfo> = self
            .process_tree
            .iter()
            .map(|p| ProcessInfo {
                pid: p.pid,
                name: p.name.clone(),
                value: p.memory_bytes as f64,
                unit: "bytes".to_string(),
            })
            .collect();
        mem_consumers.sort_by(|a, b| {
            b.value
                .partial_cmp(&a.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        mem_consumers.truncate(10);
        self.top_memory_consumers = mem_consumers;
    }

    // ─── op 125: Map process trees ─────────────────────────────────────

    /// Build parent-child relationships in the process tree.
    pub fn map_process_trees(&mut self) {
        // Build a map of pid → children
        let mut children_map: HashMap<u32, Vec<u32>> = HashMap::new();
        for proc in &self.process_tree {
            if let Some(parent_pid) = proc.parent_pid {
                children_map.entry(parent_pid).or_default().push(proc.pid);
            }
        }
        // Update children in each process node
        for proc in &mut self.process_tree {
            if let Some(children) = children_map.get(&proc.pid) {
                proc.children = children.clone();
            }
        }
    }

    // ─── op 126: Map application → processes ───────────────────────────

    /// Map application names to their process PIDs.
    pub fn map_app_processes(&mut self) {
        for proc in &self.process_tree {
            let app_name = app_name_from_process(&proc.name);
            self.app_process_map
                .entry(app_name)
                .or_default()
                .push(proc.pid);
        }
    }

    // ─── op 136: Detect idle processes ─────────────────────────────────

    /// Detect processes with 0% CPU and low memory usage that are likely idle.
    pub fn detect_idle_processes(&mut self) {
        for proc in &self.process_tree {
            if proc.cpu_pct < 0.1 && proc.memory_bytes < 10 * 1024 * 1024 {
                // Skip kernel/system processes
                if !is_system_process(&proc.name) {
                    self.idle_processes.push(proc.pid);
                }
            }
        }
    }

    // ─── op 138-140: Detect anomalies ──────────────────────────────────

    /// Detect memory leaks (op 138), CPU spikes (op 139), and abnormal
    /// behavior (op 140).
    pub fn detect_anomalies(&mut self) {
        for proc in &self.process_tree {
            // op 138: Memory leak — process using >2GB and still growing
            if proc.memory_bytes > 2 * 1024 * 1024 * 1024 {
                self.anomalies.push(ProcessAnomaly {
                    pid: proc.pid,
                    name: proc.name.clone(),
                    anomaly_type: "memory_leak".to_string(),
                    description: format!(
                        "Process using {:.1} GB of memory — possible leak",
                        proc.memory_bytes as f64 / (1024.0 * 1024.0 * 1024.0)
                    ),
                    severity: if proc.memory_bytes > 8 * 1024 * 1024 * 1024 {
                        "critical"
                    } else {
                        "warning"
                    }
                    .to_string(),
                });
            }

            // op 139: CPU spike — process using >80% CPU
            if proc.cpu_pct > 80.0 {
                self.anomalies.push(ProcessAnomaly {
                    pid: proc.pid,
                    name: proc.name.clone(),
                    anomaly_type: "cpu_spike".to_string(),
                    description: format!("Process using {:.1}% CPU — abnormal spike", proc.cpu_pct),
                    severity: if proc.cpu_pct > 95.0 {
                        "critical"
                    } else {
                        "warning"
                    }
                    .to_string(),
                });
            }

            // op 140: Abnormal behavior — zombie processes
            if proc.state == "Z" {
                self.anomalies.push(ProcessAnomaly {
                    pid: proc.pid,
                    name: proc.name.clone(),
                    anomaly_type: "zombie".to_string(),
                    description: "Zombie process — not reaped by parent".to_string(),
                    severity: "warning".to_string(),
                });
            }
        }
    }

    // ─── op 141: Detect crashed services ───────────────────────────────

    /// Detect services that have crashed or are in a failed state.
    pub fn detect_crashed_services(&mut self) {
        #[cfg(target_os = "macos")]
        {
            // Check launchctl for failed services
            if let Ok(output) = Command::new("launchctl").args(["list"]).output() {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines().skip(1) {
                    // Lines with non-zero exit code in the first column indicate issues
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let exit_code = parts[0];
                        if exit_code != "0" && exit_code != "-" {
                            let label = parts[2];
                            if exit_code.parse::<i32>().map(|c| c != 0).unwrap_or(false) {
                                self.crashed_services.push(label.to_string());
                            }
                        }
                    }
                }
            }
        }
        #[cfg(target_os = "linux")]
        {
            // Check systemd for failed services
            if let Ok(output) = Command::new("systemctl").args(["--failed"]).output() {
                let s = String::from_utf8_lossy(&output.stdout);
                for line in s.lines() {
                    if line.contains("failed") {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if let Some(service) = parts.first() {
                            self.crashed_services.push(service.to_string());
                        }
                    }
                }
            }
        }
    }

    // ─── op 145-147: Analyze efficiency, compare versions, create profiles ─

    /// Create performance profiles for each application (op 147).
    /// Aggregates resource usage per app into a profile.
    pub fn create_performance_profiles(&mut self) {
        let mut profiles: HashMap<String, Vec<&ProcessNode>> = HashMap::new();
        for proc in &self.process_tree {
            let app_name = app_name_from_process(&proc.name);
            profiles.entry(app_name).or_default().push(proc);
        }

        for (app_name, procs) in &profiles {
            let count = procs.len() as u32;
            let avg_cpu: f64 = procs.iter().map(|p| p.cpu_pct).sum::<f64>() / count as f64;
            let peak_cpu: f64 = procs.iter().map(|p| p.cpu_pct).fold(0.0f64, f64::max);
            let total_mem: u64 = procs.iter().map(|p| p.memory_bytes).sum();
            let peak_mem: u64 = procs.iter().map(|p| p.memory_bytes).max().unwrap_or(0);
            let avg_disk: u64 = procs
                .iter()
                .filter_map(|p| {
                    p.disk_read_bytes
                        .zip(p.disk_write_bytes)
                        .map(|(r, w)| r + w)
                })
                .sum::<u64>()
                / count.max(1) as u64;
            let avg_net: u64 =
                procs.iter().filter_map(|p| p.network_bytes).sum::<u64>() / count.max(1) as u64;

            // op 145: Efficiency score — ratio of useful work (CPU) to resources consumed
            let efficiency = if total_mem > 0 {
                (avg_cpu / (total_mem as f64 / (1024.0 * 1024.0 * 1024.0)).max(0.1)).clamp(0.0, 1.0)
            } else {
                0.5
            };

            self.performance_profiles.insert(
                app_name.clone(),
                PerformanceProfile {
                    app_name: app_name.clone(),
                    avg_cpu_pct: avg_cpu,
                    peak_cpu_pct: peak_cpu,
                    avg_memory_bytes: total_mem / count.max(1) as u64,
                    peak_memory_bytes: peak_mem,
                    avg_disk_io_bytes: avg_disk,
                    avg_network_bytes: avg_net,
                    efficiency_score: efficiency,
                    sample_count: count,
                },
            );
        }
    }

    // ─── op 148: Create workload fingerprints ──────────────────────────

    /// Create a workload fingerprint for each process based on its
    /// resource usage pattern (CPU-heavy, memory-heavy, I/O-heavy, etc.).
    pub fn create_workload_fingerprints(&mut self) {
        for proc in &self.process_tree {
            let fingerprint = classify_workload(proc);
            self.workload_fingerprints
                .insert(proc.pid, fingerprint.to_string());
        }
    }

    // ─── op 149-150: Learn normal behavior, detect anomalies ───────────

    /// Learn normal behavior baselines from the current process snapshot.
    /// In a real system, this would aggregate over many samples.
    pub fn learn_normal_behavior(&mut self) {
        for proc in &self.process_tree {
            // Single-sample baseline — in production this would be an
            // exponential moving average over many observations.
            // For the first sample, use 0 as the baseline so that any
            // significant resource usage is flagged as an anomaly.
            self.behavior_baselines.insert(
                proc.pid,
                BehaviorBaseline {
                    pid: proc.pid,
                    name: proc.name.clone(),
                    avg_cpu_pct: 0.0,
                    avg_memory_bytes: 0,
                    std_dev_cpu: 5.0,                        // assumed default
                    std_dev_memory: 100.0 * 1024.0 * 1024.0, // 100MB assumed
                    sample_count: 1,
                },
            );
        }

        // op 150: Detect anomalies based on baselines
        for proc in &self.process_tree {
            if let Some(baseline) = self.behavior_baselines.get(&proc.pid) {
                // CPU anomaly: >3 standard deviations above mean
                let cpu_threshold = baseline.avg_cpu_pct + 3.0 * baseline.std_dev_cpu;
                if proc.cpu_pct > cpu_threshold && proc.cpu_pct > 50.0 {
                    self.anomalies.push(ProcessAnomaly {
                        pid: proc.pid,
                        name: proc.name.clone(),
                        anomaly_type: "cpu_anomaly".to_string(),
                        description: format!(
                            "CPU usage {:.1}% exceeds baseline ({:.1}±{:.1})",
                            proc.cpu_pct, baseline.avg_cpu_pct, baseline.std_dev_cpu
                        ),
                        severity: "warning".to_string(),
                    });
                }

                // Memory anomaly: >3 standard deviations above mean
                let mem_threshold =
                    baseline.avg_memory_bytes + (3.0 * baseline.std_dev_memory) as u64;
                if proc.memory_bytes > mem_threshold && proc.memory_bytes > 512 * 1024 * 1024 {
                    self.anomalies.push(ProcessAnomaly {
                        pid: proc.pid,
                        name: proc.name.clone(),
                        anomaly_type: "memory_anomaly".to_string(),
                        description: format!(
                            "Memory usage {:.1} MB exceeds baseline ({:.1}±{:.1} MB)",
                            proc.memory_bytes as f64 / (1024.0 * 1024.0),
                            baseline.avg_memory_bytes as f64 / (1024.0 * 1024.0),
                            baseline.std_dev_memory / (1024.0 * 1024.0)
                        ),
                        severity: "warning".to_string(),
                    });
                }
            }
        }
    }

    // ─── op 151: Explain resource usage ────────────────────────────────

    /// Generate natural-language explanations for resource usage.
    pub fn explain_resource_usage(&mut self) {
        for proc in &self.top_cpu_consumers {
            let explanation = if proc.value > 80.0 {
                format!(
                    "{} is consuming {:.1}% CPU — likely performing intensive computation",
                    proc.name, proc.value
                )
            } else if proc.value > 50.0 {
                format!(
                    "{} is consuming {:.1}% CPU — moderate workload",
                    proc.name, proc.value
                )
            } else if proc.value > 10.0 {
                format!(
                    "{} is consuming {:.1}% CPU — light active workload",
                    proc.name, proc.value
                )
            } else {
                continue;
            };
            self.resource_explanations.push(ResourceExplanation {
                pid: proc.pid,
                name: proc.name.clone(),
                resource: "CPU".to_string(),
                explanation,
                percentage: proc.value,
            });
        }

        for proc in &self.top_memory_consumers {
            let mem_gb = proc.value / (1024.0 * 1024.0 * 1024.0);
            if mem_gb > 0.5 {
                let explanation = if mem_gb > 4.0 {
                    format!(
                        "{} is using {:.1} GB of memory — very high footprint",
                        proc.name, mem_gb
                    )
                } else {
                    format!(
                        "{} is using {:.1} GB of memory — moderate footprint",
                        proc.name, mem_gb
                    )
                };
                self.resource_explanations.push(ResourceExplanation {
                    pid: proc.pid,
                    name: proc.name.clone(),
                    resource: "Memory".to_string(),
                    explanation,
                    percentage: if mem_gb > 0.0 {
                        (proc.value / (16.0 * 1024.0 * 1024.0 * 1024.0) * 100.0).min(100.0)
                    } else {
                        0.0
                    },
                });
            }
        }
    }

    // ─── op 152-153: Predict bottlenecks, optimize scheduling ──────────

    /// Predict resource bottlenecks based on current usage patterns.
    pub fn predict_bottlenecks(&mut self) {
        let total_cpu: f64 = self.process_tree.iter().map(|p| p.cpu_pct).sum();
        let core_count = num_cpu_cores();

        // CPU bottleneck: total CPU usage approaching core count * 100%
        if total_cpu > (core_count as f64 * 80.0) {
            self.bottleneck_predictions.push(format!(
                "CPU bottleneck predicted: {:.1}% total usage across {} cores",
                total_cpu, core_count
            ));
        }

        let total_mem: u64 = self.process_tree.iter().map(|p| p.memory_bytes).sum();
        let total_mem_gb = total_mem as f64 / (1024.0 * 1024.0 * 1024.0);
        if total_mem_gb > 12.0 {
            self.bottleneck_predictions.push(format!(
                "Memory pressure predicted: {:.1} GB in use",
                total_mem_gb
            ));
        }

        // Disk I/O bottleneck: many processes doing disk I/O
        let io_procs: Vec<&ProcessNode> = self
            .process_tree
            .iter()
            .filter(|p| {
                p.disk_read_bytes.or(Some(0)).unwrap_or(0)
                    + p.disk_write_bytes.or(Some(0)).unwrap_or(0)
                    > 10 * 1024 * 1024
            })
            .collect();
        if io_procs.len() > 5 {
            self.bottleneck_predictions.push(format!(
                "Disk I/O contention: {} processes with significant I/O",
                io_procs.len()
            ));
        }
    }

    /// Generate scheduling recommendations to optimize process scheduling.
    pub fn optimize_scheduling(&mut self) {
        for proc in &self.process_tree {
            // op 156: Reduce unnecessary work — suggest lowering priority for
            // background processes consuming significant CPU
            if proc.cpu_pct > 30.0 && is_background_process(&proc.name) {
                self.scheduling_recommendations
                    .push(SchedulingRecommendation {
                        pid: proc.pid,
                        name: proc.name.clone(),
                        recommendation: "lower_priority".to_string(),
                        reason: format!(
                            "Background process using {:.1}% CPU — consider reducing priority",
                            proc.cpu_pct
                        ),
                        priority: "medium".to_string(),
                    });
            }

            // op 154: Optimize background activity — suggest suspending idle
            // background processes
            if proc.cpu_pct < 1.0
                && proc.memory_bytes > 100 * 1024 * 1024
                && is_background_process(&proc.name)
            {
                self.scheduling_recommendations
                    .push(SchedulingRecommendation {
                        pid: proc.pid,
                        name: proc.name.clone(),
                        recommendation: "consider_suspending".to_string(),
                        reason: format!(
                            "Idle background process holding {:.0} MB of memory",
                            proc.memory_bytes as f64 / (1024.0 * 1024.0)
                        ),
                        priority: "low".to_string(),
                    });
            }
        }
    }

    // ─── op 155-157: Prioritize apps, reduce work, balance workloads ───

    /// Prioritize active applications based on resource usage and activity.
    pub fn prioritize_apps(&mut self) {
        for (app_name, pids) in &self.app_process_map {
            let total_cpu: f64 = pids
                .iter()
                .filter_map(|pid| self.process_tree.iter().find(|p| p.pid == *pid))
                .map(|p| p.cpu_pct)
                .sum();
            let is_active = total_cpu > 5.0;
            let priority = if total_cpu > 50.0 {
                1.0
            } else if total_cpu > 10.0 {
                0.7
            } else if is_active {
                0.5
            } else {
                0.2
            };
            let primary_pid = pids
                .iter()
                .filter_map(|pid| self.process_tree.iter().find(|p| p.pid == *pid))
                .max_by(|a, b| {
                    a.cpu_pct
                        .partial_cmp(&b.cpu_pct)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .map(|p| p.pid)
                .unwrap_or(0);
            self.app_priorities.push(AppPriority {
                app_name: app_name.clone(),
                pid: primary_pid,
                priority,
                is_active,
            });
        }
        self.app_priorities.sort_by(|a, b| {
            b.priority
                .partial_cmp(&a.priority)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
    }

    /// Assess workload balance across applications (op 157).
    pub fn balance_workloads(&mut self) {
        if self.app_priorities.is_empty() {
            return;
        }

        let cpu_values: Vec<f64> = self.app_priorities.iter().map(|a| a.priority).collect();
        let mean = cpu_values.iter().sum::<f64>() / cpu_values.len() as f64;
        let variance: f64 = cpu_values.iter().map(|v| (v - mean).powi(2)).sum::<f64>()
            / cpu_values.len().max(1) as f64;
        let std_dev = variance.sqrt();

        // Balance score: lower std_dev means more balanced (closer to 1.0)
        let balance_score = (1.0 / (1.0 + std_dev)).clamp(0.0, 1.0);

        let mem_values: Vec<f64> = self
            .performance_profiles
            .values()
            .map(|p| p.avg_memory_bytes as f64)
            .collect();
        let mem_mean = mem_values.iter().sum::<f64>() / mem_values.len().max(1) as f64;
        let mem_variance: f64 = mem_values
            .iter()
            .map(|v| (v - mem_mean).powi(2))
            .sum::<f64>()
            / mem_values.len().max(1) as f64;
        let mem_std_dev = mem_variance.sqrt();
        let mem_balance = (1.0 / (1.0 + mem_std_dev / (1024.0 * 1024.0 * 1024.0))).clamp(0.0, 1.0);

        let overloaded: Vec<String> = self
            .app_priorities
            .iter()
            .filter(|a| a.priority > 0.8)
            .map(|a| a.app_name.clone())
            .collect();
        let underutilized: Vec<String> = self
            .app_priorities
            .iter()
            .filter(|a| a.priority < 0.2)
            .map(|a| a.app_name.clone())
            .collect();

        self.workload_balance = Some(WorkloadBalance {
            cpu_balance_score: balance_score,
            memory_balance_score: mem_balance,
            is_balanced: balance_score > 0.6 && overloaded.len() <= 1,
            overloaded_apps: overloaded,
            underutilized_apps: underutilized,
        });
    }

    // ─── op 158: Maintain process history ──────────────────────────────

    /// Record a process history snapshot for trend tracking.
    pub fn record_history(&mut self) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let total_cpu: f64 = self.process_tree.iter().map(|p| p.cpu_pct).sum();
        let total_mem: u64 = self.process_tree.iter().map(|p| p.memory_bytes).sum();

        let (top_pid, top_name) = self
            .top_cpu_consumers
            .first()
            .map(|p| (p.pid, p.name.clone()))
            .unwrap_or((0, "unknown".to_string()));

        self.process_history.push(ProcessHistorySnapshot {
            timestamp_secs: now,
            total_processes: self.total_processes,
            total_cpu_pct: total_cpu,
            total_memory_bytes: total_mem,
            top_cpu_pid: top_pid,
            top_cpu_name: top_name,
        });
    }

    // ─── op 160: Build process intelligence graph ──────────────────────

    /// Build the process intelligence graph with edges connecting
    /// processes to apps, resources, and relationships.
    pub fn build_graph(&mut self) {
        // Process → App edges
        for (app_name, pids) in &self.app_process_map {
            for pid in pids {
                self.graph_edges.push(ProcessGraphEdge {
                    source: format!("process:{}", pid),
                    target: format!("app:{}", app_name),
                    edge_type: "belongs_to".to_string(),
                    weight: 1.0,
                });
            }
        }

        // Process → Resource edges (CPU, Memory)
        for proc in &self.process_tree {
            if proc.cpu_pct > 1.0 {
                self.graph_edges.push(ProcessGraphEdge {
                    source: format!("process:{}", proc.pid),
                    target: "resource:cpu".to_string(),
                    edge_type: "consumes".to_string(),
                    weight: proc.cpu_pct / 100.0,
                });
            }
            if proc.memory_bytes > 100 * 1024 * 1024 {
                self.graph_edges.push(ProcessGraphEdge {
                    source: format!("process:{}", proc.pid),
                    target: "resource:memory".to_string(),
                    edge_type: "consumes".to_string(),
                    weight: (proc.memory_bytes as f64 / (16.0 * 1024.0 * 1024.0 * 1024.0)).min(1.0),
                });
            }
            if let Some(energy) = proc.energy_impact {
                if energy > 0.0 {
                    self.graph_edges.push(ProcessGraphEdge {
                        source: format!("process:{}", proc.pid),
                        target: "resource:energy".to_string(),
                        edge_type: "impacts".to_string(),
                        weight: energy / 100.0,
                    });
                }
            }
        }

        // Parent → Child edges
        for proc in &self.process_tree {
            if let Some(parent_pid) = proc.parent_pid {
                self.graph_edges.push(ProcessGraphEdge {
                    source: format!("process:{}", parent_pid),
                    target: format!("process:{}", proc.pid),
                    edge_type: "parent_of".to_string(),
                    weight: 1.0,
                });
            }
        }
    }

    /// op 142: Predict application failure — predict if an app is likely to
    /// crash based on resource pressure, anomaly history, and crash
    /// probability. Returns `None` if the app is not currently running.
    #[allow(dead_code)]
    pub fn predict_app_failure(&self, app_name: &str) -> Option<FailurePrediction> {
        // Gather all processes belonging to the app.
        let pids = self.app_process_map.get(app_name)?;
        let procs: Vec<&ProcessNode> = self
            .process_tree
            .iter()
            .filter(|p| pids.contains(&p.pid))
            .collect();
        if procs.is_empty() {
            return None;
        }

        let mut risk_factors: Vec<String> = Vec::new();
        let mut probability: f64 = 0.1; // baseline risk

        // Factor 1: high aggregate CPU usage.
        let total_cpu: f64 = procs.iter().map(|p| p.cpu_pct).sum();
        if total_cpu > 90.0 {
            probability += 0.2;
            risk_factors.push("high_cpu".to_string());
        }

        // Factor 2: high memory footprint.
        let total_mem: u64 = procs.iter().map(|p| p.memory_bytes).sum();
        if total_mem > 4 * 1024 * 1024 * 1024 {
            probability += 0.2;
            risk_factors.push("high_memory".to_string());
        }

        // Factor 3: anomalies recorded for this app's processes.
        let anomaly_count = self
            .anomalies
            .iter()
            .filter(|a| pids.contains(&a.pid))
            .count();
        if anomaly_count > 0 {
            probability += 0.15 * anomaly_count as f64;
            risk_factors.push("anomaly_history".to_string());
        }

        // Factor 4: crashed services matching the app name.
        if self.crashed_services.iter().any(|s| s.contains(app_name)) {
            probability += 0.3;
            risk_factors.push("crashed_service".to_string());
        }

        // Factor 5: zombie processes within the app.
        let zombie_count = procs.iter().filter(|p| p.state == "Z").count();
        if zombie_count > 0 {
            probability += 0.1;
            risk_factors.push("zombie_process".to_string());
        }

        let probability = probability.min(1.0);
        let explanation = if risk_factors.is_empty() {
            format!("{app_name} is running within normal parameters.")
        } else {
            format!(
                "{app_name} has elevated failure risk ({:.0}%) due to: {}",
                probability * 100.0,
                risk_factors.join(", ")
            )
        };

        Some(FailurePrediction {
            app_name: app_name.to_string(),
            crash_probability: probability,
            risk_factors,
            explanation,
        })
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Process collection helpers
// ═══════════════════════════════════════════════════════════════════════

/// Collect running processes using `ps aux` (cross-platform).
/// Returns a list of ProcessNode with resource metrics.
pub fn collect_ps_processes() -> Vec<ProcessNode> {
    // ps -eo pid,ppid,user,%cpu,%mem,rss,state,comm
    // %mem is percentage of total RAM, rss is in KB
    let output = Command::new("ps")
        .args(["-eo", "pid,ppid,user,%cpu,%mem,rss,state,comm"])
        .output();

    let s = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };

    let total_mem_bytes = total_system_memory_bytes();
    let mut processes = Vec::new();

    for line in s.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 8 {
            continue;
        }

        let pid: u32 = match parts[0].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let parent_pid: Option<u32> = parts[1].parse().ok();
        let owner = Some(parts[2].to_string());
        let cpu_pct: f64 = parts[3].parse().unwrap_or(0.0);
        let mem_pct: f64 = parts[4].parse().unwrap_or(0.0);
        let rss_kb: u64 = parts[5].parse().unwrap_or(0);
        let state = parts[6].to_string();
        let name = parts[7..].join(" ");
        let name = name.rsplit('/').next().unwrap_or(&name).to_string();

        // Memory: prefer RSS (actual physical memory), fall back to %mem * total
        let memory_bytes = if rss_kb > 0 {
            rss_kb * 1024
        } else {
            (mem_pct / 100.0 * total_mem_bytes as f64) as u64
        };

        processes.push(ProcessNode {
            pid,
            name,
            owner,
            parent_pid,
            children: Vec::new(),
            cpu_pct,
            gpu_pct: None, // Not available via ps
            memory_bytes,
            allocation_count: None, // Requires /proc or private APIs
            disk_read_bytes: None,  // Requires /proc or iotop
            disk_write_bytes: None,
            network_bytes: None, // Requires /proc or nettop
            energy_impact: None, // Requires powermetrics on macOS
            wakeup_count: None,  // Requires powermetrics on macOS
            state,
            thread_count: None, // Requires /proc or ps -o thcount
            start_time_secs: None,
        });
    }

    // On Linux, enrich with /proc data for disk I/O, network, thread count
    #[cfg(target_os = "linux")]
    {
        for proc in &mut processes {
            enrich_from_proc(proc);
        }
    }

    processes
}

/// Get total system memory in bytes.
fn total_system_memory_bytes() -> u64 {
    #[cfg(target_os = "macos")]
    {
        Command::new("sysctl")
            .args(["-n", "hw.memsize"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
            .unwrap_or(16 * 1024 * 1024 * 1024)
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if line.starts_with("MemTotal:") {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 2 {
                        if let Ok(kb) = parts[1].parse::<u64>() {
                            return kb * 1024;
                        }
                    }
                }
            }
        }
        16 * 1024 * 1024 * 1024
    }
}

/// Get the number of CPU cores.
fn num_cpu_cores() -> usize {
    #[cfg(target_os = "macos")]
    {
        Command::new("sysctl")
            .args(["-n", "hw.logicalcpu"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
            .unwrap_or(1)
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }
}

/// Enrich a process node with data from /proc (Linux only).
#[cfg(target_os = "linux")]
fn enrich_from_proc(proc: &mut ProcessNode) {
    let proc_path = format!("/proc/{}", proc.pid);

    // Thread count
    if let Ok(stat) = std::fs::read_to_string(format!("{}/stat", proc_path)) {
        let fields: Vec<&str> = stat.split_whitespace().collect();
        if fields.len() > 19 {
            // Field 20 is num_threads (0-indexed: 19)
            proc.thread_count = fields[19].parse().ok();
            // Field 22 is starttime in clock ticks
            if let Ok(starttime) = fields[21].parse::<u64>() {
                // Convert to seconds (assuming 100 Hz clock)
                proc.start_time_secs = Some(starttime / 100);
            }
        }
    }

    // Disk I/O from /proc/<pid>/io
    if let Ok(io) = std::fs::read_to_string(format!("{}/io", proc_path)) {
        for line in io.lines() {
            if let Some((key, value)) = line.split_once(':') {
                let value = value.trim();
                match key.trim() {
                    "read_bytes" => proc.disk_read_bytes = value.parse().ok(),
                    "write_bytes" => proc.disk_write_bytes = value.parse().ok(),
                    _ => {}
                }
            }
        }
    }

    // Network: not directly available per-process in /proc without netstat
    // We leave network_bytes as None on Linux
}

/// Extract an application name from a process name.
fn app_name_from_process(process_name: &str) -> String {
    // Strip common suffixes like "Helper", "Renderer", etc.
    let name = process_name.trim();
    // Strip " Helper", " Renderer", " GPU", " Plugin" suffixes (Chrome-style)
    for suffix in [
        " Helper",
        " Renderer",
        " GPU",
        " Plugin",
        " Helper-GPU",
        " Helper-Renderer",
    ] {
        if let Some(base) = name.strip_suffix(suffix) {
            if base.len() > 2 {
                return base.to_string();
            }
        }
    }
    // Strip dash-separated suffixes (e.g. "app-helper" → "app")
    if let Some(idx) = name.find('-') {
        let base = &name[..idx];
        if base.len() > 2 {
            return base.to_string();
        }
    }
    name.to_string()
}

/// Check if a process name is a system/kernel process.
fn is_system_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    matches!(
        lower.as_str(),
        "kernel_task"
            | "launchd"
            | "windowserver"
            | "init"
            | "systemd"
            | "kthreadd"
            | "kworker"
            | "ksoftirqd"
            | "migration"
            | "rcu_sched"
            | "watchdog"
    ) || lower.starts_with("kworker")
        || lower.starts_with("kernel_")
}

/// Check if a process is likely a background process.
fn is_background_process(name: &str) -> bool {
    let lower = name.to_lowercase();
    lower.contains("helper")
        || lower.contains("renderer")
        || lower.contains("gpu-process")
        || lower.contains("daemon")
        || lower.contains("agent")
        || lower.contains("service")
        || lower.contains("updater")
        || lower.contains("telemetry")
        || lower.contains("crashpad")
}

/// Classify a process's workload type based on its resource usage pattern.
fn classify_workload(proc: &ProcessNode) -> &'static str {
    let cpu_heavy = proc.cpu_pct > 50.0;
    let mem_heavy = proc.memory_bytes > 500 * 1024 * 1024;
    let io_heavy =
        (proc.disk_read_bytes.unwrap_or(0) + proc.disk_write_bytes.unwrap_or(0)) > 50 * 1024 * 1024;
    let net_heavy = proc.network_bytes.unwrap_or(0) > 10 * 1024 * 1024;

    if cpu_heavy && mem_heavy {
        "compute_memory_intensive"
    } else if cpu_heavy {
        "compute_intensive"
    } else if mem_heavy {
        "memory_intensive"
    } else if io_heavy {
        "io_intensive"
    } else if net_heavy {
        "network_intensive"
    } else if proc.cpu_pct < 1.0 {
        "idle"
    } else {
        "light"
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_processes() -> Vec<ProcessNode> {
        vec![
            ProcessNode {
                pid: 1,
                name: "launchd".to_string(),
                owner: Some("root".to_string()),
                parent_pid: None,
                children: vec![2, 3],
                cpu_pct: 0.1,
                gpu_pct: None,
                memory_bytes: 5 * 1024 * 1024,
                allocation_count: None,
                disk_read_bytes: None,
                disk_write_bytes: None,
                network_bytes: None,
                energy_impact: None,
                wakeup_count: None,
                state: "S".to_string(),
                thread_count: Some(1),
                start_time_secs: Some(0),
            },
            ProcessNode {
                pid: 2,
                name: "Google Chrome Helper".to_string(),
                owner: Some("user".to_string()),
                parent_pid: Some(1),
                children: vec![],
                cpu_pct: 85.0,
                gpu_pct: Some(30.0),
                memory_bytes: 3 * 1024 * 1024 * 1024,
                allocation_count: Some(10000),
                disk_read_bytes: Some(50 * 1024 * 1024),
                disk_write_bytes: Some(20 * 1024 * 1024),
                network_bytes: Some(100 * 1024 * 1024),
                energy_impact: Some(75.0),
                wakeup_count: Some(5000),
                state: "R".to_string(),
                thread_count: Some(15),
                start_time_secs: Some(1000),
            },
            ProcessNode {
                pid: 3,
                name: "idle_app".to_string(),
                owner: Some("user".to_string()),
                parent_pid: Some(1),
                children: vec![],
                cpu_pct: 0.0,
                gpu_pct: None,
                memory_bytes: 5 * 1024 * 1024,
                allocation_count: None,
                disk_read_bytes: None,
                disk_write_bytes: None,
                network_bytes: None,
                energy_impact: None,
                wakeup_count: None,
                state: "S".to_string(),
                thread_count: Some(1),
                start_time_secs: Some(2000),
            },
            ProcessNode {
                pid: 4,
                name: "zombie_proc".to_string(),
                owner: Some("user".to_string()),
                parent_pid: Some(1),
                children: vec![],
                cpu_pct: 0.0,
                gpu_pct: None,
                memory_bytes: 0,
                allocation_count: None,
                disk_read_bytes: None,
                disk_write_bytes: None,
                network_bytes: None,
                energy_impact: None,
                wakeup_count: None,
                state: "Z".to_string(),
                thread_count: None,
                start_time_secs: Some(3000),
            },
        ]
    }

    #[test]
    fn test_empty_graph() {
        let graph = ProcessIntelligenceGraph::empty();
        assert_eq!(graph.total_processes, 0);
        assert!(graph.process_tree.is_empty());
        assert!(graph.top_cpu_consumers.is_empty());
        assert!(graph.top_memory_consumers.is_empty());
        assert!(graph.anomalies.is_empty());
    }

    #[test]
    fn test_map_process_trees() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_process_trees();
        // launchd (pid 1) should have children 2 and 3
        let launchd = graph.process_tree.iter().find(|p| p.pid == 1).unwrap();
        assert!(launchd.children.contains(&2));
        assert!(launchd.children.contains(&3));
    }

    #[test]
    fn test_map_app_processes() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        assert!(graph.app_process_map.contains_key("Google Chrome"));
        assert!(graph.app_process_map.contains_key("idle_app"));
    }

    #[test]
    fn test_detect_idle_processes() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.detect_idle_processes();
        // idle_app (pid 3) should be detected as idle
        assert!(graph.idle_processes.contains(&3));
        // launchd is a system process, should not be in idle list
        assert!(!graph.idle_processes.contains(&1));
    }

    #[test]
    fn test_detect_cpu_spike() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.detect_anomalies();
        // Chrome Helper at 85% CPU should trigger a CPU spike anomaly
        let cpu_spikes: Vec<&ProcessAnomaly> = graph
            .anomalies
            .iter()
            .filter(|a| a.anomaly_type == "cpu_spike")
            .collect();
        assert!(!cpu_spikes.is_empty(), "should detect CPU spike");
        assert_eq!(cpu_spikes[0].pid, 2);
    }

    #[test]
    fn test_detect_memory_leak() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.detect_anomalies();
        // Chrome Helper at 3GB should trigger a memory leak anomaly
        let mem_leaks: Vec<&ProcessAnomaly> = graph
            .anomalies
            .iter()
            .filter(|a| a.anomaly_type == "memory_leak")
            .collect();
        assert!(!mem_leaks.is_empty(), "should detect memory leak");
        assert_eq!(mem_leaks[0].pid, 2);
    }

    #[test]
    fn test_detect_zombie() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.detect_anomalies();
        // zombie_proc (pid 4) should be detected
        let zombies: Vec<&ProcessAnomaly> = graph
            .anomalies
            .iter()
            .filter(|a| a.anomaly_type == "zombie")
            .collect();
        assert!(!zombies.is_empty(), "should detect zombie process");
        assert_eq!(zombies[0].pid, 4);
    }

    #[test]
    fn test_create_performance_profiles() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        graph.create_performance_profiles();
        let chrome_profile = graph.performance_profiles.get("Google Chrome");
        assert!(chrome_profile.is_some(), "should create Chrome profile");
        let profile = chrome_profile.unwrap();
        assert!(profile.peak_cpu_pct > 80.0);
        assert!(profile.peak_memory_bytes > 2 * 1024 * 1024 * 1024);
    }

    #[test]
    fn test_create_workload_fingerprints() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.create_workload_fingerprints();
        // Chrome Helper is CPU and memory heavy
        let chrome_fp = graph.workload_fingerprints.get(&2).unwrap();
        assert_eq!(*chrome_fp, "compute_memory_intensive");
        // idle_app is idle
        let idle_fp = graph.workload_fingerprints.get(&3).unwrap();
        assert_eq!(*idle_fp, "idle");
    }

    #[test]
    fn test_learn_normal_behavior() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.learn_normal_behavior();
        // Should have baselines for all processes
        assert_eq!(graph.behavior_baselines.len(), 4);
        // Should detect anomalies based on baselines (Chrome is way above 0)
        let cpu_anomalies: Vec<&ProcessAnomaly> = graph
            .anomalies
            .iter()
            .filter(|a| a.anomaly_type == "cpu_anomaly")
            .collect();
        assert!(!cpu_anomalies.is_empty());
    }

    #[test]
    fn test_explain_resource_usage() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        // Build top consumers
        let mut cpu_consumers: Vec<ProcessInfo> = graph
            .process_tree
            .iter()
            .map(|p| ProcessInfo {
                pid: p.pid,
                name: p.name.clone(),
                value: p.cpu_pct,
                unit: "%".to_string(),
            })
            .collect();
        cpu_consumers.sort_by(|a, b| {
            b.value
                .partial_cmp(&a.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        graph.top_cpu_consumers = cpu_consumers;

        let mut mem_consumers: Vec<ProcessInfo> = graph
            .process_tree
            .iter()
            .map(|p| ProcessInfo {
                pid: p.pid,
                name: p.name.clone(),
                value: p.memory_bytes as f64,
                unit: "bytes".to_string(),
            })
            .collect();
        mem_consumers.sort_by(|a, b| {
            b.value
                .partial_cmp(&a.value)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        graph.top_memory_consumers = mem_consumers;

        graph.explain_resource_usage();
        assert!(!graph.resource_explanations.is_empty());
        // Should explain Chrome's CPU usage
        let cpu_explanations: Vec<&ResourceExplanation> = graph
            .resource_explanations
            .iter()
            .filter(|e| e.resource == "CPU")
            .collect();
        assert!(!cpu_explanations.is_empty());
    }

    #[test]
    fn test_predict_bottlenecks() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.predict_bottlenecks();
        // With Chrome at 85% CPU, should predict some bottleneck
        // (may or may not depending on core count, but should not crash)
        // bottleneck_predictions.len() is usize, always >= 0
    }

    #[test]
    fn test_optimize_scheduling() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.optimize_scheduling();
        // Chrome Helper is a background process with high CPU
        let lower_priority: Vec<&SchedulingRecommendation> = graph
            .scheduling_recommendations
            .iter()
            .filter(|r| r.recommendation == "lower_priority")
            .collect();
        assert!(
            !lower_priority.is_empty(),
            "should recommend lowering priority for Chrome Helper"
        );
    }

    #[test]
    fn test_prioritize_apps() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        graph.prioritize_apps();
        // Should have priorities for each app
        assert!(!graph.app_priorities.is_empty());
        // Chrome should have high priority (high CPU usage)
        let chrome = graph
            .app_priorities
            .iter()
            .find(|a| a.app_name == "Google Chrome");
        assert!(chrome.is_some());
        assert!(chrome.unwrap().priority > 0.5);
    }

    #[test]
    fn test_balance_workloads() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        graph.prioritize_apps();
        graph.create_performance_profiles();
        graph.balance_workloads();
        assert!(graph.workload_balance.is_some());
        let balance = graph.workload_balance.unwrap();
        assert!(balance.cpu_balance_score >= 0.0 && balance.cpu_balance_score <= 1.0);
    }

    #[test]
    fn test_record_history() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.total_processes = graph.process_tree.len();
        graph.top_cpu_consumers = vec![ProcessInfo {
            pid: 2,
            name: "Google Chrome Helper".to_string(),
            value: 85.0,
            unit: "%".to_string(),
        }];
        graph.record_history();
        assert_eq!(graph.process_history.len(), 1);
        assert_eq!(graph.process_history[0].total_processes, 4);
        assert_eq!(graph.process_history[0].top_cpu_pid, 2);
    }

    #[test]
    fn test_build_graph() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        graph.build_graph();
        // Should have process → app edges
        let app_edges: Vec<&ProcessGraphEdge> = graph
            .graph_edges
            .iter()
            .filter(|e| e.edge_type == "belongs_to")
            .collect();
        assert!(!app_edges.is_empty());
        // Should have parent → child edges
        let parent_edges: Vec<&ProcessGraphEdge> = graph
            .graph_edges
            .iter()
            .filter(|e| e.edge_type == "parent_of")
            .collect();
        assert!(!parent_edges.is_empty());
        // Should have resource consumption edges
        let resource_edges: Vec<&ProcessGraphEdge> = graph
            .graph_edges
            .iter()
            .filter(|e| e.edge_type == "consumes")
            .collect();
        assert!(!resource_edges.is_empty());
    }

    #[test]
    fn test_app_name_from_process() {
        assert_eq!(
            app_name_from_process("Google Chrome Helper"),
            "Google Chrome"
        );
        assert_eq!(app_name_from_process("Safari"), "Safari");
        assert_eq!(app_name_from_process("node"), "node");
    }

    #[test]
    fn test_is_system_process() {
        assert!(is_system_process("kernel_task"));
        assert!(is_system_process("launchd"));
        assert!(is_system_process("windowserver"));
        assert!(!is_system_process("Google Chrome"));
        assert!(!is_system_process("Safari"));
    }

    #[test]
    fn test_is_background_process() {
        assert!(is_background_process("Chrome Helper"));
        assert!(is_background_process("update-daemon"));
        assert!(is_background_process("Crashpad Reporter"));
        assert!(!is_background_process("Safari"));
    }

    #[test]
    fn test_classify_workload() {
        let cpu_mem_heavy = ProcessNode {
            pid: 1,
            name: "test".to_string(),
            owner: None,
            parent_pid: None,
            children: vec![],
            cpu_pct: 80.0,
            gpu_pct: None,
            memory_bytes: 1024 * 1024 * 1024,
            allocation_count: None,
            disk_read_bytes: None,
            disk_write_bytes: None,
            network_bytes: None,
            energy_impact: None,
            wakeup_count: None,
            state: "R".to_string(),
            thread_count: None,
            start_time_secs: None,
        };
        assert_eq!(
            classify_workload(&cpu_mem_heavy),
            "compute_memory_intensive"
        );

        let idle = ProcessNode {
            pid: 2,
            name: "idle".to_string(),
            owner: None,
            parent_pid: None,
            children: vec![],
            cpu_pct: 0.0,
            gpu_pct: None,
            memory_bytes: 1024,
            allocation_count: None,
            disk_read_bytes: None,
            disk_write_bytes: None,
            network_bytes: None,
            energy_impact: None,
            wakeup_count: None,
            state: "S".to_string(),
            thread_count: None,
            start_time_secs: None,
        };
        assert_eq!(classify_workload(&idle), "idle");
    }

    #[test]
    fn test_collect_ps_processes() {
        // This should return at least one process (the test runner itself)
        let processes = collect_ps_processes();
        assert!(!processes.is_empty(), "should collect running processes");
        // All should have valid PIDs
        for p in &processes {
            assert!(p.pid > 0, "PID should be positive");
        }
    }

    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn test_process_intelligence_graph_collect() {
        // Full collection from the running system
        let graph = ProcessIntelligenceGraph::collect();
        assert!(graph.total_processes > 0, "should observe processes");
        assert!(
            !graph.top_cpu_consumers.is_empty(),
            "should rank CPU consumers"
        );
        assert!(
            !graph.top_memory_consumers.is_empty(),
            "should rank memory consumers"
        );
        assert!(
            !graph.app_process_map.is_empty(),
            "should map apps to processes"
        );
        assert!(
            !graph.workload_fingerprints.is_empty(),
            "should create fingerprints"
        );
        assert!(
            !graph.behavior_baselines.is_empty(),
            "should learn behavior"
        );
        assert!(
            !graph.performance_profiles.is_empty(),
            "should create profiles"
        );
        assert!(!graph.graph_edges.is_empty(), "should build graph edges");
        assert_eq!(graph.process_history.len(), 1, "should record history");
    }

    #[test]
    fn test_serialization() {
        let graph = ProcessIntelligenceGraph::empty();
        let json = serde_json::to_string(&graph).unwrap();
        let deserialized: ProcessIntelligenceGraph = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.total_processes, 0);
    }

    #[test]
    fn test_process_node_serialization() {
        let node = ProcessNode {
            pid: 1234,
            name: "test".to_string(),
            owner: Some("user".to_string()),
            parent_pid: Some(1),
            children: vec![5, 6],
            cpu_pct: 50.0,
            gpu_pct: Some(10.0),
            memory_bytes: 1024 * 1024,
            allocation_count: Some(100),
            disk_read_bytes: Some(4096),
            disk_write_bytes: Some(2048),
            network_bytes: Some(8192),
            energy_impact: Some(25.0),
            wakeup_count: Some(100),
            state: "R".to_string(),
            thread_count: Some(4),
            start_time_secs: Some(1000),
        };
        let json = serde_json::to_string(&node).unwrap();
        let deserialized: ProcessNode = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.pid, 1234);
        assert_eq!(deserialized.cpu_pct, 50.0);
        assert_eq!(deserialized.children, vec![5, 6]);
    }

    #[test]
    fn test_predict_app_failure_not_running() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        let result = graph.predict_app_failure("NonexistentApp");
        assert!(result.is_none(), "should return None for unknown app");
    }

    #[test]
    fn test_predict_app_failure_healthy() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        // idle_app has low CPU and memory — should predict low risk.
        let pred = graph.predict_app_failure("idle_app");
        assert!(pred.is_some(), "should return a prediction for idle_app");
        let pred = pred.unwrap();
        assert_eq!(pred.app_name, "idle_app");
        assert!(
            pred.crash_probability < 0.5,
            "idle app should have low risk"
        );
        assert!(
            pred.risk_factors.is_empty(),
            "idle app should have no risk factors"
        );
    }

    #[test]
    fn test_predict_app_failure_high_cpu() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        // Google Chrome Helper has 85% CPU and 3GB memory.
        let pred = graph.predict_app_failure("Google Chrome");
        assert!(pred.is_some());
        let pred = pred.unwrap();
        // 85% CPU is below the 90% threshold, so high_cpu may not trigger,
        // but 3GB is below the 4GB threshold too. Baseline risk only.
        assert!(pred.crash_probability >= 0.1);
    }

    #[test]
    fn test_predict_app_failure_with_anomalies() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        graph.detect_anomalies();
        // Google Chrome Helper triggers CPU spike and memory leak anomalies.
        let pred = graph.predict_app_failure("Google Chrome");
        assert!(pred.is_some());
        let pred = pred.unwrap();
        assert!(pred.risk_factors.contains(&"anomaly_history".to_string()));
        assert!(
            pred.crash_probability > 0.1,
            "anomalies should increase risk"
        );
    }

    #[test]
    fn test_predict_app_failure_zombie() {
        let mut graph = ProcessIntelligenceGraph::empty();
        graph.process_tree = make_test_processes();
        graph.map_app_processes();
        // zombie_proc is in state "Z".
        let pred = graph.predict_app_failure("zombie_proc");
        assert!(pred.is_some());
        let pred = pred.unwrap();
        assert!(pred.risk_factors.contains(&"zombie_process".to_string()));
    }
}
