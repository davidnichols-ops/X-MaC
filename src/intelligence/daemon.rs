use std::path::PathBuf;
use std::time::Duration;
use tokio::signal;
use tokio::time::interval;

use crate::config::store::ConfigManager;

/// The background daemon — runs on a schedule, collects telemetry,
/// checks automation rules, and performs proactive optimization.
pub struct Daemon {
    config: ConfigManager,
    interval_secs: u64,
    verbose: bool,
}

impl Daemon {
    pub fn new(config: ConfigManager, interval_secs: u64, verbose: bool) -> Self {
        Self {
            config,
            interval_secs,
            verbose,
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
        let shutdown = async {
            #[cfg(unix)]
            {
                use signal::unix::{SignalKind, signal};
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

        let sig = tokio::select! {
            _ = tick.tick() => {
                // First tick fires immediately — run a cycle.
                loop {
                    self.run_cycle().await;
                    tick.tick().await;
                }
            }
            sig = shutdown => {
                if self.verbose {
                    eprintln!("xmac daemon: received {}, shutting down", sig);
                }
                sig
            }
        };

        let _ = sig;
        self.remove_pid_file();
        Ok(())
    }

    /// Run a single daemon cycle — telemetry collection, rule checking,
    /// and proactive actions.
    async fn run_cycle(&self) {
        if self.verbose {
            eprintln!("xmac daemon: cycle at {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"));
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
                    eprintln!("  auto-purging memory (utilization {:.0}% > threshold {:.0}%)",
                        snapshot.memory.utilization * 100.0, threshold * 100.0);
                }
                self.purge_memory().await;
            }
        }

        // 4. Auto-clean if reclaimable space exceeds threshold
        let auto_clean_mb = self.config.config().daemon.auto_clean_threshold_mb;
        if auto_clean_mb > 0 {
            // We'd need to run a quick clean scan here. For now, just log.
            if self.verbose {
                eprintln!("  auto-clean threshold: {} MB (scan skipped in daemon mode)", auto_clean_mb);
            }
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
        use crate::config::store::{AutomationAction, AutomationCondition};

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
            AutomationCondition::ProcessMemory { process_name, threshold_mb } => {
                // Check if any process exceeds the threshold
                let threshold_bytes = *threshold_mb as u64 * 1024 * 1024;
                let procs = crate::engines::optimize::telemetry::ProcessTelemetry::collect_all();
                procs.iter().any(|p| p.name.contains(process_name) && p.resident_size > threshold_bytes)
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
            }
            AutomationAction::ScanClean => {
                if self.verbose {
                    eprintln!("  action: scan clean (would run xmac clean)");
                }
            }
            AutomationAction::RunMaintenance => {
                if self.verbose {
                    eprintln!("  action: run maintenance (would run xmac maintain)");
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
            let script = format!(
                "display notification \"{}\" with title \"{}\"",
                message.replace('"', "\\\""),
                title.replace('"', "\\\"")
            );
            let _ = Command::new("osascript")
                .args(["-e", &script])
                .output();
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
        use libc::{kill, SIGTERM};
                        if unsafe { kill(pid, 0) } == 0 {
                            anyhow::bail!(
                                "xmac daemon is already running (pid {}). Use --stop to stop it first.",
                                pid
                            );
                        }
                    }
                }
            }
        }

        let pid = std::process::id();
        std::fs::write(pid_file, pid.to_string())?;
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
            use libc::{kill, SIGTERM};
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
        let pid = Self::is_running(pid_file)
            .ok_or_else(|| anyhow::anyhow!("No running daemon found (PID file: {})", pid_file.display()))?;

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
}
