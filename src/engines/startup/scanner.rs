//! Startup item scanning — Login Items, LaunchAgents, LaunchDaemons, and
//! systemd units.
//!
//! Implements operations 146-149 (scan login items, launch agents/daemons)
//! and 159 (detect delayed launch apps).

use std::path::{Path, PathBuf};

use super::plist_parser::{parse_plist, ParsedPlist};

/// The scope/origin of a startup item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupScope {
    /// User-level LaunchAgent (`~/Library/LaunchAgents`).
    UserAgent,
    /// System-level LaunchAgent (`/Library/LaunchAgents`).
    SystemAgent,
    /// System-level LaunchDaemon (`/Library/LaunchDaemons`).
    SystemDaemon,
    /// macOS Login Item (via System Events).
    LoginItem,
    /// Linux systemd user unit.
    #[allow(dead_code)]
    SystemdUser,
    /// Linux systemd system unit.
    #[allow(dead_code)]
    SystemdSystem,
}

impl StartupScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::UserAgent => "user-agent",
            Self::SystemAgent => "system-agent",
            Self::SystemDaemon => "system-daemon",
            Self::LoginItem => "login-item",
            Self::SystemdUser => "systemd-user",
            Self::SystemdSystem => "systemd-system",
        }
    }
}

/// A discovered startup item (LaunchAgent, LaunchDaemon, Login Item, or
/// systemd unit).
#[derive(Debug, Clone)]
pub struct StartupItem {
    /// The label / identifier (launchd `Label` or systemd unit name).
    pub label: String,
    /// The scope where this item was found.
    pub scope: StartupScope,
    /// The path to the plist or unit file (if applicable).
    pub path: Option<PathBuf>,
    /// The parsed plist contents (for launchd items).
    #[allow(dead_code)]
    pub plist: Option<ParsedPlist>,
    /// The command string that will be executed.
    pub command: String,
    /// Whether the item is disabled.
    pub disabled: bool,
    /// Whether the item runs at boot/login.
    pub run_at_load: bool,
    /// Whether the item is kept alive (auto-restarted).
    pub keep_alive: bool,
    /// Scheduled start interval in seconds (if any).
    pub start_interval: Option<u64>,
    /// Whether the item has a calendar-based schedule.
    pub start_calendar_interval: bool,
    /// The process type hint (e.g. "Background").
    pub process_type: Option<String>,
}

impl StartupItem {
    /// Whether this item launches at boot or login.
    pub fn launches_at_boot(&self) -> bool {
        self.run_at_load || self.keep_alive
    }

    /// Whether this is a scheduled (delayed/periodic) item. (op 159)
    pub fn is_delayed_launch(&self) -> bool {
        self.start_interval.is_some() || self.start_calendar_interval
    }

    /// Whether the label indicates an update agent. (op 153)
    pub fn is_update_agent(&self) -> bool {
        let lower = self.label.to_lowercase();
        lower.contains("update") || lower.contains("updater")
    }

    /// Whether the label indicates a telemetry/analytics process. (op 154)
    pub fn is_telemetry(&self) -> bool {
        let lower = self.label.to_lowercase();
        lower.contains("telemetry")
            || lower.contains("analytics")
            || lower.contains("tracking")
            || lower.contains("metrics")
    }

    /// Whether the item looks like a background helper. (op 152)
    pub fn is_background_helper(&self) -> bool {
        let lower = self.label.to_lowercase();
        lower.contains("helper") || lower.contains("agent") || lower.contains("daemon")
    }

    /// Heuristic necessity score (0.0 = unnecessary, 1.0 = essential).
    /// (op 155)
    ///
    /// Lower scores indicate items that may be unnecessary:
    /// - Telemetry processes score low
    /// - Update agents score medium
    /// - Apple system services score high
    /// - Disabled items are not scored (they don't run)
    pub fn necessity_score(&self) -> f64 {
        if self.disabled {
            return 1.0; // disabled items aren't running, not "unnecessary"
        }

        let mut score: f64 = 0.5; // baseline

        // Apple system services are essential
        if self.label.starts_with("com.apple.") {
            score += 0.4;
        }

        // Telemetry processes are less necessary
        if self.is_telemetry() {
            score -= 0.3;
        }

        // Update agents are moderately necessary
        if self.is_update_agent() {
            score -= 0.1;
        }

        // KeepAlive services that run at boot have more impact
        if self.keep_alive && self.run_at_load {
            score -= 0.05;
        }

        // Scheduled (non-boot) items have lower impact
        if self.is_delayed_launch() && !self.launches_at_boot() {
            score += 0.1;
        }

        score.clamp(0.0, 1.0)
    }

    /// Estimated CPU impact at boot (0.0 = none, 1.0 = high). (op 161)
    pub fn cpu_impact(&self) -> f64 {
        let mut impact: f64 = 0.0;

        if self.run_at_load {
            impact += 0.3;
        }
        if self.keep_alive {
            impact += 0.2;
        }
        if self.is_delayed_launch() {
            impact += 0.1;
        }
        // Telemetry/update agents tend to do network I/O at launch
        if self.is_telemetry() || self.is_update_agent() {
            impact += 0.1;
        }

        impact.min(1.0)
    }

    /// Estimated memory impact (0.0 = none, 1.0 = high). (op 162)
    pub fn memory_impact(&self) -> f64 {
        let mut impact: f64 = 0.0;

        if self.keep_alive {
            impact += 0.4; // persistent process
        }
        if self.run_at_load {
            impact += 0.2;
        }
        // Background helpers tend to be lightweight but persistent
        if self.is_background_helper() && self.keep_alive {
            impact += 0.1;
        }

        impact.min(1.0)
    }

    /// Estimated boot impact score (0.0 = none, 1.0 = high). (op 160)
    pub fn boot_impact(&self) -> f64 {
        if !self.launches_at_boot() {
            return 0.0;
        }

        let cpu = self.cpu_impact();
        let mem = self.memory_impact();
        // Boot impact is a blend of CPU and memory impact
        (cpu * 0.6 + mem * 0.4).min(1.0)
    }
}

// ---------------------------------------------------------------------------
// macOS scanning
// ---------------------------------------------------------------------------

/// Scan macOS Login Items via `osascript`. (op 146)
///
/// Returns a list of login item names. Returns an empty vec if osascript
/// is unavailable or the command fails.
#[cfg(target_os = "macos")]
pub fn scan_login_items() -> Vec<String> {
    let output = std::process::Command::new("osascript")
        .args([
            "-e",
            "tell application \"System Events\" to get the name of every login item",
        ])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let text = String::from_utf8_lossy(&out.stdout);
            text.split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect()
        }
        _ => Vec::new(),
    }
}

/// Non-macOS stub for scanning login items.
#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
pub fn scan_login_items() -> Vec<String> {
    Vec::new()
}

/// Build a `StartupItem` from a login item name. (op 146)
pub fn login_item_to_startup(name: &str) -> StartupItem {
    StartupItem {
        label: name.to_string(),
        scope: StartupScope::LoginItem,
        path: None,
        plist: None,
        command: name.to_string(),
        disabled: false,
        run_at_load: true,
        keep_alive: false,
        start_interval: None,
        start_calendar_interval: false,
        process_type: None,
    }
}

/// Scan a LaunchAgents/LaunchDaemons directory for plist files. (op 147-149)
///
/// Parses each `.plist` file and returns a `StartupItem` per file.
pub fn scan_launchd_dir(dir: &Path, scope: StartupScope) -> Vec<StartupItem> {
    let mut items = Vec::new();

    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return items,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("plist") {
            continue;
        }

        let plist = match parse_plist(&path) {
            Ok(p) => p,
            Err(_) => continue,
        };

        items.push(StartupItem {
            label: plist.label.clone(),
            scope,
            path: Some(path),
            command: plist.command_string(),
            disabled: plist.disabled,
            run_at_load: plist.run_at_load,
            keep_alive: plist.keep_alive,
            start_interval: plist.start_interval,
            start_calendar_interval: plist.start_calendar_interval,
            process_type: plist.process_type.clone(),
            plist: Some(plist),
        });
    }

    items
}

/// Standard macOS LaunchAgent/LaunchDaemon directories.
#[cfg(target_os = "macos")]
#[allow(dead_code)]
pub fn macos_launchd_dirs() -> Vec<(PathBuf, StartupScope)> {
    let home = crate::util::macos::MacosUtils::home_dir();
    vec![
        (home.join("Library/LaunchAgents"), StartupScope::UserAgent),
        (
            PathBuf::from("/Library/LaunchAgents"),
            StartupScope::SystemAgent,
        ),
        (
            PathBuf::from("/Library/LaunchDaemons"),
            StartupScope::SystemDaemon,
        ),
    ]
}

// ---------------------------------------------------------------------------
// Linux scanning (systemd)
// ---------------------------------------------------------------------------

/// Scan systemd units (user or system) for enabled service files. (op 147-149
/// Linux equivalent)
#[cfg(target_os = "linux")]
pub fn scan_systemd_units(scope: StartupScope) -> Vec<StartupItem> {
    let (flag, unit_dir): (&str, PathBuf) = match scope {
        StartupScope::SystemdUser => (
            "--user",
            crate::util::macos::MacosUtils::home_dir().join(".config/systemd/user"),
        ),
        _ => ("--system", PathBuf::from("/etc/systemd/system")),
    };

    let output = std::process::Command::new("systemctl")
        .args([flag, "list-unit-files", "--type=service", "--no-legend"])
        .output();

    let mut items = Vec::new();
    if let Ok(out) = output {
        if out.status.success() {
            let text = String::from_utf8_lossy(&out.stdout);
            for line in text.lines() {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 && parts[0].ends_with(".service") {
                    let unit_name = parts[0];
                    let state = parts[1];
                    let path = unit_dir.join(unit_name);
                    items.push(StartupItem {
                        label: unit_name.to_string(),
                        scope,
                        path: if path.exists() { Some(path) } else { None },
                        plist: None,
                        command: unit_name.to_string(),
                        disabled: state != "enabled",
                        run_at_load: state == "enabled",
                        keep_alive: false,
                        start_interval: None,
                        start_calendar_interval: false,
                        process_type: None,
                    });
                }
            }
        }
    }
    items
}

/// Non-Linux stub for systemd scanning.
#[cfg(not(target_os = "linux"))]
#[allow(dead_code)]
pub fn scan_systemd_units(_scope: StartupScope) -> Vec<StartupItem> {
    Vec::new()
}

/// Scan all startup items on the current platform.
///
/// On macOS: Login Items + LaunchAgents (user + system) + LaunchDaemons.
/// On Linux: systemd user + system units.
pub fn scan_all() -> Vec<StartupItem> {
    let mut items = Vec::new();

    #[cfg(target_os = "macos")]
    {
        // op 146: Login Items
        for name in scan_login_items() {
            items.push(login_item_to_startup(&name));
        }
        // op 147-149: LaunchAgents and LaunchDaemons
        for (dir, scope) in macos_launchd_dirs() {
            items.extend(scan_launchd_dir(&dir, scope));
        }
    }

    #[cfg(target_os = "linux")]
    {
        items.extend(scan_systemd_units(StartupScope::SystemdUser));
        items.extend(scan_systemd_units(StartupScope::SystemdSystem));
    }

    items
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn write_plist(dir: &Path, name: &str, content: &str) -> PathBuf {
        let path = dir.join(name);
        let mut f = std::fs::File::create(&path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
        path
    }

    const PLIST_TEMPLATE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{LABEL}</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/bin/test</string>
    </array>
    <key>RunAtLoad</key>
    <{RUN_AT_LOAD}/>
    <key>KeepAlive</key>
    <{KEEP_ALIVE}/>
</dict>
</plist>"#;

    #[test]
    fn test_scan_launchd_dir_parses_plists() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();

        let plist1 = PLIST_TEMPLATE
            .replace("{LABEL}", "com.example.agent1")
            .replace("{RUN_AT_LOAD}", "true")
            .replace("{KEEP_ALIVE}", "false");
        write_plist(&dir, "com.example.agent1.plist", &plist1);

        let plist2 = PLIST_TEMPLATE
            .replace("{LABEL}", "com.example.agent2")
            .replace("{RUN_AT_LOAD}", "false")
            .replace("{KEEP_ALIVE}", "true");
        write_plist(&dir, "com.example.agent2.plist", &plist2);

        // Non-plist file should be ignored
        write_plist(&dir, "readme.txt", "not a plist");

        let items = scan_launchd_dir(&dir, StartupScope::UserAgent);
        assert_eq!(items.len(), 2);
        assert!(items.iter().any(|i| i.label == "com.example.agent1"));
        assert!(items.iter().any(|i| i.label == "com.example.agent2"));
    }

    #[test]
    fn test_scan_launchd_dir_nonexistent_returns_empty() {
        let items = scan_launchd_dir(Path::new("/nonexistent/path"), StartupScope::UserAgent);
        assert!(items.is_empty());
    }

    #[test]
    fn test_scan_launchd_dir_skips_invalid_plists() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();

        write_plist(&dir, "bad.plist", "not valid xml");
        let valid = PLIST_TEMPLATE
            .replace("{LABEL}", "com.example.good")
            .replace("{RUN_AT_LOAD}", "true")
            .replace("{KEEP_ALIVE}", "false");
        write_plist(&dir, "com.example.good.plist", &valid);

        let items = scan_launchd_dir(&dir, StartupScope::SystemAgent);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].label, "com.example.good");
    }

    #[test]
    fn test_startup_item_is_update_agent() {
        let item = StartupItem {
            label: "com.example.update-agent".to_string(),
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
        assert!(item.is_update_agent());
        assert!(!item.is_telemetry());
    }

    #[test]
    fn test_startup_item_is_telemetry() {
        let item = StartupItem {
            label: "com.example.analytics-tracker".to_string(),
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
        assert!(item.is_telemetry());
        assert!(!item.is_update_agent());
    }

    #[test]
    fn test_startup_item_is_background_helper() {
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
        assert!(item.is_background_helper());
    }

    #[test]
    fn test_startup_item_delayed_launch() {
        let mut item = StartupItem {
            label: "com.example.scheduled".to_string(),
            scope: StartupScope::UserAgent,
            path: None,
            plist: None,
            command: "/usr/bin/scheduled".to_string(),
            disabled: false,
            run_at_load: false,
            keep_alive: false,
            start_interval: Some(600),
            start_calendar_interval: false,
            process_type: None,
        };
        assert!(item.is_delayed_launch());

        item.start_interval = None;
        item.start_calendar_interval = true;
        assert!(item.is_delayed_launch());
    }

    #[test]
    fn test_necessity_score_apple_high() {
        let item = StartupItem {
            label: "com.apple.system.daemon".to_string(),
            scope: StartupScope::SystemDaemon,
            path: None,
            plist: None,
            command: "/usr/libexec/daemon".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: true,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        assert!(item.necessity_score() > 0.7);
    }

    #[test]
    fn test_necessity_score_telemetry_low() {
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
        assert!(item.necessity_score() < 0.5);
    }

    #[test]
    fn test_necessity_score_disabled_is_neutral() {
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
        assert_eq!(item.necessity_score(), 1.0);
    }

    #[test]
    fn test_boot_impact_zero_for_non_boot_items() {
        let item = StartupItem {
            label: "com.example.scheduled".to_string(),
            scope: StartupScope::UserAgent,
            path: None,
            plist: None,
            command: "/usr/bin/scheduled".to_string(),
            disabled: false,
            run_at_load: false,
            keep_alive: false,
            start_interval: Some(3600),
            start_calendar_interval: false,
            process_type: None,
        };
        assert_eq!(item.boot_impact(), 0.0);
    }

    #[test]
    fn test_boot_impact_positive_for_boot_items() {
        let item = StartupItem {
            label: "com.example.boot".to_string(),
            scope: StartupScope::SystemDaemon,
            path: None,
            plist: None,
            command: "/usr/bin/boot".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: true,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        assert!(item.boot_impact() > 0.0);
    }

    #[test]
    fn test_cpu_impact_and_memory_impact() {
        let item = StartupItem {
            label: "com.example.heavy".to_string(),
            scope: StartupScope::SystemDaemon,
            path: None,
            plist: None,
            command: "/usr/bin/heavy".to_string(),
            disabled: false,
            run_at_load: true,
            keep_alive: true,
            start_interval: None,
            start_calendar_interval: false,
            process_type: None,
        };
        assert!(item.cpu_impact() > 0.0);
        assert!(item.memory_impact() > 0.0);
    }

    #[test]
    fn test_login_item_to_startup() {
        let item = login_item_to_startup("Slack");
        assert_eq!(item.label, "Slack");
        assert_eq!(item.scope, StartupScope::LoginItem);
        assert!(item.run_at_load);
        assert!(!item.keep_alive);
    }

    #[test]
    fn test_scope_as_str() {
        assert_eq!(StartupScope::UserAgent.as_str(), "user-agent");
        assert_eq!(StartupScope::SystemDaemon.as_str(), "system-daemon");
        assert_eq!(StartupScope::LoginItem.as_str(), "login-item");
    }
}
