use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

// ═══════════════════════════════════════════════════════════════════════
//  System-wide telemetry (from host_statistics64 + sysctl)
// ═══════════════════════════════════════════════════════════════════════

/// System-wide memory telemetry snapshot.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemTelemetry {
    pub timestamp_ms: u64,
    pub total_bytes: u64,
    pub page_size: u64,

    // Page counts (bytes)
    pub free_bytes: u64,
    pub active_bytes: u64,
    pub inactive_bytes: u64,
    pub speculative_bytes: u64,
    pub wired_bytes: u64,
    pub purgeable_bytes: u64,

    // Compression
    pub compressor_bytes_used: u64,
    pub compressor_pool_size: u64,
    pub compressions: u64,
    pub decompressions: u64,

    // Swap
    pub swap_total_bytes: u64,
    pub swap_used_bytes: u64,
    pub swapins: u64,
    pub swapouts: u64,

    // Page fault stats
    pub pageins: u64,
    pub pageouts: u64,
    pub faults: u64,
    pub cow_faults: u64,
    pub reactivations: u64,

    // Pressure level: 1=normal, 2=warn, 4=critical
    pub pressure_level: u32,

    // GPU memory (Apple Silicon)
    pub gpu_working_set_limit: u64,
    pub gpu_wired_bytes: u64,
}

impl SystemTelemetry {
    /// Collect system-wide telemetry.
    pub fn collect() -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        #[cfg(target_os = "macos")]
        {
            Self::collect_macos(now)
        }
        #[cfg(not(target_os = "macos"))]
        {
            Self::collect_stub(now)
        }
    }

    #[cfg(target_os = "macos")]
    fn collect_macos(now: u64) -> Self {
        use std::process::Command;

        let total = sysctl_u64("hw.memsize").unwrap_or(0);
        let page_size = sysctl_u64("hw.pagesize").unwrap_or(4096);

        // Parse vm_stat for page counts
        let vm_stat_out = Command::new("vm_stat")
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default();
        let pages = parse_vm_stat_pages(&vm_stat_out);

        let free = pages.get("free").copied().unwrap_or(0) * page_size;
        let active = pages.get("active").copied().unwrap_or(0) * page_size;
        let inactive = pages.get("inactive").copied().unwrap_or(0) * page_size;
        let speculative = pages.get("speculative").copied().unwrap_or(0) * page_size;
        let wired = pages
            .get("wired down")
            .or_else(|| pages.get("wired"))
            .copied()
            .unwrap_or(0)
            * page_size;
        let purgeable = pages.get("purgeable").copied().unwrap_or(0) * page_size;
        let compressor = pages
            .get("occupied by compressor")
            .or_else(|| pages.get("compressed"))
            .copied()
            .unwrap_or(0)
            * page_size;

        // Compressor stats via sysctl
        let compressor_pool = sysctl_u64("vm.compressor_pool_size").unwrap_or(0);
        let compressions = pages.get("compressions").copied().unwrap_or(0);
        let decompressions = pages.get("decompressions").copied().unwrap_or(0);

        // Swap
        let (swap_total, swap_used) = parse_swapusage();
        let swapins = pages.get("swapins").copied().unwrap_or(0);
        let swapouts = pages.get("swapouts").copied().unwrap_or(0);

        // Page stats
        let pageins = pages.get("pageins").copied().unwrap_or(0);
        let pageouts = pages.get("pageouts").copied().unwrap_or(0);
        let faults = pages.get("faults").copied().unwrap_or(0);
        let cow_faults = pages.get("copy-on-write").copied().unwrap_or(0);
        let reactivations = pages.get("reactivations").copied().unwrap_or(0);

        // Pressure level
        let pressure_level = sysctl_u32("kern.memorystatus_vm_pressure_level").unwrap_or(1);

        // GPU working set limit (Apple Silicon)
        let gpu_working_set_limit = sysctl_u64("iogpu.wired_limit_mb")
            .map(|mb| mb * 1024 * 1024)
            .unwrap_or(0);

        SystemTelemetry {
            timestamp_ms: now,
            total_bytes: total,
            page_size,
            free_bytes: free,
            active_bytes: active,
            inactive_bytes: inactive,
            speculative_bytes: speculative,
            wired_bytes: wired,
            purgeable_bytes: purgeable,
            compressor_bytes_used: compressor,
            compressor_pool_size: compressor_pool,
            compressions,
            decompressions,
            swap_total_bytes: swap_total,
            swap_used_bytes: swap_used,
            swapins,
            swapouts,
            pageins,
            pageouts,
            faults,
            cow_faults,
            reactivations,
            pressure_level,
            gpu_working_set_limit,
            gpu_wired_bytes: 0, // not directly observable without IOReport
        }
    }

    #[cfg(not(target_os = "macos"))]
    fn collect_stub(now: u64) -> Self {
        SystemTelemetry {
            timestamp_ms: now,
            total_bytes: 0,
            page_size: 4096,
            free_bytes: 0,
            active_bytes: 0,
            inactive_bytes: 0,
            speculative_bytes: 0,
            wired_bytes: 0,
            purgeable_bytes: 0,
            compressor_bytes_used: 0,
            compressor_pool_size: 0,
            compressions: 0,
            decompressions: 0,
            swap_total_bytes: 0,
            swap_used_bytes: 0,
            swapins: 0,
            swapouts: 0,
            pageins: 0,
            pageouts: 0,
            faults: 0,
            cow_faults: 0,
            reactivations: 0,
            pressure_level: 1,
            gpu_working_set_limit: 0,
            gpu_wired_bytes: 0,
        }
    }

    /// Derived: used bytes (active + wired + compressor + speculative)
    pub fn used_bytes(&self) -> u64 {
        self.active_bytes + self.wired_bytes + self.compressor_bytes_used + self.speculative_bytes
    }

    /// Derived: available bytes (free + inactive, since inactive is reclaimable)
    pub fn available_bytes(&self) -> u64 {
        self.free_bytes + self.inactive_bytes
    }

    /// Derived: compression ratio
    pub fn compression_ratio(&self) -> f64 {
        if self.compressor_bytes_used > 0 {
            (self.compressions as f64 * self.page_size as f64) / self.compressor_bytes_used as f64
        } else {
            1.0
        }
    }

    /// Derived: memory utilization (0.0 - 1.0)
    pub fn utilization(&self) -> f64 {
        if self.total_bytes > 0 {
            self.used_bytes() as f64 / self.total_bytes as f64
        } else {
            0.0
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Per-process telemetry (from proc_pidinfo + ps)
// ═══════════════════════════════════════════════════════════════════════

/// Per-process memory telemetry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessTelemetry {
    pub pid: u32,
    pub ppid: u32,
    pub name: String,
    pub exec_path: Option<String>,

    // Memory (bytes)
    pub virtual_size: u64,
    pub resident_size: u64,    // RSS
    pub phys_footprint: u64,   // Physical footprint (key metric on macOS)
    pub compressed_bytes: u64, // Compressed memory
    pub internal_bytes: u64,   // Internal (heap, stack)
    pub external_bytes: u64,   // External (mmap'd files)
    pub reusable_bytes: u64,   // Reusable memory
    pub purgeable_volatile: u64,

    // Tagged ledgers (bytes)
    pub network_footprint: u64,
    pub media_footprint: u64,
    pub graphics_footprint: u64,
    pub neural_footprint: u64,

    // Page stats
    pub page_faults: u32,
    pub pageins: u32,
    pub cow_faults: u32,

    // CPU
    pub cpu_user_us: u64,
    pub cpu_system_us: u64,
    pub thread_count: u32,
    pub priority: i32,

    // State
    pub is_foreground: bool,
    pub is_system: bool,
}

impl ProcessTelemetry {
    /// Collect telemetry for all processes.
    pub fn collect_all() -> Vec<ProcessTelemetry> {
        #[cfg(target_os = "macos")]
        {
            Self::collect_all_macos()
        }
        #[cfg(not(target_os = "macos"))]
        {
            Vec::new()
        }
    }

    #[cfg(target_os = "macos")]
    fn collect_all_macos() -> Vec<ProcessTelemetry> {
        use std::process::Command;

        // Use ps to get pid, ppid, rss, vsz, comm, pri, time for all processes
        let output = Command::new("ps")
            .args(["-eo", "pid,ppid,rss,vsz,pri,time,comm"])
            .output();
        let s = match output {
            Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
            Err(_) => return Vec::new(),
        };

        let mut procs: Vec<ProcessTelemetry> = Vec::new();
        let mut foreground_pids = get_foreground_pids_macos();

        for line in s.lines().skip(1) {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() < 7 {
                continue;
            }
            let pid: u32 = match parts[0].parse() {
                Ok(p) => p,
                Err(_) => continue,
            };
            let ppid: u32 = parts[1].parse().unwrap_or(0);
            let rss_kb: u64 = parts[2].parse().unwrap_or(0);
            let vsz_kb: u64 = parts[3].parse().unwrap_or(0);
            let priority: i32 = parts[4].parse().unwrap_or(0);
            // parts[5] is CPU time like "0:00.03"
            let name = parts[6..].join(" ");
            let name = name.rsplit('/').next().unwrap_or(&name).to_string();

            let is_system = is_system_process(&name, pid);
            let is_foreground = foreground_pids.remove(&pid);

            procs.push(ProcessTelemetry {
                pid,
                ppid,
                name: name.clone(),
                exec_path: None,
                virtual_size: vsz_kb * 1024,
                resident_size: rss_kb * 1024,
                phys_footprint: rss_kb * 1024, // approximated by RSS for now
                compressed_bytes: 0,           // requires task_info, filled below
                internal_bytes: 0,
                external_bytes: 0,
                reusable_bytes: 0,
                purgeable_volatile: 0,
                network_footprint: 0,
                media_footprint: 0,
                graphics_footprint: 0,
                neural_footprint: 0,
                page_faults: 0,
                pageins: 0,
                cow_faults: 0,
                cpu_user_us: 0,
                cpu_system_us: 0,
                thread_count: 0,
                priority,
                is_foreground,
                is_system,
            });
        }

        // Enrich with task_info for top processes (by RSS) to get phys_footprint,
        // compressed, tagged ledgers. This requires mach APIs.
        // For processes we can access (same user), get detailed memory breakdown.
        procs.sort_by_key(|b| std::cmp::Reverse(b.resident_size));
        for proc_tel in procs.iter_mut().take(50) {
            if let Some(detailed) = get_task_vm_info_macos(proc_tel.pid) {
                proc_tel.phys_footprint = detailed.phys_footprint;
                proc_tel.compressed_bytes = detailed.compressed;
                proc_tel.internal_bytes = detailed.internal;
                proc_tel.external_bytes = detailed.external;
                proc_tel.reusable_bytes = detailed.reusable;
                proc_tel.purgeable_volatile = detailed.purgeable_volatile;
                proc_tel.network_footprint = detailed.network_footprint;
                proc_tel.media_footprint = detailed.media_footprint;
                proc_tel.graphics_footprint = detailed.graphics_footprint;
                proc_tel.neural_footprint = detailed.neural_footprint;
                proc_tel.page_faults = detailed.page_faults;
                proc_tel.pageins = detailed.pageins;
                proc_tel.cow_faults = detailed.cow_faults;
                proc_tel.thread_count = detailed.thread_count;
            }
        }

        procs
    }
}

/// Detailed per-process VM info extracted via task_info(TASK_VM_INFO).
#[cfg(target_os = "macos")]
struct TaskVmInfo {
    phys_footprint: u64,
    compressed: u64,
    internal: u64,
    external: u64,
    reusable: u64,
    purgeable_volatile: u64,
    network_footprint: u64,
    media_footprint: u64,
    graphics_footprint: u64,
    neural_footprint: u64,
    page_faults: u32,
    pageins: u32,
    cow_faults: u32,
    thread_count: u32,
}

/// Get detailed VM info for a process via proc_pidinfo (PROC_PIDTASKINFO).
/// This works for same-user processes without root.
#[cfg(target_os = "macos")]
fn get_task_vm_info_macos(pid: u32) -> Option<TaskVmInfo> {
    use std::ffi::c_void;
    use std::mem;

    // PROC_PIDTASKINFO = 4
    const PROC_PIDTASKINFO: i32 = 4;

    #[repr(C)]
    struct ProcTaskInfo {
        pti_virtual_size: u64,
        pti_resident_size: u64,
        pti_total_user: u64,
        pti_total_system: u64,
        pti_threads_user: u64,
        pti_threads_system: u64,
        pti_policy: i32,
        pti_faults: i32,
        pti_pageins: i32,
        pti_cow_faults: i32,
        pti_messages_sent: i32,
        pti_messages_received: i32,
        pti_syscalls_mach: i32,
        pti_syscalls_unix: i32,
        pti_csw: i32,
        pti_threadnum: i32,
        pti_numrunning: i32,
        pti_priority: i32,
    }

    extern "C" {
        fn proc_pidinfo(
            pid: i32,
            flavor: i32,
            arg: u64,
            buffer: *mut c_void,
            buffersize: i32,
        ) -> i32;
    }

    let mut info: ProcTaskInfo = unsafe { mem::zeroed() };
    let size = mem::size_of::<ProcTaskInfo>() as i32;
    let ret = unsafe {
        proc_pidinfo(
            pid as i32,
            PROC_PIDTASKINFO,
            0,
            &mut info as *mut _ as *mut c_void,
            size,
        )
    };

    if ret <= 0 {
        return None;
    }

    Some(TaskVmInfo {
        phys_footprint: info.pti_resident_size, // RSS as proxy for footprint
        compressed: 0,                          // not available via PROC_PIDTASKINFO
        internal: 0,
        external: 0,
        reusable: 0,
        purgeable_volatile: 0,
        network_footprint: 0,
        media_footprint: 0,
        graphics_footprint: 0,
        neural_footprint: 0,
        page_faults: info.pti_faults.max(0) as u32,
        pageins: info.pti_pageins.max(0) as u32,
        cow_faults: info.pti_cow_faults.max(0) as u32,
        thread_count: info.pti_threadnum.max(0) as u32,
    })
}

/// Get the set of foreground process PIDs on macOS.
#[cfg(target_os = "macos")]
fn get_foreground_pids_macos() -> std::collections::HashSet<u32> {
    use std::collections::HashSet;
    // The frontmost application can be queried via AppleScript, but for a
    // lightweight approach, we mark the process with the lowest PID that
    // matches common app names. For now, return empty — the GUI layer
    // will provide foreground state.
    let _ = HashSet::<u32>::new();
    HashSet::new()
}

/// Check if a process is a system process that should be protected.
#[allow(dead_code)]
fn is_system_process(name: &str, pid: u32) -> bool {
    if pid <= 1 {
        return true;
    }
    let system_procs = [
        "kernel_task",
        "launchd",
        "WindowServer",
        "Finder",
        "Dock",
        "SystemUIServer",
        "loginwindow",
        "coreaudiod",
        "bluetoothd",
        "configd",
        "cfprefsd",
        "distnoted",
        "fontservicesd",
        "iconservicesd",
        "logd",
        "mds",
        "mds_stores",
        "mdworker",
        "notifyd",
        "opendirectoryd",
        "powerd",
        "secinitd",
        "securityd",
        "trustd",
        "tccd",
        "usermanagerd",
        "wifid",
        "runningboardd",
        "systemextensionsd",
    ];
    system_procs.contains(&name)
}

// ═══════════════════════════════════════════════════════════════════════
//  Complete telemetry snapshot
// ═══════════════════════════════════════════════════════════════════════

/// A complete telemetry snapshot — system + all processes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetrySnapshot {
    pub system: SystemTelemetry,
    pub processes: Vec<ProcessTelemetry>,
}

impl TelemetrySnapshot {
    /// Collect a complete snapshot.
    pub fn collect() -> Self {
        Self {
            system: SystemTelemetry::collect(),
            processes: ProcessTelemetry::collect_all(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Telemetry ring buffer (for trend analysis)
// ═══════════════════════════════════════════════════════════════════════

/// A ring buffer of telemetry snapshots for computing trends.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelemetryBuffer {
    pub snapshots: Vec<TelemetrySnapshot>,
    pub max_size: usize,
}

#[allow(dead_code)]
impl TelemetryBuffer {
    pub fn new(max_size: usize) -> Self {
        Self {
            snapshots: Vec::with_capacity(max_size),
            max_size,
        }
    }

    pub fn push(&mut self, snapshot: TelemetrySnapshot) {
        if self.snapshots.len() >= self.max_size {
            self.snapshots.remove(0);
        }
        self.snapshots.push(snapshot);
    }

    pub fn len(&self) -> usize {
        self.snapshots.len()
    }

    pub fn is_empty(&self) -> bool {
        self.snapshots.is_empty()
    }

    pub fn latest(&self) -> Option<&TelemetrySnapshot> {
        self.snapshots.last()
    }

    /// Compute the rate of change of free memory (bytes/sec) over the buffer.
    pub fn free_memory_rate(&self) -> f64 {
        if self.snapshots.len() < 2 {
            return 0.0;
        }
        let first = &self.snapshots[0];
        let last = self.snapshots.last().unwrap();
        let delta_bytes = last.system.free_bytes as f64 - first.system.free_bytes as f64;
        let delta_ms = last.system.timestamp_ms as f64 - first.system.timestamp_ms as f64;
        if delta_ms > 0.0 {
            delta_bytes / (delta_ms / 1000.0)
        } else {
            0.0
        }
    }

    /// Compute the rate of change of a process's RSS (bytes/sec).
    pub fn process_rss_rate(&self, pid: u32) -> f64 {
        if self.snapshots.len() < 2 {
            return 0.0;
        }
        let mut rates = Vec::new();
        for window in self.snapshots.windows(2) {
            let before = window[0].processes.iter().find(|p| p.pid == pid);
            let after = window[1].processes.iter().find(|p| p.pid == pid);
            if let (Some(b), Some(a)) = (before, after) {
                let delta = a.resident_size as f64 - b.resident_size as f64;
                let dt = (window[1].system.timestamp_ms as f64
                    - window[0].system.timestamp_ms as f64)
                    / 1000.0;
                if dt > 0.0 {
                    rates.push(delta / dt);
                }
            }
        }
        if rates.is_empty() {
            0.0
        } else {
            rates.iter().sum::<f64>() / rates.len() as f64
        }
    }

    /// Predict free memory in `seconds` ahead, based on current trend.
    pub fn predict_free_memory(&self, seconds: f64) -> u64 {
        let rate = self.free_memory_rate();
        let current = self.latest().map(|s| s.system.free_bytes).unwrap_or(0);
        let predicted = current as f64 + rate * seconds;
        predicted.max(0.0) as u64
    }

    /// Predict when free memory will reach `threshold` bytes, in seconds.
    /// Returns None if the trend is not decreasing toward the threshold.
    pub fn time_to_threshold(&self, threshold: u64) -> Option<f64> {
        if self.snapshots.len() < 2 {
            return None;
        }
        let rate = self.free_memory_rate();
        if rate >= 0.0 {
            return None; // not decreasing
        }
        let current = self.latest()?.system.free_bytes;
        if current <= threshold {
            return Some(0.0);
        }
        let delta = current as f64 - threshold as f64;
        Some(delta / rate.abs())
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Sysctl helpers
// ═══════════════════════════════════════════════════════════════════════

#[cfg(target_os = "macos")]
fn sysctl_u64(name: &str) -> Option<u64> {
    use std::process::Command;
    let output = Command::new("sysctl").args(["-n", name]).output().ok()?;
    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
    s.split_whitespace()
        .next()
        .and_then(|n| n.parse::<u64>().ok())
}

#[cfg(target_os = "macos")]
fn sysctl_u32(name: &str) -> Option<u32> {
    sysctl_u64(name).map(|v| v as u32)
}

/// Parse vm_stat output into a HashMap of page-name → page-count.
#[cfg(target_os = "macos")]
fn parse_vm_stat_pages(output: &str) -> HashMap<String, u64> {
    let mut result = HashMap::new();
    for line in output.lines() {
        if let Some(colon_pos) = line.find(':') {
            let key = line[..colon_pos]
                .trim()
                .trim_start_matches("Pages ")
                .to_lowercase();
            let value_str = line[colon_pos + 1..]
                .trim()
                .trim_end_matches('.')
                .replace(',', "");
            if let Ok(pages) = value_str.parse::<u64>() {
                result.insert(key, pages);
            }
        }
    }
    result
}

/// Parse `sysctl vm.swapusage` output.
#[cfg(target_os = "macos")]
fn parse_swapusage() -> (u64, u64) {
    use std::process::Command;
    let output = Command::new("sysctl").args(["-n", "vm.swapusage"]).output();
    let s = match output {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return (0, 0),
    };
    let mut total = 0u64;
    let mut used = 0u64;
    if let Some(ti) = s.find("total =") {
        let rest = &s[ti + 7..].trim_start();
        if let Some(end) = rest.find(char::is_whitespace) {
            total = rest[..end].parse().unwrap_or(0);
        }
    }
    if let Some(ui) = s.find("used =") {
        let rest = &s[ui + 6..].trim_start();
        if let Some(end) = rest.find(char::is_whitespace) {
            used = rest[..end].parse().unwrap_or(0);
        }
    }
    (total, used)
}

// ═══════════════════════════════════════════════════════════════════════
//  Tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_system_telemetry() {
        let tel = SystemTelemetry::collect();
        assert!(tel.total_bytes > 0, "total_bytes should be > 0");
        assert!(tel.page_size > 0);
        assert!(tel.pressure_level >= 1 && tel.pressure_level <= 4);
    }

    #[test]
    fn collect_all_processes() {
        let procs = ProcessTelemetry::collect_all();
        assert!(!procs.is_empty(), "should find processes");
        // Should find at least one system process (kernel_task, launchd, etc.)
        assert!(
            procs.iter().any(|p| p.is_system),
            "should find at least one system process"
        );
    }

    #[test]
    fn collect_full_snapshot() {
        let snap = TelemetrySnapshot::collect();
        assert!(snap.system.total_bytes > 0);
        assert!(!snap.processes.is_empty());
    }

    #[test]
    fn telemetry_buffer_tracks_trends() {
        let mut buf = TelemetryBuffer::new(10);
        for _ in 0..3 {
            buf.push(TelemetrySnapshot::collect());
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
        assert_eq!(buf.len(), 3);
        assert!(buf.latest().is_some());
        // Rate should be computable (may be 0 if memory didn't change)
        let _rate = buf.free_memory_rate();
    }

    #[test]
    fn system_telemetry_derived_metrics() {
        let tel = SystemTelemetry::collect();
        let used = tel.used_bytes();
        let avail = tel.available_bytes();
        assert!(used + avail <= tel.total_bytes + tel.wired_bytes); // rough check
        let util = tel.utilization();
        assert!(util >= 0.0 && util <= 1.0);
    }

    #[test]
    fn is_system_process_detection() {
        assert!(is_system_process("kernel_task", 0));
        assert!(is_system_process("launchd", 1));
        assert!(!is_system_process("Safari", 500));
        assert!(!is_system_process("xmac", 1234));
    }
}
