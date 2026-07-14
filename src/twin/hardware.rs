use serde::{Deserialize, Serialize};

/// Hardware Reality Model — complete hardware profile of the Mac.
///
/// Covers: SoC, CPU cores (P/E), GPU, Neural Engine, unified memory,
/// SSD, battery, thermal, peripherals, USB/Thunderbolt topology,
/// network interfaces, audio/camera/microphone, and power states.
///
/// Maps to Digital Twin operations 1-40.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareProfile {
    pub model_identifier: String,
    pub soc_generation: String,
    pub cpu_cores: CpuTopology,
    pub gpu: GpuInfo,
    pub neural_engine: NeuralEngineInfo,
    pub memory: HardwareMemory,
    pub storage: Vec<StorageDevice>,
    pub battery: Option<BatteryHardware>,
    pub thermal: ThermalInfo,
    pub peripherals: Peripherals,
    pub power_state: PowerState,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuTopology {
    pub total_logical_cores: usize,
    pub performance_cores: usize,
    pub efficiency_cores: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpuInfo {
    pub core_count: usize,
    pub utilization_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NeuralEngineInfo {
    pub core_count: usize,
    pub utilization_pct: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareMemory {
    pub total_bytes: u64,
    pub bandwidth_gbps: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageDevice {
    pub name: String,
    pub capacity_bytes: u64,
    pub is_ssd: bool,
    pub smart_health: Option<SmartHealth>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SmartHealth {
    pub status: String,
    pub temperature_celsius: Option<f64>,
    pub power_on_hours: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryHardware {
    pub model: String,
    pub chemistry: String,
    pub cycle_count: u64,
    pub design_capacity_mah: Option<u64>,
    pub current_capacity_mah: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalInfo {
    pub cpu_temp_c: Option<f64>,
    pub gpu_temp_c: Option<f64>,
    pub fan_speeds_rpm: Vec<u32>,
    pub thermal_pressure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Peripherals {
    pub usb_devices: Vec<String>,
    pub thunderbolt_devices: Vec<String>,
    pub bluetooth_devices: Vec<String>,
    pub external_displays: Vec<String>,
    pub audio_devices: Vec<String>,
    pub network_interfaces: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PowerState {
    pub is_on_battery: bool,
    pub is_charging: bool,
    pub sleep_state: String,
    pub low_power_mode: bool,
}

impl HardwareProfile {
    /// Collect hardware profile via system_profiler, sysctl, and ioreg.
    pub fn collect() -> Self {
        // TODO: implement full hardware collection (ops 1-40)
        // Start with what system_awareness already provides
        let snapshot = crate::intelligence::SystemSnapshot::collect();

        Self {
            model_identifier: detect_mac_model(),
            soc_generation: detect_soc_generation(),
            cpu_cores: CpuTopology {
                total_logical_cores: snapshot.cpu.core_count,
                performance_cores: detect_perf_cores(),
                efficiency_cores: detect_efficiency_cores(),
            },
            gpu: GpuInfo {
                core_count: detect_gpu_cores(),
                utilization_pct: None, // Not available without private APIs
            },
            neural_engine: NeuralEngineInfo {
                core_count: detect_ane_cores(),
                utilization_pct: None, // Not available without private APIs
            },
            memory: HardwareMemory {
                total_bytes: snapshot.memory.total_bytes,
                bandwidth_gbps: None, // Requires private APIs
            },
            storage: collect_storage_devices(),
            battery: collect_battery_hardware(&snapshot.battery),
            thermal: ThermalInfo {
                cpu_temp_c: snapshot.thermal.cpu_temp_c,
                gpu_temp_c: snapshot.thermal.gpu_temp_c,
                fan_speeds_rpm: snapshot.thermal.fan_speed_rpm.clone(),
                thermal_pressure: snapshot.thermal.thermal_pressure.clone(),
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
                is_on_battery: !snapshot.battery.is_plugged,
                is_charging: snapshot.battery.is_charging,
                sleep_state: "Active".to_string(),
                low_power_mode: false,
            },
            fingerprint: String::new(),
        }
    }
}

#[cfg(target_os = "macos")]
fn detect_mac_model() -> String {
    use std::process::Command;
    Command::new("sysctl")
        .args(["-n", "hw.model"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
        .unwrap_or_else(|| "Unknown".to_string())
}

#[cfg(not(target_os = "macos"))]
fn detect_mac_model() -> String {
    "N/A".to_string()
}

#[cfg(target_os = "macos")]
fn detect_soc_generation() -> String {
    use std::process::Command;
    // Apple Silicon: check hw.optional.arm64
    let out = Command::new("sysctl")
        .args(["-n", "hw.optional.arm64"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default();
    if out == "1" {
        "Apple Silicon".to_string()
    } else {
        "Intel".to_string()
    }
}

#[cfg(not(target_os = "macos"))]
fn detect_soc_generation() -> String {
    "N/A".to_string()
}

#[cfg(target_os = "macos")]
fn detect_perf_cores() -> usize {
    use std::process::Command;
    Command::new("sysctl")
        .args(["-n", "hw.perflevel0.logicalcpu"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
        .unwrap_or(0)
}

#[cfg(not(target_os = "macos"))]
fn detect_perf_cores() -> usize {
    0
}

#[cfg(target_os = "macos")]
fn detect_efficiency_cores() -> usize {
    use std::process::Command;
    Command::new("sysctl")
        .args(["-n", "hw.perflevel1.logicalcpu"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
        .unwrap_or(0)
}

#[cfg(not(target_os = "macos"))]
fn detect_efficiency_cores() -> usize {
    0
}

#[cfg(target_os = "macos")]
fn detect_gpu_cores() -> usize {
    // GPU core count is not directly exposed via sysctl.
    // Would need Metal device enumeration.
    0
}

#[cfg(not(target_os = "macos"))]
fn detect_gpu_cores() -> usize {
    0
}

#[cfg(target_os = "macos")]
fn detect_ane_cores() -> usize {
    // Neural Engine core count is not publicly exposed.
    // Known: M1=16, M1 Pro/Max=16, M2=16, M3=16, M4=16
    16
}

#[cfg(not(target_os = "macos"))]
fn detect_ane_cores() -> usize {
    0
}

fn collect_storage_devices() -> Vec<StorageDevice> {
    // TODO: implement via diskutil list + diskutil info
    Vec::new()
}

fn collect_battery_hardware(battery: &crate::intelligence::system_awareness::BatteryDimension) -> Option<BatteryHardware> {
    if !battery.is_present {
        return None;
    }
    Some(BatteryHardware {
        model: "Unknown".to_string(),
        chemistry: "Lithium".to_string(),
        cycle_count: battery.cycle_count,
        design_capacity_mah: None,
        current_capacity_mah: None,
    })
}
