import Foundation

// MARK: - XMacError

enum XMacError: Error {
    case invalidOutput(String)
    case processFailed(String)
}

// MARK: - Config Profile

struct ConfigProfile: Codable, Identifiable {
    let name: String
    let description: String

    var id: String { name }
}

// MARK: - Digital Twin Models
// Swift Codable models matching the Rust `DigitalTwin` JSON output from `xmac twin --action collect`.

struct DigitalTwin: Codable {
    let timestamp_ms: UInt64
    let hardware: HardwareProfile
    let software_genome: SoftwareGenome
    let filesystem: FilesystemGraph
    let processes: ProcessIntelligenceGraph
    let memory: MemoryIntelligence
    let energy: EnergyTwin
    let applications: AppIntelligenceGraph
    let health_score: Double
    let trust_score: Double
}

// MARK: - Hardware

struct HardwareProfile: Codable {
    let model_identifier: String
    let soc_generation: String
    let cpu_cores: CpuTopology
    let gpu: GpuInfo
    let neural_engine: NeuralEngineInfo
    let memory: HardwareMemory
    let storage: [StorageDevice]
    let battery: BatteryHardware?
    let thermal: ThermalInfo
    let peripherals: Peripherals
    let power_state: PowerState
    let fingerprint: String
}

struct CpuTopology: Codable {
    let total_logical_cores: Int
    let performance_cores: Int
    let efficiency_cores: Int
}

struct GpuInfo: Codable {
    let core_count: Int
    let utilization_pct: Double?
}

struct NeuralEngineInfo: Codable {
    let core_count: Int
    let utilization_pct: Double?
}

struct HardwareMemory: Codable {
    let total_bytes: UInt64
    let bandwidth_gbps: Double?
}

struct StorageDevice: Codable {
    let name: String
    let capacity_bytes: UInt64
    let is_ssd: Bool
    let smart_health: SmartHealth?
}

struct SmartHealth: Codable {
    let status: String
    let temperature_celsius: Double?
    let power_on_hours: UInt64?
}

struct BatteryHardware: Codable {
    let model: String
    let chemistry: String
    let cycle_count: UInt64
    let design_capacity_mah: UInt64?
    let current_capacity_mah: UInt64?
}

struct ThermalInfo: Codable {
    let cpu_temp_c: Double?
    let gpu_temp_c: Double?
    let fan_speeds_rpm: [UInt32]
    let thermal_pressure: String
}

struct Peripherals: Codable {
    let usb_devices: [String]
    let thunderbolt_devices: [String]
    let bluetooth_devices: [String]
    let external_displays: [String]
    let audio_devices: [String]
    let network_interfaces: [String]
}

struct PowerState: Codable {
    let is_on_battery: Bool
    let is_charging: Bool
    let sleep_state: String
    let low_power_mode: Bool
}

// MARK: - Software Genome

struct SoftwareGenome: Codable {
    let applications: [AppComponent]
    let frameworks: [SoftwareComponent]
    let dynamic_libraries: [SoftwareComponent]
    let kernel_extensions: [SoftwareComponent]
    let system_extensions: [SoftwareComponent]
    let launch_agents: [SoftwareComponent]
    let launch_daemons: [SoftwareComponent]
    let login_items: [SoftwareComponent]
    let plugins: [SoftwareComponent]
    let browser_extensions: [SoftwareComponent]
    let fonts: [SoftwareComponent]
    let developer_tools: [SoftwareComponent]
    let sdks: [SoftwareComponent]
    let package_managers: [SoftwareComponent]
    let python_envs: [SoftwareComponent]
    let node_envs: [SoftwareComponent]
    let rust_toolchains: [SoftwareComponent]
    let docker_images: [SoftwareComponent]
    let containers: [SoftwareComponent]
    let virtual_machines: [SoftwareComponent]
    let ai_models: [SoftwareComponent]
    let datasets: [SoftwareComponent]
    let total_components: Int
}

struct AppComponent: Codable, Identifiable {
    let bundle_id: String
    let name: String
    let version: String
    let path: String
    let size_bytes: UInt64
    let developer: String?
    let is_signed: Bool
    let last_launched: UInt64?

    var id: String { bundle_id }
    var sizeFormatted: String { formatBytes(size_bytes) }
}

struct SoftwareComponent: Codable, Identifiable {
    let name: String
    let version: String?
    let path: String
    let size_bytes: UInt64

    var id: String { "\(name)@\(path)" }
    var sizeFormatted: String { formatBytes(size_bytes) }
}

// MARK: - Filesystem Graph

struct FilesystemGraph: Codable {
    let total_files: UInt64
    let total_size_bytes: UInt64
    let duplicate_clusters: [DuplicateCluster]
    let abandoned_files: [String]
    let orphan_files: [String]
    let storage_growth_trend: Double?
    let exhaustion_forecast_days: UInt64?
}

struct DuplicateCluster: Codable, Identifiable {
    let hash: String
    let files: [String]
    let total_size_bytes: UInt64
    let recommended_keep: String?

    var id: String { hash }
    var sizeFormatted: String { formatBytes(total_size_bytes) }
    var fileCount: Int { files.count }
}

// MARK: - Process Intelligence

struct ProcessIntelligenceGraph: Codable {
    let process_tree: [ProcessNode]
    let anomalies: [ProcessAnomaly]
    let total_processes: Int
    let bottleneck_predictions: [String]
    let idle_processes: [UInt32]
    let crashed_services: [String]
}

struct ProcessNode: Codable, Identifiable {
    let pid: UInt32
    let name: String
    let owner: String?
    let parent_pid: UInt32?
    let children: [UInt32]
    let cpu_pct: Double
    let gpu_pct: Double?
    let memory_bytes: UInt64
    let energy_impact: Double?
    let wakeup_count: UInt64?
    let state: String
    let thread_count: UInt32?

    var id: UInt32 { pid }
    var memoryFormatted: String { formatBytes(memory_bytes) }
}

struct ProcessAnomaly: Codable, Identifiable {
    let pid: UInt32
    let name: String
    let anomaly_type: String
    let description: String
    let severity: String

    var id: String { "\(pid)-\(anomaly_type)" }
}

// MARK: - Memory Intelligence

struct MemoryIntelligence: Codable {
    let total_bytes: UInt64
    let used_bytes: UInt64
    let available_bytes: UInt64
    let compressed_bytes: UInt64
    let swap_used_bytes: UInt64
    let swap_total_bytes: UInt64
    let purgeable_bytes: UInt64
    let utilization: Double
    let pressure_level: UInt32
    let top_consumers: [MemoryConsumer]
    let leak_candidates: [MemoryLeakCandidate]
    let fragmentation_pct: Double?
}

struct MemoryConsumer: Codable, Identifiable {
    let pid: UInt32
    let name: String
    let memory_bytes: UInt64
    let is_idle: Bool
    let allocation_rate_bytes_per_min: Double?
    let efficiency_score: Double?

    var id: UInt32 { pid }
    var memoryFormatted: String { formatBytes(memory_bytes) }
}

struct MemoryLeakCandidate: Codable, Identifiable {
    let pid: UInt32
    let name: String
    let growth_rate_bytes_per_min: Double
    let duration_mins: UInt64

    var id: UInt32 { pid }
}

// MARK: - Energy Twin

struct EnergyTwin: Codable {
    let battery: BatteryModel?
    let energy_consumers: [EnergyConsumer]
    let thermal_efficiency: Double?
    let sleep_efficiency: Double?
    let wake_causes: [WakeEvent]
    let recommended_power_mode: String
}

struct BatteryModel: Codable {
    let charge_pct: Double
    let cycle_count: UInt64
    let condition: String
    let is_charging: Bool
    let is_plugged: Bool
    let time_remaining_mins: UInt64?
    let design_capacity_mah: UInt64?
    let current_capacity_mah: UInt64?
    let health_pct: Double?
    let abnormal_aging: Bool
}

struct EnergyConsumer: Codable, Identifiable {
    let pid: UInt32
    let name: String
    let energy_impact: Double
    let category: String

    var id: UInt32 { pid }
}

struct WakeEvent: Codable, Identifiable {
    let timestamp_ms: UInt64
    let cause: String

    var id: UInt64 { timestamp_ms }
}

// MARK: - App Intelligence

struct AppIntelligenceGraph: Codable {
    let apps: [AppIntelligenceNode]
    let suspicious_apps: [String]
    let unused_apps: [String]
    let duplicate_apps: [[String]]
    let total_apps: Int
}

struct AppIntelligenceNode: Codable, Identifiable {
    let bundle_id: String
    let name: String
    let version: String
    let purpose: String?
    let behavior: String?
    let dependencies: [String]
    let files: [String]
    let permissions: [String]
    let health_score: Double
    let last_launched_ms: UInt64?
    let is_unused: Bool
    let is_suspicious: Bool
    let crash_probability: Double?
    let preferences_corrupted: Bool
    let uninstall_impact_bytes: UInt64?

    var id: String { bundle_id }
}

// MARK: - Reasoning Engine Results

struct ReasoningResult: Codable {
    let question: String
    let answer: String
    let confidence: Double
    let evidence: [String]
    let recommended_actions: [RecommendedAction]
}

struct RecommendedAction: Codable, Identifiable {
    let action: String
    let estimated_impact: String
    let risk_level: String
    let is_reversible: Bool
    let command: String?

    var id: String { action }
}

struct CleanupImpact: Codable {
    let space_freed_bytes: UInt64
    let time_saved_secs: UInt64
    let risk_level: String
    let summary: String
}

struct WorkflowChange: Codable, Identifiable {
    let change: String
    let estimated_benefit: String
    let risk: String

    var id: String { change }
}

struct UpgradeRecommendation: Codable, Identifiable {
    let component: String
    let current: String
    let recommended: String
    let reason: String
    let estimated_cost: String

    var id: String { component }
}

struct SoftwareRecommendation: Codable, Identifiable {
    let action: String
    let target: String
    let reason: String
    let estimated_impact: String

    var id: String { "\(action)-\(target)" }
}

struct SandboxResult: Codable {
    let action: String
    let simulated: Bool
    let predicted_outcome: String
    let risk_level: String
    let safe_to_execute: Bool
    let side_effects: [String]
}

struct QueryResult: Codable {
    let query: String
    let answer: String
    let confidence: Double
}

struct BenchmarkData: Codable {
    let model_identifier: String
    let soc_generation: String
    let cpu_cores: Int
    let memory_bytes: UInt64
    let storage_bytes: UInt64
    let health_score: Double
    let memory_utilization: Double
    let disk_utilization: Double
    let process_count: Int
    let anomaly_count: Int
}

struct MonitoringPlan: Codable {
    let intervals: [MonitoringInterval]
    let alerts: [String]
}

struct MonitoringInterval: Codable, Identifiable {
    let dimension: String
    let interval_secs: UInt64
    let metrics: [String]

    var id: String { dimension }
}

// MARK: - Twin Recommendations (composite for --action recommend)

struct TwinRecommendations: Codable {
    let health_score: Double
    let trust_score: Double
    let cleanup_impact: CleanupImpact
    let workflow_changes: [WorkflowChange]
    let hardware_upgrades: [UpgradeRecommendation]
    let software_changes: [SoftwareRecommendation]
    let preventive_actions: [RecommendedAction]
}
