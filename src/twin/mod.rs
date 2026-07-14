// X-MaC Digital Twin - twin module
//
// This module implements the macOS Digital Twin: a live computational model
// of the Mac's hardware, software, filesystem, processes, memory, energy,
// and behavior. See docs/INTEGRATION_PLAN.md and docs/OPERATIONS_MANIFEST.md
// for the full operation mapping.

pub mod app_agent;
pub mod energy;
pub mod fs_graph;
pub mod hardware;
pub mod memory;
pub mod model;
pub mod process;
pub mod reasoning;
pub mod software_genome;

pub use model::DigitalTwin;
