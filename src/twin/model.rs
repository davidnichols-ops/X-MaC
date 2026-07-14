use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::app_agent::AppIntelligenceGraph;
use super::energy::EnergyTwin;
use super::fs_graph::FilesystemGraph;
use super::hardware::HardwareProfile;
use super::memory::MemoryIntelligence;
use super::process::ProcessIntelligenceGraph;
use super::reasoning::ReasoningEngine;
use super::software_genome::SoftwareGenome;

/// The complete macOS Digital Twin — a live computational model of the Mac.
///
/// This aggregates all twin dimensions into a single queryable model.
/// The reasoning engine uses this to answer questions, predict problems,
/// simulate changes, and recommend optimizations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DigitalTwin {
    pub timestamp_ms: u64,
    pub hardware: HardwareProfile,
    pub software_genome: SoftwareGenome,
    pub filesystem: FilesystemGraph,
    pub processes: ProcessIntelligenceGraph,
    pub memory: MemoryIntelligence,
    pub energy: EnergyTwin,
    pub applications: AppIntelligenceGraph,
    /// Overall system health score (0-100, higher is better).
    pub health_score: f64,
    /// Trust score for autonomous actions (0-1, grows with successful outcomes).
    pub trust_score: f64,
    /// Metadata bag for extensible properties.
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl DigitalTwin {
    /// Collect a complete digital twin snapshot. This is expensive —
    /// call from the daemon on a schedule, not on every request.
    pub fn collect() -> Self {
        let hardware = HardwareProfile::collect();
        let software_genome = SoftwareGenome::collect();
        let filesystem = FilesystemGraph::collect();
        let processes = ProcessIntelligenceGraph::collect();
        let memory = MemoryIntelligence::collect();
        let energy = EnergyTwin::collect();
        let applications = AppIntelligenceGraph::collect();

        let health_score = compute_twin_health_score(
            &hardware,
            &memory,
            &energy,
            &processes,
        );

        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            timestamp_ms,
            hardware,
            software_genome,
            filesystem,
            processes,
            memory,
            energy,
            applications,
            health_score,
            trust_score: 0.5, // Start at neutral, grows with successful actions
            metadata: HashMap::new(),
        }
    }

    /// Get the reasoning engine for this twin.
    pub fn reason(&self) -> ReasoningEngine {
        ReasoningEngine::new(self.clone())
    }
}

fn compute_twin_health_score(
    _hardware: &HardwareProfile,
    _memory: &MemoryIntelligence,
    _energy: &EnergyTwin,
    _processes: &ProcessIntelligenceGraph,
) -> f64 {
    // TODO: implement composite health score from twin dimensions
    // For now, delegate to the existing system_awareness health score
    let snapshot = crate::intelligence::SystemSnapshot::collect();
    snapshot.health_score
}
