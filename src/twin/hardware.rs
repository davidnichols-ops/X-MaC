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
            peripherals: collect_peripherals(),
            power_state: PowerState {
                is_on_battery: !snapshot.battery.is_plugged,
                is_charging: snapshot.battery.is_charging,
                sleep_state: "Active".to_string(),
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
    let smart_health = smart_status.map(|status| SmartHealth {
        status,
        temperature_celsius: None,
        power_on_hours: None,
    });

    Some(StorageDevice {
        name,
        capacity_bytes,
        is_ssd,
        smart_health,
    })
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

fn collect_battery_hardware(
    battery: &crate::intelligence::system_awareness::BatteryDimension,
) -> Option<BatteryHardware> {
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
}
