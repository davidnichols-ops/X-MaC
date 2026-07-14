use serde::{Deserialize, Serialize};

use super::model::DigitalTwin;
use super::process::ProcessAnomaly;

/// AI Reasoning Layer — the cognitive core of the Digital Twin.
///
/// Covers: complete Mac knowledge graph, historical snapshots, regression
/// detection, predictive maintenance, causal analysis ("why is my Mac slow?"),
/// optimization simulation, reversible plans, trust scoring, and
/// autonomous optimization with sandboxing.
///
/// Maps to Digital Twin operations 276-330.
pub struct ReasoningEngine {
    twin: DigitalTwin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReasoningResult {
    pub question: String,
    pub answer: String,
    pub confidence: f64,
    pub evidence: Vec<String>,
    pub recommended_actions: Vec<RecommendedAction>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecommendedAction {
    pub action: String,
    pub estimated_impact: String,
    pub risk_level: String,
    pub is_reversible: bool,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationPlan {
    pub steps: Vec<OptimizationStep>,
    pub estimated_total_impact: String,
    pub overall_risk: String,
    pub is_reversible: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationStep {
    pub name: String,
    pub action: String,
    pub estimated_impact: String,
    pub risk_level: String,
    pub is_reversible: bool,
}

/// op 119: Estimated impact of cleanup actions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupImpact {
    /// Estimated bytes freed by cleanup.
    pub space_freed_bytes: u64,
    /// Estimated time saved in seconds (e.g. faster boots, less swap).
    pub time_saved_secs: u64,
    /// Risk level of the cleanup actions ("Low", "Medium", "High").
    pub risk_level: String,
    /// Human-readable summary of the cleanup impact.
    pub summary: String,
}

/// op 272: A recommended workflow change.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowChange {
    pub name: String,
    pub current_behavior: String,
    pub recommended_behavior: String,
    pub estimated_benefit: String,
    pub risk_level: String,
}

/// op 276: A queryable knowledge graph aggregating all twin dimensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KnowledgeGraph {
    pub nodes: Vec<GraphNode>,
    pub edges: Vec<GraphEdge>,
    pub dimensions: Vec<String>,
}

/// op 276: A node in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphNode {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub properties: Vec<(String, String)>,
}

/// op 276: An edge in the knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphEdge {
    pub source: String,
    pub target: String,
    pub relationship: String,
}

/// op 277: A historical snapshot of the digital twin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoricalSnapshot {
    pub timestamp_ms: u64,
    pub health_score: f64,
    pub trust_score: f64,
    pub memory_utilization: f64,
    pub memory_pressure_level: u32,
    pub disk_used_bytes: u64,
    pub disk_total_bytes: u64,
    pub process_count: usize,
    pub anomaly_count: usize,
    pub battery_health_pct: Option<f64>,
    pub battery_cycle_count: Option<u64>,
}

/// op 277: A detected change between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Change {
    pub dimension: String,
    pub description: String,
    pub delta: String,
}

/// op 289: Risk level of an action.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
    Critical,
}

/// op 293: A learned acceptable tradeoff.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tradeoff {
    pub name: String,
    pub preference: String,
    pub rationale: String,
}

/// op 294: A personalized optimization policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub name: String,
    pub trigger: String,
    pub action: String,
    pub priority: String,
    pub enabled: bool,
}

/// op 302: A hardware upgrade recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpgradeRecommendation {
    pub component: String,
    pub current: String,
    pub recommended: String,
    pub reason: String,
    pub estimated_benefit: String,
    pub priority: String,
}

/// op 303: A software change recommendation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareRecommendation {
    /// "update", "alternative", or "remove".
    pub kind: String,
    pub target: String,
    pub recommendation: String,
    pub reason: String,
    pub priority: String,
}

/// op 309: A structured query result for AI agents.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryResult {
    pub query: String,
    pub answer: String,
    pub data: Vec<(String, String)>,
    pub confidence: f64,
}

/// op 310: Result of autonomous optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationResult {
    pub actions_taken: Vec<String>,
    pub actions_skipped: Vec<String>,
    pub estimated_impact: String,
    pub trust_score_before: f64,
    pub trust_score_after: f64,
    pub success: bool,
}

/// op 311: Result of a sandbox simulation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxResult {
    pub action: String,
    pub simulated: bool,
    pub predicted_outcome: String,
    pub risk_level: String,
    pub safe_to_execute: bool,
    pub side_effects: Vec<String>,
}

/// op 317: Anonymized benchmark data for comparison.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkData {
    pub model_identifier: String,
    pub soc_generation: String,
    pub cpu_cores: usize,
    pub memory_bytes: u64,
    pub storage_bytes: u64,
    pub health_score: f64,
    pub memory_utilization: f64,
    pub disk_utilization: f64,
    pub process_count: usize,
    pub anomaly_count: usize,
}

/// op 318: Comparison against similar Macs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonResult {
    pub percentile_health: f64,
    pub percentile_memory: f64,
    pub percentile_disk: f64,
    pub strengths: Vec<String>,
    pub weaknesses: Vec<String>,
    pub summary: String,
}

/// op 320: A continuous monitoring plan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringPlan {
    pub intervals: Vec<MonitoringInterval>,
    pub alerts: Vec<MonitoringAlert>,
    pub summary: String,
}

/// op 320: A monitoring interval for a dimension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringInterval {
    pub dimension: String,
    pub interval_secs: u64,
    pub reason: String,
}

/// op 320: A monitoring alert condition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringAlert {
    pub condition: String,
    pub threshold: String,
    pub action: String,
}

impl ReasoningEngine {
    pub fn new(twin: DigitalTwin) -> Self {
        Self { twin }
    }

    /// Answer a natural-language question about the system.
    /// e.g. "Why is my Mac slow?", "What changed?", "What caused this?"
    ///
    /// Uses heuristic pattern matching on the question to build an
    /// evidence-backed answer from the twin's process anomalies, memory
    /// pressure, energy state, and storage data (ops 282-285).
    pub fn ask(&self, question: &str) -> ReasoningResult {
        let q = question.to_lowercase();
        if q.contains("slow") || q.contains("sluggish") || q.contains("laggy") {
            self.answer_why_slow(question)
        } else if q.contains("what changed") || q.contains("what's different") {
            self.answer_what_changed(question)
        } else if q.contains("caused") || q.contains("cause") || q.contains("why is this") {
            self.answer_what_caused(question)
        } else {
            self.answer_general(question)
        }
    }

    /// "Why is my Mac slow?" — analyze CPU usage, memory pressure, disk
    /// space, and thermal state to explain sluggishness.
    fn answer_why_slow(&self, question: &str) -> ReasoningResult {
        let mut evidence = Vec::new();
        let mut causes = Vec::new();
        let mut actions = Vec::new();

        // CPU analysis — look for high-CPU processes and anomalies.
        let cpu_anomalies: Vec<&ProcessAnomaly> = self
            .twin
            .processes
            .anomalies
            .iter()
            .filter(|a| a.anomaly_type == "cpu_spike")
            .collect();
        let total_cpu: f64 = self
            .twin
            .processes
            .process_tree
            .iter()
            .map(|p| p.cpu_pct)
            .sum();
        if !cpu_anomalies.is_empty() {
            for a in &cpu_anomalies {
                evidence.push(format!(
                    "{} (pid {}) is using abnormal CPU: {}",
                    a.name, a.pid, a.description
                ));
            }
            causes.push(format!(
                "{} process(es) with CPU spikes (total CPU ~{:.0}%)",
                cpu_anomalies.len(),
                total_cpu
            ));
            // Recommend killing the worst offender.
            if let Some(worst) = cpu_anomalies
                .iter()
                .max_by(|a, b| {
                    self.twin
                        .processes
                        .process_tree
                        .iter()
                        .find(|p| p.pid == b.pid)
                        .map(|p| p.cpu_pct)
                        .unwrap_or(0.0)
                        .partial_cmp(
                            &self
                                .twin
                                .processes
                                .process_tree
                                .iter()
                                .find(|p| p.pid == a.pid)
                                .map(|p| p.cpu_pct)
                                .unwrap_or(0.0),
                        )
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .and_then(|a| {
                    self.twin
                        .processes
                        .process_tree
                        .iter()
                        .find(|p| p.pid == a.pid)
                })
            {
                actions.push(RecommendedAction {
                    action: format!("Quit or force-kill {} (pid {})", worst.name, worst.pid),
                    estimated_impact: format!("Free ~{:.0}% CPU", worst.cpu_pct),
                    risk_level: "Medium".to_string(),
                    is_reversible: true,
                    command: Some(format!("kill {}", worst.pid)),
                });
            }
        } else if total_cpu > 80.0 {
            evidence.push(format!("Total CPU usage is high (~{:.0}%).", total_cpu));
            causes.push("Overall CPU load is elevated".to_string());
        }

        // Memory pressure analysis.
        let pressure = self.twin.memory.pressure_level;
        if pressure >= 2 {
            evidence.push(format!(
                "Memory pressure is elevated (level {}, {:.0}% used, {:.1} GB swap in use).",
                pressure,
                self.twin.memory.utilization * 100.0,
                self.twin.memory.swap_used_bytes as f64 / 1_073_741_824.0
            ));
            causes.push("Memory pressure is forcing compression and swap".to_string());
            if let Some(forecast) = &self.twin.memory.pressure_forecast {
                if let Some(mins) = forecast.minutes_to_critical {
                    evidence.push(format!("Pressure forecast: critical in ~{} minutes.", mins));
                }
            }
            actions.push(RecommendedAction {
                action: "Free memory by quitting idle heavy apps".to_string(),
                estimated_impact: "Reduce swap and compression overhead".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
                command: None,
            });
        }

        // Disk space analysis.
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        let used_disk = self.twin.filesystem.total_size_bytes;
        if total_disk > 0 {
            let free_pct = (1.0 - used_disk as f64 / total_disk as f64) * 100.0;
            if free_pct < 10.0 {
                evidence.push(format!(
                    "Disk space is low (~{:.0}% free). Low free space slows I/O.",
                    free_pct
                ));
                causes.push("Low disk space is degrading I/O performance".to_string());
                actions.push(RecommendedAction {
                    action: "Clear caches and remove large unused files".to_string(),
                    estimated_impact: "Reclaim disk space and improve I/O throughput".to_string(),
                    risk_level: "Low".to_string(),
                    is_reversible: true,
                    command: None,
                });
            }
        }

        // Thermal state analysis.
        let thermal = &self.twin.hardware.thermal;
        if thermal.thermal_pressure != "Nominal" && !thermal.thermal_pressure.is_empty() {
            evidence.push(format!(
                "Thermal pressure is {} (CPU temp {:?}°C).",
                thermal.thermal_pressure, thermal.cpu_temp_c
            ));
            causes.push("Thermal throttling is reducing CPU performance".to_string());
            actions.push(RecommendedAction {
                action: "Reduce workload intensity to lower thermal pressure".to_string(),
                estimated_impact: "Allow CPU to return to full clock speed".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
                command: None,
            });
        }

        // Fallback if no clear cause found.
        if causes.is_empty() {
            evidence.push("No single dominant cause detected in current telemetry.".to_string());
            causes
                .push("No obvious bottleneck — system metrics are within normal range".to_string());
        }

        let answer = format!(
            "Your Mac may be slow because: {}. Evidence: {}",
            causes.join("; "),
            evidence.join("; ")
        );

        let confidence = if causes.len() == 1 { 0.7 } else { 0.85 };

        ReasoningResult {
            question: question.to_string(),
            answer,
            confidence,
            evidence,
            recommended_actions: actions,
        }
    }

    /// "What changed?" — compare twin dimensions to expected baselines.
    fn answer_what_changed(&self, question: &str) -> ReasoningResult {
        let regressions = self.detect_regressions();
        let evidence: Vec<String> = regressions.clone();
        let answer = if regressions.is_empty() {
            "No significant regressions detected relative to expected baselines.".to_string()
        } else {
            format!(
                "The following changes were detected: {}",
                regressions.join("; ")
            )
        };
        ReasoningResult {
            question: question.to_string(),
            answer,
            confidence: 0.65,
            evidence,
            recommended_actions: Vec::new(),
        }
    }

    /// "What caused this?" — trace anomalies to their root causes.
    fn answer_what_caused(&self, question: &str) -> ReasoningResult {
        let mut evidence = Vec::new();
        let mut root_causes = Vec::new();
        let mut actions = Vec::new();

        // Trace process anomalies to root causes.
        for anomaly in &self.twin.processes.anomalies {
            evidence.push(format!(
                "Anomaly: {} on {} (pid {}) — {} [severity: {}]",
                anomaly.anomaly_type,
                anomaly.name,
                anomaly.pid,
                anomaly.description,
                anomaly.severity
            ));
            match anomaly.anomaly_type.as_str() {
                "cpu_spike" => {
                    root_causes.push(format!(
                        "{} is consuming excessive CPU, starving other processes",
                        anomaly.name
                    ));
                }
                "memory_leak" => {
                    root_causes.push(format!(
                        "{} appears to be leaking memory, increasing pressure on the unified memory fabric",
                        anomaly.name
                    ));
                }
                "zombie" => {
                    root_causes.push(format!(
                        "{} became a zombie process — its parent failed to reap it",
                        anomaly.name
                    ));
                }
                other => {
                    root_causes.push(format!(
                        "{} exhibits abnormal behavior: {}",
                        anomaly.name, other
                    ));
                }
            }
        }

        // Trace crashed services.
        for svc in &self.twin.processes.crashed_services {
            evidence.push(format!("Crashed service: {}", svc));
            root_causes.push(format!(
                "Service {} has crashed and may need restarting",
                svc
            ));
        }

        // Trace memory leak candidates.
        for leak in &self.twin.memory.leak_candidates {
            evidence.push(format!(
                "Memory leak candidate: {} (pid {}) growing at {:.0} MB/min",
                leak.name,
                leak.pid,
                leak.growth_rate_bytes_per_min / 1_048_576.0
            ));
            root_causes.push(format!(
                "{} is steadily consuming memory and may eventually trigger pressure",
                leak.name
            ));
            actions.push(RecommendedAction {
                action: format!("Restart {} to reclaim leaked memory", leak.name),
                estimated_impact: "Reclaim leaked memory and reduce pressure".to_string(),
                risk_level: "Medium".to_string(),
                is_reversible: false,
                command: None,
            });
        }

        // Trace battery abnormal aging.
        if self.twin.energy.has_abnormal_aging() {
            if let Some(b) = &self.twin.energy.battery {
                evidence.push(format!(
                    "Battery aging is abnormal: {} cycles, health {:?}%",
                    b.cycle_count, b.health_pct
                ));
                root_causes.push(
                    "Battery is degrading faster than expected for its cycle count".to_string(),
                );
            }
        }

        if root_causes.is_empty() {
            evidence.push(
                "No anomalies or crashed services detected in current telemetry.".to_string(),
            );
            root_causes.push("No root cause identified — system appears stable".to_string());
        }

        let answer = format!(
            "Root cause analysis: {}. Evidence: {}",
            root_causes.join("; "),
            evidence.join("; ")
        );

        ReasoningResult {
            question: question.to_string(),
            answer,
            confidence: if root_causes.is_empty() { 0.4 } else { 0.8 },
            evidence,
            recommended_actions: actions,
        }
    }

    /// General fallback answer — summarize system health.
    fn answer_general(&self, question: &str) -> ReasoningResult {
        let health = self.compute_system_health();
        let trust = self.compute_trust_score();
        let evidence = vec![
            format!("System health score: {:.1}/100", health),
            format!("Trust score for autonomous action: {:.2}/1.0", trust),
            format!(
                "Process anomalies: {}, crashed services: {}",
                self.twin.processes.anomalies.len(),
                self.twin.processes.crashed_services.len()
            ),
            format!(
                "Memory: {:.0}% used, pressure level {}",
                self.twin.memory.utilization * 100.0,
                self.twin.memory.pressure_level
            ),
        ];
        let answer = format!(
            "System health is {:.1}/100. {} anomalies detected. \
             I can answer 'why is my Mac slow?', 'what changed?', or 'what caused this?' for more detail.",
            health,
            self.twin.processes.anomalies.len()
        );
        ReasoningResult {
            question: question.to_string(),
            answer,
            confidence: 0.5,
            evidence,
            recommended_actions: self.propose_autonomous_actions(),
        }
    }

    /// Create an optimization plan ranked by impact (ops 286-288).
    ///
    /// Generates steps for killing high-CPU processes, clearing caches,
    /// disabling startup items, running maintenance, and freeing memory.
    /// Each step is ranked by estimated impact, marked for reversibility,
    /// and assigned a risk level.
    pub fn create_optimization_plan(&self) -> OptimizationPlan {
        let mut steps: Vec<OptimizationStep> = Vec::new();

        // Kill high-CPU processes if anomalies are detected.
        let cpu_anomalies: Vec<&ProcessAnomaly> = self
            .twin
            .processes
            .anomalies
            .iter()
            .filter(|a| a.anomaly_type == "cpu_spike")
            .collect();
        if !cpu_anomalies.is_empty() {
            let total_cpu: f64 = self
                .twin
                .processes
                .process_tree
                .iter()
                .map(|p| p.cpu_pct)
                .sum();
            steps.push(OptimizationStep {
                name: "Kill high-CPU processes".to_string(),
                action: format!(
                    "Force-quit {} process(es) with CPU spikes (total ~{:.0}% CPU)",
                    cpu_anomalies.len(),
                    total_cpu
                ),
                estimated_impact: "High".to_string(),
                risk_level: "Medium".to_string(),
                is_reversible: true,
            });
        }

        // Clear caches if disk space is low.
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        let used_disk = self.twin.filesystem.total_size_bytes;
        if total_disk > 0 && (used_disk as f64 / total_disk as f64) > 0.90 {
            steps.push(OptimizationStep {
                name: "Clear caches".to_string(),
                action: "Purge application caches and temporary files to reclaim disk space"
                    .to_string(),
                estimated_impact: "Medium".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
            });
        }

        // Disable startup items if there are too many.
        let startup_count = self.twin.software_genome.login_items.len()
            + self.twin.software_genome.launch_agents.len();
        if startup_count > 20 {
            steps.push(OptimizationStep {
                name: "Disable startup items".to_string(),
                action: format!(
                    "Disable {} non-essential login items and launch agents",
                    startup_count - 20
                ),
                estimated_impact: "Medium".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
            });
        }

        // Run maintenance if health score is low.
        let health = self.compute_system_health();
        if health < 70.0 {
            steps.push(OptimizationStep {
                name: "Run maintenance".to_string(),
                action: "Run periodic maintenance: repair permissions, rebuild Spotlight index, flush DNS"
                    .to_string(),
                estimated_impact: "Medium".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
            });
        }

        // Free memory if pressure is high.
        if self.twin.memory.pressure_level >= 2 {
            steps.push(OptimizationStep {
                name: "Free memory".to_string(),
                action: "Quit idle memory-heavy apps and purge inactive memory to reduce pressure"
                    .to_string(),
                estimated_impact: "High".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
            });
        }

        // Rank steps by impact (High > Medium > Low).
        let impact_rank = |s: &OptimizationStep| match s.estimated_impact.as_str() {
            "High" => 0,
            "Medium" => 1,
            _ => 2,
        };
        steps.sort_by_key(impact_rank);

        let all_reversible = steps.iter().all(|s| s.is_reversible);
        let any_high_risk = steps.iter().any(|s| s.risk_level == "High");
        let overall_risk = if any_high_risk {
            "High"
        } else if steps.iter().any(|s| s.risk_level == "Medium") {
            "Medium"
        } else {
            "Low"
        };
        let estimated_total_impact = if steps.is_empty() {
            "None — system is already well-optimized".to_string()
        } else {
            format!("{} optimization step(s) identified", steps.len())
        };

        OptimizationPlan {
            steps,
            estimated_total_impact,
            overall_risk: overall_risk.to_string(),
            is_reversible: all_reversible,
        }
    }

    /// op 274: Simulate the outcome of an optimization action without
    /// executing it (op 286).
    ///
    /// Parses the action string (e.g. "kill Chrome", "clear caches",
    /// "free memory") and estimates the predicted outcome in terms of
    /// memory freed, CPU reduction, and user impact.
    pub fn simulate(&self, action: &str) -> ReasoningResult {
        let a = action.to_lowercase();
        let mut evidence = Vec::new();
        let mut predicted_effects = Vec::new();
        let mut user_impact = "Minimal user disruption expected".to_string();
        let confidence;

        if a.contains("kill") || a.contains("quit") || a.contains("force") {
            // Extract a process name from the action (heuristic: word after
            // "kill"/"quit").
            let target = extract_process_name(&a);
            if let Some(name) = target {
                let proc = self
                    .twin
                    .processes
                    .process_tree
                    .iter()
                    .find(|p| p.name.to_lowercase().contains(&name));
                if let Some(p) = proc {
                    let mem_gb = p.memory_bytes as f64 / 1_073_741_824.0;
                    predicted_effects.push(format!("Free ~{:.1} GB of memory", mem_gb));
                    predicted_effects.push(format!("Reduce CPU usage by ~{:.0}%", p.cpu_pct));
                    evidence.push(format!(
                        "{} (pid {}) currently uses {:.1} GB memory and {:.0}% CPU",
                        p.name, p.pid, mem_gb, p.cpu_pct
                    ));
                    if p.cpu_pct > 50.0 || mem_gb > 2.0 {
                        user_impact = "App will close — unsaved work may be lost".to_string();
                    }
                    confidence = 0.8;
                } else {
                    evidence.push(format!(
                        "Process '{}' not found in current process tree",
                        name
                    ));
                    predicted_effects
                        .push("No measurable effect — process not running".to_string());
                    confidence = 0.3;
                }
            } else {
                evidence.push("No process name detected in the action".to_string());
                confidence = 0.2;
            }
        } else if a.contains("clear") && a.contains("cache") {
            let purgeable_gb = self.twin.memory.purgeable_bytes as f64 / 1_073_741_824.0;
            predicted_effects.push(format!(
                "Reclaim ~{:.1} GB of purgeable memory",
                purgeable_gb
            ));
            predicted_effects.push("Free disk space from cache files".to_string());
            evidence.push(format!(
                "Purgeable memory: {:.1} GB; cache relationships: {}",
                purgeable_gb,
                self.twin.filesystem.cache_relationships.len()
            ));
            user_impact = "No user disruption — caches regenerate as needed".to_string();
            confidence = 0.75;
        } else if a.contains("free") && a.contains("memory") {
            let idle_heavy: u64 = self
                .twin
                .memory
                .top_consumers
                .iter()
                .filter(|c| c.is_idle)
                .map(|c| c.memory_bytes)
                .sum();
            let freed_gb = idle_heavy as f64 / 1_073_741_824.0;
            let new_util = self.twin.memory.simulate_change(idle_heavy);
            predicted_effects.push(format!("Free ~{:.1} GB from idle apps", freed_gb));
            predicted_effects.push(format!(
                "Reduce memory utilization from {:.0}% to ~{:.0}%",
                self.twin.memory.utilization * 100.0,
                new_util * 100.0
            ));
            evidence.push(format!(
                "{} idle heavy memory consumer(s) identified",
                self.twin
                    .memory
                    .top_consumers
                    .iter()
                    .filter(|c| c.is_idle)
                    .count()
            ));
            user_impact = "Idle apps will be suspended — they resume on next use".to_string();
            confidence = 0.7;
        } else if a.contains("disable") && (a.contains("startup") || a.contains("login")) {
            let count = self.twin.software_genome.login_items.len();
            predicted_effects.push(format!("Skip loading {} login items at next boot", count));
            predicted_effects.push("Faster boot and login time".to_string());
            evidence.push(format!("{} login items currently configured", count));
            user_impact = "Disabled items won't auto-launch — they can be re-enabled".to_string();
            confidence = 0.65;
        } else {
            evidence.push(format!("Action '{}' not recognized for simulation", action));
            predicted_effects.push("Unable to predict outcome for unrecognized action".to_string());
            confidence = 0.1;
        }

        let answer = format!(
            "Simulating '{}': {}. {}",
            action,
            predicted_effects.join("; "),
            user_impact
        );

        ReasoningResult {
            question: format!("Simulate: {}", action),
            answer,
            confidence,
            evidence,
            recommended_actions: Vec::new(),
        }
    }

    /// Detect performance regressions by comparing to expected baselines
    /// (op 279).
    ///
    /// Checks for: increased CPU usage, decreased disk space, increased
    /// memory pressure, and battery health decline.
    pub fn detect_regressions(&self) -> Vec<String> {
        let mut regressions = Vec::new();

        // CPU usage regression — total CPU above 70% is a regression.
        let total_cpu: f64 = self
            .twin
            .processes
            .process_tree
            .iter()
            .map(|p| p.cpu_pct)
            .sum();
        if total_cpu > 70.0 {
            regressions.push(format!(
                "CPU usage increased to {:.0}% (baseline: <70%)",
                total_cpu
            ));
        }

        // Disk space regression — less than 15% free is a regression.
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        let used_disk = self.twin.filesystem.total_size_bytes;
        if total_disk > 0 {
            let free_pct = (1.0 - used_disk as f64 / total_disk as f64) * 100.0;
            if free_pct < 15.0 {
                regressions.push(format!(
                    "Disk free space decreased to {:.0}% (baseline: >15%)",
                    free_pct
                ));
            }
        }

        // Memory pressure regression — level 2+ is a regression.
        if self.twin.memory.pressure_level >= 2 {
            regressions.push(format!(
                "Memory pressure increased to level {} (baseline: <=1)",
                self.twin.memory.pressure_level
            ));
        }

        // Battery health decline — below 80% is a regression.
        if let Some(b) = &self.twin.energy.battery {
            if let Some(health) = b.health_pct {
                if health < 80.0 {
                    regressions.push(format!(
                        "Battery health declined to {:.0}% (baseline: >80%)",
                        health
                    ));
                }
            }
        }

        // Process anomaly count regression — more than 5 anomalies is unusual.
        if self.twin.processes.anomalies.len() > 5 {
            regressions.push(format!(
                "Process anomaly count increased to {} (baseline: <=5)",
                self.twin.processes.anomalies.len()
            ));
        }

        regressions
    }

    /// op 280: Predict future problems: SSD wear, battery decline, storage
    /// exhaustion, and memory pressure (ops 298-301).
    pub fn predict_problems(&self) -> Vec<String> {
        let mut predictions = Vec::new();

        // SSD wear extrapolation from SMART data.
        for disk in &self.twin.hardware.storage {
            if disk.is_ssd {
                if let Some(smart) = &disk.smart_health {
                    if smart.status != "OK" && !smart.status.is_empty() {
                        predictions.push(format!(
                            "SSD '{}' reports SMART status '{}' — wear may be accelerating",
                            disk.name, smart.status
                        ));
                    }
                    if let Some(hours) = smart.power_on_hours {
                        // Typical SSD lifespan ~5 years (~43800 hours).
                        if hours > 35_000 {
                            predictions.push(format!(
                                "SSD '{}' has {} power-on hours — approaching end of rated lifespan",
                                disk.name, hours
                            ));
                        }
                    }
                }
            }
        }

        // Battery decline from cycle count.
        if let Some(b) = &self.twin.energy.battery {
            if b.cycle_count > 800 {
                predictions.push(format!(
                    "Battery has {} cycles — expect noticeable capacity decline within a year",
                    b.cycle_count
                ));
            }
            if let Some(forecast) = &self.twin.energy.degradation_forecast {
                if forecast.predicted_health_pct_1y < 75.0 {
                    predictions.push(format!(
                        "Battery health predicted to drop to {:.0}% within a year",
                        forecast.predicted_health_pct_1y
                    ));
                }
            }
        }

        // Storage exhaustion from growth trend.
        if let Some(trend) = self.twin.filesystem.storage_growth_trend {
            if trend > 0.0 {
                if let Some(days) = self.twin.filesystem.exhaustion_forecast_days {
                    if days < 90 {
                        predictions.push(format!(
                            "Storage will be exhausted in ~{} days at current growth rate ({:.1} GB/day)",
                            days,
                            trend
                        ));
                    }
                } else {
                    predictions.push(format!(
                        "Storage is growing at {:.1} GB/day — monitor for exhaustion",
                        trend
                    ));
                }
            }
        }

        // Memory pressure from allocation patterns.
        if let Some(forecast) = &self.twin.memory.pressure_forecast {
            if forecast.predicted_utilization_1h > 0.90 {
                predictions.push(format!(
                    "Memory utilization predicted to reach {:.0}% within an hour — pressure likely",
                    forecast.predicted_utilization_1h * 100.0
                ));
            }
            if let Some(mins) = forecast.minutes_to_critical {
                if mins < 60 {
                    predictions.push(format!(
                        "Memory pressure will reach critical in ~{} minutes",
                        mins
                    ));
                }
            }
        }
        for leak in &self.twin.memory.leak_candidates {
            if leak.growth_rate_bytes_per_min > 100_000_000.0 {
                predictions.push(format!(
                    "{} is leaking ~{:.0} MB/min — will cause pressure if left unchecked",
                    leak.name,
                    leak.growth_rate_bytes_per_min / 1_048_576.0
                ));
            }
        }

        predictions
    }

    /// Compute the system health score (0-100) by aggregating health across
    /// all twin dimensions (op 290).
    ///
    /// Combines process stability, memory health, energy health, storage
    /// health, and thermal state into a single score.
    pub fn compute_system_health(&self) -> f64 {
        // Process stability: penalize anomalies and crashed services.
        let anomaly_penalty = self.twin.processes.anomalies.len() as f64 * 8.0;
        let crash_penalty = self.twin.processes.crashed_services.len() as f64 * 15.0;
        let process_health = (100.0 - anomaly_penalty - crash_penalty).max(0.0);

        // Memory health: based on utilization and pressure level.
        let mem_util = self.twin.memory.utilization;
        let mem_pressure_penalty = self.twin.memory.pressure_level as f64 * 15.0;
        let mem_swap_penalty = if self.twin.memory.swap_used_bytes > 1_073_741_824 {
            10.0
        } else {
            0.0
        };
        let memory_health =
            (100.0 - mem_util * 50.0 - mem_pressure_penalty - mem_swap_penalty).max(0.0);

        // Energy health: based on battery health and thermal efficiency.
        let energy_health = self
            .twin
            .energy
            .battery
            .as_ref()
            .and_then(|b| b.health_pct)
            .unwrap_or(100.0);
        let thermal_penalty = match self.twin.hardware.thermal.thermal_pressure.as_str() {
            "Critical" => 30.0,
            "Heavy" => 20.0,
            "Moderate" => 10.0,
            "Light" => 5.0,
            _ => 0.0,
        };
        let energy_health = (energy_health - thermal_penalty).max(0.0);

        // Storage health: based on free space fraction.
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        let storage_health = if total_disk > 0 {
            let free_frac = 1.0 - self.twin.filesystem.total_size_bytes as f64 / total_disk as f64;
            (free_frac * 100.0).clamp(0.0, 100.0)
        } else {
            100.0
        };

        // Weighted average: process 40%, memory 25%, energy 15%, storage 20%.
        let score = process_health * 0.40
            + memory_health * 0.25
            + energy_health * 0.15
            + storage_health * 0.20;

        score.clamp(0.0, 100.0)
    }

    /// op 305: Compute the trust score (0-1) for autonomous actions (op 291).
    ///
    /// Evaluates how safe autonomous actions would be based on system
    /// stability, anomaly count, and health score. A higher score means
    /// the system is stable enough to safely apply automated changes.
    pub fn compute_trust_score(&self) -> f64 {
        let health = self.compute_system_health();

        // Stability: fewer anomalies and crashes → higher trust.
        let anomaly_count = self.twin.processes.anomalies.len();
        let crash_count = self.twin.processes.crashed_services.len();
        let stability = (1.0 - (anomaly_count as f64 + crash_count as f64) / 10.0).max(0.0);

        // Health factor: 0-1 normalized.
        let health_factor = health / 100.0;

        // Memory stability: high pressure lowers trust.
        let pressure_factor = 1.0 - (self.twin.memory.pressure_level as f64 / 4.0).min(1.0);

        // Weighted combination: stability is the dominant factor.
        let trust = stability * 0.50 + health_factor * 0.30 + pressure_factor * 0.20;

        trust.clamp(0.0, 1.0)
    }

    /// Propose safe, reversible autonomous actions (ops 295-297).
    ///
    /// Only suggests actions that are low-risk and reversible — the system
    /// could apply these automatically without user confirmation.
    pub fn propose_autonomous_actions(&self) -> Vec<RecommendedAction> {
        let mut actions = Vec::new();

        // Only propose autonomous actions if trust is reasonably high.
        if self.compute_trust_score() < 0.4 {
            return actions;
        }

        // Purge inactive memory — always safe and reversible.
        if self.twin.memory.pressure_level >= 1 {
            actions.push(RecommendedAction {
                action: "Purge inactive memory to reduce pressure".to_string(),
                estimated_impact: "Low — reclaim purgeable memory".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
                command: None,
            });
        }

        // Suspend idle heavy apps — reversible (apps resume on next use).
        for name in self.twin.memory.optimize_background() {
            actions.push(RecommendedAction {
                action: format!("Suspend idle app '{}'", name),
                estimated_impact: "Low — free memory, app resumes on demand".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
                command: None,
            });
        }

        // Clear caches — safe and reversible (caches regenerate).
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        if total_disk > 0
            && (self.twin.filesystem.total_size_bytes as f64 / total_disk as f64) > 0.85
        {
            actions.push(RecommendedAction {
                action: "Clear application caches to reclaim disk space".to_string(),
                estimated_impact: "Low — caches regenerate as needed".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
                command: None,
            });
        }

        // Reduce background energy consumers — reversible.
        for name in self.twin.energy.adjust_background() {
            actions.push(RecommendedAction {
                action: format!("Throttle background activity of '{}'", name),
                estimated_impact: "Low — reduce energy drain".to_string(),
                risk_level: "Low".to_string(),
                is_reversible: true,
                command: None,
            });
        }

        actions
    }

    /// op 119: Simulate cleanup impact — estimate space freed, time saved,
    /// and risk of cleanup actions based on the filesystem graph and
    /// duplicate/abandoned/orphan file detection.
    #[allow(dead_code)]
    pub fn simulate_cleanup(&self) -> CleanupImpact {
        let mut space_freed_bytes: u64 = 0;
        let mut high_risk_count = 0;

        // Duplicate clusters — safe to dedupe (keep one copy).
        for cluster in &self.twin.filesystem.duplicate_clusters {
            if cluster.files.len() > 1 {
                let per_file = cluster.total_size_bytes / cluster.files.len() as u64;
                space_freed_bytes += per_file * (cluster.files.len() as u64 - 1);
            }
        }

        // Abandoned and orphan files — low risk to remove (~1 MB each estimate).
        space_freed_bytes += self.twin.filesystem.abandoned_files.len() as u64 * 1_048_576;
        space_freed_bytes += self.twin.filesystem.orphan_files.len() as u64 * 1_048_576;

        // Purgeable memory — low risk.
        space_freed_bytes += self.twin.memory.purgeable_bytes;

        // If there are many abandoned files, cleanup risk rises.
        if self.twin.filesystem.abandoned_files.len() > 100 {
            high_risk_count += 1;
        }

        // Time saved: estimate from reduced swap and faster boots.
        let time_saved_secs = if self.twin.memory.swap_used_bytes > 1_073_741_824 {
            300 // ~5 min from reduced swap thrashing
        } else {
            60
        };

        let risk_level = if high_risk_count > 0 { "Medium" } else { "Low" };

        let summary = format!(
            "Cleanup would free ~{:.1} GB and save ~{} seconds. Risk: {}.",
            space_freed_bytes as f64 / 1_073_741_824.0,
            time_saved_secs,
            risk_level
        );

        CleanupImpact {
            space_freed_bytes,
            time_saved_secs,
            risk_level: risk_level.to_string(),
            summary,
        }
    }

    /// op 272: Recommend workflow changes — suggest workflow improvements
    /// based on app usage patterns, startup items, and energy/memory
    /// consumption.
    #[allow(dead_code)]
    pub fn recommend_workflow_changes(&self) -> Vec<WorkflowChange> {
        let mut changes = Vec::new();

        // Too many login items slow boot and consume background resources.
        let startup_count = self.twin.software_genome.login_items.len()
            + self.twin.software_genome.launch_agents.len();
        if startup_count > 15 {
            changes.push(WorkflowChange {
                name: "Reduce startup items".to_string(),
                current_behavior: format!(
                    "{} login items and launch agents load at startup",
                    startup_count
                ),
                recommended_behavior: "Disable non-essential startup items".to_string(),
                estimated_benefit: "Faster boot time and lower background resource use".to_string(),
                risk_level: "Low".to_string(),
            });
        }

        // Idle heavy memory consumers waste memory.
        let idle_heavy: Vec<&crate::twin::memory::MemoryConsumer> = self
            .twin
            .memory
            .top_consumers
            .iter()
            .filter(|c| c.is_idle)
            .collect();
        if !idle_heavy.is_empty() {
            changes.push(WorkflowChange {
                name: "Suspend idle heavy apps".to_string(),
                current_behavior: format!(
                    "{} idle app(s) consuming significant memory",
                    idle_heavy.len()
                ),
                recommended_behavior: "Suspend idle apps until next use".to_string(),
                estimated_benefit: "Free memory for active workloads".to_string(),
                risk_level: "Low".to_string(),
            });
        }

        // High background energy drain.
        if self.twin.energy.profile.background_drain_pct > 30.0 {
            changes.push(WorkflowChange {
                name: "Reduce background energy drain".to_string(),
                current_behavior: format!(
                    "{:.0}% of energy consumed by background activity",
                    self.twin.energy.profile.background_drain_pct
                ),
                recommended_behavior: "Throttle background activity of non-essential apps"
                    .to_string(),
                estimated_benefit: "Extended battery life".to_string(),
                risk_level: "Low".to_string(),
            });
        }

        // Frequent wake events waste energy.
        if self.twin.energy.wake_causes.len() > 5 {
            changes.push(WorkflowChange {
                name: "Reduce sleep wake events".to_string(),
                current_behavior: format!(
                    "{} recent wake events detected",
                    self.twin.energy.wake_causes.len()
                ),
                recommended_behavior: "Identify and disable apps preventing sleep".to_string(),
                estimated_benefit: "Better sleep efficiency and battery life".to_string(),
                risk_level: "Medium".to_string(),
            });
        }

        changes
    }

    /// op 276: Build a complete Mac knowledge graph — aggregate all twin
    /// dimensions into a queryable graph of nodes and edges.
    #[allow(dead_code)]
    pub fn build_knowledge_graph(&self) -> KnowledgeGraph {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();
        let mut dimensions = Vec::new();

        // Hardware dimension.
        dimensions.push("hardware".to_string());
        nodes.push(GraphNode {
            id: "hardware:cpu".to_string(),
            label: format!(
                "CPU ({} cores)",
                self.twin.hardware.cpu_cores.total_logical_cores
            ),
            kind: "hardware".to_string(),
            properties: vec![
                (
                    "performance_cores".to_string(),
                    self.twin.hardware.cpu_cores.performance_cores.to_string(),
                ),
                (
                    "efficiency_cores".to_string(),
                    self.twin.hardware.cpu_cores.efficiency_cores.to_string(),
                ),
            ],
        });
        nodes.push(GraphNode {
            id: "hardware:memory".to_string(),
            label: format!(
                "Memory ({:.0} GB)",
                self.twin.hardware.memory.total_bytes as f64 / 1_073_741_824.0
            ),
            kind: "hardware".to_string(),
            properties: Vec::new(),
        });
        for disk in &self.twin.hardware.storage {
            nodes.push(GraphNode {
                id: format!("hardware:storage:{}", disk.name),
                label: format!(
                    "{} ({:.0} GB)",
                    disk.name,
                    disk.capacity_bytes as f64 / 1_073_741_824.0
                ),
                kind: "hardware".to_string(),
                properties: vec![("is_ssd".to_string(), disk.is_ssd.to_string())],
            });
        }
        edges.push(GraphEdge {
            source: "hardware:cpu".to_string(),
            target: "hardware:memory".to_string(),
            relationship: "shares_unified_memory".to_string(),
        });

        // Software dimension.
        dimensions.push("software".to_string());
        nodes.push(GraphNode {
            id: "software:genome".to_string(),
            label: format!(
                "Software Genome ({} components)",
                self.twin.software_genome.total_components
            ),
            kind: "software".to_string(),
            properties: vec![
                (
                    "applications".to_string(),
                    self.twin.software_genome.applications.len().to_string(),
                ),
                (
                    "login_items".to_string(),
                    self.twin.software_genome.login_items.len().to_string(),
                ),
            ],
        });

        // Process dimension.
        dimensions.push("processes".to_string());
        for proc in &self.twin.processes.process_tree {
            nodes.push(GraphNode {
                id: format!("process:{}", proc.pid),
                label: proc.name.clone(),
                kind: "process".to_string(),
                properties: vec![
                    ("cpu_pct".to_string(), format!("{:.1}", proc.cpu_pct)),
                    (
                        "memory_gb".to_string(),
                        format!("{:.2}", proc.memory_bytes as f64 / 1_073_741_824.0),
                    ),
                ],
            });
            if let Some(parent) = proc.parent_pid {
                edges.push(GraphEdge {
                    source: format!("process:{}", parent),
                    target: format!("process:{}", proc.pid),
                    relationship: "parent_of".to_string(),
                });
            }
        }

        // Memory dimension.
        dimensions.push("memory".to_string());
        nodes.push(GraphNode {
            id: "memory:pressure".to_string(),
            label: format!(
                "Memory Pressure (level {})",
                self.twin.memory.pressure_level
            ),
            kind: "memory".to_string(),
            properties: vec![
                (
                    "utilization".to_string(),
                    format!("{:.0}%", self.twin.memory.utilization * 100.0),
                ),
                (
                    "swap_gb".to_string(),
                    format!(
                        "{:.1}",
                        self.twin.memory.swap_used_bytes as f64 / 1_073_741_824.0
                    ),
                ),
            ],
        });

        // Energy dimension.
        dimensions.push("energy".to_string());
        if let Some(b) = &self.twin.energy.battery {
            nodes.push(GraphNode {
                id: "energy:battery".to_string(),
                label: format!("Battery ({:.0}%, {} cycles)", b.charge_pct, b.cycle_count),
                kind: "energy".to_string(),
                properties: vec![(
                    "health_pct".to_string(),
                    b.health_pct
                        .map(|h| format!("{:.0}", h))
                        .unwrap_or_default(),
                )],
            });
        }

        // Application dimension.
        dimensions.push("applications".to_string());
        for app in &self.twin.applications.apps {
            nodes.push(GraphNode {
                id: format!("app:{}", app.bundle_id),
                label: app.name.clone(),
                kind: "application".to_string(),
                properties: vec![
                    ("version".to_string(), app.version.clone()),
                    ("health".to_string(), format!("{:.0}", app.health_score)),
                ],
            });
        }

        KnowledgeGraph {
            nodes,
            edges,
            dimensions,
        }
    }

    /// op 277: Save a historical snapshot of the current twin state.
    #[allow(dead_code)]
    pub fn save_snapshot(&self) -> HistoricalSnapshot {
        let disk_total: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        HistoricalSnapshot {
            timestamp_ms: self.twin.timestamp_ms,
            health_score: self.compute_system_health(),
            trust_score: self.compute_trust_score(),
            memory_utilization: self.twin.memory.utilization,
            memory_pressure_level: self.twin.memory.pressure_level,
            disk_used_bytes: self.twin.filesystem.total_size_bytes,
            disk_total_bytes: disk_total,
            process_count: self.twin.processes.process_tree.len(),
            anomaly_count: self.twin.processes.anomalies.len(),
            battery_health_pct: self.twin.energy.battery.as_ref().and_then(|b| b.health_pct),
            battery_cycle_count: self.twin.energy.battery.as_ref().map(|b| b.cycle_count),
        }
    }

    /// op 277: Compare an old snapshot to the current twin state and
    /// return a list of detected changes.
    #[allow(dead_code)]
    pub fn compare_snapshots(old: &HistoricalSnapshot, new: &DigitalTwin) -> Vec<Change> {
        let mut changes = Vec::new();

        // Health score change.
        let new_health = new.reason().compute_system_health();
        let health_delta = new_health - old.health_score;
        if health_delta.abs() > 1.0 {
            changes.push(Change {
                dimension: "health".to_string(),
                description: "System health score changed".to_string(),
                delta: format!("{:+.1}", health_delta),
            });
        }

        // Memory utilization change.
        let mem_delta = new.memory.utilization - old.memory_utilization;
        if mem_delta.abs() > 0.05 {
            changes.push(Change {
                dimension: "memory".to_string(),
                description: "Memory utilization changed".to_string(),
                delta: format!("{:+.1}%", mem_delta * 100.0),
            });
        }

        // Memory pressure level change.
        if new.memory.pressure_level != old.memory_pressure_level {
            changes.push(Change {
                dimension: "memory".to_string(),
                description: "Memory pressure level changed".to_string(),
                delta: format!(
                    "{} -> {}",
                    old.memory_pressure_level, new.memory.pressure_level
                ),
            });
        }

        // Disk usage change.
        let disk_delta = new.filesystem.total_size_bytes as i64 - old.disk_used_bytes as i64;
        if disk_delta.abs() > 1_073_741_824 {
            changes.push(Change {
                dimension: "storage".to_string(),
                description: "Disk usage changed".to_string(),
                delta: format!("{:+.1} GB", disk_delta as f64 / 1_073_741_824.0),
            });
        }

        // Process count change.
        let proc_delta = new.processes.process_tree.len() as i64 - old.process_count as i64;
        if proc_delta.abs() > 10 {
            changes.push(Change {
                dimension: "processes".to_string(),
                description: "Process count changed".to_string(),
                delta: format!("{:+}", proc_delta),
            });
        }

        // Anomaly count change.
        let anomaly_delta = new.processes.anomalies.len() as i64 - old.anomaly_count as i64;
        if anomaly_delta != 0 {
            changes.push(Change {
                dimension: "processes".to_string(),
                description: "Anomaly count changed".to_string(),
                delta: format!("{:+}", anomaly_delta),
            });
        }

        // Battery health change.
        if let (Some(old_h), Some(new_b)) = (old.battery_health_pct, &new.energy.battery) {
            if let Some(new_h) = new_b.health_pct {
                let bat_delta = new_h - old_h;
                if bat_delta.abs() > 1.0 {
                    changes.push(Change {
                        dimension: "energy".to_string(),
                        description: "Battery health changed".to_string(),
                        delta: format!("{:+.1}%", bat_delta),
                    });
                }
            }
        }

        changes
    }

    /// op 281: Recommend preventive actions — suggest actions to prevent
    /// predicted problems before they occur.
    #[allow(dead_code)]
    pub fn recommend_preventive_actions(&self) -> Vec<RecommendedAction> {
        let predictions = self.predict_problems();
        let mut actions = Vec::new();

        for pred in &predictions {
            let p = pred.to_lowercase();
            if p.contains("ssd") || p.contains("storage") {
                actions.push(RecommendedAction {
                    action: "Reduce write-heavy workloads to extend SSD lifespan".to_string(),
                    estimated_impact: "Slow SSD wear progression".to_string(),
                    risk_level: "Low".to_string(),
                    is_reversible: true,
                    command: None,
                });
            }
            if p.contains("battery") {
                actions.push(RecommendedAction {
                    action: "Enable Optimized Battery Charging to reduce cycle accumulation"
                        .to_string(),
                    estimated_impact: "Slow battery degradation".to_string(),
                    risk_level: "Low".to_string(),
                    is_reversible: true,
                    command: None,
                });
            }
            if p.contains("memory") || p.contains("pressure") {
                actions.push(RecommendedAction {
                    action: "Close memory-heavy idle apps proactively".to_string(),
                    estimated_impact: "Prevent memory pressure before it occurs".to_string(),
                    risk_level: "Low".to_string(),
                    is_reversible: true,
                    command: None,
                });
            }
            if p.contains("leak") {
                actions.push(RecommendedAction {
                    action: "Restart leaking processes on a schedule".to_string(),
                    estimated_impact: "Reclaim leaked memory before pressure builds".to_string(),
                    risk_level: "Medium".to_string(),
                    is_reversible: false,
                    command: None,
                });
            }
        }

        // Deduplicate actions by action text.
        let mut seen = std::collections::HashSet::new();
        actions.retain(|a| seen.insert(a.action.clone()));

        actions
    }

    /// op 289: Estimate the risk level of an action (Low/Medium/High/Critical).
    ///
    /// Uses keyword matching on the action description to classify risk.
    #[allow(dead_code)]
    pub fn estimate_risk(action: &str) -> RiskLevel {
        let a = action.to_lowercase();

        // Critical: irreversible system-wide changes.
        if a.contains("format")
            || a.contains("erase")
            || a.contains("factory reset")
            || a.contains("rm -rf")
            || a.contains("diskutil erase")
        {
            return RiskLevel::Critical;
        }

        // High: potentially irreversible or system-impacting.
        if a.contains("delete system")
            || a.contains("uninstall")
            || a.contains("remove framework")
            || a.contains("remove system")
            || a.contains("kill -9")
            || a.contains("disable sip")
            || a.contains("modify system")
        {
            return RiskLevel::High;
        }

        // Medium: reversible but impactful.
        if a.contains("kill")
            || a.contains("quit")
            || a.contains("restart")
            || a.contains("disable")
            || a.contains("clear cache")
            || a.contains("purge")
        {
            return RiskLevel::Medium;
        }

        // Low: safe, reversible, minimal impact.
        RiskLevel::Low
    }

    /// op 292: Learn workflows — identify recurring usage patterns from
    /// the process tree, memory consumers, and app intelligence.
    #[allow(dead_code)]
    pub fn learn_workflows(&self) -> Vec<crate::twin::memory::WorkflowPattern> {
        // Start with any workflows already learned by the memory dimension.
        let mut patterns = self.twin.memory.learned_workflows.clone();

        // Infer workflows from currently running processes: group by
        // apparent category and treat co-running apps as a workflow.
        let mut running_apps: Vec<String> = self
            .twin
            .processes
            .process_tree
            .iter()
            .map(|p| p.name.clone())
            .filter(|n| !n.is_empty())
            .collect();
        running_apps.sort();
        running_apps.dedup();

        if running_apps.len() >= 2 {
            patterns.push(crate::twin::memory::WorkflowPattern {
                name: "Current active workload".to_string(),
                app_names: running_apps.iter().take(10).cloned().collect(),
                frequency: 0.5,
                estimated_footprint_bytes: self.twin.memory.used_bytes,
            });
        }

        // Infer from app intelligence: group apps by purpose.
        let mut by_purpose: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();
        for app in &self.twin.applications.apps {
            if let Some(purpose) = &app.purpose {
                by_purpose
                    .entry(purpose.clone())
                    .or_default()
                    .push(app.name.clone());
            }
        }
        for (purpose, apps) in by_purpose {
            if apps.len() >= 2 {
                patterns.push(crate::twin::memory::WorkflowPattern {
                    name: format!("{} workflow", purpose),
                    app_names: apps,
                    frequency: 0.3,
                    estimated_footprint_bytes: 0,
                });
            }
        }

        patterns
    }

    /// op 293: Learn acceptable tradeoffs — learn what tradeoffs the user
    /// accepts (e.g. speed vs battery, storage vs convenience).
    #[allow(dead_code)]
    pub fn learn_tradeoffs(&self) -> Vec<Tradeoff> {
        let mut tradeoffs = Vec::new();

        // Speed vs battery: if low power mode is off despite high energy use,
        // the user prefers performance over battery life.
        if !self.twin.hardware.power_state.low_power_mode
            && self.twin.energy.profile.compute_drain_pct > 40.0
        {
            tradeoffs.push(Tradeoff {
                name: "Performance over battery".to_string(),
                preference: "speed".to_string(),
                rationale: "Low Power Mode is disabled despite high compute energy drain"
                    .to_string(),
            });
        } else if self.twin.hardware.power_state.low_power_mode {
            tradeoffs.push(Tradeoff {
                name: "Battery over performance".to_string(),
                preference: "battery".to_string(),
                rationale: "Low Power Mode is enabled, indicating battery priority".to_string(),
            });
        }

        // Storage vs convenience: if many unused apps are kept, the user
        // prefers convenience over storage savings.
        let unused_count = self.twin.applications.unused_apps.len();
        if unused_count > 10 {
            tradeoffs.push(Tradeoff {
                name: "Convenience over storage".to_string(),
                preference: "convenience".to_string(),
                rationale: format!(
                    "{} unused apps retained, suggesting storage is not a priority",
                    unused_count
                ),
            });
        } else if unused_count <= 2 && self.twin.filesystem.total_size_bytes > 0 {
            let total_disk: u64 = self
                .twin
                .hardware
                .storage
                .iter()
                .map(|d| d.capacity_bytes)
                .sum();
            if total_disk > 0 {
                let free_pct = (1.0
                    - self.twin.filesystem.total_size_bytes as f64 / total_disk as f64)
                    * 100.0;
                if free_pct < 20.0 {
                    tradeoffs.push(Tradeoff {
                        name: "Storage efficiency over convenience".to_string(),
                        preference: "storage".to_string(),
                        rationale: "Few unused apps kept despite low disk space".to_string(),
                    });
                }
            }
        }

        // Background apps vs memory: if many background apps run under
        // pressure, the user prefers multitasking over memory headroom.
        if self.twin.memory.pressure_level >= 2 {
            let bg_count = self
                .twin
                .memory
                .top_consumers
                .iter()
                .filter(|c| c.is_idle)
                .count();
            if bg_count > 3 {
                tradeoffs.push(Tradeoff {
                    name: "Multitasking over memory headroom".to_string(),
                    preference: "multitasking".to_string(),
                    rationale: "Many idle apps kept open despite memory pressure".to_string(),
                });
            }
        }

        tradeoffs
    }

    /// op 294: Build personalized policies — create custom optimization
    /// policies based on learned patterns and tradeoffs.
    #[allow(dead_code)]
    pub fn build_personalized_policies(&self) -> Vec<Policy> {
        let tradeoffs = self.learn_tradeoffs();
        let mut policies = Vec::new();

        for t in &tradeoffs {
            match t.name.as_str() {
                "Performance over battery" => {
                    policies.push(Policy {
                        name: "Allow high-performance mode".to_string(),
                        trigger: "On AC power with compute-heavy workload".to_string(),
                        action: "Keep High Power Mode enabled".to_string(),
                        priority: "normal".to_string(),
                        enabled: true,
                    });
                }
                "Battery over performance" => {
                    policies.push(Policy {
                        name: "Prefer Low Power Mode".to_string(),
                        trigger: "On battery with moderate workload".to_string(),
                        action: "Enable Low Power Mode automatically".to_string(),
                        priority: "high".to_string(),
                        enabled: true,
                    });
                }
                "Convenience over storage" => {
                    policies.push(Policy {
                        name: "Tolerate unused apps".to_string(),
                        trigger: "Storage above 30% free".to_string(),
                        action: "Do not auto-remove unused apps".to_string(),
                        priority: "low".to_string(),
                        enabled: true,
                    });
                }
                "Storage efficiency over convenience" => {
                    policies.push(Policy {
                        name: "Auto-clean unused apps".to_string(),
                        trigger: "Storage below 20% free".to_string(),
                        action: "Recommend removal of unused apps".to_string(),
                        priority: "high".to_string(),
                        enabled: true,
                    });
                }
                "Multitasking over memory headroom" => {
                    policies.push(Policy {
                        name: "Aggressive memory management".to_string(),
                        trigger: "Memory pressure level >= 2".to_string(),
                        action: "Suspend idle apps and purge inactive memory".to_string(),
                        priority: "high".to_string(),
                        enabled: true,
                    });
                }
                _ => {}
            }
        }

        // Always include a baseline safety policy.
        policies.push(Policy {
            name: "Never auto-execute high-risk actions".to_string(),
            trigger: "Any action with risk level High or Critical".to_string(),
            action: "Require user confirmation before execution".to_string(),
            priority: "critical".to_string(),
            enabled: true,
        });

        policies
    }

    /// op 302: Recommend hardware upgrades — suggest hardware upgrades
    /// based on usage patterns and identified bottlenecks.
    #[allow(dead_code)]
    pub fn recommend_hardware_upgrades(&self) -> Vec<UpgradeRecommendation> {
        let mut recs = Vec::new();

        // Memory bottleneck.
        if self.twin.memory.pressure_level >= 2
            || self.twin.memory.utilization > 0.85
            || self.twin.memory.swap_used_bytes > 2_000_000_000
        {
            let current_gb = self.twin.hardware.memory.total_bytes as f64 / 1_073_741_824.0;
            let recommended_gb = (current_gb * 2.0).max(32.0);
            recs.push(UpgradeRecommendation {
                component: "Memory".to_string(),
                current: format!("{:.0} GB", current_gb),
                recommended: format!("{:.0} GB", recommended_gb),
                reason: "Frequent memory pressure and swap usage indicate insufficient RAM"
                    .to_string(),
                estimated_benefit: "Reduce swap, improve multitasking and app responsiveness"
                    .to_string(),
                priority: "High".to_string(),
            });
        }

        // Storage bottleneck.
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        if total_disk > 0 {
            let free_pct =
                (1.0 - self.twin.filesystem.total_size_bytes as f64 / total_disk as f64) * 100.0;
            if free_pct < 15.0 {
                recs.push(UpgradeRecommendation {
                    component: "Storage".to_string(),
                    current: format!("{:.0} GB", total_disk as f64 / 1_073_741_824.0),
                    recommended: format!(
                        "{:.0} GB",
                        (total_disk as f64 / 1_073_741_824.0 * 2.0).max(512.0)
                    ),
                    reason: "Low free disk space is degrading performance".to_string(),
                    estimated_benefit: "More headroom for caches, temp files, and future growth"
                        .to_string(),
                    priority: "Medium".to_string(),
                });
            }
        }

        // SSD wear.
        for disk in &self.twin.hardware.storage {
            if disk.is_ssd {
                if let Some(smart) = &disk.smart_health {
                    if let Some(hours) = smart.power_on_hours {
                        if hours > 35_000 {
                            recs.push(UpgradeRecommendation {
                                component: format!("SSD ({})", disk.name),
                                current: format!("{} power-on hours", hours),
                                recommended: "New SSD".to_string(),
                                reason: "SSD approaching end of rated lifespan".to_string(),
                                estimated_benefit: "Avoid data loss from SSD failure".to_string(),
                                priority: "High".to_string(),
                            });
                        }
                    }
                }
            }
        }

        // Battery degradation.
        if let Some(b) = &self.twin.energy.battery {
            if let Some(health) = b.health_pct {
                if health < 80.0 {
                    recs.push(UpgradeRecommendation {
                        component: "Battery".to_string(),
                        current: format!("{:.0}% health, {} cycles", health, b.cycle_count),
                        recommended: "Battery replacement".to_string(),
                        reason: "Battery health below 80% significantly reduces runtime"
                            .to_string(),
                        estimated_benefit: "Restore original battery runtime".to_string(),
                        priority: "Medium".to_string(),
                    });
                }
            }
        }

        recs
    }

    /// op 303: Recommend software changes — suggest software updates,
    /// alternative apps, or removals.
    #[allow(dead_code)]
    pub fn recommend_software_changes(&self) -> Vec<SoftwareRecommendation> {
        let mut recs = Vec::new();

        // Recommend removing unused apps.
        for app_name in &self.twin.applications.unused_apps {
            recs.push(SoftwareRecommendation {
                kind: "remove".to_string(),
                target: app_name.clone(),
                recommendation: format!("Remove unused app '{}'", app_name),
                reason: "App has not been launched recently and consumes storage".to_string(),
                priority: "Low".to_string(),
            });
        }

        // Recommend alternatives for unused apps.
        for (app_name, alt) in self.twin.applications.recommend_alternatives() {
            recs.push(SoftwareRecommendation {
                kind: "alternative".to_string(),
                target: app_name,
                recommendation: alt,
                reason: "A lighter or more efficient alternative may serve the same purpose"
                    .to_string(),
                priority: "Low".to_string(),
            });
        }

        // Recommend fixes for inefficient apps.
        for (app_name, fix) in self.twin.applications.recommend_fixes() {
            recs.push(SoftwareRecommendation {
                kind: "update".to_string(),
                target: app_name,
                recommendation: fix,
                reason: "App has elevated crash risk or corrupted preferences".to_string(),
                priority: "Medium".to_string(),
            });
        }

        // Recommend reducing startup items if there are too many.
        let startup_count = self.twin.software_genome.login_items.len();
        if startup_count > 15 {
            recs.push(SoftwareRecommendation {
                kind: "remove".to_string(),
                target: "Login items".to_string(),
                recommendation: format!("Disable {} non-essential login items", startup_count - 15),
                reason: "Too many startup items slow boot and consume background resources"
                    .to_string(),
                priority: "Medium".to_string(),
            });
        }

        recs
    }

    /// op 309: Allow AI agents to query state — provide a structured query
    /// interface for AI agents to ask about the twin's state.
    #[allow(dead_code)]
    pub fn query(&self, query: &str) -> QueryResult {
        let q = query.to_lowercase();
        let mut data = Vec::new();
        let answer;
        let confidence;

        if q.contains("health") {
            let health = self.compute_system_health();
            data.push(("health_score".to_string(), format!("{:.1}", health)));
            data.push((
                "anomaly_count".to_string(),
                self.twin.processes.anomalies.len().to_string(),
            ));
            answer = format!("System health is {:.1}/100.", health);
            confidence = 0.9;
        } else if q.contains("memory") {
            data.push((
                "utilization".to_string(),
                format!("{:.0}%", self.twin.memory.utilization * 100.0),
            ));
            data.push((
                "pressure_level".to_string(),
                self.twin.memory.pressure_level.to_string(),
            ));
            data.push((
                "swap_gb".to_string(),
                format!(
                    "{:.1}",
                    self.twin.memory.swap_used_bytes as f64 / 1_073_741_824.0
                ),
            ));
            answer = format!(
                "Memory is {:.0}% utilized, pressure level {}.",
                self.twin.memory.utilization * 100.0,
                self.twin.memory.pressure_level
            );
            confidence = 0.9;
        } else if q.contains("disk") || q.contains("storage") {
            let total: u64 = self
                .twin
                .hardware
                .storage
                .iter()
                .map(|d| d.capacity_bytes)
                .sum();
            let used = self.twin.filesystem.total_size_bytes;
            let free_pct = if total > 0 {
                (1.0 - used as f64 / total as f64) * 100.0
            } else {
                0.0
            };
            data.push((
                "total_gb".to_string(),
                format!("{:.0}", total as f64 / 1_073_741_824.0),
            ));
            data.push((
                "used_gb".to_string(),
                format!("{:.0}", used as f64 / 1_073_741_824.0),
            ));
            data.push(("free_pct".to_string(), format!("{:.1}", free_pct)));
            answer = format!("Disk is {:.1}% free.", free_pct);
            confidence = 0.9;
        } else if q.contains("battery") || q.contains("energy") {
            if let Some(b) = &self.twin.energy.battery {
                data.push(("charge_pct".to_string(), format!("{:.0}", b.charge_pct)));
                data.push(("cycle_count".to_string(), b.cycle_count.to_string()));
                if let Some(h) = b.health_pct {
                    data.push(("health_pct".to_string(), format!("{:.0}", h)));
                }
                answer = format!(
                    "Battery at {:.0}%, {} cycles, health {:?}%.",
                    b.charge_pct, b.cycle_count, b.health_pct
                );
                confidence = 0.9;
            } else {
                answer = "No battery detected (desktop Mac).".to_string();
                confidence = 0.8;
            }
        } else if q.contains("process") || q.contains("cpu") {
            let total_cpu: f64 = self
                .twin
                .processes
                .process_tree
                .iter()
                .map(|p| p.cpu_pct)
                .sum();
            data.push((
                "process_count".to_string(),
                self.twin.processes.process_tree.len().to_string(),
            ));
            data.push(("total_cpu_pct".to_string(), format!("{:.0}", total_cpu)));
            data.push((
                "anomaly_count".to_string(),
                self.twin.processes.anomalies.len().to_string(),
            ));
            answer = format!(
                "{} processes running, total CPU {:.0}%, {} anomalies.",
                self.twin.processes.process_tree.len(),
                total_cpu,
                self.twin.processes.anomalies.len()
            );
            confidence = 0.85;
        } else if q.contains("trust") {
            let trust = self.compute_trust_score();
            data.push(("trust_score".to_string(), format!("{:.2}", trust)));
            answer = format!("Trust score for autonomous action: {:.2}/1.0.", trust);
            confidence = 0.85;
        } else {
            data.push((
                "health_score".to_string(),
                format!("{:.1}", self.compute_system_health()),
            ));
            data.push((
                "trust_score".to_string(),
                format!("{:.2}", self.compute_trust_score()),
            ));
            answer =
                "Query not recognized. Available: health, memory, disk, battery, process, trust."
                    .to_string();
            confidence = 0.3;
        }

        QueryResult {
            query: query.to_string(),
            answer,
            data,
            confidence,
        }
    }

    /// op 310: Allow autonomous optimization — execute safe, reversible
    /// autonomous optimizations based on the trust score.
    #[allow(dead_code)]
    pub fn autonomous_optimize(&self) -> OptimizationResult {
        let trust_before = self.compute_trust_score();
        let mut actions_taken = Vec::new();
        let mut actions_skipped = Vec::new();

        // Only execute autonomous actions if trust is high enough.
        if trust_before < 0.5 {
            actions_skipped.push(
                "All actions skipped — trust score too low for autonomous execution".to_string(),
            );
            return OptimizationResult {
                actions_taken,
                actions_skipped,
                estimated_impact: "No actions taken".to_string(),
                trust_score_before: trust_before,
                trust_score_after: trust_before,
                success: false,
            };
        }

        // Execute only low-risk, reversible actions.
        for action in self.propose_autonomous_actions() {
            if action.risk_level == "Low" && action.is_reversible {
                actions_taken.push(action.action);
            } else {
                actions_skipped.push(format!(
                    "Skipped '{}': risk {} (not autonomous-safe)",
                    action.action, action.risk_level
                ));
            }
        }

        // Trust grows slightly with each successful autonomous action.
        let trust_after = (trust_before + actions_taken.len() as f64 * 0.01).min(1.0);
        let success = !actions_taken.is_empty();

        let estimated_impact = if success {
            format!(
                "{} safe action(s) executed autonomously",
                actions_taken.len()
            )
        } else {
            "No safe actions available".to_string()
        };

        OptimizationResult {
            actions_taken,
            actions_skipped,
            estimated_impact,
            trust_score_before: trust_before,
            trust_score_after: trust_after,
            success,
        }
    }

    /// op 311: Sandbox proposed changes — simulate a change in a sandbox
    /// before execution to assess safety and predicted outcome.
    #[allow(dead_code)]
    pub fn sandbox_simulation(action: &str) -> SandboxResult {
        let risk = Self::estimate_risk(action);
        let risk_str = match risk {
            RiskLevel::Low => "Low",
            RiskLevel::Medium => "Medium",
            RiskLevel::High => "High",
            RiskLevel::Critical => "Critical",
        };

        let a = action.to_lowercase();
        let mut side_effects = Vec::new();
        let predicted_outcome;

        if a.contains("kill") || a.contains("quit") {
            predicted_outcome = "Target process will terminate; memory and CPU freed".to_string();
            side_effects.push("Unsaved work in the target app may be lost".to_string());
        } else if a.contains("clear") && a.contains("cache") {
            predicted_outcome = "cache files removed; disk space reclaimed".to_string();
            side_effects
                .push("Apps may rebuild caches on next launch (slower first launch)".to_string());
        } else if a.contains("disable") {
            predicted_outcome = "Target component disabled until re-enabled".to_string();
            side_effects.push("Dependent features may stop working".to_string());
        } else if a.contains("delete") || a.contains("remove") {
            predicted_outcome = "Target files or apps permanently removed".to_string();
            side_effects.push("Data loss — action may not be reversible".to_string());
        } else if a.contains("format") || a.contains("erase") {
            predicted_outcome = "All data on target volume will be destroyed".to_string();
            side_effects.push("Complete and irreversible data loss".to_string());
        } else {
            predicted_outcome = "Action simulated — outcome uncertain".to_string();
        }

        let safe_to_execute = matches!(risk, RiskLevel::Low | RiskLevel::Medium);

        SandboxResult {
            action: action.to_string(),
            simulated: true,
            predicted_outcome,
            risk_level: risk_str.to_string(),
            safe_to_execute,
            side_effects,
        }
    }

    /// op 317: Share anonymized benchmarks — create anonymized performance
    /// data for comparison with similar machines.
    #[allow(dead_code)]
    pub fn generate_anonymized_benchmark(&self) -> BenchmarkData {
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        let disk_utilization = if total_disk > 0 {
            self.twin.filesystem.total_size_bytes as f64 / total_disk as f64
        } else {
            0.0
        };

        BenchmarkData {
            // Anonymized: no serial numbers or personal identifiers.
            model_identifier: self.twin.hardware.model_identifier.clone(),
            soc_generation: self.twin.hardware.soc_generation.clone(),
            cpu_cores: self.twin.hardware.cpu_cores.total_logical_cores,
            memory_bytes: self.twin.hardware.memory.total_bytes,
            storage_bytes: total_disk,
            health_score: self.compute_system_health(),
            memory_utilization: self.twin.memory.utilization,
            disk_utilization,
            process_count: self.twin.processes.process_tree.len(),
            anomaly_count: self.twin.processes.anomalies.len(),
        }
    }

    /// op 318: Compare against similar Macs — compare this Mac's performance
    /// to similar machines using anonymized benchmark data.
    #[allow(dead_code)]
    pub fn compare_to_similar(&self, benchmark: &BenchmarkData) -> ComparisonResult {
        let my_health = self.compute_system_health();
        let my_mem_util = self.twin.memory.utilization;
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        let my_disk_util = if total_disk > 0 {
            self.twin.filesystem.total_size_bytes as f64 / total_disk as f64
        } else {
            0.0
        };

        // Heuristic percentile estimation: compare this Mac's metrics to
        // the benchmark's. Better-than-benchmark metrics push toward higher
        // percentiles (i.e. this Mac is healthier than the comparison).
        let percentile_health =
            ((my_health - benchmark.health_score + 50.0) / 100.0 * 100.0).clamp(1.0, 99.0);
        let percentile_memory = ((1.0 - my_mem_util - (1.0 - benchmark.memory_utilization) + 0.5)
            * 100.0)
            .clamp(1.0, 99.0);
        let percentile_disk = ((1.0 - my_disk_util - (1.0 - benchmark.disk_utilization) + 0.5)
            * 100.0)
            .clamp(1.0, 99.0);

        let mut strengths = Vec::new();
        let mut weaknesses = Vec::new();

        if percentile_health > 70.0 {
            strengths.push(format!(
                "Health score {:.1} is above average for similar Macs",
                my_health
            ));
        } else if percentile_health < 30.0 {
            weaknesses.push(format!(
                "Health score {:.1} is below average for similar Macs",
                my_health
            ));
        }

        if percentile_memory > 70.0 {
            strengths.push("Memory utilization is lower than similar Macs".to_string());
        } else if percentile_memory < 30.0 {
            weaknesses.push("Memory utilization is higher than similar Macs".to_string());
        }

        if percentile_disk > 70.0 {
            strengths.push("More free disk space than similar Macs".to_string());
        } else if percentile_disk < 30.0 {
            weaknesses.push("Less free disk space than similar Macs".to_string());
        }

        if self.twin.processes.anomalies.len() < benchmark.anomaly_count {
            strengths.push("Fewer process anomalies than similar Macs".to_string());
        } else if self.twin.processes.anomalies.len() > benchmark.anomaly_count {
            weaknesses.push("More process anomalies than similar Macs".to_string());
        }

        let summary = format!(
            "This Mac ranks at the {:.0}th percentile for health, {:.0}th for memory, and {:.0}th for disk vs. similar machines.",
            percentile_health, percentile_memory, percentile_disk
        );

        ComparisonResult {
            percentile_health,
            percentile_memory,
            percentile_disk,
            strengths,
            weaknesses,
            summary,
        }
    }

    /// op 319: Detect unique problems — find problems specific to this
    /// Mac's configuration that wouldn't appear on a typical machine.
    #[allow(dead_code)]
    pub fn detect_unique_problems(&self) -> Vec<String> {
        let mut problems = Vec::new();

        // Unique: suspicious apps (not common on most Macs).
        for app in &self.twin.applications.suspicious_apps {
            problems.push(format!("Suspicious app detected: {}", app));
        }

        // Unique: corrupted preferences.
        for bundle in self.twin.applications.corrupted_preference_apps() {
            problems.push(format!("App '{}' has corrupted preference files", bundle));
        }

        // Unique: duplicate app installations.
        for group in &self.twin.applications.duplicate_apps {
            if group.len() > 1 {
                problems.push(format!(
                    "Duplicate app installation detected: {} copies of {}",
                    group.len(),
                    group.first().unwrap_or(&"unknown".to_string())
                ));
            }
        }

        // Unique: abnormal battery aging.
        if self.twin.energy.has_abnormal_aging() {
            problems.push("Battery is aging abnormally for its cycle count".to_string());
        }

        // Unique: excessive wake events.
        if self.twin.energy.wake_causes.len() > 10 {
            problems.push(format!(
                "Excessive sleep wake events ({}): a process is preventing sleep",
                self.twin.energy.wake_causes.len()
            ));
        }

        // Unique: very high startup item count.
        let startup_count = self.twin.software_genome.login_items.len()
            + self.twin.software_genome.launch_agents.len();
        if startup_count > 30 {
            problems.push(format!(
                "Unusually high startup item count ({}): boot is significantly slowed",
                startup_count
            ));
        }

        // Unique: memory leak candidates.
        for leak in &self.twin.memory.leak_candidates {
            if leak.growth_rate_bytes_per_min > 100_000_000.0 {
                problems.push(format!(
                    "Active memory leak in '{}' (growing ~{:.0} MB/min)",
                    leak.name,
                    leak.growth_rate_bytes_per_min / 1_048_576.0
                ));
            }
        }

        problems
    }

    /// op 320: Become a continuous Mac OS layer — define a continuous
    /// monitoring schedule for all twin dimensions.
    #[allow(dead_code)]
    pub fn continuous_monitoring_plan(&self) -> MonitoringPlan {
        let mut intervals = Vec::new();
        let mut alerts = Vec::new();

        // Process monitoring — frequent, since processes change quickly.
        intervals.push(MonitoringInterval {
            dimension: "processes".to_string(),
            interval_secs: 10,
            reason: "Processes change rapidly; detect anomalies and crashes quickly".to_string(),
        });

        // Memory monitoring — frequent under pressure.
        let mem_interval = if self.twin.memory.pressure_level >= 2 {
            15
        } else {
            60
        };
        intervals.push(MonitoringInterval {
            dimension: "memory".to_string(),
            interval_secs: mem_interval,
            reason: "Memory pressure can escalate quickly under heavy workloads".to_string(),
        });

        // Energy monitoring — moderate frequency.
        intervals.push(MonitoringInterval {
            dimension: "energy".to_string(),
            interval_secs: 120,
            reason: "Battery and energy drain change gradually".to_string(),
        });

        // Storage monitoring — infrequent.
        intervals.push(MonitoringInterval {
            dimension: "storage".to_string(),
            interval_secs: 600,
            reason: "Disk usage changes slowly; monitor for exhaustion trends".to_string(),
        });

        // Hardware monitoring — infrequent.
        intervals.push(MonitoringInterval {
            dimension: "hardware".to_string(),
            interval_secs: 300,
            reason: "Hardware state (thermal, power) changes moderately".to_string(),
        });

        // Software monitoring — infrequent.
        intervals.push(MonitoringInterval {
            dimension: "software".to_string(),
            interval_secs: 3600,
            reason: "Software inventory changes rarely (installs/updates)".to_string(),
        });

        // Alerts based on current state.
        if self.twin.memory.pressure_level >= 2 {
            alerts.push(MonitoringAlert {
                condition: "Memory pressure level >= 2".to_string(),
                threshold: "level 2".to_string(),
                action: "Suspend idle apps and purge inactive memory".to_string(),
            });
        }
        let total_disk: u64 = self
            .twin
            .hardware
            .storage
            .iter()
            .map(|d| d.capacity_bytes)
            .sum();
        if total_disk > 0 {
            let free_pct =
                (1.0 - self.twin.filesystem.total_size_bytes as f64 / total_disk as f64) * 100.0;
            if free_pct < 15.0 {
                alerts.push(MonitoringAlert {
                    condition: "Disk free space below 15%".to_string(),
                    threshold: "15%".to_string(),
                    action: "Recommend cleanup of caches and large unused files".to_string(),
                });
            }
        }
        if self.twin.hardware.thermal.thermal_pressure != "Nominal"
            && !self.twin.hardware.thermal.thermal_pressure.is_empty()
        {
            alerts.push(MonitoringAlert {
                condition: "Thermal pressure above Nominal".to_string(),
                threshold: "Nominal".to_string(),
                action: "Reduce workload intensity to prevent throttling".to_string(),
            });
        }
        if !self.twin.processes.anomalies.is_empty() {
            alerts.push(MonitoringAlert {
                condition: "Process anomalies detected".to_string(),
                threshold: "0 anomalies".to_string(),
                action: "Investigate and optionally restart anomalous processes".to_string(),
            });
        }

        let summary = format!(
            "Continuous monitoring plan: {} dimension(s) monitored at intervals from {}s to {}s, with {} active alert(s).",
            intervals.len(),
            intervals.iter().map(|i| i.interval_secs).min().unwrap_or(0),
            intervals.iter().map(|i| i.interval_secs).max().unwrap_or(0),
            alerts.len()
        );

        MonitoringPlan {
            intervals,
            alerts,
            summary,
        }
    }

    /// Get the twin reference.
    pub fn twin(&self) -> &DigitalTwin {
        &self.twin
    }
}

/// Extract a process name from an action string like "kill Chrome".
fn extract_process_name(action: &str) -> Option<String> {
    let words: Vec<&str> = action.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        if (*word == "kill" || *word == "quit" || *word == "force-kill" || *word == "force")
            && i + 1 < words.len()
        {
            return Some(words[i + 1].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::twin::app_agent::AppIntelligenceGraph;
    use crate::twin::energy::EnergyTwin;
    use crate::twin::fs_graph::FilesystemGraph;
    use crate::twin::hardware::{
        CpuTopology, GpuInfo, HardwareMemory, HardwareProfile, NeuralEngineInfo, Peripherals,
        PowerState, StorageDevice, ThermalInfo,
    };
    use crate::twin::memory::MemoryIntelligence;
    use crate::twin::model::DigitalTwin;
    use crate::twin::process::{ProcessAnomaly, ProcessIntelligenceGraph, ProcessNode};
    use crate::twin::software_genome::SoftwareGenome;
    use std::collections::HashMap;

    /// Build a mock DigitalTwin with controllable fields for testing.
    fn mock_twin() -> DigitalTwin {
        DigitalTwin {
            timestamp_ms: 1_000_000,
            hardware: HardwareProfile {
                model_identifier: "Mac14,2".to_string(),
                soc_generation: "Apple Silicon".to_string(),
                cpu_cores: CpuTopology {
                    total_logical_cores: 8,
                    performance_cores: 4,
                    efficiency_cores: 4,
                },
                gpu: GpuInfo {
                    core_count: 10,
                    utilization_pct: None,
                },
                neural_engine: NeuralEngineInfo {
                    core_count: 16,
                    utilization_pct: None,
                },
                memory: HardwareMemory {
                    total_bytes: 16_000_000_000,
                    bandwidth_gbps: None,
                },
                storage: vec![StorageDevice {
                    name: "Macintosh HD".to_string(),
                    capacity_bytes: 500_000_000_000,
                    is_ssd: true,
                    smart_health: None,
                }],
                battery: None,
                thermal: ThermalInfo {
                    cpu_temp_c: Some(45.0),
                    gpu_temp_c: None,
                    fan_speeds_rpm: Vec::new(),
                    thermal_pressure: "Nominal".to_string(),
                },
                peripherals: Peripherals {
                    usb_devices: Vec::new(),
                    thunderbolt_devices: Vec::new(),
                    bluetooth_devices: Vec::new(),
                    external_displays: Vec::new(),
                    audio_devices: Vec::new(),
                    network_interfaces: Vec::new(),
                },
                power_state: PowerState {
                    is_on_battery: false,
                    is_charging: false,
                    sleep_state: "Active".to_string(),
                    low_power_mode: false,
                },
                fingerprint: String::new(),
            },
            software_genome: SoftwareGenome::collect(),
            filesystem: FilesystemGraph::collect(),
            processes: ProcessIntelligenceGraph::empty(),
            memory: MemoryIntelligence::collect(),
            energy: EnergyTwin::collect(),
            applications: AppIntelligenceGraph::collect(),
            health_score: 80.0,
            trust_score: 0.5,
            metadata: HashMap::new(),
        }
    }

    #[test]
    fn test_ask_why_slow_with_cpu_spike() {
        let mut twin = mock_twin();
        twin.processes.process_tree.push(ProcessNode {
            pid: 1234,
            name: "Chrome".to_string(),
            owner: Some("user".to_string()),
            parent_pid: Some(1),
            children: Vec::new(),
            cpu_pct: 95.0,
            gpu_pct: None,
            memory_bytes: 2_000_000_000,
            allocation_count: None,
            disk_read_bytes: None,
            disk_write_bytes: None,
            network_bytes: None,
            energy_impact: None,
            wakeup_count: None,
            state: "R".to_string(),
            thread_count: Some(20),
            start_time_secs: Some(1000),
        });
        twin.processes.anomalies.push(ProcessAnomaly {
            pid: 1234,
            name: "Chrome".to_string(),
            anomaly_type: "cpu_spike".to_string(),
            description: "Process using 95.0% CPU".to_string(),
            severity: "critical".to_string(),
        });

        let engine = ReasoningEngine::new(twin);
        let result = engine.ask("Why is my Mac slow?");
        assert!(result.answer.contains("CPU"));
        assert!(!result.evidence.is_empty());
        assert!(result.confidence > 0.5);
        assert!(!result.recommended_actions.is_empty());
    }

    #[test]
    fn test_ask_why_slow_with_memory_pressure() {
        let mut twin = mock_twin();
        twin.memory.pressure_level = 3;
        twin.memory.utilization = 0.92;
        twin.memory.swap_used_bytes = 2_000_000_000;

        let engine = ReasoningEngine::new(twin);
        let result = engine.ask("why is my mac slow?");
        assert!(result.answer.contains("Memory pressure"));
        assert!(result
            .evidence
            .iter()
            .any(|e| e.contains("Memory pressure")));
    }

    #[test]
    fn test_ask_what_changed_with_regressions() {
        let mut twin = mock_twin();
        twin.memory.pressure_level = 2;

        let engine = ReasoningEngine::new(twin);
        let result = engine.ask("What changed?");
        assert!(result.answer.contains("Memory pressure"));
        assert!(!result.evidence.is_empty());
    }

    #[test]
    fn test_ask_what_changed_no_regressions() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let result = engine.ask("What changed?");
        assert!(result.answer.contains("No significant regressions"));
    }

    #[test]
    fn test_ask_what_caused_with_anomalies() {
        let mut twin = mock_twin();
        twin.processes.anomalies.push(ProcessAnomaly {
            pid: 100,
            name: "Slack".to_string(),
            anomaly_type: "memory_leak".to_string(),
            description: "Using 3.0 GB".to_string(),
            severity: "warning".to_string(),
        });

        let engine = ReasoningEngine::new(twin);
        let result = engine.ask("What caused this?");
        assert!(result.answer.contains("Slack"));
        assert!(result.answer.contains("leaking memory"));
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_ask_general_question() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let result = engine.ask("How is my system doing?");
        assert!(result.answer.contains("System health"));
        assert!(!result.evidence.is_empty());
    }

    #[test]
    fn test_create_optimization_plan_empty() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let plan = engine.create_optimization_plan();
        assert!(plan.is_reversible);
        assert!(plan.overall_risk == "Low");
    }

    #[test]
    fn test_create_optimization_plan_with_cpu_anomalies() {
        let mut twin = mock_twin();
        twin.processes.anomalies.push(ProcessAnomaly {
            pid: 1,
            name: "Xcode".to_string(),
            anomaly_type: "cpu_spike".to_string(),
            description: "90% CPU".to_string(),
            severity: "critical".to_string(),
        });

        let engine = ReasoningEngine::new(twin);
        let plan = engine.create_optimization_plan();
        assert!(plan.steps.iter().any(|s| s.name.contains("high-CPU")));
        assert!(plan.steps[0].estimated_impact == "High");
    }

    #[test]
    fn test_create_optimization_plan_with_memory_pressure() {
        let mut twin = mock_twin();
        twin.memory.pressure_level = 2;

        let engine = ReasoningEngine::new(twin);
        let plan = engine.create_optimization_plan();
        assert!(plan.steps.iter().any(|s| s.name == "Free memory"));
    }

    #[test]
    fn test_create_optimization_plan_with_low_disk() {
        let mut twin = mock_twin();
        twin.filesystem.total_size_bytes = 480_000_000_000; // 96% of 500GB

        let engine = ReasoningEngine::new(twin);
        let plan = engine.create_optimization_plan();
        assert!(plan.steps.iter().any(|s| s.name == "Clear caches"));
    }

    #[test]
    fn test_create_optimization_plan_with_many_startup_items() {
        let mut twin = mock_twin();
        twin.software_genome.login_items = (0..25)
            .map(|i| crate::twin::software_genome::SoftwareComponent {
                name: format!("item_{}", i),
                version: None,
                path: format!("/path/{}", i),
                size_bytes: 1000,
            })
            .collect();

        let engine = ReasoningEngine::new(twin);
        let plan = engine.create_optimization_plan();
        assert!(plan.steps.iter().any(|s| s.name == "Disable startup items"));
    }

    #[test]
    fn test_simulate_kill_known_process() {
        let mut twin = mock_twin();
        twin.processes.process_tree.push(ProcessNode {
            pid: 500,
            name: "Chrome".to_string(),
            owner: None,
            parent_pid: None,
            children: Vec::new(),
            cpu_pct: 60.0,
            gpu_pct: None,
            memory_bytes: 3_000_000_000,
            allocation_count: None,
            disk_read_bytes: None,
            disk_write_bytes: None,
            network_bytes: None,
            energy_impact: None,
            wakeup_count: None,
            state: "R".to_string(),
            thread_count: None,
            start_time_secs: None,
        });

        let engine = ReasoningEngine::new(twin);
        let result = engine.simulate("kill Chrome");
        assert!(result.answer.contains("Chrome"));
        assert!(result.answer.contains("memory"));
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_simulate_kill_unknown_process() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let result = engine.simulate("kill NonexistentApp");
        assert!(result.answer.contains("not found") || result.answer.contains("No measurable"));
        assert!(result.confidence < 0.5);
    }

    #[test]
    fn test_simulate_clear_caches() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let result = engine.simulate("clear caches");
        assert!(result.answer.contains("purgeable") || result.answer.contains("cache"));
        assert!(result.confidence > 0.5);
    }

    #[test]
    fn test_simulate_free_memory() {
        let mut twin = mock_twin();
        twin.memory.total_bytes = 16_000_000_000;
        twin.memory.used_bytes = 12_000_000_000;
        twin.memory.utilization = 0.75;

        let engine = ReasoningEngine::new(twin);
        let result = engine.simulate("free memory");
        assert!(result.answer.contains("memory") || result.answer.contains("utilization"));
    }

    #[test]
    fn test_simulate_unrecognized_action() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let result = engine.simulate("do something weird");
        assert!(result.confidence < 0.3);
    }

    #[test]
    fn test_detect_regressions_clean_system() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let regressions = engine.detect_regressions();
        assert!(regressions.is_empty());
    }

    #[test]
    fn test_detect_regressions_high_cpu() {
        let mut twin = mock_twin();
        twin.processes.process_tree.push(ProcessNode {
            pid: 1,
            name: "heavy".to_string(),
            owner: None,
            parent_pid: None,
            children: Vec::new(),
            cpu_pct: 75.0,
            gpu_pct: None,
            memory_bytes: 0,
            allocation_count: None,
            disk_read_bytes: None,
            disk_write_bytes: None,
            network_bytes: None,
            energy_impact: None,
            wakeup_count: None,
            state: "R".to_string(),
            thread_count: None,
            start_time_secs: None,
        });

        let engine = ReasoningEngine::new(twin);
        let regressions = engine.detect_regressions();
        assert!(regressions.iter().any(|r| r.contains("CPU usage")));
    }

    #[test]
    fn test_detect_regressions_memory_pressure() {
        let mut twin = mock_twin();
        twin.memory.pressure_level = 2;

        let engine = ReasoningEngine::new(twin);
        let regressions = engine.detect_regressions();
        assert!(regressions.iter().any(|r| r.contains("Memory pressure")));
    }

    #[test]
    fn test_detect_regressions_low_disk() {
        let mut twin = mock_twin();
        twin.filesystem.total_size_bytes = 490_000_000_000; // 2% free

        let engine = ReasoningEngine::new(twin);
        let regressions = engine.detect_regressions();
        assert!(regressions.iter().any(|r| r.contains("Disk free space")));
    }

    #[test]
    fn test_predict_problems_clean_system() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let predictions = engine.predict_problems();
        assert!(predictions.is_empty());
    }

    #[test]
    fn test_predict_problems_high_cycle_count() {
        use crate::twin::energy::BatteryModel;
        let mut twin = mock_twin();
        twin.energy.battery = Some(BatteryModel {
            charge_pct: 80.0,
            cycle_count: 900,
            condition: "Normal".to_string(),
            is_charging: false,
            is_plugged: true,
            time_remaining_mins: None,
            design_capacity_mah: None,
            current_capacity_mah: None,
            health_pct: Some(78.0),
            abnormal_aging: false,
        });

        let engine = ReasoningEngine::new(twin);
        let predictions = engine.predict_problems();
        assert!(predictions.iter().any(|p| p.contains("cycles")));
    }

    #[test]
    fn test_predict_problems_storage_growth() {
        let mut twin = mock_twin();
        twin.filesystem.storage_growth_trend = Some(5.0);
        twin.filesystem.exhaustion_forecast_days = Some(30);

        let engine = ReasoningEngine::new(twin);
        let predictions = engine.predict_problems();
        assert!(predictions.iter().any(|p| p.contains("exhausted")));
    }

    #[test]
    fn test_predict_problems_memory_pressure_forecast() {
        use crate::twin::memory::PressureForecast;
        let mut twin = mock_twin();
        twin.memory.pressure_forecast = Some(PressureForecast {
            minutes_to_critical: Some(30),
            predicted_utilization_1h: 0.95,
            confidence: 0.8,
        });

        let engine = ReasoningEngine::new(twin);
        let predictions = engine.predict_problems();
        assert!(predictions.iter().any(|p| p.contains("pressure")));
    }

    #[test]
    fn test_compute_system_health_clean() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let health = engine.compute_system_health();
        assert!(
            health > 70.0,
            "clean system should have high health, got {}",
            health
        );
        assert!(health <= 100.0);
    }

    #[test]
    fn test_compute_system_health_with_anomalies() {
        let mut twin = mock_twin();
        for i in 0..10 {
            twin.processes.anomalies.push(ProcessAnomaly {
                pid: i,
                name: format!("bad_{}", i),
                anomaly_type: "cpu_spike".to_string(),
                description: "spike".to_string(),
                severity: "warning".to_string(),
            });
        }

        let engine = ReasoningEngine::new(twin);
        let health = engine.compute_system_health();
        assert!(
            health < 70.0,
            "system with many anomalies should have lower health, got {}",
            health
        );
    }

    #[test]
    fn test_compute_system_health_with_thermal_pressure() {
        let mut twin = mock_twin();
        twin.hardware.thermal.thermal_pressure = "Critical".to_string();

        let engine = ReasoningEngine::new(twin);
        let health = engine.compute_system_health();
        assert!(health < 90.0);
    }

    #[test]
    fn test_compute_trust_score_stable() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let trust = engine.compute_trust_score();
        assert!(
            trust > 0.5,
            "stable system should have high trust, got {}",
            trust
        );
        assert!(trust <= 1.0);
    }

    #[test]
    fn test_compute_trust_score_unstable() {
        let mut twin = mock_twin();
        for i in 0..15 {
            twin.processes.anomalies.push(ProcessAnomaly {
                pid: i,
                name: format!("bad_{}", i),
                anomaly_type: "cpu_spike".to_string(),
                description: "spike".to_string(),
                severity: "critical".to_string(),
            });
        }
        twin.memory.pressure_level = 3;

        let engine = ReasoningEngine::new(twin);
        let trust = engine.compute_trust_score();
        assert!(
            trust < 0.5,
            "unstable system should have low trust, got {}",
            trust
        );
    }

    #[test]
    fn test_propose_autonomous_actions_stable() {
        let mut twin = mock_twin();
        twin.memory.pressure_level = 1;

        let engine = ReasoningEngine::new(twin);
        let actions = engine.propose_autonomous_actions();
        assert!(!actions.is_empty());
        assert!(actions.iter().all(|a| a.is_reversible));
        assert!(actions.iter().all(|a| a.risk_level == "Low"));
    }

    #[test]
    fn test_propose_autonomous_actions_low_trust() {
        let mut twin = mock_twin();
        for i in 0..20 {
            twin.processes.anomalies.push(ProcessAnomaly {
                pid: i,
                name: format!("bad_{}", i),
                anomaly_type: "cpu_spike".to_string(),
                description: "spike".to_string(),
                severity: "critical".to_string(),
            });
        }

        let engine = ReasoningEngine::new(twin);
        let actions = engine.propose_autonomous_actions();
        assert!(
            actions.is_empty(),
            "low-trust system should have no autonomous actions"
        );
    }

    #[test]
    fn test_extract_process_name() {
        assert_eq!(
            extract_process_name("kill Chrome"),
            Some("Chrome".to_string())
        );
        assert_eq!(
            extract_process_name("quit Slack"),
            Some("Slack".to_string())
        );
        assert_eq!(
            extract_process_name("force-kill Xcode"),
            Some("Xcode".to_string())
        );
        assert_eq!(extract_process_name("clear caches"), None);
    }

    #[test]
    fn test_twin_accessor() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin.clone());
        assert_eq!(engine.twin().timestamp_ms, twin.timestamp_ms);
    }

    // ═══════════════════════════════════════════════════════════════════
    //  Tests for newly added operations (ops 119, 272, 276, 277, 281,
    //  289, 292, 293, 294, 302, 303, 309, 310, 311, 317, 318, 319, 320)
    // ═══════════════════════════════════════════════════════════════════

    #[test]
    fn test_simulate_cleanup_empty() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let impact = engine.simulate_cleanup();
        assert!(impact.time_saved_secs > 0);
        assert!(impact.risk_level == "Low" || impact.risk_level == "Medium");
        assert!(impact.summary.contains("GB"));
    }

    #[test]
    fn test_simulate_cleanup_with_duplicates() {
        use crate::twin::fs_graph::DuplicateCluster;
        use std::path::PathBuf;
        let mut twin = mock_twin();
        twin.filesystem.duplicate_clusters.push(DuplicateCluster {
            hash: "abc".to_string(),
            files: vec![
                PathBuf::from("/a.txt"),
                PathBuf::from("/b.txt"),
                PathBuf::from("/c.txt"),
            ],
            total_size_bytes: 300_000_000,
            recommended_keep: Some(PathBuf::from("/a.txt")),
        });

        let engine = ReasoningEngine::new(twin);
        let impact = engine.simulate_cleanup();
        // 3 copies of 300MB → 200MB freed (keep one).
        assert!(impact.space_freed_bytes >= 200_000_000);
    }

    #[test]
    fn test_simulate_cleanup_with_swap() {
        let mut twin = mock_twin();
        twin.memory.swap_used_bytes = 3_000_000_000;

        let engine = ReasoningEngine::new(twin);
        let impact = engine.simulate_cleanup();
        assert_eq!(impact.time_saved_secs, 300);
    }

    #[test]
    fn test_recommend_workflow_changes_clean() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let changes = engine.recommend_workflow_changes();
        // Clean system may have zero or few changes.
        assert!(changes.iter().all(|c| !c.name.is_empty()));
    }

    #[test]
    fn test_recommend_workflow_changes_many_startup_items() {
        let mut twin = mock_twin();
        twin.software_genome.login_items = (0..20)
            .map(|i| crate::twin::software_genome::SoftwareComponent {
                name: format!("item_{}", i),
                version: None,
                path: format!("/path/{}", i),
                size_bytes: 1000,
            })
            .collect();

        let engine = ReasoningEngine::new(twin);
        let changes = engine.recommend_workflow_changes();
        assert!(changes.iter().any(|c| c.name.contains("startup items")));
    }

    #[test]
    fn test_recommend_workflow_changes_background_drain() {
        let mut twin = mock_twin();
        twin.energy.profile.background_drain_pct = 50.0;

        let engine = ReasoningEngine::new(twin);
        let changes = engine.recommend_workflow_changes();
        assert!(changes.iter().any(|c| c.name.contains("background energy")));
    }

    #[test]
    fn test_build_knowledge_graph() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let graph = engine.build_knowledge_graph();
        // Should have nodes from multiple dimensions.
        assert!(!graph.nodes.is_empty());
        assert!(!graph.dimensions.is_empty());
        assert!(graph.dimensions.contains(&"hardware".to_string()));
        assert!(graph.dimensions.contains(&"software".to_string()));
        assert!(graph.dimensions.contains(&"memory".to_string()));
        // CPU and memory should be connected by an edge.
        assert!(graph
            .edges
            .iter()
            .any(|e| e.relationship == "shares_unified_memory"));
    }

    #[test]
    fn test_build_knowledge_graph_with_processes() {
        let mut twin = mock_twin();
        twin.processes.process_tree.push(ProcessNode {
            pid: 42,
            name: "TestApp".to_string(),
            owner: None,
            parent_pid: Some(1),
            children: Vec::new(),
            cpu_pct: 10.0,
            gpu_pct: None,
            memory_bytes: 500_000_000,
            allocation_count: None,
            disk_read_bytes: None,
            disk_write_bytes: None,
            network_bytes: None,
            energy_impact: None,
            wakeup_count: None,
            state: "R".to_string(),
            thread_count: None,
            start_time_secs: None,
        });

        let engine = ReasoningEngine::new(twin);
        let graph = engine.build_knowledge_graph();
        assert!(graph.nodes.iter().any(|n| n.id == "process:42"));
        assert!(graph.edges.iter().any(|e| e.relationship == "parent_of"));
    }

    #[test]
    fn test_save_snapshot() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin.clone());
        let snapshot = engine.save_snapshot();
        assert_eq!(snapshot.timestamp_ms, twin.timestamp_ms);
        assert!(snapshot.health_score > 0.0);
        assert!(snapshot.trust_score > 0.0);
        assert_eq!(snapshot.process_count, twin.processes.process_tree.len());
    }

    #[test]
    fn test_compare_snapshots_no_change() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin.clone());
        let snapshot = engine.save_snapshot();
        let changes = ReasoningEngine::compare_snapshots(&snapshot, &twin);
        assert!(changes.is_empty(), "no changes expected, got {:?}", changes);
    }

    #[test]
    fn test_compare_snapshots_memory_change() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin.clone());
        let mut snapshot = engine.save_snapshot();
        snapshot.memory_utilization = 0.3; // was ~collected value

        let changes = ReasoningEngine::compare_snapshots(&snapshot, &twin);
        assert!(changes.iter().any(|c| c.dimension == "memory"));
    }

    #[test]
    fn test_compare_snapshots_anomaly_change() {
        let mut twin = mock_twin();
        // Add an anomaly to the new twin.
        twin.processes.anomalies.push(ProcessAnomaly {
            pid: 1,
            name: "bad".to_string(),
            anomaly_type: "cpu_spike".to_string(),
            description: "spike".to_string(),
            severity: "warning".to_string(),
        });
        let engine = ReasoningEngine::new(mock_twin());
        let snapshot = engine.save_snapshot(); // 0 anomalies
        let changes = ReasoningEngine::compare_snapshots(&snapshot, &twin);
        assert!(changes.iter().any(|c| c.description.contains("Anomaly")));
    }

    #[test]
    fn test_recommend_preventive_actions_clean() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let actions = engine.recommend_preventive_actions();
        assert!(
            actions.is_empty(),
            "clean system should have no preventive actions"
        );
    }

    #[test]
    fn test_recommend_preventive_actions_with_predictions() {
        let mut twin = mock_twin();
        twin.filesystem.storage_growth_trend = Some(5.0);
        twin.filesystem.exhaustion_forecast_days = Some(30);

        let engine = ReasoningEngine::new(twin);
        let actions = engine.recommend_preventive_actions();
        assert!(!actions.is_empty());
        assert!(actions
            .iter()
            .any(|a| a.action.contains("SSD") || a.action.contains("write")));
    }

    #[test]
    fn test_recommend_preventive_actions_battery() {
        use crate::twin::energy::BatteryModel;
        let mut twin = mock_twin();
        twin.energy.battery = Some(BatteryModel {
            charge_pct: 80.0,
            cycle_count: 900,
            condition: "Normal".to_string(),
            is_charging: false,
            is_plugged: true,
            time_remaining_mins: None,
            design_capacity_mah: None,
            current_capacity_mah: None,
            health_pct: Some(78.0),
            abnormal_aging: false,
        });

        let engine = ReasoningEngine::new(twin);
        let actions = engine.recommend_preventive_actions();
        assert!(actions.iter().any(|a| a.action.contains("Battery")));
    }

    #[test]
    fn test_estimate_risk_low() {
        assert_eq!(
            ReasoningEngine::estimate_risk("show system status"),
            RiskLevel::Low
        );
        assert_eq!(
            ReasoningEngine::estimate_risk("monitor cpu"),
            RiskLevel::Low
        );
    }

    #[test]
    fn test_estimate_risk_medium() {
        assert_eq!(
            ReasoningEngine::estimate_risk("kill Chrome"),
            RiskLevel::Medium
        );
        assert_eq!(
            ReasoningEngine::estimate_risk("clear caches"),
            RiskLevel::Medium
        );
        assert_eq!(
            ReasoningEngine::estimate_risk("disable startup item"),
            RiskLevel::Medium
        );
    }

    #[test]
    fn test_estimate_risk_high() {
        assert_eq!(
            ReasoningEngine::estimate_risk("uninstall Xcode"),
            RiskLevel::High
        );
        assert_eq!(
            ReasoningEngine::estimate_risk("remove system framework"),
            RiskLevel::High
        );
    }

    #[test]
    fn test_estimate_risk_critical() {
        assert_eq!(
            ReasoningEngine::estimate_risk("format disk"),
            RiskLevel::Critical
        );
        assert_eq!(
            ReasoningEngine::estimate_risk("erase Macintosh HD"),
            RiskLevel::Critical
        );
        assert_eq!(
            ReasoningEngine::estimate_risk("factory reset"),
            RiskLevel::Critical
        );
    }

    #[test]
    fn test_learn_workflows_empty() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let patterns = engine.learn_workflows();
        // With no running processes, may still infer from app purposes.
        assert!(patterns.iter().all(|p| !p.name.is_empty()));
    }

    #[test]
    fn test_learn_workflows_with_processes() {
        let mut twin = mock_twin();
        twin.processes.process_tree.push(ProcessNode {
            pid: 1,
            name: "Safari".to_string(),
            owner: None,
            parent_pid: None,
            children: Vec::new(),
            cpu_pct: 5.0,
            gpu_pct: None,
            memory_bytes: 500_000_000,
            allocation_count: None,
            disk_read_bytes: None,
            disk_write_bytes: None,
            network_bytes: None,
            energy_impact: None,
            wakeup_count: None,
            state: "R".to_string(),
            thread_count: None,
            start_time_secs: None,
        });
        twin.processes.process_tree.push(ProcessNode {
            pid: 2,
            name: "Mail".to_string(),
            owner: None,
            parent_pid: None,
            children: Vec::new(),
            cpu_pct: 2.0,
            gpu_pct: None,
            memory_bytes: 300_000_000,
            allocation_count: None,
            disk_read_bytes: None,
            disk_write_bytes: None,
            network_bytes: None,
            energy_impact: None,
            wakeup_count: None,
            state: "R".to_string(),
            thread_count: None,
            start_time_secs: None,
        });

        let engine = ReasoningEngine::new(twin);
        let patterns = engine.learn_workflows();
        assert!(patterns.iter().any(|p| p.name.contains("active workload")));
    }

    #[test]
    fn test_learn_tradeoffs_performance() {
        let mut twin = mock_twin();
        twin.energy.profile.compute_drain_pct = 50.0;
        twin.hardware.power_state.low_power_mode = false;

        let engine = ReasoningEngine::new(twin);
        let tradeoffs = engine.learn_tradeoffs();
        assert!(tradeoffs
            .iter()
            .any(|t| t.name.contains("Performance over battery")));
    }

    #[test]
    fn test_learn_tradeoffs_battery() {
        let mut twin = mock_twin();
        twin.hardware.power_state.low_power_mode = true;

        let engine = ReasoningEngine::new(twin);
        let tradeoffs = engine.learn_tradeoffs();
        assert!(tradeoffs
            .iter()
            .any(|t| t.name.contains("Battery over performance")));
    }

    #[test]
    fn test_learn_tradeoffs_multitasking() {
        use crate::twin::memory::MemoryConsumer;
        let mut twin = mock_twin();
        twin.memory.pressure_level = 2;
        for i in 0..5 {
            twin.memory.top_consumers.push(MemoryConsumer {
                pid: i,
                name: format!("app_{}", i),
                memory_bytes: 1_000_000_000,
                is_idle: true,
                allocation_rate_bytes_per_min: None,
                efficiency_score: None,
            });
        }

        let engine = ReasoningEngine::new(twin);
        let tradeoffs = engine.learn_tradeoffs();
        assert!(tradeoffs.iter().any(|t| t.name.contains("Multitasking")));
    }

    #[test]
    fn test_build_personalized_policies() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let policies = engine.build_personalized_policies();
        // Always includes the baseline safety policy.
        assert!(policies.iter().any(|p| p.name.contains("high-risk")));
        assert!(policies.iter().all(|p| p.enabled));
    }

    #[test]
    fn test_build_personalized_policies_with_tradeoffs() {
        let mut twin = mock_twin();
        twin.hardware.power_state.low_power_mode = true;

        let engine = ReasoningEngine::new(twin);
        let policies = engine.build_personalized_policies();
        assert!(policies.iter().any(|p| p.name.contains("Low Power Mode")));
    }

    #[test]
    fn test_recommend_hardware_upgrades_clean() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let recs = engine.recommend_hardware_upgrades();
        assert!(recs.is_empty(), "clean system should have no upgrade recs");
    }

    #[test]
    fn test_recommend_hardware_upgrades_memory() {
        let mut twin = mock_twin();
        twin.memory.pressure_level = 3;
        twin.memory.utilization = 0.90;
        twin.memory.swap_used_bytes = 3_000_000_000;

        let engine = ReasoningEngine::new(twin);
        let recs = engine.recommend_hardware_upgrades();
        assert!(recs.iter().any(|r| r.component == "Memory"));
    }

    #[test]
    fn test_recommend_hardware_upgrades_storage() {
        let mut twin = mock_twin();
        twin.filesystem.total_size_bytes = 490_000_000_000; // ~2% free

        let engine = ReasoningEngine::new(twin);
        let recs = engine.recommend_hardware_upgrades();
        assert!(recs.iter().any(|r| r.component == "Storage"));
    }

    #[test]
    fn test_recommend_hardware_upgrades_battery() {
        use crate::twin::energy::BatteryModel;
        let mut twin = mock_twin();
        twin.energy.battery = Some(BatteryModel {
            charge_pct: 50.0,
            cycle_count: 500,
            condition: "Normal".to_string(),
            is_charging: false,
            is_plugged: false,
            time_remaining_mins: None,
            design_capacity_mah: None,
            current_capacity_mah: None,
            health_pct: Some(75.0),
            abnormal_aging: false,
        });

        let engine = ReasoningEngine::new(twin);
        let recs = engine.recommend_hardware_upgrades();
        assert!(recs.iter().any(|r| r.component == "Battery"));
    }

    #[test]
    fn test_recommend_software_changes() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let recs = engine.recommend_software_changes();
        // May be empty on a clean mock, but should always return a Vec.
        assert!(recs.iter().all(|r| !r.kind.is_empty()));
    }

    #[test]
    fn test_recommend_software_changes_many_startup_items() {
        let mut twin = mock_twin();
        twin.software_genome.login_items = (0..20)
            .map(|i| crate::twin::software_genome::SoftwareComponent {
                name: format!("item_{}", i),
                version: None,
                path: format!("/path/{}", i),
                size_bytes: 1000,
            })
            .collect();

        let engine = ReasoningEngine::new(twin);
        let recs = engine.recommend_software_changes();
        assert!(recs.iter().any(|r| r.target == "Login items"));
    }

    #[test]
    fn test_query_health() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let result = engine.query("health");
        assert!(result.answer.contains("health"));
        assert!(result.confidence > 0.5);
        assert!(result.data.iter().any(|(k, _)| k == "health_score"));
    }

    #[test]
    fn test_query_memory() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let result = engine.query("memory");
        assert!(result.answer.contains("Memory"));
        assert!(result.data.iter().any(|(k, _)| k == "utilization"));
    }

    #[test]
    fn test_query_disk() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let result = engine.query("disk");
        assert!(result.answer.contains("free"));
        assert!(result.data.iter().any(|(k, _)| k == "free_pct"));
    }

    #[test]
    fn test_query_unrecognized() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let result = engine.query("something random");
        assert!(result.confidence < 0.5);
        assert!(result.answer.contains("not recognized"));
    }

    #[test]
    fn test_autonomous_optimize_low_trust() {
        let mut twin = mock_twin();
        for i in 0..20 {
            twin.processes.anomalies.push(ProcessAnomaly {
                pid: i,
                name: format!("bad_{}", i),
                anomaly_type: "cpu_spike".to_string(),
                description: "spike".to_string(),
                severity: "critical".to_string(),
            });
        }

        let engine = ReasoningEngine::new(twin);
        let result = engine.autonomous_optimize();
        assert!(!result.success);
        assert!(result.actions_taken.is_empty());
        assert!(!result.actions_skipped.is_empty());
    }

    #[test]
    fn test_autonomous_optimize_high_trust() {
        let mut twin = mock_twin();
        twin.memory.pressure_level = 1;

        let engine = ReasoningEngine::new(twin);
        let result = engine.autonomous_optimize();
        assert!(result.success);
        assert!(!result.actions_taken.is_empty());
        assert!(result.trust_score_after >= result.trust_score_before);
    }

    #[test]
    fn test_sandbox_simulation_kill() {
        let result = ReasoningEngine::sandbox_simulation("kill Chrome");
        assert!(result.simulated);
        assert_eq!(result.risk_level, "Medium");
        assert!(result.safe_to_execute);
        assert!(!result.side_effects.is_empty());
    }

    #[test]
    fn test_sandbox_simulation_format() {
        let result = ReasoningEngine::sandbox_simulation("format disk");
        assert!(result.simulated);
        assert_eq!(result.risk_level, "Critical");
        assert!(!result.safe_to_execute);
        assert!(result
            .side_effects
            .iter()
            .any(|s| s.contains("irreversible")));
    }

    #[test]
    fn test_sandbox_simulation_clear_cache() {
        let result = ReasoningEngine::sandbox_simulation("clear caches");
        assert!(result.simulated);
        assert_eq!(result.risk_level, "Medium");
        assert!(result.predicted_outcome.contains("cache"));
    }

    #[test]
    fn test_generate_anonymized_benchmark() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let benchmark = engine.generate_anonymized_benchmark();
        assert_eq!(benchmark.model_identifier, "Mac14,2");
        assert_eq!(benchmark.cpu_cores, 8);
        assert!(benchmark.health_score > 0.0);
        assert!(benchmark.memory_bytes > 0);
    }

    #[test]
    fn test_compare_to_similar() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin.clone());
        let benchmark = engine.generate_anonymized_benchmark();
        let result = engine.compare_to_similar(&benchmark);
        assert!(result.percentile_health > 0.0 && result.percentile_health < 100.0);
        assert!(result.percentile_memory > 0.0 && result.percentile_memory < 100.0);
        assert!(!result.summary.is_empty());
    }

    #[test]
    fn test_compare_to_similar_worse_benchmark() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        // Benchmark with very good health → this Mac should rank lower.
        let benchmark = BenchmarkData {
            model_identifier: "Mac14,2".to_string(),
            soc_generation: "Apple Silicon".to_string(),
            cpu_cores: 8,
            memory_bytes: 16_000_000_000,
            storage_bytes: 500_000_000_000,
            health_score: 99.0,
            memory_utilization: 0.2,
            disk_utilization: 0.3,
            process_count: 50,
            anomaly_count: 0,
        };
        let result = engine.compare_to_similar(&benchmark);
        // This Mac has lower health than the benchmark.
        assert!(result.percentile_health < 60.0);
    }

    #[test]
    fn test_detect_unique_problems_clean() {
        let mut twin = mock_twin();
        // Ensure a truly clean state regardless of the host machine.
        twin.applications.suspicious_apps.clear();
        twin.applications.duplicate_apps.clear();
        twin.energy.wake_causes.clear();
        twin.memory.leak_candidates.clear();
        twin.software_genome.login_items.clear();
        twin.software_genome.launch_agents.clear();
        let engine = ReasoningEngine::new(twin);
        let problems = engine.detect_unique_problems();
        assert!(
            problems.is_empty(),
            "clean system should have no unique problems: {:?}",
            problems
        );
    }

    #[test]
    fn test_detect_unique_problems_many_wake_events() {
        use crate::twin::energy::WakeEvent;
        let mut twin = mock_twin();
        for i in 0..15 {
            twin.energy.wake_causes.push(WakeEvent {
                timestamp_ms: i as u64 * 1000,
                cause: "network".to_string(),
            });
        }

        let engine = ReasoningEngine::new(twin);
        let problems = engine.detect_unique_problems();
        assert!(problems.iter().any(|p| p.contains("wake events")));
    }

    #[test]
    fn test_detect_unique_problems_memory_leak() {
        use crate::twin::memory::MemoryLeakCandidate;
        let mut twin = mock_twin();
        twin.memory.leak_candidates.push(MemoryLeakCandidate {
            pid: 100,
            name: "LeakyApp".to_string(),
            growth_rate_bytes_per_min: 200_000_000.0,
            duration_mins: 30,
        });

        let engine = ReasoningEngine::new(twin);
        let problems = engine.detect_unique_problems();
        assert!(problems.iter().any(|p| p.contains("LeakyApp")));
    }

    #[test]
    fn test_detect_unique_problems_many_startup_items() {
        let mut twin = mock_twin();
        twin.software_genome.login_items = (0..35)
            .map(|i| crate::twin::software_genome::SoftwareComponent {
                name: format!("item_{}", i),
                version: None,
                path: format!("/path/{}", i),
                size_bytes: 1000,
            })
            .collect();

        let engine = ReasoningEngine::new(twin);
        let problems = engine.detect_unique_problems();
        assert!(problems.iter().any(|p| p.contains("startup item")));
    }

    #[test]
    fn test_continuous_monitoring_plan() {
        let twin = mock_twin();
        let engine = ReasoningEngine::new(twin);
        let plan = engine.continuous_monitoring_plan();
        assert!(!plan.intervals.is_empty());
        assert!(plan.intervals.len() >= 5); // at least 5 dimensions
        assert!(plan.intervals.iter().any(|i| i.dimension == "processes"));
        assert!(plan.intervals.iter().any(|i| i.dimension == "memory"));
        assert!(!plan.summary.is_empty());
    }

    #[test]
    fn test_continuous_monitoring_plan_with_alerts() {
        let mut twin = mock_twin();
        twin.memory.pressure_level = 2;
        twin.processes.anomalies.push(ProcessAnomaly {
            pid: 1,
            name: "bad".to_string(),
            anomaly_type: "cpu_spike".to_string(),
            description: "spike".to_string(),
            severity: "warning".to_string(),
        });

        let engine = ReasoningEngine::new(twin);
        let plan = engine.continuous_monitoring_plan();
        assert!(!plan.alerts.is_empty());
        assert!(plan
            .alerts
            .iter()
            .any(|a| a.condition.contains("Memory pressure")));
    }

    #[test]
    fn test_continuous_monitoring_plan_memory_interval_under_pressure() {
        let mut twin = mock_twin();
        twin.memory.pressure_level = 2;

        let engine = ReasoningEngine::new(twin);
        let plan = engine.continuous_monitoring_plan();
        let mem_interval = plan
            .intervals
            .iter()
            .find(|i| i.dimension == "memory")
            .unwrap();
        assert_eq!(mem_interval.interval_secs, 15);
    }
}
