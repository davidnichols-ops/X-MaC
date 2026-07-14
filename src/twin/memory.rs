use serde::{Deserialize, Serialize};

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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PressureForecast {
    pub minutes_to_critical: Option<u64>,
    pub predicted_utilization_1h: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConsumer {
    pub pid: u32,
    pub name: String,
    pub memory_bytes: u64,
    pub is_idle: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryLeakCandidate {
    pub pid: u32,
    pub name: String,
    pub growth_rate_bytes_per_min: f64,
    pub duration_mins: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnifiedMemoryModel {
    pub is_apple_silicon: bool,
    pub gpu_allocated_bytes: Option<u64>,
    pub ane_allocated_bytes: Option<u64>,
    pub metal_allocated_bytes: Option<u64>,
}

impl MemoryIntelligence {
    /// Collect memory intelligence.
    pub fn collect() -> Self {
        // TODO: implement full memory intelligence (ops 161-200)
        // Start with what system_awareness already provides
        let snapshot = crate::intelligence::system_awareness::MemoryDimension::collect();

        Self {
            total_bytes: snapshot.total_bytes,
            used_bytes: snapshot.used_bytes,
            available_bytes: snapshot.available_bytes,
            compressed_bytes: snapshot.compressed_bytes,
            swap_used_bytes: snapshot.swap_used_bytes,
            swap_total_bytes: snapshot.swap_total_bytes,
            purgeable_bytes: 0, // TODO: detect purgeable memory
            utilization: snapshot.utilization,
            pressure_level: snapshot.pressure_level,
            pressure_forecast: None,
            top_consumers: Vec::new(),
            leak_candidates: Vec::new(),
            fragmentation_pct: None,
            unified_memory_model: UnifiedMemoryModel {
                is_apple_silicon: cfg!(target_os = "macos") && is_apple_silicon(),
                gpu_allocated_bytes: None,
                ane_allocated_bytes: None,
                metal_allocated_bytes: None,
            },
        }
    }
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
