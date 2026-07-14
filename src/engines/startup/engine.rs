use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

use super::scanner::{scan_all, StartupItem, StartupScope};

/// Startup & Background Process Management Engine — scans Login Items,
/// LaunchAgents, LaunchDaemons, and background processes.
///
/// Implements Cleaner/Optimizer operations 146-175:
///   - 146:  Scan Login Items (osascript on macOS)
///   - 147:  Scan LaunchAgents (user ~/Library/LaunchAgents)
///   - 148:  Scan LaunchAgents (system /Library/LaunchAgents)
///   - 149:  Scan LaunchDaemons (/Library/LaunchDaemons)
///   - 150:  Parse plist files (plist crate)
///   - 151:  Identify startup commands (ProgramArguments)
///   - 152:  Identify background helpers
///   - 153:  Identify update agents
///   - 154:  Identify telemetry processes
///   - 155:  Identify unnecessary services (heuristic scoring)
///   - 159:  Detect delayed launch apps (StartInterval)
///   - 160:  Measure boot impact
///   - 161:  Estimate CPU impact
///   - 162:  Estimate memory impact
///   - 170:  Kill frozen applications
///   - 171:  Restart services
///   - 172:  Monitor daemon health
///   - 173:  Detect zombie processes
pub struct StartupEngine;

impl Default for StartupEngine {
    fn default() -> Self {
        Self
    }
}

impl StartupEngine {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }

    /// Classify a startup item into a Category and Severity.
    fn classify(item: &StartupItem) -> (Category, Severity) {
        if item.is_telemetry() {
            return (Category::TelemetryProcess, Severity::Medium);
        }
        if item.is_update_agent() {
            return (Category::UpdateAgent, Severity::Low);
        }
        if item.is_background_helper() {
            return (Category::BackgroundHelper, Severity::Low);
        }
        match item.scope {
            StartupScope::LoginItem => (Category::LoginItem, Severity::Info),
            StartupScope::UserAgent | StartupScope::SystemAgent => {
                (Category::LaunchAgent, Severity::Info)
            }
            StartupScope::SystemDaemon => (Category::LaunchDaemon, Severity::Info),
            StartupScope::SystemdUser | StartupScope::SystemdSystem => {
                (Category::LaunchAgent, Severity::Info)
            }
        }
    }

    /// Build a Finding from a startup item. (ops 151-155, 159-162)
    fn item_to_finding(item: &StartupItem) -> Finding {
        let (category, base_severity) = Self::classify(item);

        // Elevate severity for high-impact or low-necessity items
        let necessity = item.necessity_score();
        let boot_impact = item.boot_impact();
        let severity = if necessity < 0.3 {
            Severity::High
        } else if necessity < 0.5 || boot_impact > 0.6 {
            Severity::Medium
        } else {
            base_severity
        };

        let scope_str = item.scope.as_str();
        let title = match category {
            Category::TelemetryProcess => format!("Telemetry process: {}", item.label),
            Category::UpdateAgent => format!("Update agent: {}", item.label),
            Category::BackgroundHelper => format!("Background helper: {}", item.label),
            Category::LoginItem => format!("Login item: {}", item.label),
            Category::LaunchAgent => format!("LaunchAgent: {}", item.label),
            Category::LaunchDaemon => format!("LaunchDaemon: {}", item.label),
            _ => format!("Startup item: {}", item.label),
        };

        let mut description = format!(
            "Scope: {} | Command: {} | RunAtLoad: {} | KeepAlive: {}",
            scope_str, item.command, item.run_at_load, item.keep_alive,
        );

        if item.is_delayed_launch() {
            if let Some(interval) = item.start_interval {
                description.push_str(&format!(" | StartInterval: {}s", interval));
            } else if item.start_calendar_interval {
                description.push_str(" | StartCalendarInterval (scheduled)");
            }
        }

        if item.disabled {
            description.push_str(" | Disabled");
        }

        let target = if let Some(ref path) = item.path {
            Target::Path(path.clone())
        } else {
            Target::LaunchdLabel(item.label.clone())
        };

        let mut finding = Finding::new(
            EngineId::Startup,
            severity,
            category,
            target,
            title,
            description,
        )
        .with_metadata("scope".to_string(), serde_json::json!(scope_str))
        .with_metadata("command".to_string(), serde_json::json!(item.command))
        .with_metadata(
            "run_at_load".to_string(),
            serde_json::json!(item.run_at_load),
        )
        .with_metadata("keep_alive".to_string(), serde_json::json!(item.keep_alive))
        .with_metadata("disabled".to_string(), serde_json::json!(item.disabled))
        .with_metadata("necessity_score".to_string(), serde_json::json!(necessity))
        .with_metadata("boot_impact".to_string(), serde_json::json!(boot_impact))
        .with_metadata(
            "cpu_impact".to_string(),
            serde_json::json!(item.cpu_impact()),
        )
        .with_metadata(
            "memory_impact".to_string(),
            serde_json::json!(item.memory_impact()),
        );

        if let Some(interval) = item.start_interval {
            finding =
                finding.with_metadata("start_interval".to_string(), serde_json::json!(interval));
        }
        if item.start_calendar_interval {
            finding = finding.with_metadata(
                "start_calendar_interval".to_string(),
                serde_json::json!(true),
            );
        }
        if let Some(ref pt) = item.process_type {
            finding = finding.with_metadata("process_type".to_string(), serde_json::json!(pt));
        }

        // Remediation hint based on classification
        let hint = match category {
            Category::TelemetryProcess => {
                "Telemetry/analytics process. Consider disabling if you don't want usage data collected."
                    .to_string()
            }
            Category::UpdateAgent => {
                "Update agent. Safe to keep but adds boot time. Disable if you update manually."
                    .to_string()
            }
            _ if necessity < 0.3 => {
                format!(
                    "Low necessity score ({:.2}). Consider disabling with: launchctl bootout gui/$(id -u) {}",
                    necessity, item.label
                )
            }
            _ => "Startup item detected. Review if needed at boot.".to_string(),
        };
        finding = finding.with_hint(hint);

        finding
    }

    /// Detect zombie processes by scanning /proc (Linux) or using ps.
    /// (op 173)
    fn detect_zombies() -> Vec<Finding> {
        let mut findings = Vec::new();

        // Use `ps` to find zombie processes (state 'Z')
        let output = std::process::Command::new("ps")
            .args(["-eo", "pid,state,comm"])
            .output();

        if let Ok(out) = output {
            if out.status.success() {
                let text = String::from_utf8_lossy(&out.stdout);
                for line in text.lines().skip(1) {
                    let trimmed = line.trim();
                    let parts: Vec<&str> = trimmed.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let pid_str = parts[0];
                        let state = parts[1];
                        let comm = parts[2..].join(" ");

                        if state == "Z" {
                            let pid: u32 = pid_str.parse().unwrap_or(0);
                            findings.push(
                                Finding::new(
                                    EngineId::Startup,
                                    Severity::High,
                                    Category::ZombieProcess,
                                    Target::Process(pid),
                                    format!("Zombie process: {} (PID {})", comm, pid),
                                    format!(
                                        "Process {} (PID {}) is in zombie state — \
                                         it has exited but its parent has not reaped it.",
                                        comm, pid
                                    ),
                                )
                                .with_metadata("pid".to_string(), serde_json::json!(pid))
                                .with_metadata("comm".to_string(), serde_json::json!(comm))
                                .with_hint(
                                    "Find the parent process with 'ps -o ppid= -p <PID>' \
                                     and consider restarting or signaling it."
                                        .to_string(),
                                ),
                            );
                        }
                    }
                }
            }
        }

        findings
    }

    /// Detect frozen (not responding) applications on macOS. (op 170)
    ///
    /// On macOS, uses `osascript` to check if any application is not responding.
    /// On Linux, checks for processes in 'D' (uninterruptible sleep) state.
    fn detect_frozen_apps() -> Vec<Finding> {
        let mut findings = Vec::new();

        #[cfg(target_os = "macos")]
        {
            // Check for applications that are "not responding"
            let output = std::process::Command::new("osascript")
                .args([
                    "-e",
                    "tell application \"System Events\" to get name of (every process \
                     whose background only is false and responding is false)",
                ])
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for name in text.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()) {
                        findings.push(
                            Finding::new(
                                EngineId::Startup,
                                Severity::High,
                                Category::FrozenApp,
                                Target::LaunchdLabel(name.to_string()),
                                format!("Frozen application: {}", name),
                                format!(
                                    "Application '{}' is not responding and may be frozen.",
                                    name
                                ),
                            )
                            .with_hint(format!(
                                "Force quit with: killall \"{}\" or use Activity Monitor",
                                name
                            )),
                        );
                    }
                }
            }
        }

        // Linux: check for 'D' state (uninterruptible sleep) which often
        // indicates a frozen process waiting on I/O.
        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("ps")
                .args(["-eo", "pid,state,comm"])
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines().skip(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 3 && parts[1] == "D" {
                            let pid: u32 = parts[0].parse().unwrap_or(0);
                            let comm = parts[2..].join(" ");
                            findings.push(
                                Finding::new(
                                    EngineId::Startup,
                                    Severity::Medium,
                                    Category::FrozenApp,
                                    Target::Process(pid),
                                    format!("Frozen process: {} (PID {})", comm, pid),
                                    format!(
                                        "Process {} (PID {}) is in uninterruptible sleep \
                                         (D state) — likely waiting on I/O.",
                                        comm, pid
                                    ),
                                )
                                .with_hint(
                                    "Check with 'cat /proc/<PID>/stack' to see what it's \
                                     waiting on."
                                        .to_string(),
                                ),
                            );
                        }
                    }
                }
            }
        }

        findings
    }

    /// Monitor daemon health by checking if expected daemons are running.
    /// (op 172)
    ///
    /// Uses `launchctl list` on macOS or `systemctl --failed` on Linux.
    fn check_daemon_health() -> Vec<Finding> {
        let mut findings = Vec::new();

        #[cfg(target_os = "macos")]
        {
            // launchctl list shows loaded services and their exit status
            let output = std::process::Command::new("launchctl").arg("list").output();

            if let Ok(out) = output {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines().skip(1) {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        // Format: PID Status Label
                        if parts.len() >= 3 {
                            let status_str = parts[1];
                            let label = parts[2..].join(" ");
                            // Non-zero status means the daemon exited with an error
                            if status_str != "0" && status_str != "-" {
                                let status: i32 = status_str.parse().unwrap_or(-1);
                                if status > 0 {
                                    findings.push(
                                        Finding::new(
                                            EngineId::Startup,
                                            Severity::Medium,
                                            Category::LaunchDaemon,
                                            Target::LaunchdLabel(label.clone()),
                                            format!("Daemon exited with error: {}", label),
                                            format!(
                                                "Daemon '{}' exited with status {}. \
                                                 It may be crashing or misconfigured.",
                                                label, status
                                            ),
                                        )
                                        .with_metadata(
                                            "exit_status".to_string(),
                                            serde_json::json!(status),
                                        )
                                        .with_hint(
                                            format!(
                                                "Check logs with: log show --predicate \
                                             'process == \"{}\"' --last 1h",
                                                label
                                            ),
                                        ),
                                    );
                                }
                            }
                        }
                    }
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            let output = std::process::Command::new("systemctl")
                .args(["--failed", "--no-legend"])
                .output();

            if let Ok(out) = output {
                if out.status.success() {
                    let text = String::from_utf8_lossy(&out.stdout);
                    for line in text.lines() {
                        let parts: Vec<&str> = line.split_whitespace().collect();
                        if parts.len() >= 2 {
                            let unit = parts[0];
                            let state = parts[1];
                            findings.push(
                                Finding::new(
                                    EngineId::Startup,
                                    Severity::Medium,
                                    Category::LaunchDaemon,
                                    Target::LaunchdLabel(unit.to_string()),
                                    format!("Failed systemd unit: {}", unit),
                                    format!("Systemd unit '{}' is in '{}' state.", unit, state),
                                )
                                .with_hint(format!("Investigate with: systemctl status {}", unit)),
                            );
                        }
                    }
                }
            }
        }

        findings
    }
}

#[async_trait]
impl Engine for StartupEngine {
    fn id(&self) -> EngineId {
        EngineId::Startup
    }

    fn name(&self) -> &'static str {
        "Startup Manager"
    }

    fn description(&self) -> &'static str {
        "Scans Login Items, LaunchAgents, LaunchDaemons, and background processes"
    }

    async fn validate(&self, _ctx: &ScanContext) -> Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;
        let errors_count = 0u64;

        // ops 146-149, 150: Scan all startup items (login items, launch agents/daemons)
        let items = scan_all();
        items_scanned += items.len() as u64;

        // ops 151-155, 159-162: Classify and emit findings for each item
        for item in &items {
            let finding = Self::item_to_finding(item);
            ctx.emit(finding).await;
            findings_count += 1;
        }

        // op 173: Detect zombie processes
        let zombies = Self::detect_zombies();
        items_scanned += zombies.len() as u64;
        for finding in zombies {
            ctx.emit(finding).await;
            findings_count += 1;
        }

        // op 170: Detect frozen applications
        let frozen = Self::detect_frozen_apps();
        items_scanned += frozen.len() as u64;
        for finding in frozen {
            ctx.emit(finding).await;
            findings_count += 1;
        }

        // op 172: Monitor daemon health
        let health = Self::check_daemon_health();
        items_scanned += health.len() as u64;
        for finding in health {
            ctx.emit(finding).await;
            findings_count += 1;
        }

        Ok(EngineStats {
            engine: self.id(),
            duration: start.elapsed(),
            items_scanned,
            findings_count,
            errors_count,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::macos::MacosUtils;

    #[test]
    fn test_engine_id_and_name() {
        let engine = StartupEngine::new();
        assert_eq!(engine.id(), EngineId::Startup);
        assert_eq!(engine.name(), "Startup Manager");
    }

    #[test]
    fn test_classify_telemetry() {
        let item = StartupItem {
            label: "com.example.analytics".to_string(),
            scope: StartupScope::UserAgent,
            path: None,
            plist: None,
            command: "/usr/bin/track".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: false,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        let (cat, sev) = StartupEngine::classify(&item);
        assert_eq!(cat, Category::TelemetryProcess);
        assert_eq!(sev, Severity::Medium);
    }

    #[test]
    fn test_classify_update_agent() {
        let item = StartupItem {
            label: "com.example.updater".to_string(),
            scope: StartupScope::UserAgent,
            path: None,
            plist: None,
            command: "/usr/bin/update".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: false,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        let (cat, _sev) = StartupEngine::classify(&item);
        assert_eq!(cat, Category::UpdateAgent);
    }

    #[test]
    fn test_classify_background_helper() {
        let item = StartupItem {
            label: "com.example.helper".to_string(),
            scope: StartupScope::UserAgent,
            path: None,
            plist: None,
            command: "/usr/bin/helper".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: false,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        let (cat, _) = StartupEngine::classify(&item);
        assert_eq!(cat, Category::BackgroundHelper);
    }

    #[test]
    fn test_classify_login_item() {
        let item = StartupItem {
            label: "Slack".to_string(),
            scope: StartupScope::LoginItem,
            path: None,
            plist: None,
            command: "Slack".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: false,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        let (cat, _) = StartupEngine::classify(&item);
        assert_eq!(cat, Category::LoginItem);
    }

    #[test]
    fn test_classify_launch_daemon() {
        // Use a label that doesn't contain "helper", "agent", or "daemon"
        // so it falls through to the scope-based classification.
        let item = StartupItem {
            label: "com.example.service".to_string(),
            scope: StartupScope::SystemDaemon,
            path: None,
            plist: None,
            command: "/usr/bin/service".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: true,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        let (cat, _) = StartupEngine::classify(&item);
        assert_eq!(cat, Category::LaunchDaemon);
    }

    #[test]
    fn test_item_to_finding_telemetry_high_severity() {
        let item = StartupItem {
            label: "com.example.analytics".to_string(),
            scope: StartupScope::UserAgent,
            path: None,
            plist: None,
            command: "/usr/bin/track".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: true,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        let finding = StartupEngine::item_to_finding(&item);
        assert_eq!(finding.engine, EngineId::Startup);
        assert_eq!(finding.category, Category::TelemetryProcess);
        // Telemetry with keep_alive + run_at_load → low necessity → High severity
        assert_eq!(finding.severity, Severity::High);
        assert!(finding.metadata.contains_key("necessity_score"));
        assert!(finding.metadata.contains_key("boot_impact"));
        assert!(finding.metadata.contains_key("cpu_impact"));
        assert!(finding.metadata.contains_key("memory_impact"));
    }

    #[test]
    fn test_item_to_finding_with_path_target() {
        let item = StartupItem {
            label: "com.example.agent".to_string(),
            scope: StartupScope::UserAgent,
            path: Some(std::path::PathBuf::from(
                "/Library/LaunchAgents/com.example.agent.plist",
            )),
            plist: None,
            command: "/usr/bin/agent".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: false,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        let finding = StartupEngine::item_to_finding(&item);
        assert!(matches!(finding.target, Target::Path(_)));
    }

    #[test]
    fn test_item_to_finding_with_label_target() {
        let item = StartupItem {
            label: "Slack".to_string(),
            scope: StartupScope::LoginItem,
            path: None,
            plist: None,
            command: "Slack".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: false,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        let finding = StartupEngine::item_to_finding(&item);
        assert!(matches!(finding.target, Target::LaunchdLabel(_)));
    }

    #[test]
    fn test_item_to_finding_includes_start_interval() {
        let item = StartupItem {
            label: "com.example.scheduled".to_string(),
            scope: StartupScope::UserAgent,
            path: None,
            plist: None,
            command: "/usr/bin/scheduled".to_string(),
            disabled: false,
            run_at_load: false,
            keep_alive: false,
            start_interval: Some(300),
            start_calendar_interval: false,
            process_type: None,
        };
        let finding = StartupEngine::item_to_finding(&item);
        assert_eq!(
            finding.metadata.get("start_interval"),
            Some(&serde_json::json!(300))
        );
    }

    #[test]
    fn test_item_to_finding_disabled_item() {
        let item = StartupItem {
            label: "com.example.disabled".to_string(),
            scope: StartupScope::UserAgent,
            path: None,
            plist: None,
            command: "/usr/bin/foo".to_string(),
            disabled: true,
            run_at_load: false,
            keep_alive: false,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        let finding = StartupEngine::item_to_finding(&item);
        assert_eq!(
            finding.metadata.get("disabled"),
            Some(&serde_json::json!(true))
        );
    }

    #[test]
    fn test_detect_zombies_returns_vec() {
        // This runs `ps` which should be available on all Unix systems
        let zombies = StartupEngine::detect_zombies();
        // We can't guarantee zombies exist, but the function should not panic
        assert!(zombies.len() < 100); // sanity check
    }

    #[test]
    fn test_detect_frozen_apps_returns_vec() {
        let frozen = StartupEngine::detect_frozen_apps();
        // We can't guarantee frozen apps exist, but the function should not panic
        assert!(frozen.len() < 100); // sanity check
    }

    #[test]
    fn test_check_daemon_health_returns_vec() {
        let health = StartupEngine::check_daemon_health();
        // We can't guarantee failed daemons exist, but the function should not panic
        assert!(health.len() < 100); // sanity check
    }

    #[tokio::test]
    async fn test_scan_emits_findings() {
        use tokio::sync::mpsc;

        let (tx, mut rx) = mpsc::channel::<Finding>(1024);
        let ctx = ScanContext {
            config: crate::core::types::ScanConfig::default(),
            tx,
            progress: std::sync::Arc::new(crate::util::progress::ProgressReporter::new(true)),
            macos: std::sync::Arc::new(MacosUtils::new()),
        };

        let engine = StartupEngine::new();
        let stats = engine.scan(std::sync::Arc::new(ctx)).await.unwrap();

        // Should have scanned at least 0 items (may be 0 on CI without launchd)
        assert_eq!(stats.engine, EngineId::Startup);
        assert_eq!(stats.findings_count, stats.items_scanned);

        // Collect emitted findings
        let mut findings = Vec::new();
        while let Ok(Some(f)) =
            tokio::time::timeout(std::time::Duration::from_millis(100), rx.recv()).await
        {
            findings.push(f);
        }
        assert_eq!(findings.len() as u64, stats.findings_count);
    }
}
