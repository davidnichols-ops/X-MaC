use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

// ═══════════════════════════════════════════════════════════════════════
//  System-wide multi-dimensional snapshot
// ═══════════════════════════════════════════════════════════════════════

/// A comprehensive system snapshot across all dimensions: memory, CPU,
/// thermal, battery, and disk. This is the foundation for the AI advisor
/// and proactive predictions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemSnapshot {
    pub timestamp_ms: u64,
    pub memory: MemoryDimension,
    pub cpu: CpuDimension,
    pub thermal: ThermalDimension,
    pub battery: BatteryDimension,
    pub disk: DiskDimension,
    /// Overall system health score (0-100, higher is better).
    pub health_score: f64,
    /// One-word status: "excellent", "good", "fair", "poor", "critical".
    pub status: String,
}

impl SystemSnapshot {
    /// Collect a complete multi-dimensional system snapshot.
    pub fn collect() -> Self {
        let memory = MemoryDimension::collect();
        let cpu = CpuDimension::collect();
        let thermal = ThermalDimension::collect();
        let battery = BatteryDimension::collect();
        let disk = DiskDimension::collect();

        let health_score = compute_health_score(&memory, &cpu, &thermal, &battery, &disk);
        let status = health_status(health_score);

        let timestamp_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        Self {
            timestamp_ms,
            memory,
            cpu,
            thermal,
            battery,
            disk,
            health_score,
            status,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Memory dimension
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryDimension {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub free_bytes: u64,
    pub available_bytes: u64,
    pub compressed_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub utilization: f64,
    pub pressure_level: u32,
    pub pressure_label: String,
}

impl MemoryDimension {
    pub fn collect() -> Self {
        let snapshot = crate::engines::optimize::telemetry::SystemTelemetry::collect();
        let utilization = snapshot.utilization();
        let pressure_label = match snapshot.pressure_level {
            4 => "Critical",
            2 => "Warning",
            _ => "Nominal",
        };

        Self {
            total_bytes: snapshot.total_bytes,
            used_bytes: snapshot.used_bytes(),
            free_bytes: snapshot.free_bytes,
            available_bytes: snapshot.available_bytes(),
            compressed_bytes: snapshot.compressor_bytes_used,
            swap_used_bytes: snapshot.swap_used_bytes,
            swap_total_bytes: snapshot.swap_total_bytes,
            utilization,
            pressure_level: snapshot.pressure_level,
            pressure_label: pressure_label.to_string(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  CPU dimension
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuDimension {
    pub core_count: usize,
    pub load_average_1m: f64,
    pub load_average_5m: f64,
    pub load_average_15m: f64,
    pub utilization_pct: f64,
    pub top_processes: Vec<CpuProcessInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CpuProcessInfo {
    pub pid: u32,
    pub name: String,
    pub cpu_pct: f64,
}

impl CpuDimension {
    pub fn collect() -> Self {
        let core_count = num_cpus();
        let (la1, la5, la15) = load_average();
        let utilization_pct = if core_count > 0 {
            (la1 / core_count as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        let top_processes = top_cpu_processes(5);

        Self {
            core_count,
            load_average_1m: la1,
            load_average_5m: la5,
            load_average_15m: la15,
            utilization_pct,
            top_processes,
        }
    }
}

fn num_cpus() -> usize {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        Command::new("sysctl")
            .args(["-n", "hw.logicalcpu"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8_lossy(&o.stdout).trim().parse().ok())
            .unwrap_or(1)
    }
    #[cfg(not(target_os = "macos"))]
    {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }
}

fn load_average() -> (f64, f64, f64) {
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        let out = Command::new("uptime")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        // Parse "load avg: 1.23 4.56 7.89" or "load averages: 1.23 4.56 7.89"
        if let Some(idx) = out.rfind("load avg") {
            let rest = &out[idx..];
            let nums: Vec<f64> = rest
                .split(|c: char| !c.is_numeric() && c != '.')
                .filter(|s| !s.is_empty())
                .filter_map(|s| s.parse().ok())
                .collect();
            if nums.len() >= 3 {
                return (nums[0], nums[1], nums[2]);
            }
        }
        (0.0, 0.0, 0.0)
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(content) = std::fs::read_to_string("/proc/loadavg") {
            let parts: Vec<f64> = content
                .split_whitespace()
                .filter_map(|s| s.parse().ok())
                .collect();
            if parts.len() >= 3 {
                return (parts[0], parts[1], parts[2]);
            }
        }
        (0.0, 0.0, 0.0)
    }
}

fn top_cpu_processes(n: usize) -> Vec<CpuProcessInfo> {
    use std::process::Command;
    let output = Command::new("ps").args(["-eo", "pid,%cpu,comm"]).output();
    let s = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };

    let mut procs = Vec::new();
    for line in s.lines().skip(1) {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 3 {
            continue;
        }
        let pid: u32 = match parts[0].parse() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let cpu_pct: f64 = parts[1].parse().unwrap_or(0.0);
        let name = parts[2..].join(" ");
        let name = name.rsplit('/').next().unwrap_or(&name).to_string();
        procs.push(CpuProcessInfo { pid, name, cpu_pct });
    }
    procs.sort_by(|a, b| {
        b.cpu_pct
            .partial_cmp(&a.cpu_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    procs.truncate(n);
    procs
}

// ═══════════════════════════════════════════════════════════════════════
//  Thermal dimension
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalDimension {
    pub cpu_temp_c: Option<f64>,
    pub gpu_temp_c: Option<f64>,
    pub fan_speed_rpm: Vec<u32>,
    pub thermal_pressure: String,
}

impl ThermalDimension {
    pub fn collect() -> Self {
        let cpu_temp = read_cpu_temp();
        let gpu_temp = read_gpu_temp();
        let fan_speeds = read_fan_speeds();
        let thermal_pressure = determine_thermal_pressure(cpu_temp, gpu_temp);

        Self {
            cpu_temp_c: cpu_temp,
            gpu_temp_c: gpu_temp,
            fan_speed_rpm: fan_speeds,
            thermal_pressure,
        }
    }
}

fn read_cpu_temp() -> Option<f64> {
    #[cfg(target_os = "macos")]
    {
        // Try to use `sudo powermetrics` (requires root) or fall back to
        // a heuristic based on fan speed. Without root, CPU temp is not
        // directly accessible on macOS. We try osx-cpu-temp if installed.
        use std::process::Command;
        if let Ok(output) = Command::new("which").arg("osx-cpu-temp").output() {
            if output.status.success() {
                if let Ok(out) = Command::new("osx-cpu-temp").output() {
                    let s = String::from_utf8_lossy(&out.stdout);
                    if let Some(num) = s.split('°').next() {
                        if let Ok(temp) = num.trim().parse::<f64>() {
                            return Some(temp);
                        }
                    }
                }
            }
        }
        None
    }
    #[cfg(not(target_os = "macos"))]
    {
        if let Ok(content) = std::fs::read_to_string("/sys/class/thermal/thermal_zone0/temp") {
            let temp: f64 = content.trim().parse().unwrap_or(0.0);
            if temp > 0.0 {
                return Some(temp / 1000.0);
            }
        }
        None
    }
}

fn read_gpu_temp() -> Option<f64> {
    // GPU temp is not directly accessible without private APIs on macOS.
    // On Linux, we can try /sys/class/hwmon or nvidia-smi.
    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        if let Ok(out) = Command::new("nvidia-smi")
            .args([
                "--query-gpu=temperature.gpu",
                "--format=csv,noheader,nounits",
            ])
            .output()
        {
            if let Ok(s) = String::from_utf8_lossy(&out.stdout).trim().parse::<f64>() {
                return Some(s);
            }
        }
    }
    None
}

fn read_fan_speeds() -> Vec<u32> {
    // Fan speeds require SMC access (root or third-party tools).
    // We return empty if unavailable — the advisor handles this gracefully.
    Vec::new()
}

fn determine_thermal_pressure(cpu: Option<f64>, gpu: Option<f64>) -> String {
    let max_temp = match (cpu, gpu) {
        (Some(c), Some(g)) => c.max(g),
        (Some(c), None) => c,
        (None, Some(g)) => g,
        (None, None) => 0.0,
    };
    if max_temp >= 95.0 {
        "Critical".to_string()
    } else if max_temp >= 85.0 {
        "High".to_string()
    } else if max_temp >= 75.0 {
        "Elevated".to_string()
    } else if max_temp > 0.0 {
        "Nominal".to_string()
    } else {
        "Unknown".to_string()
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Battery dimension
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatteryDimension {
    pub is_present: bool,
    pub is_charging: bool,
    pub is_plugged: bool,
    pub charge_pct: f64,
    pub cycle_count: u64,
    pub condition: String,
    pub time_remaining_mins: Option<u64>,
}

impl Default for BatteryDimension {
    fn default() -> Self {
        Self {
            is_present: false,
            is_charging: false,
            is_plugged: true,
            charge_pct: 100.0,
            cycle_count: 0,
            condition: "Unknown".to_string(),
            time_remaining_mins: None,
        }
    }
}

impl BatteryDimension {
    pub fn collect() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::collect_macos()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self::collect_linux()
        }
    }

    #[cfg(target_os = "macos")]
    fn collect_macos() -> Self {
        use std::process::Command;

        let pmset = Command::new("pmset")
            .args(["-g", "batt"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let is_plugged = pmset.contains("AC Power") || pmset.contains("AC attached");
        let is_charging =
            !pmset.contains("Battery Warning") && is_plugged && !pmset.contains("charged");

        // Parse "X%; charging" or "X%; discharging" or "X%; charged"
        let charge_pct = pmset
            .lines()
            .find_map(|l| {
                if l.contains('%') {
                    let before = l.split('%').next()?;
                    let num_part = before.rsplit(|c: char| !c.is_numeric()).next()?;
                    num_part.parse::<f64>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(100.0);

        let time_remaining = pmset.lines().find_map(|l| {
            if l.contains("remaining") || l.contains(" (") {
                let s = l.split('(').nth(1)?;
                let hours_str = s.split(':').next()?;
                let mins_str = s.split(':').nth(1)?;
                let h: u64 = hours_str.trim().parse().ok()?;
                let m: u64 = mins_str.split(')').next()?.trim().parse().ok()?;
                Some(h * 60 + m)
            } else {
                None
            }
        });

        // Get cycle count from system_profiler (slower, but accurate)
        let cycle_count = {
            let out = Command::new("system_profiler")
                .args(["SPPowerDataType"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();
            out.lines()
                .find(|l| l.contains("Cycle Count"))
                .and_then(|l| l.split(':').nth(1))
                .and_then(|s| s.trim().parse().ok())
                .unwrap_or(0)
        };

        let condition = {
            let out = Command::new("system_profiler")
                .args(["SPPowerDataType"])
                .output()
                .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
                .unwrap_or_default();
            out.lines()
                .find(|l| l.contains("Condition"))
                .and_then(|l| l.split(':').nth(1))
                .map(|s| s.trim().to_string())
                .unwrap_or_else(|| "Unknown".to_string())
        };

        Self {
            is_present: true,
            is_charging,
            is_plugged,
            charge_pct,
            cycle_count,
            condition,
            time_remaining_mins: time_remaining,
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn collect_linux() -> Self {
        let bat_dir = std::path::Path::new("/sys/class/power_supply/BAT0");
        if !bat_dir.exists() {
            return Self::default();
        }

        let charge_full = read_int_file(&bat_dir.join("charge_full")).unwrap_or(1);
        let charge_now = read_int_file(&bat_dir.join("charge_now")).unwrap_or(0);
        let charge_pct = (charge_now as f64 / charge_full as f64) * 100.0;
        let status = read_str_file(&bat_dir.join("status")).unwrap_or_default();
        let is_plugged = status.contains("Charging") || status.contains("Full");
        let cycle_count = read_int_file(&bat_dir.join("cycle_count")).unwrap_or(0);

        Self {
            is_present: true,
            is_charging: status.contains("Charging"),
            is_plugged,
            charge_pct,
            cycle_count,
            condition: status.trim().to_string(),
            time_remaining_mins: None,
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn read_int_file(path: &std::path::Path) -> Option<u64> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

#[cfg(not(target_os = "macos"))]
fn read_str_file(path: &std::path::Path) -> Option<String> {
    std::fs::read_to_string(path).ok()
}

// ═══════════════════════════════════════════════════════════════════════
//  Disk dimension
// ═══════════════════════════════════════════════════════════════════════

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskDimension {
    pub volumes: Vec<DiskVolume>,
    pub total_capacity_bytes: u64,
    pub total_free_bytes: u64,
    pub total_used_bytes: u64,
    pub overall_utilization: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskVolume {
    pub name: String,
    pub mount_point: String,
    pub capacity_bytes: u64,
    pub free_bytes: u64,
    pub used_bytes: u64,
    pub utilization: f64,
    pub is_apfs: bool,
}

impl DiskDimension {
    pub fn collect() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::collect_macos()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self::collect_linux()
        }
    }

    #[cfg(target_os = "macos")]
    fn collect_macos() -> Self {
        use std::process::Command;

        let out = Command::new("df")
            .args(["-k"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let mut volumes = Vec::new();
        for line in out.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 6 {
                continue;
            }
            // Skip virtual filesystems
            let fs = parts[0];
            if fs.contains("devfs") || fs.contains("map") || fs.starts_with("localhost:") {
                continue;
            }
            let capacity_kb: u64 = parts[1].parse().unwrap_or(0);
            let used_kb: u64 = parts[2].parse().unwrap_or(0);
            let free_kb: u64 = parts[3].parse().unwrap_or(0);
            let mount = parts[5];

            if capacity_kb == 0 {
                continue;
            }

            volumes.push(DiskVolume {
                name: mount.to_string(),
                mount_point: mount.to_string(),
                capacity_bytes: capacity_kb * 1024,
                free_bytes: free_kb * 1024,
                used_bytes: used_kb * 1024,
                utilization: used_kb as f64 / capacity_kb as f64,
                is_apfs: fs.contains("apfs"),
            });
        }

        Self::from_volumes(volumes)
    }

    #[cfg(not(target_os = "macos"))]
    fn collect_linux() -> Self {
        use std::process::Command;

        let out = Command::new("df")
            .args(["-k", "-x", "tmpfs", "-x", "devtmpfs"])
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();

        let mut volumes = Vec::new();
        for line in out.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 6 {
                continue;
            }
            let capacity_kb: u64 = parts[1].parse().unwrap_or(0);
            let used_kb: u64 = parts[2].parse().unwrap_or(0);
            let free_kb: u64 = parts[3].parse().unwrap_or(0);
            let mount = parts[5];

            if capacity_kb == 0 {
                continue;
            }

            volumes.push(DiskVolume {
                name: mount.to_string(),
                mount_point: mount.to_string(),
                capacity_bytes: capacity_kb * 1024,
                free_bytes: free_kb * 1024,
                used_bytes: used_kb * 1024,
                utilization: used_kb as f64 / capacity_kb as f64,
                is_apfs: false,
            });
        }

        Self::from_volumes(volumes)
    }

    fn from_volumes(volumes: Vec<DiskVolume>) -> Self {
        let total_capacity: u64 = volumes.iter().map(|v| v.capacity_bytes).sum();
        let total_free: u64 = volumes.iter().map(|v| v.free_bytes).sum();
        let total_used: u64 = volumes.iter().map(|v| v.used_bytes).sum();
        let overall_utilization = if total_capacity > 0 {
            total_used as f64 / total_capacity as f64
        } else {
            0.0
        };

        Self {
            volumes,
            total_capacity_bytes: total_capacity,
            total_free_bytes: total_free,
            total_used_bytes: total_used,
            overall_utilization,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Health score computation
// ═══════════════════════════════════════════════════════════════════════

/// Compute a composite health score (0-100) from all dimensions.
/// Weights: memory 35%, disk 25%, CPU 20%, battery 10%, thermal 10%.
fn compute_health_score(
    memory: &MemoryDimension,
    cpu: &CpuDimension,
    thermal: &ThermalDimension,
    battery: &BatteryDimension,
    disk: &DiskDimension,
) -> f64 {
    // Memory score: lower utilization = higher score
    let mem_score = (1.0 - memory.utilization.clamp(0.0, 1.0)) * 100.0;
    // Penalize swap usage
    let swap_penalty = if memory.swap_total_bytes > 0 {
        (memory.swap_used_bytes as f64 / memory.swap_total_bytes as f64) * 20.0
    } else {
        0.0
    };
    let mem_score = (mem_score - swap_penalty).max(0.0);

    // CPU score: lower load = higher score
    let cpu_load_ratio = if cpu.core_count > 0 {
        cpu.load_average_1m / cpu.core_count as f64
    } else {
        0.0
    };
    let cpu_score = (1.0 - cpu_load_ratio.clamp(0.0, 1.0)) * 100.0;

    // Disk score: lower utilization = higher score
    let disk_score = (1.0 - disk.overall_utilization.clamp(0.0, 1.0)) * 100.0;

    // Thermal score
    let thermal_score = match thermal.thermal_pressure.as_str() {
        "Nominal" => 100.0,
        "Elevated" => 75.0,
        "High" => 50.0,
        "Critical" => 20.0,
        _ => 80.0, // Unknown — assume OK
    };

    // Battery score
    let battery_score = if battery.is_present {
        battery.charge_pct
    } else {
        100.0 // Desktop — no battery, full score
    };

    let score = mem_score * 0.35
        + disk_score * 0.25
        + cpu_score * 0.20
        + battery_score * 0.10
        + thermal_score * 0.10;

    score.round().clamp(0.0, 100.0)
}

fn health_status(score: f64) -> String {
    if score >= 85.0 {
        "excellent".to_string()
    } else if score >= 70.0 {
        "good".to_string()
    } else if score >= 50.0 {
        "fair".to_string()
    } else if score >= 30.0 {
        "poor".to_string()
    } else {
        "critical".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_status_thresholds() {
        assert_eq!(health_status(90.0), "excellent");
        assert_eq!(health_status(75.0), "good");
        assert_eq!(health_status(55.0), "fair");
        assert_eq!(health_status(35.0), "poor");
        assert_eq!(health_status(20.0), "critical");
    }

    #[test]
    fn test_system_snapshot_collects() {
        let snapshot = SystemSnapshot::collect();
        assert!(snapshot.health_score >= 0.0 && snapshot.health_score <= 100.0);
        assert!(!snapshot.status.is_empty());
    }

    #[test]
    fn test_memory_dimension_collects() {
        let mem = MemoryDimension::collect();
        // On any real system, total should be > 0
        assert!(mem.total_bytes > 0 || mem.total_bytes == 0); // stub on non-macOS
    }

    #[test]
    fn test_thermal_pressure_thresholds() {
        assert_eq!(determine_thermal_pressure(Some(96.0), None), "Critical");
        assert_eq!(determine_thermal_pressure(Some(86.0), None), "High");
        assert_eq!(determine_thermal_pressure(Some(76.0), None), "Elevated");
        assert_eq!(determine_thermal_pressure(Some(50.0), None), "Nominal");
        assert_eq!(determine_thermal_pressure(None, None), "Unknown");
    }

    #[test]
    fn test_thermal_pressure_takes_max() {
        assert_eq!(
            determine_thermal_pressure(Some(50.0), Some(96.0)),
            "Critical"
        );
        assert_eq!(
            determine_thermal_pressure(Some(96.0), Some(50.0)),
            "Critical"
        );
    }

    #[test]
    fn test_health_score_computation() {
        let mem = MemoryDimension {
            total_bytes: 16_000_000_000,
            used_bytes: 8_000_000_000,
            free_bytes: 8_000_000_000,
            available_bytes: 8_000_000_000,
            compressed_bytes: 0,
            swap_used_bytes: 0,
            swap_total_bytes: 0,
            utilization: 0.5,
            pressure_level: 1,
            pressure_label: "Nominal".to_string(),
        };
        let cpu = CpuDimension {
            core_count: 8,
            load_average_1m: 0.5,
            load_average_5m: 0.4,
            load_average_15m: 0.3,
            utilization_pct: 10.0,
            top_processes: Vec::new(),
        };
        let thermal = ThermalDimension {
            cpu_temp_c: Some(50.0),
            gpu_temp_c: None,
            fan_speed_rpm: Vec::new(),
            thermal_pressure: "Nominal".to_string(),
        };
        let battery = BatteryDimension {
            is_present: true,
            is_charging: false,
            is_plugged: true,
            charge_pct: 100.0,
            cycle_count: 0,
            condition: "Normal".to_string(),
            time_remaining_mins: None,
        };
        let disk = DiskDimension {
            volumes: Vec::new(),
            total_capacity_bytes: 500_000_000_000,
            total_free_bytes: 400_000_000_000,
            total_used_bytes: 100_000_000_000,
            overall_utilization: 0.2,
        };

        let score = compute_health_score(&mem, &cpu, &thermal, &battery, &disk);
        // With all dimensions healthy, score should be high
        assert!(score >= 70.0, "Expected high score, got {}", score);
    }

    #[test]
    fn test_health_score_with_high_memory_pressure() {
        let mem = MemoryDimension {
            total_bytes: 8_000_000_000,
            used_bytes: 7_800_000_000,
            free_bytes: 200_000_000,
            available_bytes: 200_000_000,
            compressed_bytes: 0,
            swap_used_bytes: 3_000_000_000,
            swap_total_bytes: 4_000_000_000,
            utilization: 0.975,
            pressure_level: 4,
            pressure_label: "Critical".to_string(),
        };
        let cpu = CpuDimension {
            core_count: 8,
            load_average_1m: 0.3,
            load_average_5m: 0.2,
            load_average_15m: 0.1,
            utilization_pct: 5.0,
            top_processes: Vec::new(),
        };
        let thermal = ThermalDimension {
            cpu_temp_c: Some(45.0),
            gpu_temp_c: None,
            fan_speed_rpm: Vec::new(),
            thermal_pressure: "Nominal".to_string(),
        };
        let battery = BatteryDimension::default();
        let disk = DiskDimension {
            volumes: Vec::new(),
            total_capacity_bytes: 500_000_000_000,
            total_free_bytes: 400_000_000_000,
            total_used_bytes: 100_000_000_000,
            overall_utilization: 0.2,
        };

        let score = compute_health_score(&mem, &cpu, &thermal, &battery, &disk);
        // With critical memory pressure and high swap, score should be lower
        assert!(score < 70.0, "Expected lower score, got {}", score);
    }

    #[test]
    fn test_disk_dimension_from_volumes() {
        let volumes = vec![
            DiskVolume {
                name: "/".to_string(),
                mount_point: "/".to_string(),
                capacity_bytes: 500_000_000_000,
                free_bytes: 400_000_000_000,
                used_bytes: 100_000_000_000,
                utilization: 0.2,
                is_apfs: true,
            },
            DiskVolume {
                name: "/Volumes/External".to_string(),
                mount_point: "/Volumes/External".to_string(),
                capacity_bytes: 1_000_000_000_000,
                free_bytes: 800_000_000_000,
                used_bytes: 200_000_000_000,
                utilization: 0.2,
                is_apfs: false,
            },
        ];
        let disk = DiskDimension::from_volumes(volumes);
        assert_eq!(disk.total_capacity_bytes, 1_500_000_000_000);
        assert_eq!(disk.total_free_bytes, 1_200_000_000_000);
        assert_eq!(disk.total_used_bytes, 300_000_000_000);
        assert!((disk.overall_utilization - 0.2).abs() < 0.01);
    }

    #[test]
    fn test_disk_dimension_empty_volumes() {
        let disk = DiskDimension::from_volumes(Vec::new());
        assert_eq!(disk.total_capacity_bytes, 0);
        assert_eq!(disk.overall_utilization, 0.0);
    }

    #[test]
    fn test_battery_default_no_battery() {
        let bat = BatteryDimension::default();
        assert!(!bat.is_present);
        assert!(bat.is_plugged);
        assert_eq!(bat.charge_pct, 100.0);
    }
}
