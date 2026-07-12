use serde::Serialize;
use std::collections::HashMap;

/// Memory statistics for the system.
#[derive(Debug, Clone, Serialize)]
pub struct MemoryStats {
    pub total_bytes: u64,
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub app_memory_bytes: u64,
    pub wired_bytes: u64,
    pub compressed_bytes: u64,
    pub swap_used_bytes: u64,
    pub swap_total_bytes: u64,
    pub pageins: u64,
    pub pageouts: u64,
    pub swap_used: u64,
    pub memory_pressure: MemoryPressure,
    pub top_consumers: Vec<ProcessMemory>,
}

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryPressure {
    Nominal,
    Warning,
    Critical,
}

impl MemoryPressure {
    pub fn from_usage(used: u64, total: u64) -> Self {
        if total == 0 {
            return MemoryPressure::Nominal;
        }
        let ratio = used as f64 / total as f64;
        if ratio >= 0.90 {
            MemoryPressure::Critical
        } else if ratio >= 0.75 {
            MemoryPressure::Warning
        } else {
            MemoryPressure::Nominal
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            MemoryPressure::Nominal => "Nominal",
            MemoryPressure::Warning => "Warning",
            MemoryPressure::Critical => "Critical",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ProcessMemory {
    pub pid: u32,
    pub name: String,
    pub rss_bytes: u64,
    pub percent: f64,
}

impl MemoryStats {
    /// Collect memory statistics on the current platform.
    pub fn collect() -> Self {
        #[cfg(target_os = "macos")]
        {
            Self::collect_macos()
        }
        #[cfg(target_os = "linux")]
        {
            Self::collect_linux()
        }
        #[cfg(not(any(target_os = "macos", target_os = "linux")))]
        {
            Self::collect_unknown()
        }
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux")))]
    fn collect_unknown() -> Self {
        Self {
            total_bytes: 0,
            used_bytes: 0,
            available_bytes: 0,
            app_memory_bytes: 0,
            wired_bytes: 0,
            compressed_bytes: 0,
            swap_used_bytes: 0,
            swap_total_bytes: 0,
            pageins: 0,
            pageouts: 0,
            swap_used: 0,
            memory_pressure: MemoryPressure::Nominal,
            top_consumers: Vec::new(),
        }
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  macOS — sysctl + vm_stat + ps
    // ═══════════════════════════════════════════════════════════════════════

    #[cfg(target_os = "macos")]
    fn collect_macos() -> Self {
        let total = Self::sysctl_u64("hw.memsize").unwrap_or(0);
        let page_size = Self::sysctl_u64("hw.pagesize").unwrap_or(4096);

        // Parse vm_stat output
        let vm_stat = Self::run_command("vm_stat");
        let parsed = Self::parse_vm_stat(&vm_stat, page_size);

        let app_memory = parsed.get("active").copied().unwrap_or(0);
        let wired = parsed.get("wired down").copied().unwrap_or(0)
            + parsed.get("wired").copied().unwrap_or(0);
        let compressed = parsed.get("occupied by compressor").copied().unwrap_or(0)
            + parsed.get("compressed").copied().unwrap_or(0);
        let speculative = parsed.get("speculative").copied().unwrap_or(0);
        let free = parsed.get("free").copied().unwrap_or(0);
        let inactive = parsed.get("inactive").copied().unwrap_or(0);

        // "Used" on macOS = app memory + wired + compressed + speculative
        let used = app_memory + wired + compressed + speculative;
        // "Available" = free + inactive (inactive can be reclaimed)
        let available = free + inactive;

        // Swap info — parse sysctl vm.swapusage output
        let (swap_total, swap_used_val) = Self::parse_swap_macos();

        let top = Self::top_processes_macos();

        let pressure = if used > 0 && total > 0 {
            // Account for inactive memory being reclaimable
            let effective_used = used.saturating_sub(inactive.min(used / 4));
            MemoryPressure::from_usage(effective_used, total)
        } else {
            MemoryPressure::Nominal
        };

        Self {
            total_bytes: total,
            used_bytes: used,
            available_bytes: available,
            app_memory_bytes: app_memory + speculative,
            wired_bytes: wired,
            compressed_bytes: compressed,
            swap_used_bytes: swap_used_val,
            swap_total_bytes: swap_total,
            pageins: 0,
            pageouts: 0,
            swap_used: swap_used_val,
            memory_pressure: pressure,
            top_consumers: top,
        }
    }

    #[cfg(target_os = "macos")]
    fn sysctl_u64(name: &str) -> Option<u64> {
        let output = std::process::Command::new("sysctl")
            .args(["-n", name])
            .output()
            .ok()?;
        let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Some sysctl values return "X Y Z" — try to parse the first number
        s.split_whitespace()
            .next()
            .and_then(|n| n.parse::<u64>().ok())
    }

    #[cfg(target_os = "macos")]
    fn parse_vm_stat(output: &str, page_size: u64) -> HashMap<String, u64> {
        let mut result = HashMap::new();
        for line in output.lines() {
            // Lines look like:
            // "Pages active:                         123456."
            // "Pages wired down:                     789012."
            // "Pages occupied by compressor:         345678."
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
                    result.insert(key, pages * page_size);
                }
            }
        }
        result
    }

    #[cfg(target_os = "macos")]
    fn parse_swap_macos() -> (u64, u64) {
        let output = std::process::Command::new("sysctl")
            .args(["-n", "vm.swapusage"])
            .output();
        let s = match output {
            Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
            Err(_) => return (0, 0),
        };
        // "total = 8589934592  used = 2147483648  free = 6442450944"
        let mut total = 0u64;
        let mut used = 0u64;
        for part in s.split_whitespace() {
            if part == "total" {
                // next value after "="
            }
        }
        // Simple parsing
        let total_idx = s.find("total =");
        let used_idx = s.find("used =");
        if let Some(ti) = total_idx {
            let rest = &s[ti + 7..].trim_start();
            if let Some(end) = rest.find(char::is_whitespace) {
                total = rest[..end].parse().unwrap_or(0);
            }
        }
        if let Some(ui) = used_idx {
            let rest = &s[ui + 6..].trim_start();
            if let Some(end) = rest.find(char::is_whitespace) {
                used = rest[..end].parse().unwrap_or(0);
            }
        }
        (total, used)
    }

    #[cfg(target_os = "macos")]
    fn top_processes_macos() -> Vec<ProcessMemory> {
        // ps -eo pid,rss,comm with sorting
        let output = std::process::Command::new("ps")
            .args(["-eo", "pid,rss,comm"])
            .output();
        let s = match output {
            Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
            Err(_) => return Vec::new(),
        };
        let mut procs: Vec<ProcessMemory> = s
            .lines()
            .skip(1) // header
            .filter_map(|line| {
                let mut parts = line.split_whitespace();
                let pid: u32 = parts.next()?.parse().ok()?;
                let rss_kb: u64 = parts.next()?.parse().ok()?;
                let name = parts.collect::<Vec<_>>().join(" ");
                let name = name.rsplit('/').next().unwrap_or(&name).to_string();
                Some(ProcessMemory {
                    pid,
                    name,
                    rss_bytes: rss_kb * 1024,
                    percent: 0.0, // filled in later
                })
            })
            .collect();

        procs.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));
        procs.truncate(10);

        // Fill in percentages
        let total = Self::sysctl_u64("hw.memsize").unwrap_or(1);
        for p in &mut procs {
            p.percent = (p.rss_bytes as f64 / total as f64) * 100.0;
        }
        procs
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Linux — /proc/meminfo + /proc/[pid]/status
    // ═══════════════════════════════════════════════════════════════════════

    #[cfg(target_os = "linux")]
    fn collect_linux() -> Self {
        let meminfo = Self::read_meminfo();
        let total = meminfo.get("MemTotal").copied().unwrap_or(0) * 1024;
        let free = meminfo.get("MemFree").copied().unwrap_or(0) * 1024;
        let available = meminfo.get("MemAvailable").copied().unwrap_or(free / 1024) * 1024;
        let buffers = meminfo.get("Buffers").copied().unwrap_or(0) * 1024;
        let cached = meminfo.get("Cached").copied().unwrap_or(0) * 1024;
        let swap_total = meminfo.get("SwapTotal").copied().unwrap_or(0) * 1024;
        let swap_free = meminfo.get("SwapFree").copied().unwrap_or(0) * 1024;
        let swap_used = swap_total.saturating_sub(swap_free);
        let sreclaimable = meminfo.get("SReclaimable").copied().unwrap_or(0) * 1024;

        // On Linux, "used" = total - free - buffers - cache - sreclaimable
        let used = total.saturating_sub(free).saturating_sub(buffers).saturating_sub(cached).saturating_sub(sreclaimable);
        let app_memory = used; // app memory is the non-cache portion

        let top = Self::top_processes_linux(total);

        let pressure = MemoryPressure::from_usage(used, total);

        Self {
            total_bytes: total,
            used_bytes: used,
            available_bytes: available,
            app_memory_bytes: app_memory,
            wired_bytes: 0, // not a Linux concept
            compressed_bytes: meminfo.get("Compressed").copied().unwrap_or(0) * 1024,
            swap_used_bytes: swap_used,
            swap_total_bytes: swap_total,
            pageins: 0,
            pageouts: 0,
            swap_used,
            memory_pressure: pressure,
            top_consumers: top,
        }
    }

    #[cfg(target_os = "linux")]
    fn read_meminfo() -> HashMap<String, u64> {
        let mut result = HashMap::new();
        if let Ok(content) = std::fs::read_to_string("/proc/meminfo") {
            for line in content.lines() {
                if let Some(colon) = line.find(':') {
                    let key = line[..colon].trim().to_string();
                    let rest = line[colon + 1..].trim();
                    // "12345 kB"
                    let num: u64 = rest
                        .split_whitespace()
                        .next()
                        .and_then(|n| n.parse().ok())
                        .unwrap_or(0);
                    result.insert(key, num);
                }
            }
        }
        result
    }

    #[cfg(target_os = "linux")]
    fn top_processes_linux(total: u64) -> Vec<ProcessMemory> {
        let mut procs = Vec::new();
        if let Ok(entries) = std::fs::read_dir("/proc") {
            for entry in entries.flatten() {
                let name = entry.file_name();
                let pid: u32 = match name.to_str().and_then(|s| s.parse().ok()) {
                    Some(p) => p,
                    None => continue,
                };
                // Read /proc/[pid]/status for VmRSS
                let status_path = entry.path().join("status");
                if let Ok(content) = std::fs::read_to_string(&status_path) {
                    let mut rss = 0u64;
                    let mut proc_name = String::new();
                    for line in content.lines() {
                        if line.starts_with("Name:") {
                            proc_name = line[5..].trim().to_string();
                        }
                        if line.starts_with("VmRSS:") {
                            let rest = line[6..].trim();
                            rss = rest
                                .split_whitespace()
                                .next()
                                .and_then(|n| n.parse::<u64>().ok())
                                .map(|v| v * 1024)
                                .unwrap_or(0);
                        }
                    }
                    if rss > 0 {
                        procs.push(ProcessMemory {
                            pid,
                            name: proc_name,
                            rss_bytes: rss,
                            percent: if total > 0 { (rss as f64 / total as f64) * 100.0 } else { 0.0 },
                        });
                    }
                }
            }
        }
        procs.sort_by(|a, b| b.rss_bytes.cmp(&a.rss_bytes));
        procs.truncate(10);
        procs
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Shared helpers
    // ═══════════════════════════════════════════════════════════════════════

    fn run_command(cmd: &str) -> String {
        std::process::Command::new(cmd)
            .output()
            .map(|o| String::from_utf8_lossy(&o.stdout).to_string())
            .unwrap_or_default()
    }

    /// Format a memory report suitable for display.
    pub fn report(&self) -> String {
        let mut s = String::new();
        s.push_str(&format!("Memory Pressure: {}\n", self.memory_pressure.as_str()));
        s.push_str(&format!(
            "  Total:      {:>12}\n",
            crate::util::disk::format_bytes(self.total_bytes)
        ));
        s.push_str(&format!(
            "  Used:       {:>12}  ({:.1}%)\n",
            crate::util::disk::format_bytes(self.used_bytes),
            if self.total_bytes > 0 { self.used_bytes as f64 / self.total_bytes as f64 * 100.0 } else { 0.0 }
        ));
        s.push_str(&format!(
            "  Available:  {:>12}\n",
            crate::util::disk::format_bytes(self.available_bytes)
        ));
        if self.wired_bytes > 0 {
            s.push_str(&format!(
                "  Wired:      {:>12}\n",
                crate::util::disk::format_bytes(self.wired_bytes)
            ));
        }
        if self.compressed_bytes > 0 {
            s.push_str(&format!(
                "  Compressed: {:>12}\n",
                crate::util::disk::format_bytes(self.compressed_bytes)
            ));
        }
        if self.app_memory_bytes > 0 {
            s.push_str(&format!(
                "  App Memory: {:>12}\n",
                crate::util::disk::format_bytes(self.app_memory_bytes)
            ));
        }
        if self.swap_total_bytes > 0 {
            s.push_str(&format!(
                "  Swap:       {:>12} / {}\n",
                crate::util::disk::format_bytes(self.swap_used_bytes),
                crate::util::disk::format_bytes(self.swap_total_bytes)
            ));
        }
        if !self.top_consumers.is_empty() {
            s.push_str("\n  Top Memory Consumers:\n");
            for p in &self.top_consumers {
                s.push_str(&format!(
                    "    {:>6}  {:>10}  {:>5.1}%  {}\n",
                    p.pid,
                    crate::util::disk::format_bytes(p.rss_bytes),
                    p.percent,
                    p.name
                ));
            }
        }
        s
    }

    /// How many bytes could be freed by a purge (inactive + compressed on macOS,
    /// page cache on Linux).
    pub fn reclaimable_bytes(&self) -> u64 {
        if cfg!(target_os = "macos") {
            // Inactive memory + compressed can be reclaimed
            self.available_bytes + self.compressed_bytes
        } else {
            // On Linux, drop_caches frees page cache + dentries + inodes
            // Estimate from /proc/meminfo Buffers + Cached + SReclaimable
            // These are already subtracted from used, so reclaimable = total - used
            self.total_bytes.saturating_sub(self.used_bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pressure_nominal_under_75() {
        assert_eq!(MemoryPressure::from_usage(50, 100), MemoryPressure::Nominal);
    }

    #[test]
    fn pressure_warning_75_to_90() {
        assert_eq!(MemoryPressure::from_usage(80, 100), MemoryPressure::Warning);
    }

    #[test]
    fn pressure_critical_over_90() {
        assert_eq!(MemoryPressure::from_usage(95, 100), MemoryPressure::Critical);
    }

    #[test]
    fn pressure_zero_total_is_nominal() {
        assert_eq!(MemoryPressure::from_usage(100, 0), MemoryPressure::Nominal);
    }

    #[test]
    fn collect_runs_without_panic() {
        // This calls sysctl/reads /proc — should not panic on either platform
        let stats = MemoryStats::collect();
        assert!(stats.total_bytes > 0, "total_bytes should be > 0");
    }

    #[test]
    fn report_is_nonempty() {
        let stats = MemoryStats::collect();
        let r = stats.report();
        assert!(r.contains("Memory Pressure"));
        assert!(r.contains("Total"));
    }

    #[test]
    fn reclaimable_is_reasonable() {
        let stats = MemoryStats::collect();
        let reclaimable = stats.reclaimable_bytes();
        // Reclaimable should not exceed total
        assert!(reclaimable <= stats.total_bytes);
    }
}
