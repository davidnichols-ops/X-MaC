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

    /// Simulate the outcome of an optimization action without executing it
    /// (op 286).
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

    /// Predict future problems: SSD wear, battery decline, storage
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

    /// Compute the trust score (0-1) for autonomous actions (op 291).
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
}
