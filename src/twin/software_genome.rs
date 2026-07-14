use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Complete Software Genome — inventory of every software component on the Mac.
///
/// Covers: apps, executables, frameworks, dylibs, kexts, system extensions,
/// launch agents/daemons, login items, plugins, browser extensions, fonts,
/// dev tools, SDKs, package managers, Python/Node/Rust environments, Docker,
/// VMs, AI models, datasets, games, cloud apps, sync clients.
///
/// Maps to Digital Twin operations 41-80.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareGenome {
    pub applications: Vec<AppComponent>,
    pub frameworks: Vec<SoftwareComponent>,
    pub dynamic_libraries: Vec<SoftwareComponent>,
    pub kernel_extensions: Vec<SoftwareComponent>,
    pub system_extensions: Vec<SoftwareComponent>,
    pub launch_agents: Vec<SoftwareComponent>,
    pub launch_daemons: Vec<SoftwareComponent>,
    pub login_items: Vec<SoftwareComponent>,
    pub plugins: Vec<SoftwareComponent>,
    pub browser_extensions: Vec<SoftwareComponent>,
    pub fonts: Vec<SoftwareComponent>,
    pub developer_tools: Vec<SoftwareComponent>,
    pub sdks: Vec<SoftwareComponent>,
    pub package_managers: Vec<SoftwareComponent>,
    pub python_envs: Vec<SoftwareComponent>,
    pub node_envs: Vec<SoftwareComponent>,
    pub rust_toolchains: Vec<SoftwareComponent>,
    pub docker_images: Vec<SoftwareComponent>,
    pub containers: Vec<SoftwareComponent>,
    pub virtual_machines: Vec<SoftwareComponent>,
    pub ai_models: Vec<SoftwareComponent>,
    pub datasets: Vec<SoftwareComponent>,
    pub dependency_graph: HashMap<String, Vec<String>>,
    pub total_components: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppComponent {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
    pub path: String,
    pub size_bytes: u64,
    pub developer: Option<String>,
    pub is_signed: bool,
    pub last_launched: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareComponent {
    pub name: String,
    pub version: Option<String>,
    pub path: String,
    pub size_bytes: u64,
}

impl SoftwareGenome {
    /// Collect the complete software genome.
    pub fn collect() -> Self {
        // TODO: implement full software inventory (ops 41-80)
        // Start with what envmap and map engines already provide
        Self {
            applications: Vec::new(),
            frameworks: Vec::new(),
            dynamic_libraries: Vec::new(),
            kernel_extensions: Vec::new(),
            system_extensions: Vec::new(),
            launch_agents: Vec::new(),
            launch_daemons: Vec::new(),
            login_items: Vec::new(),
            plugins: Vec::new(),
            browser_extensions: Vec::new(),
            fonts: Vec::new(),
            developer_tools: Vec::new(),
            sdks: Vec::new(),
            package_managers: Vec::new(),
            python_envs: Vec::new(),
            node_envs: Vec::new(),
            rust_toolchains: Vec::new(),
            docker_images: Vec::new(),
            containers: Vec::new(),
            virtual_machines: Vec::new(),
            ai_models: Vec::new(),
            datasets: Vec::new(),
            dependency_graph: HashMap::new(),
            total_components: 0,
        }
    }
}
