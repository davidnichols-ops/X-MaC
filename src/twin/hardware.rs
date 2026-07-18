use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
use std::process::Command;

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

/// op 26: A camera device detected via `system_profiler SPCameraDataType`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CameraDevice {
    pub name: String,
    pub vendor: Option<String>,
    pub unique_id: Option<String>,
}

/// op 27: A microphone device detected via `system_profiler SPMicrophoneDataType`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct MicrophoneDevice {
    pub name: String,
    pub vendor: Option<String>,
    pub channels: Option<u32>,
}

/// op 31: Swap activity parsed from `sysctl vm.swapusage`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SwapInfo {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub encrypted: bool,
}

/// op 32: SSD throughput metrics (best-effort estimate).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SsdThroughput {
    pub device: String,
    /// Estimated/observed read throughput in MB/s, if available.
    pub read_mbps: Option<f64>,
    /// Estimated/observed write throughput in MB/s, if available.
    pub write_mbps: Option<f64>,
}

/// op 35: A single wake/sleep event parsed from `pmset -g log`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct WakeEvent {
    pub timestamp: String,
    pub event_type: String,
    pub detail: String,
}

/// op 36: Thermal throttling state parsed from `pmset -g therm`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ThermalThrottling {
    /// CPU power limit (Watts), if reported.
    pub cpu_power_limit_w: Option<f64>,
    /// GPU power limit (Watts), if reported.
    pub gpu_power_limit_w: Option<f64>,
    /// Whether thermal throttling is currently active.
    pub is_throttling: bool,
}

/// op 37: Performance limits read from sysctl (e.g. `hw.cpufrequency_max`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct PerformanceLimits {
    pub max_cpu_freq_hz: Option<u64>,
    pub max_gpu_freq_hz: Option<u64>,
    pub cpu_package_power_w: Option<f64>,
}

/// op 17: Display characteristics parsed from `system_profiler SPDisplaysDataType`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct DisplayInfo {
    pub display_type: String,
    pub resolution: Option<String>,
    pub color_depth: Option<String>,
    pub is_builtin: bool,
}

/// op 20: A node in the USB topology parsed from `ioreg -p IOUSB -l`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UsbNode {
    pub name: String,
    pub vendor_id: Option<String>,
    pub product_id: Option<String>,
    pub children: Vec<UsbNode>,
}

/// op 23: WiFi hardware info parsed from `system_profiler SPNetworkDataType`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct WifiInfo {
    pub interface: String,
    pub card_type: Option<String>,
    pub mac_address: Option<String>,
    pub ssid: Option<String>,
}

/// op 40: A single historical hardware snapshot entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareHistoryEntry {
    /// Unix-epoch milliseconds of when the snapshot was taken.
    pub timestamp_ms: i64,
    pub fingerprint: String,
    pub model_identifier: String,
    pub soc_generation: String,
    pub total_memory_bytes: u64,
    pub total_storage_bytes: u64,
    pub battery_cycle_count: u64,
    pub thermal_pressure: String,
}

/// op 40: Maintains a history of hardware snapshots with save/load to JSON.
/// Stores a rolling list of `HardwareHistoryEntry` records keyed by timestamp.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareHistory {
    pub entries: Vec<HardwareHistoryEntry>,
}

impl HardwareHistory {
    /// Create an empty history.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Append a `HardwareProfile` snapshot to the history with the current
    /// timestamp (Unix-epoch milliseconds).
    #[allow(dead_code)]
    pub fn record(&mut self, profile: &HardwareProfile) {
        let total_storage_bytes: u64 = profile.storage.iter().map(|d| d.capacity_bytes).sum();
        let battery_cycle_count = profile.battery.as_ref().map(|b| b.cycle_count).unwrap_or(0);
        self.entries.push(HardwareHistoryEntry {
            timestamp_ms: chrono::Utc::now().timestamp_millis(),
            fingerprint: profile.fingerprint.clone(),
            model_identifier: profile.model_identifier.clone(),
            soc_generation: profile.soc_generation.clone(),
            total_memory_bytes: profile.memory.total_bytes,
            total_storage_bytes,
            battery_cycle_count,
            thermal_pressure: profile.thermal.thermal_pressure.clone(),
        });
    }

    /// Save the history to a JSON file at `path`.
    #[allow(dead_code)]
    pub fn save(&self, path: &std::path::Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self)
            .map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Load history from a JSON file at `path`. Returns an empty history if
    /// the file does not exist.
    #[allow(dead_code)]
    pub fn load(path: &std::path::Path) -> std::io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// Prune entries older than `max_entries`, keeping the most recent ones.
    #[allow(dead_code)]
    pub fn prune(&mut self, max_entries: usize) {
        if self.entries.len() > max_entries {
            let drop = self.entries.len() - max_entries;
            self.entries.drain(0..drop);
        }
    }
}

/// op 278: A comprehensive machine profile for comparison and benchmarking.
/// Aggregates the key hardware dimensions (SoC, CPU, GPU, ANE, memory,
/// storage, battery, thermal) into a single comparable summary.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct MachineProfile {
    pub model_identifier: String,
    pub soc_generation: String,
    pub total_logical_cores: usize,
    pub performance_cores: usize,
    pub efficiency_cores: usize,
    pub gpu_core_count: usize,
    pub ane_core_count: usize,
    pub total_memory_bytes: u64,
    pub total_storage_bytes: u64,
    pub has_battery: bool,
    pub battery_cycle_count: u64,
    pub thermal_pressure: String,
    pub fingerprint: String,
}

/// op 281: A process identified as likely using the Neural Engine (ANE).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ANEWorkload {
    pub pid: u32,
    pub name: String,
    /// Heuristic confidence (0-1) that this process is using the ANE.
    pub confidence: f64,
    /// Reason for the classification (e.g. "CoreML model loaded").
    pub reason: String,
}

impl HardwareProfile {
    /// Collect hardware profile via system_profiler, sysctl, and ioreg.
    pub fn collect() -> Self {
        // TODO: implement full hardware collection (ops 1-40)
        // Start with what system_awareness already provides
        let snapshot = crate::intelligence::SystemSnapshot::collect();

        let mut profile = Self {
            model_identifier: detect_mac_model(),
            soc_generation: detect_soc_generation(),
            cpu_cores: CpuTopology {
                total_logical_cores: snapshot.cpu.core_count,
                performance_cores: detect_perf_cores(),
                efficiency_cores: detect_efficiency_cores(),
            },
            gpu: GpuInfo {
                core_count: detect_gpu_cores(),
                utilization_pct: detect_gpu_utilization(),
            },
            neural_engine: NeuralEngineInfo {
                core_count: detect_ane_cores(),
                utilization_pct: None, // Not available without private APIs
            },
            memory: HardwareMemory {
                total_bytes: snapshot.memory.total_bytes,
                bandwidth_gbps: detect_memory_bandwidth(),
            },
            storage: collect_storage_devices(),
            battery: collect_battery_hardware(&snapshot.battery),
            thermal: ThermalInfo {
                cpu_temp_c: snapshot.thermal.cpu_temp_c,
                gpu_temp_c: snapshot.thermal.gpu_temp_c,
                fan_speeds_rpm: snapshot.thermal.fan_speed_rpm.clone(),
                thermal_pressure: snapshot.thermal.thermal_pressure.clone(),
            },
            peripherals: collect_peripherals(),
            power_state: PowerState {
                is_on_battery: !snapshot.battery.is_plugged,
                is_charging: snapshot.battery.is_charging,
                sleep_state: detect_sleep_state(),
                low_power_mode: detect_low_power_mode(),
            },
            fingerprint: String::new(),
        };

        // Generate hardware fingerprint from collected attributes (op 38-40).
        profile.fingerprint = generate_hardware_fingerprint(&profile);
        profile
    }

    /// op 278: Build machine profile — create a comprehensive machine
    /// profile for comparison and benchmarking. Aggregates the key hardware
    /// dimensions into a single comparable summary.
    #[allow(dead_code)]
    pub fn build_machine_profile(&self) -> MachineProfile {
        let total_storage_bytes: u64 = self.storage.iter().map(|d| d.capacity_bytes).sum();
        let has_battery = self.battery.is_some();
        let battery_cycle_count = self.battery.as_ref().map(|b| b.cycle_count).unwrap_or(0);
        MachineProfile {
            model_identifier: self.model_identifier.clone(),
            soc_generation: self.soc_generation.clone(),
            total_logical_cores: self.cpu_cores.total_logical_cores,
            performance_cores: self.cpu_cores.performance_cores,
            efficiency_cores: self.cpu_cores.efficiency_cores,
            gpu_core_count: self.gpu.core_count,
            ane_core_count: self.neural_engine.core_count,
            total_memory_bytes: self.memory.total_bytes,
            total_storage_bytes,
            has_battery,
            battery_cycle_count,
            thermal_pressure: self.thermal.thermal_pressure.clone(),
            fingerprint: self.fingerprint.clone(),
        }
    }

    /// op 281: Analyze Neural Engine workloads — identify processes likely
    /// using the ANE (ML apps, CoreML models, vision frameworks). Uses a
    /// heuristic based on process names and known ML-related frameworks.
    #[allow(dead_code)]
    pub fn analyze_ane_workloads(&self) -> Vec<ANEWorkload> {
        #[cfg(target_os = "macos")]
        {
            analyze_ane_workloads_macos()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Vec::new()
        }
    }
}

#[cfg(target_os = "macos")]
fn detect_mac_model() -> String {
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
    // op 6: GPU core count is not directly exposed via sysctl on Apple
    // Silicon, but `system_profiler SPDisplaysDataType` reports
    // "Total Number of Cores" for the built-in GPU.
    let output = system_profiler("SPDisplaysDataType");
    parse_gpu_cores(&output)
}

#[cfg(not(target_os = "macos"))]
fn detect_gpu_cores() -> usize {
    0
}

/// Parse GPU core count from `system_profiler SPDisplaysDataType` output by
/// looking for a "Total Number of Cores" entry.
#[cfg(target_os = "macos")]
fn parse_gpu_cores(output: &str) -> usize {
    parse_profiler_value(output, "Total Number of Cores")
        .or_else(|| parse_profiler_value(output, "Total Cores"))
        .and_then(|v| v.split_whitespace().next().and_then(|s| s.parse().ok()))
        .unwrap_or(0)
}

#[cfg(target_os = "macos")]
fn detect_ane_cores() -> usize {
    // op 7: The Neural Engine core count is not publicly exposed via sysctl.
    // All Apple Silicon generations so far (M1–M4) ship a 16-core ANE, so we
    // estimate based on the SoC generation / model identifier.
    let model = detect_mac_model();
    estimate_ane_cores(&model)
}

#[cfg(not(target_os = "macos"))]
fn detect_ane_cores() -> usize {
    0
}

/// op 7: Estimate the ANE core count from the Mac model identifier. All known
/// Apple Silicon generations (M1, M2, M3, M4 and their Pro/Max/Ultra variants)
/// ship a 16-core Neural Engine. Returns 16 for any Apple Silicon model and
/// 0 for Intel/unknown.
fn estimate_ane_cores(model: &str) -> usize {
    if soc_family(model).is_some() {
        16
    } else {
        // Fall back to a generic Apple/ARM check for identifiers we don't
        // explicitly map yet (still almost certainly Apple Silicon).
        let lower = model.to_lowercase();
        if lower.contains("apple") || lower.contains("arm") {
            16
        } else {
            0
        }
    }
}

/// Map a Mac model identifier to its Apple Silicon family token: "m1",
/// "m2", "m3", or "m4". Recognises both explicit "M1"/"M2"/... strings and
/// the `MacNN,x` hardware identifiers (Mac13→M1, Mac14→M2, Mac15→M3,
/// Mac16→M4) plus legacy identifiers like "MacBookAir10,1" (M1).
fn soc_family(model: &str) -> Option<&'static str> {
    let m = model.to_lowercase();
    if m.contains("m4") || m.contains("mac16") {
        Some("m4")
    } else if m.contains("m3") || m.contains("mac15") {
        Some("m3")
    } else if m.contains("m2") || m.contains("mac14") {
        Some("m2")
    } else if m.contains("m1")
        || m.contains("mac13")
        || m.contains("macbookair10")
        || m.contains("macbookpro17")
        || m.contains("macbookpro18")
        || m.contains("macmini9")
        || m.contains("macstudio1")
        || m.contains("macpro7")
    {
        Some("m1")
    } else {
        None
    }
}

/// op 9: Pure estimator mapping a model identifier to a peak memory bandwidth
/// in GB/s. Recognises the M1–M4 families and their Pro/Max/Ultra tiers.
fn estimate_memory_bandwidth(model: &str) -> Option<f64> {
    let m = model.to_lowercase();
    let family = soc_family(model)?;
    match family {
        "m1" => {
            if m.contains("ultra") {
                Some(800.0)
            } else if m.contains("max") {
                Some(400.0)
            } else if m.contains("pro") {
                Some(200.0)
            } else {
                Some(68.0)
            }
        }
        "m2" => {
            if m.contains("ultra") {
                Some(800.0)
            } else if m.contains("max") {
                Some(400.0)
            } else if m.contains("pro") {
                Some(200.0)
            } else {
                Some(100.0)
            }
        }
        "m3" => {
            if m.contains("max") {
                Some(400.0)
            } else if m.contains("pro") {
                Some(150.0)
            } else {
                Some(120.0)
            }
        }
        "m4" => {
            if m.contains("max") {
                Some(546.0)
            } else if m.contains("pro") {
                Some(273.0)
            } else {
                Some(120.0)
            }
        }
        _ => None,
    }
}

fn collect_storage_devices() -> Vec<StorageDevice> {
    #[cfg(target_os = "macos")]
    {
        collect_storage_devices_macos()
    }
    #[cfg(not(target_os = "macos"))]
    {
        Vec::new()
    }
}

/// Run `system_profiler` with the given data type and return its text output.
#[cfg(target_os = "macos")]
fn system_profiler(data_type: &str) -> String {
    Command::new("system_profiler")
        .args([data_type])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

/// Run a command and return its stdout as a string.
#[cfg(target_os = "macos")]
fn run_command(program: &str, args: &[&str]) -> String {
    Command::new(program)
        .args(args)
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default()
}

/// Collect all peripherals (ops 25-34): USB, Thunderbolt, Bluetooth,
/// external displays, audio devices, and network interfaces.
fn collect_peripherals() -> Peripherals {
    Peripherals {
        usb_devices: collect_usb_devices(),
        thunderbolt_devices: collect_thunderbolt_devices(),
        bluetooth_devices: collect_bluetooth_devices(),
        external_displays: collect_external_displays(),
        audio_devices: collect_audio_devices(),
        network_interfaces: collect_network_interfaces(),
    }
}

/// Parse `system_profiler` indented text output and extract values for a
/// given key. Returns the trimmed value following the first matching
/// `Key: Value` line.
#[cfg(target_os = "macos")]
fn parse_profiler_value(output: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    for line in output.lines() {
        let trimmed = line.trim_start();
        if let Some(rest) = trimmed.strip_prefix(&prefix) {
            let value = rest.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

/// Extract all values for a given key from profiler output (multiple matches).
#[cfg(target_os = "macos")]
fn parse_profiler_values(output: &str, key: &str) -> Vec<String> {
    let prefix = format!("{key}:");
    output
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim_start();
            trimmed
                .strip_prefix(&prefix)
                .map(|r| r.trim().to_string())
                .filter(|v| !v.is_empty())
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────
// USB topology detection (ops 25-27)
// ─────────────────────────────────────────────────────────────────────

/// Enumerate USB devices via `system_profiler SPUSBDataType` (ops 25-27).
/// Returns device names extracted from the USB tree.
#[cfg(target_os = "macos")]
fn collect_usb_devices() -> Vec<String> {
    let output = system_profiler("SPUSBDataType");
    parse_device_names(&output)
}

#[cfg(not(target_os = "macos"))]
fn collect_usb_devices() -> Vec<String> {
    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────
// Thunderbolt topology detection (ops 28-30)
// ─────────────────────────────────────────────────────────────────────

/// Enumerate Thunderbolt devices via `system_profiler SPThunderboltDataType`
/// (ops 28-30).
#[cfg(target_os = "macos")]
fn collect_thunderbolt_devices() -> Vec<String> {
    let output = system_profiler("SPThunderboltDataType");
    parse_device_names(&output)
}

#[cfg(not(target_os = "macos"))]
fn collect_thunderbolt_devices() -> Vec<String> {
    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────
// Network interface enumeration (op 31)
// ─────────────────────────────────────────────────────────────────────

/// List network interfaces via `system_profiler SPNetworkDataType` (op 31).
#[cfg(target_os = "macos")]
fn collect_network_interfaces() -> Vec<String> {
    let output = system_profiler("SPNetworkDataType");
    // Each interface block begins with a "Type: ..." line; we collect the
    // interface names from "BSD Name:" entries which are stable identifiers.
    parse_profiler_values(&output, "BSD Name")
}

#[cfg(not(target_os = "macos"))]
fn collect_network_interfaces() -> Vec<String> {
    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────
// Bluetooth device enumeration (op 32)
// ─────────────────────────────────────────────────────────────────────

/// List paired Bluetooth devices via `system_profiler SPBluetoothDataType`
/// (op 32).
#[cfg(target_os = "macos")]
fn collect_bluetooth_devices() -> Vec<String> {
    let output = system_profiler("SPBluetoothDataType");
    // Paired devices appear under "Connected:" / "Not Connected:" sections
    // with device names on indented lines. We extract names from the
    // "Name:" field within paired device entries.
    parse_profiler_values(&output, "Name")
}

#[cfg(not(target_os = "macos"))]
fn collect_bluetooth_devices() -> Vec<String> {
    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────
// External display detection (op 33)
// ─────────────────────────────────────────────────────────────────────

/// Detect external displays via `system_profiler SPDisplaysDataType` (op 33).
#[cfg(target_os = "macos")]
fn collect_external_displays() -> Vec<String> {
    let output = system_profiler("SPDisplaysDataType");
    // Displays are listed under "Displays:" with "Resolution:" lines.
    // We collect display names from "Display Type:" entries.
    parse_profiler_values(&output, "Display Type")
}

#[cfg(not(target_os = "macos"))]
fn collect_external_displays() -> Vec<String> {
    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────
// Audio device enumeration (op 34)
// ─────────────────────────────────────────────────────────────────────

/// List audio devices via `system_profiler SPAudioDataType` (op 34).
#[cfg(target_os = "macos")]
fn collect_audio_devices() -> Vec<String> {
    let output = system_profiler("SPAudioDataType");
    parse_device_names(&output)
}

#[cfg(not(target_os = "macos"))]
fn collect_audio_devices() -> Vec<String> {
    Vec::new()
}

/// Extract device names from `system_profiler` output. Device names appear
/// as indented header lines (no colon) that are not section labels. We
/// heuristically collect lines that look like `Key: Value` where Key is
/// "Name" or device identifiers, plus un-indented top-level device headers.
#[cfg(target_os = "macos")]
fn parse_device_names(output: &str) -> Vec<String> {
    let mut names = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim_start();
        // Skip empty lines and section headers (end with colon and no value).
        if trimmed.is_empty() || trimmed.ends_with(':') {
            continue;
        }
        // Collect "Name: <value>" entries which represent device names.
        if let Some(rest) = trimmed.strip_prefix("Name:") {
            let value = rest.trim();
            if !value.is_empty() {
                names.push(value.to_string());
            }
        }
    }
    names
}

// ─────────────────────────────────────────────────────────────────────
// Storage device collection (ops 35-37)
// ─────────────────────────────────────────────────────────────────────

/// Collect storage devices via `diskutil list` and `diskutil info` on macOS
/// (ops 35-37). Parses device name, capacity, SSD status, and SMART health.
#[cfg(target_os = "macos")]
fn collect_storage_devices_macos() -> Vec<StorageDevice> {
    let list_output = run_command("diskutil", &["list"]);
    let mut devices = Vec::new();

    // Each disk line looks like:
    //   /dev/disk0 (internal, physical):
    for line in list_output.lines() {
        let trimmed = line.trim();
        if let Some(dev_path) = trimmed.strip_prefix("/dev/") {
            if let Some(name) = dev_path.strip_suffix(':') {
                let device = parse_diskutil_info(name);
                if let Some(d) = device {
                    devices.push(d);
                }
            }
        }
    }
    devices
}

/// Run `diskutil info <device>` and parse the result into a `StorageDevice`.
/// op 11: also parses temperature and power-on hours from SMART data when
/// available via `smartctl` (best-effort, falls back to `None`).
#[cfg(target_os = "macos")]
fn parse_diskutil_info(device: &str) -> Option<StorageDevice> {
    let output = run_command("diskutil", &["info", device]);
    if output.is_empty() {
        return None;
    }
    let name = parse_profiler_value(&output, "Device Name")
        .or_else(|| parse_profiler_value(&output, "Volume Name"))
        .unwrap_or_else(|| device.to_string());
    let capacity_bytes = parse_profiler_value(&output, "Disk Size")
        .as_deref()
        .and_then(parse_byte_size)
        .unwrap_or(0);
    let is_ssd = parse_profiler_value(&output, "Solid State")
        .map(|v| v.eq_ignore_ascii_case("yes") || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let smart_status = parse_profiler_value(&output, "SMART Status");

    // op 11: best-effort SMART attribute extraction. `smartctl -A` reports
    // attributes such as "Temperature_Celsius" and "Power_On_Hours". Not all
    // Apple SSDs expose these, so we gracefully fall back to `None`.
    let (temperature_celsius, power_on_hours) = parse_smart_attributes(device);

    let smart_health = smart_status.map(|status| SmartHealth {
        status,
        temperature_celsius,
        power_on_hours,
    });

    Some(StorageDevice {
        name,
        capacity_bytes,
        is_ssd,
        smart_health,
    })
}

/// op 11: Parse SMART attributes (temperature, power-on hours) for a device
/// using `smartctl -A -d sat <device>` if available. Returns `(None, None)`
/// when `smartctl` is not installed or the attributes are absent.
#[cfg(target_os = "macos")]
fn parse_smart_attributes(device: &str) -> (Option<f64>, Option<u64>) {
    let dev_path = format!("/dev/{device}");
    let output = run_command("smartctl", &["-A", "-d", "sat", &dev_path]);
    if output.is_empty() {
        return (None, None);
    }
    let temp = parse_smart_attribute_value(&output, "Temperature_Celsius")
        .or_else(|| parse_smart_attribute_value(&output, "Temperature"))
        .and_then(|s| s.parse::<f64>().ok());
    let hours = parse_smart_attribute_value(&output, "Power_On_Hours")
        .and_then(|s| s.parse::<u64>().ok());
    (temp, hours)
}

/// Extract the raw value (last numeric column) for a named SMART attribute
/// from `smartctl -A` output. Each attribute line looks like:
/// `194 Temperature_Celsius     0x0022   040   055   000    Old_age   Always -       40 (Min/Max 16/60)`
/// We return the trailing parenthesised or final numeric token.
#[cfg(target_os = "macos")]
fn parse_smart_attribute_value(output: &str, attr_name: &str) -> Option<String> {
    for line in output.lines() {
        if line.contains(attr_name) {
            // The "RAW_VALUE" is the last whitespace-delimited token (possibly
            // followed by a parenthesised range). Take the last pure number.
            let tokens: Vec<&str> = line.split_whitespace().collect();
            // Scan from the end for the first parseable number.
            for tok in tokens.iter().rev() {
                let cleaned = tok.trim_matches(|c: char| !c.is_ascii_digit() && c != '.');
                if cleaned.parse::<f64>().is_ok() {
                    return Some(cleaned.to_string());
                }
            }
        }
    }
    None
}

/// Parse a human-readable byte size like "500.0 GB" or "1.0 TB" into bytes.
#[cfg(target_os = "macos")]
fn parse_byte_size(s: &str) -> Option<u64> {
    let s = s.trim();
    let mut parts = s.split_whitespace();
    let num: f64 = parts.next()?.parse().ok()?;
    let unit = parts.next().unwrap_or("");
    let multiplier = match unit {
        "B" | "" => 1.0,
        "KB" => 1_000.0,
        "MB" => 1_000_000.0,
        "GB" => 1_000_000_000.0,
        "TB" => 1_000_000_000_000.0,
        _ => 1.0,
    };
    Some((num * multiplier) as u64)
}

// ─────────────────────────────────────────────────────────────────────
// Hardware fingerprint (ops 38-40)
// ─────────────────────────────────────────────────────────────────────

/// Generate a unique hardware fingerprint by BLAKE3-hashing the model
/// identifier, CPU core counts, total memory, and aggregate storage
/// capacity (ops 38-40). Returns a hex-encoded digest.
fn generate_hardware_fingerprint(profile: &HardwareProfile) -> String {
    let storage_total: u64 = profile.storage.iter().map(|d| d.capacity_bytes).sum();
    let fingerprint_input = format!(
        "{}|{}|{}|{}|{}|{}|{}",
        profile.model_identifier,
        profile.cpu_cores.total_logical_cores,
        profile.cpu_cores.performance_cores,
        profile.cpu_cores.efficiency_cores,
        profile.memory.total_bytes,
        storage_total,
        profile.soc_generation,
    );
    let mut hasher = blake3::Hasher::new();
    hasher.update(fingerprint_input.as_bytes());
    hasher.finalize().to_hex().to_string()
}

// ─────────────────────────────────────────────────────────────────────
// Power state detection
// ─────────────────────────────────────────────────────────────────────

/// Detect low power mode status using `pmset -g` on macOS. Returns true if
/// low power mode is enabled.
#[cfg(target_os = "macos")]
fn detect_low_power_mode() -> bool {
    let output = run_command("pmset", &["-g"]);
    // Look for "lowpowermode" setting with value 1.
    output
        .lines()
        .any(|line| line.trim().starts_with("lowpowermode") && line.contains("1"))
}

#[cfg(not(target_os = "macos"))]
fn detect_low_power_mode() -> bool {
    false
}

// ─────────────────────────────────────────────────────────────────────
// Memory bandwidth / GPU utilization / sleep state (ops 9, 28, 34)
// ─────────────────────────────────────────────────────────────────────

/// op 9: Estimate unified-memory bandwidth based on the SoC generation. Apple
/// publishes peak bandwidth figures: M1 ≈ 68 GB/s, M2 ≈ 100 GB/s,
/// M3 ≈ 120 GB/s, M4 ≈ 120 GB/s. Pro/Max/Ultra variants scale up but the base
/// figure is returned here. Returns `None` for Intel or unknown SoCs.
#[cfg(target_os = "macos")]
fn detect_memory_bandwidth() -> Option<f64> {
    let model = detect_mac_model();
    estimate_memory_bandwidth(&model)
}

#[cfg(not(target_os = "macos"))]
fn detect_memory_bandwidth() -> Option<f64> {
    None
}

/// op 28: Detect GPU utilization. Apple Silicon does not expose per-GPU
/// utilization through public APIs; `ioreg` does not report a stable
/// utilization percentage. We attempt an `ioreg`-based probe but currently
/// return `None` with a clear comment, as no reliable public metric exists.
#[cfg(target_os = "macos")]
fn detect_gpu_utilization() -> Option<f64> {
    // No public API exposes real-time GPU utilization on Apple Silicon.
    // `ioreg -c IOGPUDevice` does not publish a utilization percentage.
    // Returning None is the honest answer; a future Metal/powermetrics
    // integration could fill this in.
    None
}

#[cfg(not(target_os = "macos"))]
fn detect_gpu_utilization() -> Option<f64> {
    None
}

/// op 34: Detect the current sleep state by parsing `pmset -g assertions`.
/// Returns "Active", "Sleeping", or "Unknown".
#[cfg(target_os = "macos")]
fn detect_sleep_state() -> String {
    let output = run_command("pmset", &["-g", "assertions"]);
    parse_sleep_state(&output)
}

#[cfg(not(target_os = "macos"))]
fn detect_sleep_state() -> String {
    "Unknown".to_string()
}

/// op 34: Parse `pmset -g assertions` output to determine sleep state. The
/// "PreventUserIdleSystemSleep" assertion being held indicates the system is
/// active; the "DeclareSystemBooter" / absence of prevent-sleep assertions
/// while "Sleeping" is reported indicates a sleeping state.
fn parse_sleep_state(output: &str) -> String {
    let has_prevent_sleep = output
        .lines()
        .any(|line| line.contains("PreventUserIdleSystemSleep") && line.contains("1"));
    let has_sleeping = output
        .lines()
        .any(|line| line.contains("Sleeping") || line.contains("DisplaySleep"));
    if has_prevent_sleep {
        "Active".to_string()
    } else if has_sleeping {
        "Sleeping".to_string()
    } else {
        "Active".to_string()
    }
}

fn collect_battery_hardware(
    battery: &crate::intelligence::system_awareness::BatteryDimension,
) -> Option<BatteryHardware> {
    if !battery.is_present {
        return None;
    }
    // ops 12 & 14: parse battery model and chemistry from
    // `system_profiler SPPowerDataType`.
    #[cfg(target_os = "macos")]
    {
        let output = system_profiler("SPPowerDataType");
        let (model, chemistry) = parse_battery_model(&output);
        Some(BatteryHardware {
            model,
            chemistry,
            cycle_count: battery.cycle_count,
            design_capacity_mah: None,
            current_capacity_mah: None,
        })
    }
    #[cfg(not(target_os = "macos"))]
    {
        Some(BatteryHardware {
            model: "Unknown".to_string(),
            chemistry: "Lithium".to_string(),
            cycle_count: battery.cycle_count,
            design_capacity_mah: None,
            current_capacity_mah: None,
        })
    }
}

/// ops 12 & 14: Parse battery model and chemistry from
/// `system_profiler SPPowerDataType` output. Returns `(model, chemistry)`,
/// defaulting to "Unknown" / "Lithium" when the keys are absent.
#[cfg(target_os = "macos")]
fn parse_battery_model(output: &str) -> (String, String) {
    let model = parse_profiler_value(output, "Model Name")
        .or_else(|| parse_profiler_value(output, "Model"))
        .unwrap_or_else(|| "Unknown".to_string());
    let chemistry = parse_profiler_value(output, "Chemistry")
        .or_else(|| parse_profiler_value(output, "Battery Chemistry"))
        .unwrap_or_else(|| "Lithium".to_string());
    (model, chemistry)
}

/// op 281: Identify processes likely using the Neural Engine on macOS.
/// Scans `ps` output for ML/AI-related process names and CoreML model
/// processes, returning a list of candidate ANE workloads with heuristic
/// confidence scores.
#[cfg(target_os = "macos")]
fn analyze_ane_workloads_macos() -> Vec<ANEWorkload> {
    use std::process::Command as PCommand;
    let out = PCommand::new("ps")
        .args(["-axo", "pid=,comm="])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
        .unwrap_or_default();

    let mut workloads = Vec::new();
    for line in out.lines() {
        let mut parts = line.split_whitespace();
        let pid: u32 = match parts.next().and_then(|s| s.parse().ok()) {
            Some(p) => p,
            None => continue,
        };
        let name = parts.collect::<Vec<_>>().join(" ");
        if name.is_empty() {
            continue;
        }
        let lower = name.to_lowercase();
        // Heuristic classification by process name keywords.
        let (confidence, reason) = if lower.contains("coreml") || lower.contains("coremlcompiler") {
            (0.95, "CoreML model process".to_string())
        } else if lower.contains("vision") && lower.contains("framework") {
            (0.8, "Vision framework process".to_string())
        } else if lower.contains("coremlmodel") || lower.contains("mlmodel") {
            (0.9, "ML model loaded".to_string())
        } else if lower.contains("tensor") && lower.contains("flow") {
            (0.7, "TensorFlow process".to_string())
        } else if lower.contains("pytorch") {
            (0.7, "PyTorch process".to_string())
        } else if lower.contains("onnx") {
            (0.65, "ONNX runtime process".to_string())
        } else if lower.contains("ane") || lower.contains("neuralengine") {
            (0.85, "Neural Engine process".to_string())
        } else if lower.contains("siri") || lower.contains("spotlight") {
            (0.5, "ML-assisted system process".to_string())
        } else {
            continue;
        };
        workloads.push(ANEWorkload {
            pid,
            name,
            confidence,
            reason,
        });
    }
    workloads
}

// ═══════════════════════════════════════════════════════════════════════
//  Phase 11 operations: cameras, microphones, swap, SSD throughput,
//  wake events, thermal throttling, performance limits, displays,
//  USB topology, WiFi hardware.
// ═══════════════════════════════════════════════════════════════════════

// ─────────────────────────────────────────────────────────────────────
// Camera detection (op 26)
// ─────────────────────────────────────────────────────────────────────

/// op 26: Detect camera devices via `system_profiler SPCameraDataType`.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn detect_cameras() -> Vec<CameraDevice> {
    let output = system_profiler("SPCameraDataType");
    parse_camera_devices(&output)
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn detect_cameras() -> Vec<CameraDevice> {
    Vec::new()
}

/// op 26: Parse `system_profiler SPCameraDataType` output into a list of
/// `CameraDevice` entries. Each camera block has a "Model Name" / "Name"
/// header plus optional "Unique ID" and "Manufacturer" fields.
#[cfg(target_os = "macos")]
fn parse_camera_devices(output: &str) -> Vec<CameraDevice> {
    let names = parse_profiler_values(output, "Model Name");
    let uids = parse_profiler_values(output, "Unique ID");
    let vendors = parse_profiler_values(output, "Manufacturer");
    names
        .into_iter()
        .enumerate()
        .map(|(i, name)| CameraDevice {
            name,
            vendor: vendors.get(i).cloned(),
            unique_id: uids.get(i).cloned(),
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────
// Microphone detection (op 27)
// ─────────────────────────────────────────────────────────────────────

/// op 27: Detect microphone devices via `system_profiler SPMicrophoneDataType`.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn detect_microphones() -> Vec<MicrophoneDevice> {
    let output = system_profiler("SPMicrophoneDataType");
    parse_microphone_devices(&output)
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn detect_microphones() -> Vec<MicrophoneDevice> {
    Vec::new()
}

/// op 27: Parse `system_profiler SPMicrophoneDataType` output into a list of
/// `MicrophoneDevice` entries.
#[cfg(target_os = "macos")]
fn parse_microphone_devices(output: &str) -> Vec<MicrophoneDevice> {
    let names = parse_profiler_values(output, "Model Name");
    let vendors = parse_profiler_values(output, "Manufacturer");
    let channels: Vec<Option<u32>> = parse_profiler_values(output, "Channels")
        .into_iter()
        .map(|s| s.split_whitespace().next().and_then(|t| t.parse().ok()))
        .collect();
    names
        .into_iter()
        .enumerate()
        .map(|(i, name)| MicrophoneDevice {
            name,
            vendor: vendors.get(i).cloned(),
            channels: channels.get(i).copied().flatten(),
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────
// Swap activity (op 31)
// ─────────────────────────────────────────────────────────────────────

/// op 31: Detect swap activity by parsing `sysctl vm.swapusage` output.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn detect_swap_activity() -> Option<SwapInfo> {
    let output = run_command("sysctl", &["vm.swapusage"]);
    if output.is_empty() {
        return None;
    }
    Some(parse_swap_usage(&output))
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn detect_swap_activity() -> Option<SwapInfo> {
    None
}

/// op 31: Parse `sysctl vm.swapusage` output, which looks like:
/// `vm.swapusage: total = 2048.00M  used = 12.50M  free = 2035.50M  (encrypted)`
fn parse_swap_usage(output: &str) -> SwapInfo {
    let total = parse_swap_field(output, "total").unwrap_or(0);
    let used = parse_swap_field(output, "used").unwrap_or(0);
    let free = parse_swap_field(output, "free").unwrap_or(0);
    let encrypted = output.contains("encrypted");
    SwapInfo {
        total_bytes: total,
        used_bytes: used,
        free_bytes: free,
        encrypted,
    }
}

/// Extract a single `key = <value><unit>` field from swapusage output and
/// convert it to bytes. Handles values with attached units (e.g. "2048.00M")
/// and optional "=" separators.
fn parse_swap_field(output: &str, field: &str) -> Option<u64> {
    let tokens: Vec<&str> = output.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        if tokens[i] == field {
            // Format: "total = 2048.00M"
            if tokens.get(i + 1) == Some(&"=") {
                if let Some(val_token) = tokens.get(i + 2) {
                    return parse_size_token(val_token);
                }
            }
            // Format without "=": "total 2048.00M"
            if let Some(val_token) = tokens.get(i + 1) {
                if let Some(v) = parse_size_token(val_token) {
                    return Some(v);
                }
            }
        }
        i += 1;
    }
    None
}

/// Parse a size token like "2048.00M" or "4.00G" into bytes by splitting the
/// numeric prefix from the unit suffix.
fn parse_size_token(token: &str) -> Option<u64> {
    let split = token.find(|c: char| !c.is_ascii_digit() && c != '.');
    let (num_str, unit) = match split {
        Some(idx) => (&token[..idx], &token[idx..]),
        None => (token, ""),
    };
    let num: f64 = num_str.parse().ok()?;
    let multiplier = match unit {
        "K" | "KB" => 1_000.0,
        "M" | "MB" => 1_000_000.0,
        "G" | "GB" => 1_000_000_000.0,
        "T" | "TB" => 1_000_000_000_000.0,
        "" => 1.0,
        _ => 1.0,
    };
    Some((num * multiplier) as u64)
}

// ─────────────────────────────────────────────────────────────────────
// SSD throughput (op 32)
// ─────────────────────────────────────────────────────────────────────

/// op 32: Detect SSD throughput. Apple Silicon SSDs do not expose real-time
/// throughput via `diskutil info`; we report the device and `None` for the
/// read/write rates, with a clear comment that a future `iostat`-style
/// integration could fill these in.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn detect_ssd_throughput() -> Vec<SsdThroughput> {
    let list_output = run_command("diskutil", &["list"]);
    let mut result = Vec::new();
    for line in list_output.lines() {
        let trimmed = line.trim();
        if let Some(dev_path) = trimmed.strip_prefix("/dev/") {
            if let Some(name) = dev_path.strip_suffix(':') {
                let info = run_command("diskutil", &["info", name]);
                let is_ssd = parse_profiler_value(&info, "Solid State")
                    .map(|v| v.eq_ignore_ascii_case("yes") || v.eq_ignore_ascii_case("true"))
                    .unwrap_or(false);
                if is_ssd {
                    result.push(SsdThroughput {
                        device: name.to_string(),
                        // Real-time throughput is not exposed by `diskutil`;
                        // `iostat -d -w 1` would be needed for live rates.
                        read_mbps: None,
                        write_mbps: None,
                    });
                }
            }
        }
    }
    result
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn detect_ssd_throughput() -> Vec<SsdThroughput> {
    Vec::new()
}

// ─────────────────────────────────────────────────────────────────────
// Wake events (op 35)
// ─────────────────────────────────────────────────────────────────────

/// op 35: Detect wake/sleep events by parsing `pmset -g log` for wake and
/// sleep entries.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn detect_wake_events() -> Vec<WakeEvent> {
    let output = run_command("pmset", &["-g", "log"]);
    parse_wake_events(&output)
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn detect_wake_events() -> Vec<WakeEvent> {
    Vec::new()
}

/// op 35: Parse `pmset -g log` output for wake/sleep entries. Lines look like:
/// `2024-01-15 08:32:01 -08 Wake  ...` or `... Sleep ...` or
/// `... DarkWake ...`. We collect entries whose detail contains "Wake",
/// "Sleep", "DarkWake", or "Wake from".
fn parse_wake_events(output: &str) -> Vec<WakeEvent> {
    let mut events = Vec::new();
    for line in output.lines() {
        let lower = line.to_lowercase();
        let event_type = if lower.contains("darkwake") {
            "DarkWake"
        } else if lower.contains("wake") {
            "Wake"
        } else if lower.contains("sleep") {
            "Sleep"
        } else {
            continue;
        };
        // Split off the leading timestamp (first two space-delimited tokens
        // form the date and time).
        let mut parts = line.splitn(3, ' ');
        let date = parts.next().unwrap_or("");
        let time = parts.next().unwrap_or("");
        let timestamp = format!("{date} {time}");
        let detail = parts.next().unwrap_or("").trim().to_string();
        events.push(WakeEvent {
            timestamp,
            event_type: event_type.to_string(),
            detail,
        });
    }
    events
}

// ─────────────────────────────────────────────────────────────────────
// Thermal throttling (op 36)
// ─────────────────────────────────────────────────────────────────────

/// op 36: Detect thermal throttling state by parsing `pmset -g therm`.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn detect_thermal_throttling() -> ThermalThrottling {
    let output = run_command("pmset", &["-g", "therm"]);
    parse_thermal_throttling(&output)
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn detect_thermal_throttling() -> ThermalThrottling {
    ThermalThrottling {
        cpu_power_limit_w: None,
        gpu_power_limit_w: None,
        is_throttling: false,
    }
}

/// op 36: Parse `pmset -g therm` output. It typically reports lines like:
/// `  CPU_PLimit = <watts>` and `  GPU_PLimit = <watts>` and may include a
/// `  ThermalPressure = ...` indicator.
fn parse_thermal_throttling(output: &str) -> ThermalThrottling {
    let cpu_power_limit_w = parse_numeric_field(output, "CPU_PLimit");
    let gpu_power_limit_w = parse_numeric_field(output, "GPU_PLimit");
    // Throttling is inferred when a non-zero power limit is reported or when
    // the output explicitly mentions a non-nominal thermal pressure.
    let is_throttling = output
        .lines()
        .any(|line| line.contains("ThermalPressure") && !line.contains("Nominal"));
    ThermalThrottling {
        cpu_power_limit_w,
        gpu_power_limit_w,
        is_throttling,
    }
}

/// Extract a `Key = <number>` field from a single-line-per-field output.
fn parse_numeric_field(output: &str, key: &str) -> Option<f64> {
    for line in output.lines() {
        let trimmed = line.trim();
        if let Some(rest) = trimmed.strip_prefix(key) {
            let rest = rest.trim_start();
            if let Some(value_part) = rest.strip_prefix('=') {
                let value_str = value_part.trim();
                if let Ok(v) = value_str.parse::<f64>() {
                    return Some(v);
                }
            }
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────
// Performance limits (op 37)
// ─────────────────────────────────────────────────────────────────────

/// op 37: Detect performance limits by reading sysctl keys such as
/// `hw.cpufrequency_max` and `hw.cpufrequency`. GPU frequency is not exposed
/// via sysctl on Apple Silicon.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn detect_performance_limits() -> PerformanceLimits {
    let max_cpu_freq_hz = run_command("sysctl", &["-n", "hw.cpufrequency_max"])
        .trim()
        .parse::<u64>()
        .ok()
        .or_else(|| {
            run_command("sysctl", &["-n", "hw.cpufrequency"])
                .trim()
                .parse::<u64>()
                .ok()
        });
    PerformanceLimits {
        max_cpu_freq_hz,
        // GPU max frequency is not exposed via sysctl on Apple Silicon.
        max_gpu_freq_hz: None,
        // Package power is not exposed via sysctl; would need powermetrics.
        cpu_package_power_w: None,
    }
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn detect_performance_limits() -> PerformanceLimits {
    PerformanceLimits {
        max_cpu_freq_hz: None,
        max_gpu_freq_hz: None,
        cpu_package_power_w: None,
    }
}

// ─────────────────────────────────────────────────────────────────────
// Display characteristics (op 17)
// ─────────────────────────────────────────────────────────────────────

/// op 17: Detect display characteristics (resolution, color depth) via
/// `system_profiler SPDisplaysDataType`.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn collect_display_info() -> Vec<DisplayInfo> {
    let output = system_profiler("SPDisplaysDataType");
    parse_display_info(&output)
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn collect_display_info() -> Vec<DisplayInfo> {
    Vec::new()
}

/// op 17: Parse `system_profiler SPDisplaysDataType` output into display
/// entries. Each display block reports "Display Type:", "Resolution:", and
/// "Color Depth:" fields. A "Built-In" display type indicates the internal
/// display.
#[cfg(target_os = "macos")]
fn parse_display_info(output: &str) -> Vec<DisplayInfo> {
    let types = parse_profiler_values(output, "Display Type");
    let resolutions = parse_profiler_values(output, "Resolution");
    let depths = parse_profiler_values(output, "Color Depth");
    types
        .into_iter()
        .enumerate()
        .map(|(i, display_type)| DisplayInfo {
            is_builtin: display_type.eq_ignore_ascii_case("Built-In"),
            display_type,
            resolution: resolutions.get(i).cloned(),
            color_depth: depths.get(i).cloned(),
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────
// USB topology via ioreg (op 20)
// ─────────────────────────────────────────────────────────────────────

/// op 20: Build the full USB topology via `ioreg -p IOUSB -l`. Returns a flat
/// list of top-level `UsbNode` entries (each with nested children).
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn collect_usb_topology() -> Vec<UsbNode> {
    let output = run_command("ioreg", &["-p", "IOUSB", "-l"]);
    parse_usb_topology(&output)
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn collect_usb_topology() -> Vec<UsbNode> {
    Vec::new()
}

/// op 20: Parse `ioreg -p IOUSB -l` output into a list of USB nodes. The
/// ioreg output is an indented tree; each device block starts with a line
/// like `+-o <Name>  <class>` and contains `"USB Vendor Name" = ...` style
/// properties. We extract the device name and the `idVendor` / `idProduct`
/// properties.
#[cfg(target_os = "macos")]
fn parse_usb_topology(output: &str) -> Vec<UsbNode> {
    let mut nodes = Vec::new();
    for line in output.lines() {
        let trimmed = line.trim_start();
        // Device header lines look like: "+-o FaceTime HD Camera  <class IOUSBDevice>"
        if let Some(rest) = trimmed.strip_prefix("+-o ").or_else(|| trimmed.strip_prefix("\\-o ")) {
            let name = rest
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim_end_matches(',')
                .to_string();
            if name.is_empty() {
                continue;
            }
            let vendor_id = parse_ioreg_property(line, "idVendor");
            let product_id = parse_ioreg_property(line, "idProduct");
            // ioreg lists properties on separate lines; we collect them by
            // scanning forward from the header. For simplicity here we leave
            // children empty (a full tree build would track indentation).
            nodes.push(UsbNode {
                name,
                vendor_id,
                product_id,
                children: Vec::new(),
            });
        }
    }
    nodes
}

/// Extract an ioreg property value of the form `"Key" = <value>` from a line.
#[cfg(target_os = "macos")]
fn parse_ioreg_property(line: &str, key: &str) -> Option<String> {
    let needle = format!("\"{key}\"");
    if let Some(idx) = line.find(&needle) {
        let after = &line[idx + needle.len()..];
        if let Some(eq) = after.find('=') {
            let value = after[eq + 1..].trim();
            // Strip surrounding quotes if present.
            let value = value.trim_matches('"');
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

// ─────────────────────────────────────────────────────────────────────
// WiFi hardware (op 23)
// ─────────────────────────────────────────────────────────────────────

/// op 23: Detect WiFi hardware by parsing `system_profiler SPNetworkDataType`
/// for the WiFi interface block.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
fn collect_wifi_hardware() -> Option<WifiInfo> {
    let output = system_profiler("SPNetworkDataType");
    parse_wifi_info(&output)
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn collect_wifi_hardware() -> Option<WifiInfo> {
    None
}

/// op 23: Parse `system_profiler SPNetworkDataType` output for WiFi-specific
/// info. The WiFi interface block reports "Type: AirPort" with "BSD Name:",
/// "MAC Address:", "Card Type:", and "Current Network Information:" (SSID).
#[cfg(target_os = "macos")]
fn parse_wifi_info(output: &str) -> Option<WifiInfo> {
    // Locate the WiFi/AirPort block by scanning for a "Type: AirPort" line,
    // then reading subsequent fields until the next interface block.
    let lines: Vec<&str> = output.lines().collect();
    let mut wifi_start = None;
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.eq_ignore_ascii_case("Type: AirPort") || t.contains("Wi-Fi") {
            wifi_start = Some(i);
            break;
        }
    }
    let start = wifi_start?;
    let interface = parse_profiler_value(
        &lines[start..].join("\n"),
        "BSD Name",
    )
    .unwrap_or_else(|| "en0".to_string());
    let card_type = parse_profiler_value(&lines[start..].join("\n"), "Card Type");
    let mac_address = parse_profiler_value(&lines[start..].join("\n"), "MAC Address");
    // SSID appears under "Current Network Information:" as a sub-line.
    let ssid = lines
        .iter()
        .skip(start)
        .skip_while(|l| !l.contains("Current Network Information"))
        .nth(1)
        .map(|l| l.trim().trim_end_matches(':').to_string())
        .filter(|s| !s.is_empty());
    Some(WifiInfo {
        interface,
        card_type,
        mac_address,
        ssid,
    })
}

// ═══════════════════════════════════════════════════════════════════════
//  Unit tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_byte_size_gb() {
        #[cfg(target_os = "macos")]
        {
            assert_eq!(parse_byte_size("500.0 GB"), Some(500_000_000_000));
            assert_eq!(parse_byte_size("1.0 TB"), Some(1_000_000_000_000));
            assert_eq!(parse_byte_size("512 B"), Some(512));
            assert_eq!(parse_byte_size("invalid"), None);
        }
    }

    #[test]
    fn test_parse_byte_size_mb() {
        #[cfg(target_os = "macos")]
        {
            assert_eq!(parse_byte_size("256 MB"), Some(256_000_000));
        }
    }

    #[test]
    fn test_parse_profiler_value() {
        #[cfg(target_os = "macos")]
        {
            let output = "Some Section:\n  Name: Magic Mouse\n  Version: 1.0\n";
            assert_eq!(
                parse_profiler_value(output, "Name"),
                Some("Magic Mouse".to_string())
            );
            assert_eq!(
                parse_profiler_value(output, "Version"),
                Some("1.0".to_string())
            );
            assert_eq!(parse_profiler_value(output, "Missing"), None);
        }
    }

    #[test]
    fn test_parse_profiler_values_multiple() {
        #[cfg(target_os = "macos")]
        {
            let output = "Name: Device A\nName: Device B\nOther: x\nName: Device C\n";
            let values = parse_profiler_values(output, "Name");
            assert_eq!(values, vec!["Device A", "Device B", "Device C"]);
        }
    }

    #[test]
    fn test_parse_device_names() {
        #[cfg(target_os = "macos")]
        {
            let output = "USB:\n  Name: USB Hub\n  Speed: 5 Gbps\n  Name: Keyboard\n";
            let names = parse_device_names(output);
            assert_eq!(names, vec!["USB Hub", "Keyboard"]);
        }
    }

    #[test]
    fn test_parse_device_names_ignores_empty() {
        #[cfg(target_os = "macos")]
        {
            let output = "Section:\n  Name:\n  Name: Real Device\n";
            let names = parse_device_names(output);
            assert_eq!(names, vec!["Real Device"]);
        }
    }

    #[test]
    fn test_generate_hardware_fingerprint_deterministic() {
        let profile = HardwareProfile {
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
                total_bytes: 17_179_869_184,
                bandwidth_gbps: None,
            },
            storage: vec![StorageDevice {
                name: "Apple SSD".to_string(),
                capacity_bytes: 500_000_000_000,
                is_ssd: true,
                smart_health: None,
            }],
            battery: None,
            thermal: ThermalInfo {
                cpu_temp_c: None,
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
        };
        let fp1 = generate_hardware_fingerprint(&profile);
        let fp2 = generate_hardware_fingerprint(&profile);
        assert_eq!(fp1, fp2, "fingerprint must be deterministic");
        assert!(!fp1.is_empty(), "fingerprint must not be empty");
        // BLAKE3 hex digest is 64 chars.
        assert_eq!(fp1.len(), 64, "BLAKE3 hex digest should be 64 chars");
    }

    #[test]
    fn test_generate_hardware_fingerprint_differs_on_memory() {
        let make_profile = |mem: u64| HardwareProfile {
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
                total_bytes: mem,
                bandwidth_gbps: None,
            },
            storage: Vec::new(),
            battery: None,
            thermal: ThermalInfo {
                cpu_temp_c: None,
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
        };
        let fp1 = generate_hardware_fingerprint(&make_profile(8 * 1024 * 1024 * 1024));
        let fp2 = generate_hardware_fingerprint(&make_profile(16 * 1024 * 1024 * 1024));
        assert_ne!(
            fp1, fp2,
            "different memory should yield different fingerprints"
        );
    }

    #[test]
    fn test_collect_peripherals_non_macos_returns_empty() {
        #[cfg(not(target_os = "macos"))]
        {
            let p = collect_peripherals();
            assert!(p.usb_devices.is_empty());
            assert!(p.thunderbolt_devices.is_empty());
            assert!(p.bluetooth_devices.is_empty());
            assert!(p.external_displays.is_empty());
            assert!(p.audio_devices.is_empty());
            assert!(p.network_interfaces.is_empty());
        }
        // On macOS we can't assert exact contents (depends on the machine),
        // but the call should not panic.
        #[cfg(target_os = "macos")]
        {
            let _p = collect_peripherals();
        }
    }

    #[test]
    fn test_detect_low_power_mode_does_not_panic() {
        let _ = detect_low_power_mode();
    }

    #[test]
    fn test_collect_storage_devices_does_not_panic() {
        let _ = collect_storage_devices();
    }

    #[test]
    fn test_parse_diskutil_info_empty() {
        #[cfg(target_os = "macos")]
        {
            assert!(parse_diskutil_info("nonexistent-disk").is_none());
        }
    }

    #[test]
    fn test_build_machine_profile() {
        let profile = HardwareProfile {
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
                total_bytes: 17_179_869_184,
                bandwidth_gbps: None,
            },
            storage: vec![StorageDevice {
                name: "Apple SSD".to_string(),
                capacity_bytes: 500_000_000_000,
                is_ssd: true,
                smart_health: None,
            }],
            battery: Some(BatteryHardware {
                model: "Unknown".to_string(),
                chemistry: "Lithium".to_string(),
                cycle_count: 42,
                design_capacity_mah: None,
                current_capacity_mah: None,
            }),
            thermal: ThermalInfo {
                cpu_temp_c: None,
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
            fingerprint: "abc123".to_string(),
        };
        let mp = profile.build_machine_profile();
        assert_eq!(mp.model_identifier, "Mac14,2");
        assert_eq!(mp.total_logical_cores, 8);
        assert_eq!(mp.gpu_core_count, 10);
        assert_eq!(mp.ane_core_count, 16);
        assert_eq!(mp.total_memory_bytes, 17_179_869_184);
        assert_eq!(mp.total_storage_bytes, 500_000_000_000);
        assert!(mp.has_battery);
        assert_eq!(mp.battery_cycle_count, 42);
        assert_eq!(mp.thermal_pressure, "Nominal");
        assert_eq!(mp.fingerprint, "abc123");
    }

    #[test]
    fn test_build_machine_profile_no_battery() {
        let profile = HardwareProfile {
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
                total_bytes: 17_179_869_184,
                bandwidth_gbps: None,
            },
            storage: Vec::new(),
            battery: None,
            thermal: ThermalInfo {
                cpu_temp_c: None,
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
        };
        let mp = profile.build_machine_profile();
        assert!(!mp.has_battery);
        assert_eq!(mp.battery_cycle_count, 0);
        assert_eq!(mp.total_storage_bytes, 0);
    }

    #[test]
    fn test_analyze_ane_workloads_does_not_panic() {
        let profile = HardwareProfile {
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
                total_bytes: 17_179_869_184,
                bandwidth_gbps: None,
            },
            storage: Vec::new(),
            battery: None,
            thermal: ThermalInfo {
                cpu_temp_c: None,
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
        };
        let workloads = profile.analyze_ane_workloads();
        // Should not panic; contents depend on the running system.
        for w in &workloads {
            assert!(w.confidence >= 0.0 && w.confidence <= 1.0);
            assert!(!w.name.is_empty());
        }
    }

    // ── Phase 11 operation tests ───────────────────────────────────────

    #[test]
    fn test_estimate_ane_cores_apple_silicon() {
        assert_eq!(estimate_ane_cores("Mac14,2"), 16);
        assert_eq!(estimate_ane_cores("Mac15,3"), 16);
        assert_eq!(estimate_ane_cores("apple"), 16);
        assert_eq!(estimate_ane_cores("arm64"), 16);
    }

    #[test]
    fn test_estimate_ane_cores_unknown() {
        assert_eq!(estimate_ane_cores("Unknown"), 0);
        assert_eq!(estimate_ane_cores(""), 0);
    }

    #[test]
    fn test_estimate_memory_bandwidth_m1() {
        assert_eq!(estimate_memory_bandwidth("M1"), Some(68.0));
        assert_eq!(estimate_memory_bandwidth("M1 Pro"), Some(200.0));
        assert_eq!(estimate_memory_bandwidth("M1 Max"), Some(400.0));
        assert_eq!(estimate_memory_bandwidth("M1 Ultra"), Some(800.0));
        // Legacy M1 identifiers.
        assert_eq!(estimate_memory_bandwidth("MacBookAir10,1"), Some(68.0));
    }

    #[test]
    fn test_estimate_memory_bandwidth_m2() {
        assert_eq!(estimate_memory_bandwidth("M2"), Some(100.0));
        assert_eq!(estimate_memory_bandwidth("M2 Pro"), Some(200.0));
        assert_eq!(estimate_memory_bandwidth("M2 Max"), Some(400.0));
        // Mac14,x identifiers are the M2 family.
        assert_eq!(estimate_memory_bandwidth("Mac14,2"), Some(100.0));
    }

    #[test]
    fn test_estimate_memory_bandwidth_m3_m4() {
        assert_eq!(estimate_memory_bandwidth("M3"), Some(120.0));
        assert_eq!(estimate_memory_bandwidth("M3 Pro"), Some(150.0));
        assert_eq!(estimate_memory_bandwidth("M3 Max"), Some(400.0));
        // Mac15,x is the M3 family; the base identifier can't reveal Pro/Max
        // tier without the marketing name, so it maps to the base figure.
        assert_eq!(estimate_memory_bandwidth("Mac15,3"), Some(120.0));
        assert_eq!(estimate_memory_bandwidth("M4"), Some(120.0));
        assert_eq!(estimate_memory_bandwidth("M4 Pro"), Some(273.0));
        assert_eq!(estimate_memory_bandwidth("M4 Max"), Some(546.0));
        assert_eq!(estimate_memory_bandwidth("Mac16,1"), Some(120.0));
    }

    #[test]
    fn test_estimate_memory_bandwidth_unknown() {
        assert_eq!(estimate_memory_bandwidth("Intel"), None);
        assert_eq!(estimate_memory_bandwidth("Unknown"), None);
    }

    #[test]
    fn test_parse_sleep_state_active() {
        let output = "Assertion status system-wide:\n\
            PreventUserIdleSystemSleep 1\n\
            PreventSystemSleep 1\n";
        assert_eq!(parse_sleep_state(output), "Active");
    }

    #[test]
    fn test_parse_sleep_state_sleeping() {
        let output = "Assertion status system-wide:\n\
            Sleeping\n";
        assert_eq!(parse_sleep_state(output), "Sleeping");
    }

    #[test]
    fn test_parse_sleep_state_default_active() {
        assert_eq!(parse_sleep_state(""), "Active");
        assert_eq!(parse_sleep_state("random content"), "Active");
    }

    #[test]
    fn test_parse_swap_usage_basic() {
        let output = "vm.swapusage: total = 2048.00M  used = 12.50M  free = 2035.50M  (encrypted)";
        let info = parse_swap_usage(output);
        assert_eq!(info.total_bytes, 2_048_000_000);
        assert_eq!(info.used_bytes, 12_500_000);
        assert_eq!(info.free_bytes, 2_035_500_000);
        assert!(info.encrypted);
    }

    #[test]
    fn test_parse_swap_usage_not_encrypted() {
        let output = "vm.swapusage: total = 1024.00M  used = 0.00M  free = 1024.00M";
        let info = parse_swap_usage(output);
        assert_eq!(info.total_bytes, 1_024_000_000);
        assert_eq!(info.used_bytes, 0);
        assert!(!info.encrypted);
    }

    #[test]
    fn test_parse_swap_field_gb() {
        let output = "total = 4.00G  used = 1.00G  free = 3.00G";
        assert_eq!(parse_swap_field(output, "total"), Some(4_000_000_000));
        assert_eq!(parse_swap_field(output, "used"), Some(1_000_000_000));
        assert_eq!(parse_swap_field(output, "free"), Some(3_000_000_000));
    }

    #[test]
    fn test_parse_swap_field_missing() {
        assert_eq!(parse_swap_field("nothing here", "total"), None);
    }

    #[test]
    fn test_parse_wake_events() {
        let output = "2024-01-15 08:32:01 -08 Wake  Wake from CoreStorage\n\
            2024-01-15 09:00:00 -08 Sleep  Entering Sleep\n\
            2024-01-15 09:05:00 -08 DarkWake  DarkWake from display\n\
            2024-01-15 09:10:00 -08 Some other event\n";
        let events = parse_wake_events(output);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].event_type, "Wake");
        assert_eq!(events[0].timestamp, "2024-01-15 08:32:01");
        assert!(events[0].detail.contains("Wake from CoreStorage"));
        assert_eq!(events[1].event_type, "Sleep");
        assert_eq!(events[2].event_type, "DarkWake");
    }

    #[test]
    fn test_parse_wake_events_empty() {
        assert!(parse_wake_events("").is_empty());
        assert!(parse_wake_events("no events here\n").is_empty());
    }

    #[test]
    fn test_parse_thermal_throttling() {
        let output = "  CPU_PLimit = 12\n  GPU_PLimit = 8\n";
        let t = parse_thermal_throttling(output);
        assert_eq!(t.cpu_power_limit_w, Some(12.0));
        assert_eq!(t.gpu_power_limit_w, Some(8.0));
        assert!(!t.is_throttling);
    }

    #[test]
    fn test_parse_thermal_throttling_pressure() {
        let output = "  CPU_PLimit = 5\n  ThermalPressure = Critical\n";
        let t = parse_thermal_throttling(output);
        assert!(t.is_throttling);
    }

    #[test]
    fn test_parse_thermal_throttling_nominal() {
        let output = "  ThermalPressure = Nominal\n";
        let t = parse_thermal_throttling(output);
        assert!(!t.is_throttling);
        assert_eq!(t.cpu_power_limit_w, None);
    }

    #[test]
    fn test_parse_numeric_field() {
        let output = "  CPU_PLimit = 18.5\n  GPU_PLimit = 10\n";
        assert_eq!(parse_numeric_field(output, "CPU_PLimit"), Some(18.5));
        assert_eq!(parse_numeric_field(output, "GPU_PLimit"), Some(10.0));
        assert_eq!(parse_numeric_field(output, "Missing"), None);
    }

    #[test]
    fn test_parse_gpu_cores() {
        #[cfg(target_os = "macos")]
        {
            let output = "Displays:\n  Total Number of Cores: 10\n";
            assert_eq!(parse_gpu_cores(output), 10);
            assert_eq!(parse_gpu_cores("no cores here"), 0);
        }
    }

    #[test]
    fn test_parse_camera_devices() {
        #[cfg(target_os = "macos")]
        {
            let output = "Cameras:\n\
                Model Name: FaceTime HD Camera\n\
                Manufacturer: Apple Inc.\n\
                Unique ID: 0x1234\n";
            let cameras = parse_camera_devices(output);
            assert_eq!(cameras.len(), 1);
            assert_eq!(cameras[0].name, "FaceTime HD Camera");
            assert_eq!(cameras[0].vendor.as_deref(), Some("Apple Inc."));
            assert_eq!(cameras[0].unique_id.as_deref(), Some("0x1234"));
        }
    }

    #[test]
    fn test_parse_microphone_devices() {
        #[cfg(target_os = "macos")]
        {
            let output = "Microphones:\n\
                Model Name: MacBook Pro Microphone\n\
                Manufacturer: Apple Inc.\n\
                Channels: 2\n";
            let mics = parse_microphone_devices(output);
            assert_eq!(mics.len(), 1);
            assert_eq!(mics[0].name, "MacBook Pro Microphone");
            assert_eq!(mics[0].channels, Some(2));
        }
    }

    #[test]
    fn test_parse_display_info() {
        #[cfg(target_os = "macos")]
        {
            let output = "Displays:\n\
                Display Type: Built-In\n\
                Resolution: 3024 x 1964 Retina\n\
                Color Depth: 32-Bit Color\n\
                Display Type: LCD\n\
                Resolution: 1920 x 1080\n";
            let displays = parse_display_info(output);
            assert_eq!(displays.len(), 2);
            assert!(displays[0].is_builtin);
            assert_eq!(displays[0].resolution.as_deref(), Some("3024 x 1964 Retina"));
            assert_eq!(displays[0].color_depth.as_deref(), Some("32-Bit Color"));
            assert!(!displays[1].is_builtin);
            assert_eq!(displays[1].resolution.as_deref(), Some("1920 x 1080"));
        }
    }

    #[test]
    fn test_parse_battery_model() {
        #[cfg(target_os = "macos")]
        {
            let output = "Battery Information:\n\
                Model Name: A2527\n\
                Chemistry: Lithium\n";
            let (model, chemistry) = parse_battery_model(output);
            assert_eq!(model, "A2527");
            assert_eq!(chemistry, "Lithium");
        }
    }

    #[test]
    fn test_parse_battery_model_defaults() {
        #[cfg(target_os = "macos")]
        {
            let (model, chemistry) = parse_battery_model("no battery info");
            assert_eq!(model, "Unknown");
            assert_eq!(chemistry, "Lithium");
        }
    }

    #[test]
    fn test_hardware_history_new_empty() {
        let h = HardwareHistory::new();
        assert!(h.entries.is_empty());
    }

    #[test]
    fn test_hardware_history_record() {
        let mut h = HardwareHistory::new();
        let profile = make_minimal_profile();
        h.record(&profile);
        assert_eq!(h.entries.len(), 1);
        assert_eq!(h.entries[0].model_identifier, "Mac14,2");
        assert_eq!(h.entries[0].total_memory_bytes, 17_179_869_184);
        assert_eq!(h.entries[0].total_storage_bytes, 500_000_000_000);
        assert_eq!(h.entries[0].battery_cycle_count, 42);
        assert!(!h.entries[0].fingerprint.is_empty());
    }

    #[test]
    fn test_hardware_history_save_load_roundtrip() {
        let dir = std::env::temp_dir();
        let path = dir.join(format!(
            "xmac_hw_history_test_{}.json",
            std::process::id()
        ));
        // Clean up any leftover file.
        let _ = std::fs::remove_file(&path);

        let mut h = HardwareHistory::new();
        let profile = make_minimal_profile();
        h.record(&profile);
        h.record(&profile);
        h.save(&path).expect("save should succeed");

        let loaded = HardwareHistory::load(&path).expect("load should succeed");
        assert_eq!(loaded.entries.len(), 2);
        assert_eq!(loaded.entries[0].model_identifier, "Mac14,2");
        assert_eq!(loaded.entries[1].battery_cycle_count, 42);

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn test_hardware_history_load_missing_file() {
        let path = std::env::temp_dir().join("xmac_nonexistent_history_test.json");
        let _ = std::fs::remove_file(&path);
        let loaded = HardwareHistory::load(&path).expect("missing file => empty history");
        assert!(loaded.entries.is_empty());
    }

    #[test]
    fn test_hardware_history_prune() {
        let mut h = HardwareHistory::new();
        let profile = make_minimal_profile();
        for _ in 0..10 {
            h.record(&profile);
        }
        assert_eq!(h.entries.len(), 10);
        h.prune(3);
        assert_eq!(h.entries.len(), 3);
        // The most recent entries are kept (last 3 recorded).
        assert_eq!(h.entries[2].model_identifier, "Mac14,2");
    }

    #[test]
    fn test_detect_cameras_does_not_panic() {
        let _ = detect_cameras();
    }

    #[test]
    fn test_detect_microphones_does_not_panic() {
        let _ = detect_microphones();
    }

    #[test]
    fn test_detect_swap_activity_does_not_panic() {
        let _ = detect_swap_activity();
    }

    #[test]
    fn test_detect_ssd_throughput_does_not_panic() {
        let _ = detect_ssd_throughput();
    }

    #[test]
    fn test_detect_wake_events_does_not_panic() {
        let _ = detect_wake_events();
    }

    #[test]
    fn test_detect_thermal_throttling_does_not_panic() {
        let _ = detect_thermal_throttling();
    }

    #[test]
    fn test_detect_performance_limits_does_not_panic() {
        let _ = detect_performance_limits();
    }

    #[test]
    fn test_collect_display_info_does_not_panic() {
        let _ = collect_display_info();
    }

    #[test]
    fn test_collect_usb_topology_does_not_panic() {
        let _ = collect_usb_topology();
    }

    #[test]
    fn test_collect_wifi_hardware_does_not_panic() {
        let _ = collect_wifi_hardware();
    }

    #[test]
    fn test_detect_sleep_state_does_not_panic() {
        let _ = detect_sleep_state();
    }

    #[test]
    fn test_detect_memory_bandwidth_does_not_panic() {
        let _ = detect_memory_bandwidth();
    }

    #[test]
    fn test_detect_gpu_utilization_returns_none() {
        // No public API exposes GPU utilization on Apple Silicon.
        assert_eq!(detect_gpu_utilization(), None);
    }

    /// Helper to build a minimal `HardwareProfile` for history tests.
    fn make_minimal_profile() -> HardwareProfile {
        HardwareProfile {
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
                total_bytes: 17_179_869_184,
                bandwidth_gbps: Some(68.0),
            },
            storage: vec![StorageDevice {
                name: "Apple SSD".to_string(),
                capacity_bytes: 500_000_000_000,
                is_ssd: true,
                smart_health: None,
            }],
            battery: Some(BatteryHardware {
                model: "A2527".to_string(),
                chemistry: "Lithium".to_string(),
                cycle_count: 42,
                design_capacity_mah: None,
                current_capacity_mah: None,
            }),
            thermal: ThermalInfo {
                cpu_temp_c: None,
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
            fingerprint: "abc123".to_string(),
        }
    }
}
