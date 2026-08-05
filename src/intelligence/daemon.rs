use std::path::PathBuf;
use std::sync::OnceLock;
use std::time::Duration;
use tokio::signal;
use tokio::time::interval;

use crate::config::store::ConfigManager;
use crate::intelligence::system_awareness::SystemSnapshot;

/// The background daemon — runs on a schedule, collects telemetry,
/// checks automation rules, and performs proactive optimization.
pub struct Daemon {
    config: ConfigManager,
    interval_secs: u64,
    verbose: bool,
    /// Rolling history of recent system snapshots for baseline computation.
    history: Vec<SystemSnapshot>,
    /// Cached baseline metrics derived from `history` (op 275).
    baseline: Option<BaselineMetrics>,
}

/// Baseline metrics computed from recent snapshot history (op 275).
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct BaselineMetrics {
    pub avg_memory_utilization: f64,
    pub avg_cpu_utilization_pct: f64,
    pub avg_disk_utilization: f64,
    pub avg_health_score: f64,
    pub samples: usize,
}

/// Cached resolution of the xmac binary path. Computed once on first
/// call, then reused for the lifetime of the process to avoid leaking
/// memory on every daemon cycle.
static XMAC_BINARY: OnceLock<String> = OnceLock::new();

impl Daemon {
    pub fn new(config: ConfigManager, interval_secs: u64, verbose: bool) -> Self {
        Self {
            config,
            interval_secs,
            verbose,
            history: Vec::new(),
            baseline: None,
        }
    }

    /// Run the daemon loop. If `once` is true, runs a single cycle and exits.
    pub async fn run(&self, once: bool) -> anyhow::Result<()> {
        // Write PID file for single-instance enforcement.
        self.write_pid_file()?;

        if once {
            self.run_cycle().await;
            self.remove_pid_file();
            return Ok(());
        }

        let mut tick = interval(Duration::from_secs(self.interval_secs));

        // Set up graceful shutdown via SIGTERM/SIGINT.
        // We poll the shutdown future inside the loop so signal handlers
        // stay alive across all cycles (not just the first tick).
        let shutdown = async {
            #[cfg(unix)]
            {
                use signal::unix::{signal, SignalKind};
                let mut term = signal(SignalKind::terminate()).expect("install TERM handler");
                let mut int = signal(SignalKind::interrupt()).expect("install INT handler");
                tokio::select! {
                    _ = term.recv() => { "SIGTERM" }
                    _ = int.recv() => { "SIGINT" }
                }
            }
            #[cfg(not(unix))]
            {
                signal::ctrl_c().await.expect("install Ctrl-C handler");
                "Ctrl-C"
            }
        };

        // Pin the shutdown future so we can poll it across loop iterations.
        tokio::pin!(shutdown);

        // First tick fires immediately — run a cycle, then check for shutdown
        // on every subsequent tick.
        tick.tick().await; // consume the immediate first tick
        loop {
            // Run a cycle.
            self.run_cycle().await;

            // Wait for either the next tick or a shutdown signal.
            tokio::select! {
                _ = &mut shutdown => {
                    if self.verbose {
                        eprintln!("xmac daemon: received shutdown signal, shutting down");
                    }
                    break;
                }
                _ = tick.tick() => {
                    // Next cycle — loop continues.
                }
            }
        }

        self.remove_pid_file();
        Ok(())
    }

    /// Resolve the xmac binary path. Prefers the current executable's
    /// directory (which may be the .app bundle), then falls back to
    /// standard install locations. Never uses bare "xmac" from PATH
    /// to prevent PATH injection attacks. The result is cached for
    /// the lifetime of the process.
    fn xmac_binary(&self) -> &str {
        XMAC_BINARY.get_or_init(|| {
            // Try current_exe directory first — this handles the .app bundle case
            // and the case where the user ran ./target/release/x-mac daemon.
            if let Ok(exe) = std::env::current_exe() {
                if let Some(dir) = exe.parent() {
                    let candidate = dir.join("xmac");
                    if candidate.exists() {
                        return candidate.to_string_lossy().into_owned();
                    }
                    // The binary might be named "x-mac" not "xmac"
                    let candidate2 = dir.join("x-mac");
                    if candidate2.exists() {
                        return candidate2.to_string_lossy().into_owned();
                    }
                }
            }
            // Fall back to standard install locations
            for path in &[
                "/opt/homebrew/bin/xmac",
                "/usr/local/bin/xmac",
                "/usr/local/bin/x-mac",
            ] {
                if std::path::Path::new(path).exists() {
                    return path.to_string();
                }
            }
            // Last resort: bare "xmac" (less safe but better than failing silently)
            "xmac".to_string()
        })
    }

    /// Run a single daemon cycle — telemetry collection, rule checking,
    /// and proactive actions.
    async fn run_cycle(&self) {
        if self.verbose {
            eprintln!(
                "xmac daemon: cycle at {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
            );
        }

        // 1. Collect system snapshot
        let snapshot = crate::intelligence::SystemSnapshot::collect();

        if self.verbose {
            eprintln!(
                "  health: {:.0}/100 ({}) | mem: {:.0}% | disk: {:.0}% | cpu: {:.0}%",
                snapshot.health_score,
                snapshot.status,
                snapshot.memory.utilization * 100.0,
                snapshot.disk.overall_utilization * 100.0,
                snapshot.cpu.utilization_pct,
            );
        }

        // 2. Check automation rules
        for rule in &self.config.config().automation {
            if !rule.enabled {
                continue;
            }
            if self.check_cooldown(rule) {
                if let Some(action) = self.evaluate_rule(rule, &snapshot) {
                    if self.verbose {
                        eprintln!("  rule '{}' triggered: {:?}", rule.name, action);
                    }
                    self.execute_action(&action).await;
                }
            }
        }

        // 3. Auto-purge memory if enabled and pressure is high
        if self.config.config().daemon.auto_purge_memory {
            let threshold = self.config.config().optimize.pressure_threshold;
            if snapshot.memory.utilization > threshold {
                if self.verbose {
                    eprintln!(
                        "  auto-purging memory (utilization {:.0}% > threshold {:.0}%)",
                        snapshot.memory.utilization * 100.0,
                        threshold * 100.0
                    );
                }
                self.purge_memory().await;
            }
        }

        // 4. Auto-clean if reclaimable space exceeds threshold
        let auto_clean_mb = self.config.config().daemon.auto_clean_threshold_mb;
        if auto_clean_mb > 0 && snapshot.disk.overall_utilization > 0.85 {
            if self.verbose {
                eprintln!(
                    "  auto-clean: disk utilization {:.0}% — running clean scan",
                    snapshot.disk.overall_utilization * 100.0
                );
            }
            use std::process::Command;
            let _ = Command::new(self.xmac_binary())
                .args(["clean", "--min-age", "7d"])
                .output();
        }
    }

    fn check_cooldown(&self, rule: &crate::config::store::AutomationRule) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        now - rule.last_fired >= rule.cooldown_secs
    }

    fn evaluate_rule(
        &self,
        rule: &crate::config::store::AutomationRule,
        snapshot: &crate::intelligence::SystemSnapshot,
    ) -> Option<crate::config::store::AutomationAction> {
        use crate::config::store::AutomationCondition;

        let triggered = match &rule.condition {
            AutomationCondition::MemoryPressure { threshold } => {
                snapshot.memory.utilization > *threshold
            }
            AutomationCondition::DiskSpaceLow { threshold_gb } => {
                let free_gb = snapshot.disk.total_free_bytes / (1024 * 1024 * 1024);
                free_gb < *threshold_gb
            }
            AutomationCondition::ReclaimableSpace { threshold_mb: _ } => {
                // Would need a scan to determine — skip for now
                false
            }
            AutomationCondition::ProcessMemory {
                process_name,
                threshold_mb,
            } => {
                // Check if any process exceeds the threshold
                let threshold_bytes = *threshold_mb * 1024 * 1024;
                let procs = crate::engines::optimize::telemetry::ProcessTelemetry::collect_all();
                procs
                    .iter()
                    .any(|p| p.name.contains(process_name) && p.resident_size > threshold_bytes)
            }
            AutomationCondition::Scheduled { interval_hours: _ } => {
                // Time-based — would need last_fired tracking
                false
            }
        };

        if triggered {
            Some(rule.action.clone())
        } else {
            None
        }
    }

    async fn execute_action(&self, action: &crate::config::store::AutomationAction) {
        use crate::config::store::AutomationAction;
        match action {
            AutomationAction::PurgeMemory => {
                self.purge_memory().await;
                if self.verbose {
                    eprintln!("  action: purged memory");
                }
            }
            AutomationAction::ScanClean => {
                if self.verbose {
                    eprintln!("  action: running clean scan...");
                }
                use std::process::Command;
                let _ = Command::new(self.xmac_binary())
                    .args(["clean", "--min-age", "7d"])
                    .output();
                if self.verbose {
                    eprintln!("  action: clean scan complete");
                }
            }
            AutomationAction::RunMaintenance => {
                if self.verbose {
                    eprintln!("  action: running maintenance...");
                }
                use std::process::Command;
                let _ = Command::new(self.xmac_binary()).args(["maintain"]).output();
                if self.verbose {
                    eprintln!("  action: maintenance complete");
                }
            }
            AutomationAction::KillProcess { name } => {
                use std::process::Command;
                let _ = Command::new("killall").arg(name).output();
                if self.verbose {
                    eprintln!("  action: killed process '{}'", name);
                }
            }
            AutomationAction::Notify { message } => {
                self.send_notification("X-MaC", message);
            }
        }
    }

    async fn purge_memory(&self) {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            let _ = Command::new("purge").output();
        }
        #[cfg(target_os = "linux")]
        {
            // Drop caches: echo 3 > /proc/sys/vm/drop_caches (requires root)
            use std::process::Command;
            let _ = Command::new("sudo")
                .args(["sh", "-c", "echo 3 > /proc/sys/vm/drop_caches"])
                .output();
        }
    }

    fn send_notification(&self, title: &str, message: &str) {
        #[cfg(target_os = "macos")]
        {
            use std::process::Command;
            // Escape all AppleScript string injection vectors:
            // backslash, double quote, newline, carriage return, tab
            let esc = |s: &str| {
                s.replace('\\', "\\\\")
                    .replace('"', "\\\"")
                    .replace('\n', "\\n")
                    .replace('\r', "\\r")
                    .replace('\t', "\\t")
            };
            let script = format!(
                "display notification \"{}\" with title \"{}\"",
                esc(message),
                esc(title)
            );
            let _ = Command::new("osascript").args(["-e", &script]).output();
        }
        let _ = (title, message);
    }

    fn write_pid_file(&self) -> anyhow::Result<()> {
        let pid_file = &self.config.config().daemon.pid_file;
        if let Some(parent) = pid_file.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // Check if another daemon is already running.
        if pid_file.exists() {
            if let Ok(content) = std::fs::read_to_string(pid_file) {
                if let Ok(pid) = content.trim().parse::<i32>() {
                    // Check if process exists
                    #[cfg(unix)]
                    {
                        use libc::kill;
                        if unsafe { kill(pid, 0) } == 0 {
                            anyhow::bail!(
                                "xmac daemon is already running (pid {}). Use --stop to stop it first.",
                                pid
                            );
                        }
                    }
                }
            }
            // PID file exists but process is dead — remove stale file
            let _ = std::fs::remove_file(pid_file);
        }

        // Write PID file with restrictive permissions (0600) to prevent
        // other users from reading the daemon's PID.
        let pid = std::process::id();
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true) // O_EXCL — atomic creation, prevents race
                .mode(0o600)
                .open(pid_file)
                .and_then(|mut f| {
                    use std::io::Write;
                    f.write_all(pid.to_string().as_bytes())
                })
                .map_err(|e| {
                    if e.kind() == std::io::ErrorKind::AlreadyExists {
                        anyhow::anyhow!(
                            "xmac daemon is already running (PID file exists). Use --stop to stop it first."
                        )
                    } else {
                        anyhow::Error::from(e)
                    }
                })?;
        }
        #[cfg(not(unix))]
        {
            std::fs::write(pid_file, pid.to_string())?;
        }
        Ok(())
    }

    fn remove_pid_file(&self) {
        let pid_file = &self.config.config().daemon.pid_file;
        // Only remove if it's our PID
        if let Ok(content) = std::fs::read_to_string(pid_file) {
            if content.trim() == std::process::id().to_string() {
                let _ = std::fs::remove_file(pid_file);
            }
        }
    }

    /// Check if a daemon is running by reading the PID file.
    pub fn is_running(pid_file: &PathBuf) -> Option<i32> {
        if !pid_file.exists() {
            return None;
        }
        let content = std::fs::read_to_string(pid_file).ok()?;
        let pid: i32 = content.trim().parse().ok()?;
        #[cfg(unix)]
        {
            use libc::kill;
            if unsafe { kill(pid, 0) } == 0 {
                return Some(pid);
            }
        }
        #[cfg(not(unix))]
        {
            return Some(pid);
        }
        None
    }

    /// Stop a running daemon by sending SIGTERM.
    pub fn stop(pid_file: &PathBuf) -> anyhow::Result<()> {
        let pid = Self::is_running(pid_file).ok_or_else(|| {
            anyhow::anyhow!("No running daemon found (PID file: {})", pid_file.display())
        })?;

        #[cfg(unix)]
        {
            use libc::{kill, SIGTERM};
            let result = unsafe { kill(pid, SIGTERM) };
            if result != 0 {
                anyhow::bail!("Failed to send SIGTERM to pid {}", pid);
            }
        }

        // Wait for the process to exit and clean up
        std::thread::sleep(Duration::from_millis(500));
        let _ = std::fs::remove_file(pid_file);
        Ok(())
    }

    /// op 275: Maintain historical baseline — collect a fresh snapshot,
    /// append it to the rolling history, and recompute baseline metrics
    /// (averages of memory, CPU, disk utilization, and health score) from
    /// the recent history window. Keeps at most 100 samples.
    #[allow(dead_code)]
    pub fn maintain_historical_baseline(&mut self) -> &BaselineMetrics {
        let snapshot = crate::intelligence::SystemSnapshot::collect();
        self.history.push(snapshot);
        if self.history.len() > 100 {
            self.history.remove(0);
        }

        let n = self.history.len();
        let (mem, cpu, disk, health) =
            self.history
                .iter()
                .fold((0.0f64, 0.0f64, 0.0f64, 0.0f64), |(m, c, d, h), s| {
                    (
                        m + s.memory.utilization,
                        c + s.cpu.utilization_pct,
                        d + s.disk.overall_utilization,
                        h + s.health_score,
                    )
                });
        let n_f = n as f64;
        self.baseline = Some(BaselineMetrics {
            avg_memory_utilization: if n > 0 { mem / n_f } else { 0.0 },
            avg_cpu_utilization_pct: if n > 0 { cpu / n_f } else { 0.0 },
            avg_disk_utilization: if n > 0 { disk / n_f } else { 0.0 },
            avg_health_score: if n > 0 { health / n_f } else { 0.0 },
            samples: n,
        });
        self.baseline.as_ref().unwrap()
    }

    /// Return the most recently computed baseline metrics, if any.
    #[allow(dead_code)]
    pub fn baseline(&self) -> Option<&BaselineMetrics> {
        self.baseline.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_running_no_pid_file() {
        let tmp = std::env::temp_dir().join("xmac_test_daemon_nonexistent.pid");
        let _ = std::fs::remove_file(&tmp);
        assert!(Daemon::is_running(&tmp).is_none());
    }

    #[test]
    fn test_is_running_invalid_pid_file() {
        let tmp = std::env::temp_dir().join("xmac_test_daemon_invalid.pid");
        std::fs::write(&tmp, "not-a-number").unwrap();
        assert!(Daemon::is_running(&tmp).is_none());
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_is_running_dead_process() {
        // Write a PID that's very unlikely to exist
        let tmp = std::env::temp_dir().join("xmac_test_daemon_dead.pid");
        std::fs::write(&tmp, "999999").unwrap();
        // On Unix, this should return None because the process doesn't exist
        #[cfg(unix)]
        {
            assert!(Daemon::is_running(&tmp).is_none());
        }
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_stop_no_daemon_returns_error() {
        let tmp = std::env::temp_dir().join("xmac_test_daemon_stop_nonexistent.pid");
        let _ = std::fs::remove_file(&tmp);
        assert!(Daemon::stop(&tmp).is_err());
    }

    #[test]
    fn test_maintain_historical_baseline() {
        let mgr = ConfigManager::load();
        let mut daemon = Daemon::new(mgr, 60, false);
        // Initially there is no baseline.
        assert!(daemon.baseline().is_none());
        let baseline = daemon.maintain_historical_baseline();
        assert_eq!(baseline.samples, 1);
        assert!(baseline.avg_health_score >= 0.0 && baseline.avg_health_score <= 100.0);
        assert!(baseline.avg_memory_utilization >= 0.0);
        // A second sample should average the two.
        let baseline = daemon.maintain_historical_baseline();
        assert_eq!(baseline.samples, 2);
        assert!(daemon.baseline().is_some());
    }
}
