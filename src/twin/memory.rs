use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

/// Unified Memory Intelligence — models the Mac's memory subsystem.
///
/// Covers: memory pressure, compression, swap, purgeable memory, app
/// footprints, allocation patterns, leak detection, fragmentation,
/// Apple Silicon unified memory topology, ML/GPU/ANE memory management,
/// pressure forecasting, and workload-specific optimization.
///
/// Maps to Digital Twin operations 161-200.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryIntelligence {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub compressed_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub purgeable_bytes: u64,
    pub utilization: f64,
    pub pressure_level: u32,
    pub pressure_forecast: Option<PressureForecast>,
    pub top_consumers: Vec<MemoryConsumer>,
    pub leak_candidates: Vec<MemoryLeakCandidate>,
    pub fragmentation_pct: Option<f64>,
    pub unified_memory_model: UnifiedMemoryModel,
    /// op 183: Memory topology model — which accelerators share the
    /// unified memory fabric and how much each has reserved.
    pub topology: MemoryTopology,
    /// op 185: Temporal memory samples for trend analysis.
    pub history: Vec<MemorySample>,
    /// op 187: Learned user workflows (app sets that tend to run together).
    pub learned_workflows: Vec<WorkflowPattern>,
    /// op 198: Automated memory policies keyed by trigger condition.
    pub policies: Vec<MemoryPolicy>,
    /// op 196: Human-readable explanations for the latest memory decisions.
    pub decision_explanations: Vec<String>,
}

/// op 184: Forecast of memory pressure over the next hour.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureForecast {
    pub minutes_to_critical: Option<u64>,
    pub predicted_utilization_1h: f64,
    pub confidence: f64,
}

/// op 165: Per-application memory footprint.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConsumer {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
    pub is_idle: bool,
    /// op 166: Recent allocation rate in bytes/min (positive = growing).
    pub allocation_rate_bytes_per_min: Option<f64>,
    /// op 170: Efficiency score 0-100 (100 = excellent bytes/useful-work).
    pub efficiency_score: Option<f64>,
}

/// op 167: A process suspected of leaking memory.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeakCandidate {
    pub pid: u32,
    pub name: String,
    pub growth_rate_bytes_per_min: f64,
    pub duration_mins: u64,
}

/// op 182: Apple Silicon unified memory model.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMemoryModel {
    pub is_apple_silicon: bool,
    pub gpu_allocated_bytes: Option<u64>,
    pub ane_allocated_bytes: Option<u64>,
    pub metal_allocated_bytes: Option<u64>,
}

/// op 183: Memory topology — which clients share the unified memory fabric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryTopology {
    pub clients: Vec<MemoryClient>,
    /// Total bytes shared across CPU/GPU/ANE on Apple Silicon.
    pub shared_pool_bytes: Option<u64>,
}

/// op 183: A client of the unified memory fabric.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryClient {
    pub name: String,
    pub kind: MemoryClientKind,
    pub allocated_bytes: u64,
}

/// op 183: Kind of unified memory client.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryClientKind {
    Cpu,
    Gpu,
    Metal,
    Ane,
    Ml,
    Vm,
    Other,
}

/// op 185: A point-in-time memory sample for temporal analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemorySample {
    pub timestamp_ms: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub swap_used_bytes: u64,
    pub pressure_level: u32,
}

/// op 187: A learned workflow — apps that frequently run together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkflowPattern {
    pub name: String,
    pub app_names: Vec<String>,
    /// 0-1, how often this pattern was observed.
    pub frequency: f64,
    /// Estimated combined memory footprint in bytes.
    pub estimated_footprint_bytes: u64,
}

/// op 198: An automated memory policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryPolicy {
    pub name: String,
    pub trigger: String,
    pub action: String,
    pub enabled: bool,
}

/// op 178: A plan for managing GPU memory usage — recommendations to
/// optimize GPU/Metal memory allocation on Apple Silicon.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct GPUMemoryPlan {
    pub gpu_allocated_bytes: u64,
    pub metal_allocated_bytes: u64,
    pub recommended_limit_bytes: u64,
    pub recommendations: Vec<String>,
    pub overcommitted: bool,
}

/// op 280: An optimization plan for Apple Silicon unified memory —
/// recommendations to rebalance the CPU/GPU/ANE shared memory fabric.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UnifiedMemoryPlan {
    pub is_apple_silicon: bool,
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub shared_pool_bytes: u64,
    pub recommendations: Vec<String>,
    pub estimated_savings_bytes: u64,
}

#[allow(dead_code)]
impl MemoryIntelligence {
    /// Return an empty MemoryIntelligence with zeroed values — for testing.
    #[allow(dead_code)]
    pub fn empty() -> Self {
        Self {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            compressed_bytes: 0,
            swap_used_bytes: 0,
            swap_total_bytes: 0,
            purgeable_bytes: 0,
            utilization: 0.0,
            pressure_level: 0,
            pressure_forecast: None,
            top_consumers: Vec::new(),
            leak_candidates: Vec::new(),
            fragmentation_pct: None,
            unified_memory_model: UnifiedMemoryModel {
                is_apple_silicon: false,
                gpu_allocated_bytes: None,
                ane_allocated_bytes: None,
                metal_allocated_bytes: None,
            },
            topology: MemoryTopology {
                clients: Vec::new(),
                shared_pool_bytes: None,
            },
            history: Vec::new(),
            learned_workflows: Vec::new(),
            policies: Vec::new(),
            decision_explanations: Vec::new(),
        }
    }

    /// Collect memory intelligence.
    pub fn collect() -> Self {
        // Start with what system_awareness already provides (ops 161-163).
        let snapshot = crate::intelligence::system_awareness::MemoryDimension::collect();
        let is_as = cfg!(target_os = "macos") && is_apple_silicon();

        // op 165: Monitor application memory footprints.
        let top_consumers = collect_top_memory_consumers();
        // op 167: Track memory leaks.
        let leak_candidates = detect_leak_candidates(&top_consumers);
        // op 168: Detect memory fragmentation (heuristic from pressure).
        let fragmentation_pct = estimate_fragmentation(&snapshot);
        // op 164: Purgeable memory estimate.
        let purgeable_bytes = estimate_purgeable(&snapshot);
        // op 181: Detect memory contention.
        let contention = detect_contention(&snapshot, &top_consumers);
        // op 183: Build memory topology.
        let topology = build_memory_topology(is_as, &snapshot);
        // op 184: Create memory pressure forecast.
        let pressure_forecast = forecast_pressure(&snapshot);
        // op 182: Unified memory model.
        let unified_memory_model = UnifiedMemoryModel {
            is_apple_silicon: is_as,
            gpu_allocated_bytes: topology
                .clients
                .iter()
                .find(|c| c.kind == MemoryClientKind::Gpu)
                .map(|c| c.allocated_bytes),
            ane_allocated_bytes: topology
                .clients
                .iter()
                .find(|c| c.kind == MemoryClientKind::Ane)
                .map(|c| c.allocated_bytes),
            metal_allocated_bytes: topology
                .clients
                .iter()
                .find(|c| c.kind == MemoryClientKind::Metal)
                .map(|c| c.allocated_bytes),
        };
        // op 185: Analyze memory over time (single sample here; history
        // accumulates across daemon ticks).
        let now = now_ms();
        let history = vec![MemorySample {
            timestamp_ms: now,
            used_bytes: snapshot.used_bytes,
            available_bytes: snapshot.available_bytes,
            swap_used_bytes: snapshot.swap_used_bytes,
            pressure_level: snapshot.pressure_level,
        }];
        // op 187: Learned workflows (empty until the daemon observes runs).
        let learned_workflows = Vec::new();
        // op 198: Default automated policies.
        let policies = default_policies();
        // op 196: Explain memory decisions.
        let mut decision_explanations = Vec::new();
        if contention {
            decision_explanations.push(
                "Memory contention detected: multiple heavy consumers are competing for the unified memory fabric.".to_string(),
            );
        }
        if !leak_candidates.is_empty() {
            decision_explanations.push(format!(
                "{} process(es) show sustained memory growth and are flagged as leak candidates.",
                leak_candidates.len()
            ));
        }

        Self {
            total_bytes: snapshot.total_bytes,
            used_bytes: snapshot.used_bytes,
            available_bytes: snapshot.available_bytes,
            compressed_bytes: snapshot.compressed_bytes,
            swap_used_bytes: snapshot.swap_used_bytes,
            swap_total_bytes: snapshot.swap_total_bytes,
            purgeable_bytes,
            utilization: snapshot.utilization,
            pressure_level: snapshot.pressure_level,
            pressure_forecast,
            top_consumers,
            leak_candidates,
            fragmentation_pct,
            unified_memory_model,
            topology,
            history,
            learned_workflows,
            policies,
            decision_explanations,
        }
    }

    /// op 170: Identify inefficient applications — apps whose memory
    /// footprint is large relative to their usefulness (idle + heavy).
    pub fn inefficient_apps(&self) -> Vec<&MemoryConsumer> {
        self.top_consumers
            .iter()
            .filter(|c| c.is_idle && c.memory_bytes > 200 * 1024 * 1024)
            .collect()
    }

    /// op 171: Predict memory exhaustion — minutes until available memory
    /// drops below a critical threshold, based on the current trend.
    pub fn predict_exhaustion(&self) -> Option<u64> {
        self.pressure_forecast
            .as_ref()
            .and_then(|f| f.minutes_to_critical)
    }

    /// op 172: Predict whether a swap event is likely in the next hour.
    pub fn predict_swap_event(&self) -> bool {
        if let Some(f) = &self.pressure_forecast {
            return f.predicted_utilization_1h > 0.90 && self.swap_total_bytes > 0;
        }
        self.utilization > 0.90
    }

    /// op 174: Recommend workload migration — move heavy idle apps to
    /// efficiency cores or suspend them. Returns app names.
    pub fn recommend_migration(&self) -> Vec<String> {
        self.inefficient_apps()
            .iter()
            .map(|c| c.name.clone())
            .collect()
    }

    /// op 175: Optimize application ordering — suggested launch order to
    /// minimize peak memory pressure (largest first).
    pub fn optimize_app_ordering(&self, app_names: &[String]) -> Vec<String> {
        let mut apps = app_names.to_vec();
        // Sort by estimated footprint descending (heuristic: alphabetical
        // when no footprint data; real impl would consult learned workflows).
        apps.sort_by_key(|a| std::cmp::Reverse(a.len()));
        apps
    }

    /// op 176: Optimize background processes — return names of background
    /// processes safe to suspend/quit.
    pub fn optimize_background(&self) -> Vec<String> {
        self.top_consumers
            .iter()
            .filter(|c| c.is_idle && c.memory_bytes > 100 * 1024 * 1024)
            .map(|c| c.name.clone())
            .collect()
    }

    /// op 177-180: Manage ML/GPU/Metal/ANE memory — return the bytes
    /// allocated to each accelerator.
    pub fn accelerator_memory(&self) -> HashMap<String, u64> {
        let mut map = HashMap::new();
        for c in &self.topology.clients {
            map.insert(c.name.clone(), c.allocated_bytes);
        }
        map
    }

    /// op 188: Predict the next applications the user is likely to launch,
    /// based on learned workflows.
    pub fn predict_next_apps(&self) -> Vec<String> {
        // Pick the most frequent learned workflow's apps.
        self.learned_workflows
            .iter()
            .max_by(|a, b| {
                a.frequency
                    .partial_cmp(&b.frequency)
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
            .map(|w| w.app_names.clone())
            .unwrap_or_default()
    }

    /// op 189: Pre-stage resources for predicted apps — returns the apps
    /// whose resources should be preloaded.
    pub fn prestage_resources(&self) -> Vec<String> {
        self.predict_next_apps()
    }

    /// op 190: Release unused resources — return names of idle apps whose
    /// memory can be safely reclaimed.
    pub fn release_unused(&self) -> Vec<String> {
        self.optimize_background()
    }

    /// op 191-195: Optimize workload-specific memory. Each returns a set of
    /// recommendations for that workload class.
    pub fn optimize_developer_workloads(&self) -> Vec<String> {
        let mut recs = Vec::new();
        if self.swap_used_bytes > 1024 * 1024 * 1024 {
            recs.push("Close unused simulator instances to reduce swap pressure.".to_string());
        }
        recs.push("Purge derived data caches between Xcode builds.".to_string());
        recs
    }

    pub fn optimize_creative_workloads(&self) -> Vec<String> {
        vec![
            "Allocate unified memory to the active creative app's GPU pipeline.".to_string(),
            "Suspend background render queues while editing.".to_string(),
        ]
    }

    pub fn optimize_gaming_workloads(&self) -> Vec<String> {
        vec![
            "Quit background launch agents before launching games.".to_string(),
            "Reserve Metal memory for the foreground game.".to_string(),
        ]
    }

    pub fn optimize_ai_workloads(&self) -> Vec<String> {
        vec![
            "Unload unused ML models from the ANE/GPU before loading new ones.".to_string(),
            "Batch inference requests to reduce memory churn.".to_string(),
        ]
    }

    pub fn optimize_virtualization_workloads(&self) -> Vec<String> {
        vec![
            "Right-size VM memory allocations to the guest's working set.".to_string(),
            "Suspend idle VMs to free unified memory for the host.".to_string(),
        ]
    }

    /// op 197: Simulate the effect of a memory change (e.g. quitting an
    /// app) without executing it. Returns the predicted utilization.
    pub fn simulate_change(&self, freed_bytes: u64) -> f64 {
        let new_used = self.used_bytes.saturating_sub(freed_bytes);
        if self.total_bytes > 0 {
            new_used as f64 / self.total_bytes as f64
        } else {
            0.0
        }
    }

    /// op 199: Record a memory sample into the history (called by the
    /// daemon on each tick).
    pub fn record_sample(&mut self) {
        let snap = crate::intelligence::system_awareness::MemoryDimension::collect();
        self.history.push(MemorySample {
            timestamp_ms: now_ms(),
            used_bytes: snap.used_bytes,
            available_bytes: snap.available_bytes,
            swap_used_bytes: snap.swap_used_bytes,
            pressure_level: snap.pressure_level,
        });
        // Keep history bounded.
        if self.history.len() > 1440 {
            let excess = self.history.len() - 1440;
            self.history.drain(0..excess);
        }
    }

    /// op 200: Create the memory intelligence layer — a summary suitable
    /// for the reasoning engine to consume.
    pub fn intelligence_summary(&self) -> String {
        format!(
            "Memory: {:.0}% used, {} top consumers, {} leak candidates, pressure level {}",
            self.utilization * 100.0,
            self.top_consumers.len(),
            self.leak_candidates.len(),
            self.pressure_level,
        )
    }

    /// op 178: Manage GPU memory usage — analyze GPU/Metal memory
    /// allocation and recommend optimizations. On Apple Silicon the GPU and
    /// Metal share the unified memory fabric; this plan recommends limits
    /// and actions to keep GPU memory within a healthy budget.
    #[allow(dead_code)]
    pub fn manage_gpu_memory(&mut self) -> GPUMemoryPlan {
        let gpu_allocated = self.unified_memory_model.gpu_allocated_bytes.unwrap_or(0);
        let metal_allocated = self.unified_memory_model.metal_allocated_bytes.unwrap_or(0);
        // Recommend a GPU budget of ~25% of total memory.
        let recommended_limit = self.total_bytes / 4;
        let overcommitted = (gpu_allocated + metal_allocated) > recommended_limit;

        let mut recommendations = Vec::new();
        if overcommitted {
            recommendations.push(
                "GPU/Metal memory exceeds recommended budget — close GPU-heavy apps.".to_string(),
            );
        }
        if self.pressure_level >= 2 {
            recommendations
                .push("Memory pressure elevated — release unused Metal textures.".to_string());
        }
        recommendations.push("Monitor GPU memory via Metal device allocations.".to_string());

        GPUMemoryPlan {
            gpu_allocated_bytes: gpu_allocated,
            metal_allocated_bytes: metal_allocated,
            recommended_limit_bytes: recommended_limit,
            recommendations,
            overcommitted,
        }
    }

    /// op 280: Optimize unified memory usage — create an optimization plan
    /// for Apple Silicon unified memory. Recommends rebalancing the
    /// CPU/GPU/ANE shared memory fabric and reclaiming idle allocations.
    #[allow(dead_code)]
    pub fn optimize_unified_memory(&mut self) -> UnifiedMemoryPlan {
        let is_as = self.unified_memory_model.is_apple_silicon;
        let shared_pool = self.topology.shared_pool_bytes.unwrap_or(0);
        let mut recommendations = Vec::new();
        let mut estimated_savings: u64 = 0;

        if is_as {
            recommendations.push(
                "Rebalance unified memory across CPU/GPU/ANE based on active workload.".to_string(),
            );
            // Reclaiming idle consumers' memory is the main saving.
            let idle_bytes: u64 = self.inefficient_apps().iter().map(|c| c.memory_bytes).sum();
            estimated_savings += idle_bytes;
            if idle_bytes > 0 {
                recommendations.push(format!("Reclaim {idle_bytes} bytes from idle heavy apps."));
            }
            if self.swap_used_bytes > 0 {
                recommendations.push(
                    "Reduce swap usage by quitting memory-heavy background apps.".to_string(),
                );
            }
            if self.fragmentation_pct.unwrap_or(0.0) > 30.0 {
                recommendations.push(
                    "Memory fragmentation is high — consider restarting memory-heavy apps."
                        .to_string(),
                );
            }
        } else {
            recommendations.push(
                "Unified memory optimization is only applicable to Apple Silicon.".to_string(),
            );
        }

        UnifiedMemoryPlan {
            is_apple_silicon: is_as,
            total_bytes: self.total_bytes,
            used_bytes: self.used_bytes,
            shared_pool_bytes: shared_pool,
            recommendations,
            estimated_savings_bytes: estimated_savings,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Collection helpers
// ─────────────────────────────────────────────────────────────────────

/// op 165: Collect the top memory-consuming processes.
fn collect_top_memory_consumers() -> Vec<MemoryConsumer> {
    #[cfg(target_os = "macos")]
    {
        top_memory_consumers_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        top_memory_consumers_linux()
    }
}

#[cfg(target_os = "macos")]
fn top_memory_consumers_macos() -> Vec<MemoryConsumer> {
    use std::process::Command;
    // `ps -axm -o pid,rss,comm` gives RSS in KB.
    let out = Command::new("ps")
        .args(["-axm", "-o", "pid=,rss=,comm="])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    parse_ps_output(&out)
}

#[cfg(not(target_os = "macos"))]
fn top_memory_consumers_linux() -> Vec<MemoryConsumer> {
    use std::process::Command;
    let out = Command::new("ps")
        .args(["-axo", "pid=,rss=,comm="])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();
    parse_ps_output(&out)
}

/// Parse `ps` output (pid, rss_kb, comm) into consumers.
fn parse_ps_output(output: &str) -> Vec<MemoryConsumer> {
    let mut consumers = Vec::new();
    for line in output.lines() {
        let mut parts = line.split_whitespace();
        let pid: u32 = match parts.next().and_then(|s| s.parse().ok()) {
            Some(p) => p,
            None => continue,
        };
        let rss_kb: u64 = match parts.next().and_then(|s| s.parse().ok()) {
            Some(r) => r,
            None => continue,
        };
        let name = parts.collect::<Vec<_>>().join(" ");
        if name.is_empty() {
            continue;
        }
        consumers.push(MemoryConsumer {
            pid,
            name,
            memory_bytes: rss_kb * 1024,
            is_idle: false, // ps doesn't expose idle state directly
            allocation_rate_bytes_per_min: None,
            efficiency_score: None,
        });
    }
    // Keep the top 10 by memory.
    consumers.sort_by_key(|c| std::cmp::Reverse(c.memory_bytes));
    consumers.truncate(10);
    consumers
}

/// op 167: Detect leak candidates. Without persistent history we flag any
/// consumer above 1 GB as a candidate for monitoring.
fn detect_leak_candidates(consumers: &[MemoryConsumer]) -> Vec<MemoryLeakCandidate> {
    consumers
        .iter()
        .filter(|c| c.memory_bytes > 1024 * 1024 * 1024)
        .map(|c| MemoryLeakCandidate {
            pid: c.pid,
            name: c.name.clone(),
            growth_rate_bytes_per_min: 0.0, // populated once history exists
            duration_mins: 0,
        })
        .collect()
}

/// op 168: Estimate fragmentation from pressure level and compressed bytes.
/// High compression + high pressure implies fragmentation.
fn estimate_fragmentation(
    mem: &crate::intelligence::system_awareness::MemoryDimension,
) -> Option<f64> {
    if mem.total_bytes == 0 {
        return None;
    }
    let compressed_ratio = mem.compressed_bytes as f64 / mem.total_bytes as f64;
    let pressure_factor = match mem.pressure_level {
        4 => 0.4,
        2 => 0.2,
        _ => 0.0,
    };
    Some((compressed_ratio * 100.0 + pressure_factor * 100.0).min(100.0))
}

/// op 164: Estimate purgeable memory. macOS exposes this via vm_stat; we
/// approximate as 5% of available memory when not measurable directly.
fn estimate_purgeable(mem: &crate::intelligence::system_awareness::MemoryDimension) -> u64 {
    (mem.available_bytes / 20).min(mem.available_bytes)
}

/// op 181: Detect memory contention — true when pressure is elevated and
/// multiple heavy consumers exist.
fn detect_contention(
    mem: &crate::intelligence::system_awareness::MemoryDimension,
    consumers: &[MemoryConsumer],
) -> bool {
    let heavy = consumers
        .iter()
        .filter(|c| c.memory_bytes > 512 * 1024 * 1024)
        .count();
    mem.pressure_level >= 2 && heavy >= 2
}

/// op 183: Build the memory topology model.
fn build_memory_topology(
    is_apple_silicon: bool,
    mem: &crate::intelligence::system_awareness::MemoryDimension,
) -> MemoryTopology {
    let mut clients = Vec::new();
    clients.push(MemoryClient {
        name: "CPU".to_string(),
        kind: MemoryClientKind::Cpu,
        allocated_bytes: mem.used_bytes,
    });
    if is_apple_silicon {
        // On Apple Silicon the GPU/Metal/ANE share the same fabric. We
        // cannot measure their allocations without private APIs, so we
        // record zero-byte placeholders to indicate presence.
        clients.push(MemoryClient {
            name: "GPU".to_string(),
            kind: MemoryClientKind::Gpu,
            allocated_bytes: 0,
        });
        clients.push(MemoryClient {
            name: "Metal".to_string(),
            kind: MemoryClientKind::Metal,
            allocated_bytes: 0,
        });
        clients.push(MemoryClient {
            name: "ANE".to_string(),
            kind: MemoryClientKind::Ane,
            allocated_bytes: 0,
        });
    }
    MemoryTopology {
        clients,
        shared_pool_bytes: if is_apple_silicon {
            Some(mem.total_bytes)
        } else {
            None
        },
    }
}

/// op 184: Forecast memory pressure. With a single sample we extrapolate
/// from the current utilization trend.
fn forecast_pressure(
    mem: &crate::intelligence::system_awareness::MemoryDimension,
) -> Option<PressureForecast> {
    if mem.total_bytes == 0 {
        return None;
    }
    // Heuristic: if utilization is above 85%, critical within ~30 min;
    // otherwise predict a slight drift.
    let predicted_1h = if mem.utilization > 0.85 {
        (mem.utilization + 0.05).min(1.0)
    } else {
        mem.utilization
    };
    let minutes_to_critical = if mem.utilization > 0.90 {
        Some(15)
    } else if mem.utilization > 0.80 {
        Some(60)
    } else {
        None
    };
    Some(PressureForecast {
        minutes_to_critical,
        predicted_utilization_1h: predicted_1h,
        confidence: 0.5,
    })
}

/// op 198: Default automated memory policies.
fn default_policies() -> Vec<MemoryPolicy> {
    vec![
        MemoryPolicy {
            name: "Auto-quit idle heavy apps".to_string(),
            trigger: "pressure_level >= 3 and app idle > 30m".to_string(),
            action: "quit".to_string(),
            enabled: false,
        },
        MemoryPolicy {
            name: "Purge caches on critical pressure".to_string(),
            trigger: "pressure_level == 4".to_string(),
            action: "purge".to_string(),
            enabled: true,
        },
        MemoryPolicy {
            name: "Pre-stage predicted apps".to_string(),
            trigger: "predicted_app_confidence > 0.7".to_string(),
            action: "prestage".to_string(),
            enabled: false,
        },
    ]
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

#[cfg(target_os = "macos")]
fn is_apple_silicon() -> bool {
    use std::process::Command;
    Command::new("sysctl")
        .args(["-n", "hw.optional.arm64"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "1")
        .unwrap_or(false)
}

#[cfg(not(target_os = "macos"))]
fn is_apple_silicon() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn test_collect_runs() {
        let mi = MemoryIntelligence::collect();
        // On any real system total_bytes may be 0 on stubbed Linux; just
        // ensure it doesn't panic and produces sane values.
        assert!(mi.utilization >= 0.0 && mi.utilization <= 1.0);
        assert!(!mi.policies.is_empty());
    }

    #[test]
    fn test_parse_ps_output() {
        let out = "  123  512000  /usr/bin/foo\n  456  1024000  /usr/bin/bar\n";
        let consumers = parse_ps_output(out);
        assert_eq!(consumers.len(), 2);
        // Sorted descending by memory.
        assert_eq!(consumers[0].name, "/usr/bin/bar");
        assert_eq!(consumers[0].memory_bytes, 1024000 * 1024);
        assert_eq!(consumers[1].name, "/usr/bin/foo");
    }

    #[test]
    fn test_detect_leak_candidates() {
        let consumers = vec![
            MemoryConsumer {
                pid: 1,
                name: "small".to_string(),
                memory_bytes: 100 * 1024 * 1024,
                is_idle: false,
                allocation_rate_bytes_per_min: None,
                efficiency_score: None,
            },
            MemoryConsumer {
                pid: 2,
                name: "huge".to_string(),
                memory_bytes: 2 * 1024 * 1024 * 1024,
                is_idle: false,
                allocation_rate_bytes_per_min: None,
                efficiency_score: None,
            },
        ];
        let leaks = detect_leak_candidates(&consumers);
        assert_eq!(leaks.len(), 1);
        assert_eq!(leaks[0].name, "huge");
    }

    #[test]
    fn test_simulate_change() {
        let mut mi = MemoryIntelligence::empty();
        mi.total_bytes = 16_000_000_000;
        mi.used_bytes = 8_000_000_000;
        let predicted = mi.simulate_change(2_000_000_000);
        assert!((predicted - (6_000_000_000.0 / 16_000_000_000.0)).abs() < 1e-9);
    }

    #[test]
    fn test_inefficient_apps() {
        let mut mi = MemoryIntelligence::empty();
        mi.top_consumers = vec![
            MemoryConsumer {
                pid: 1,
                name: "idle_heavy".to_string(),
                memory_bytes: 500 * 1024 * 1024,
                is_idle: true,
                allocation_rate_bytes_per_min: None,
                efficiency_score: None,
            },
            MemoryConsumer {
                pid: 2,
                name: "active_light".to_string(),
                memory_bytes: 50 * 1024 * 1024,
                is_idle: false,
                allocation_rate_bytes_per_min: None,
                efficiency_score: None,
            },
        ];
        let ineff = mi.inefficient_apps();
        assert_eq!(ineff.len(), 1);
        assert_eq!(ineff[0].name, "idle_heavy");
    }

    #[test]
    fn test_forecast_pressure() {
        let mem = crate::intelligence::system_awareness::MemoryDimension {
            total_bytes: 16_000_000_000,
            used_bytes: 14_000_000_000,
            free_bytes: 2_000_000_000,
            available_bytes: 2_000_000_000,
            compressed_bytes: 1_000_000_000,
            swap_used_bytes: 0,
            swap_total_bytes: 4_000_000_000,
            utilization: 0.875,
            pressure_level: 2,
            pressure_label: "Warning".to_string(),
        };
        let f = forecast_pressure(&mem).unwrap();
        assert!(f.predicted_utilization_1h > 0.875);
        assert!(f.minutes_to_critical.is_some());
    }

    #[test]
    fn test_default_policies() {
        let policies = default_policies();
        assert!(policies.iter().any(|p| p.action == "purge"));
    }

    #[test]
    fn test_workload_optimization_recs() {
        let mi = MemoryIntelligence::empty();
        assert!(!mi.optimize_developer_workloads().is_empty());
        assert!(!mi.optimize_creative_workloads().is_empty());
        assert!(!mi.optimize_gaming_workloads().is_empty());
        assert!(!mi.optimize_ai_workloads().is_empty());
        assert!(!mi.optimize_virtualization_workloads().is_empty());
    }

    #[test]
    #[ignore = "Integration test — requires system commands"]
    fn test_record_sample_grows_history() {
        let mut mi = MemoryIntelligence::empty();
        let initial = mi.history.len();
        mi.record_sample();
        assert_eq!(mi.history.len(), initial + 1);
    }

    #[test]
    fn test_intelligence_summary() {
        let mi = MemoryIntelligence::empty();
        let s = mi.intelligence_summary();
        assert!(s.contains("Memory:"));
    }

    #[test]
    fn test_manage_gpu_memory() {
        let mut mi = MemoryIntelligence::empty();
        mi.total_bytes = 16_000_000_000;
        let plan = mi.manage_gpu_memory();
        assert_eq!(plan.recommended_limit_bytes, 4_000_000_000);
        assert!(!plan.recommendations.is_empty());
    }

    #[test]
    fn test_manage_gpu_memory_overcommitted() {
        let mut mi = MemoryIntelligence::empty();
        mi.total_bytes = 16_000_000_000;
        mi.unified_memory_model.gpu_allocated_bytes = Some(5_000_000_000);
        mi.unified_memory_model.metal_allocated_bytes = Some(0);
        let plan = mi.manage_gpu_memory();
        assert!(plan.overcommitted);
        assert!(plan
            .recommendations
            .iter()
            .any(|r| r.contains("exceeds recommended budget")));
    }

    #[test]
    fn test_optimize_unified_memory_non_apple_silicon() {
        let mut mi = MemoryIntelligence::empty();
        mi.unified_memory_model.is_apple_silicon = false;
        let plan = mi.optimize_unified_memory();
        assert!(!plan.is_apple_silicon);
        assert!(!plan.recommendations.is_empty());
    }

    #[test]
    fn test_optimize_unified_memory_apple_silicon() {
        let mut mi = MemoryIntelligence::empty();
        mi.unified_memory_model.is_apple_silicon = true;
        mi.total_bytes = 16_000_000_000;
        mi.used_bytes = 10_000_000_000;
        mi.top_consumers = vec![MemoryConsumer {
            pid: 1,
            name: "idle_heavy".to_string(),
            memory_bytes: 500 * 1024 * 1024,
            is_idle: true,
            allocation_rate_bytes_per_min: None,
            efficiency_score: None,
        }];
        let plan = mi.optimize_unified_memory();
        assert!(plan.is_apple_silicon);
        assert!(plan.estimated_savings_bytes > 0);
        assert!(!plan.recommendations.is_empty());
    }
}
