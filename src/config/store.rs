use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use super::profiles::OptimizationProfile;

// ═══════════════════════════════════════════════════════════════════════
//  Top-level configuration
// ═══════════════════════════════════════════════════════════════════════

/// The complete X-MaC configuration, loaded from ~/.config/xmac/config.toml.
///
/// Every section has sensible defaults so the tool works out of the box
/// even when no config file exists. Missing fields are merged from defaults
/// on load, so partial configs are always valid.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Currently active optimization profile.
    #[serde(default)]
    pub profile: OptimizationProfile,

    /// Clean engine settings.
    #[serde(default)]
    pub clean: CleanConfig,

    /// Maintain engine settings.
    #[serde(default)]
    pub maintain: MaintainConfig,

    /// Optimize / memory engine settings.
    #[serde(default)]
    pub optimize: OptimizeConfig,

    /// Daemon settings (background scheduling).
    #[serde(default)]
    pub daemon: DaemonConfig,

    /// Notification settings.
    #[serde(default)]
    pub notifications: NotificationConfig,

    /// Automation rules (safe, user-approved).
    #[serde(default)]
    pub automation: Vec<AutomationRule>,

    /// Adaptive learning state — tracks user feedback to adjust behavior.
    #[serde(default)]
    pub adaptive: AdaptiveState,

    /// History retention settings.
    #[serde(default)]
    pub history: HistoryConfig,

    /// Logging settings.
    #[serde(default)]
    pub logging: LoggingConfig,

    /// Custom exclude paths (in addition to built-in safety excludes).
    #[serde(default)]
    pub exclude_paths: Vec<String>,

    /// Custom include paths for scanning.
    #[serde(default)]
    pub include_paths: Vec<String>,

    /// Version of the config schema (for migrations).
    #[serde(default = "default_config_version")]
    pub config_version: u32,
}

fn default_config_version() -> u32 {
    1
}

impl Default for Config {
    fn default() -> Self {
        Self {
            profile: OptimizationProfile::default(),
            clean: CleanConfig::default(),
            maintain: MaintainConfig::default(),
            optimize: OptimizeConfig::default(),
            daemon: DaemonConfig::default(),
            notifications: NotificationConfig::default(),
            automation: Vec::new(),
            adaptive: AdaptiveState::default(),
            history: HistoryConfig::default(),
            logging: LoggingConfig::default(),
            exclude_paths: Vec::new(),
            include_paths: Vec::new(),
            config_version: 1,
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Section configs
// ═══════════════════════════════════════════════════════════════════════

/// Clean engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanConfig {
    /// Minimum file age to flag (in days).
    #[serde(default = "default_min_age")]
    pub min_age_days: u64,

    /// Minimum file size to flag (in MB).
    #[serde(default = "default_min_size_mb")]
    pub min_size_mb: u64,

    /// Detect duplicate files via BLAKE3 hashing.
    #[serde(default)]
    pub dedup: bool,

    /// Category toggles.
    #[serde(default = "default_true")]
    pub xcode: bool,
    #[serde(default = "default_true")]
    pub pkg_caches: bool,
    #[serde(default = "default_true")]
    pub docker: bool,
    #[serde(default = "default_true")]
    pub temp: bool,
    #[serde(default = "default_true")]
    pub build_artifacts: bool,
    #[serde(default = "default_true")]
    pub browser: bool,
    #[serde(default = "default_true")]
    pub mail: bool,
    #[serde(default = "default_true")]
    pub ios_backups: bool,
    #[serde(default = "default_true")]
    pub languages: bool,
    #[serde(default = "default_true")]
    pub trash: bool,
    #[serde(default = "default_true")]
    pub large_files: bool,

    /// Detect orphaned application support directories.
    #[serde(default = "default_true")]
    pub orphans: bool,

    /// Resource usage mode: "eco", "balanced", or "turbo".
    #[serde(default = "default_resource_mode")]
    pub resource_mode: String,

    /// Minimum size for large file detection (in MB).
    #[serde(default = "default_large_file_mb")]
    pub min_large_size_mb: u64,
}

fn default_min_age() -> u64 {
    30
}
fn default_min_size_mb() -> u64 {
    1
}
fn default_large_file_mb() -> u64 {
    100
}
fn default_true() -> bool {
    true
}
fn default_resource_mode() -> String {
    "balanced".to_string()
}

impl Default for CleanConfig {
    fn default() -> Self {
        Self {
            min_age_days: default_min_age(),
            min_size_mb: default_min_size_mb(),
            dedup: false,
            xcode: true,
            pkg_caches: true,
            docker: true,
            temp: true,
            build_artifacts: true,
            browser: true,
            mail: true,
            ios_backups: true,
            languages: true,
            trash: true,
            large_files: true,
            orphans: true,
            resource_mode: default_resource_mode(),
            min_large_size_mb: default_large_file_mb(),
        }
    }
}

/// Maintain engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MaintainConfig {
    #[serde(default = "default_true")]
    pub dns: bool,
    #[serde(default = "default_true")]
    pub spotlight: bool,
    #[serde(default = "default_true")]
    pub launchservices: bool,
    #[serde(default = "default_true")]
    pub periodic: bool,
    #[serde(default)]
    pub repair_permissions: bool,
    #[serde(default = "default_true")]
    pub purge_ram: bool,
    #[serde(default)]
    pub dyld: bool,
    #[serde(default = "default_true")]
    pub quicklook: bool,
}

impl Default for MaintainConfig {
    fn default() -> Self {
        Self {
            dns: true,
            spotlight: true,
            launchservices: true,
            periodic: true,
            repair_permissions: false,
            purge_ram: true,
            dyld: false,
            quicklook: true,
        }
    }
}

/// Optimize / memory engine configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeConfig {
    /// Maximum number of process nodes in the memory graph.
    #[serde(default = "default_max_processes")]
    pub max_processes: usize,

    /// Exclude system processes from the graph.
    #[serde(default = "default_true")]
    pub exclude_system: bool,

    /// Telemetry ring buffer size.
    #[serde(default = "default_buffer_size")]
    pub buffer_size: usize,

    /// Number of top consumers to report.
    #[serde(default = "default_top_n")]
    pub top_n: usize,

    /// Enable proactive pressure predictions.
    #[serde(default = "default_true")]
    pub proactive_predictions: bool,

    /// Memory pressure threshold (0.0–1.0) for warnings.
    #[serde(default = "default_pressure_threshold")]
    pub pressure_threshold: f64,

    /// Enable the AI advisor for natural-language recommendations.
    #[serde(default = "default_true")]
    pub ai_advisor: bool,
}

fn default_max_processes() -> usize {
    50
}
fn default_buffer_size() -> usize {
    288
}
fn default_top_n() -> usize {
    10
}
fn default_pressure_threshold() -> f64 {
    0.80
}

impl Default for OptimizeConfig {
    fn default() -> Self {
        Self {
            max_processes: default_max_processes(),
            exclude_system: true,
            buffer_size: default_buffer_size(),
            top_n: default_top_n(),
            proactive_predictions: true,
            pressure_threshold: default_pressure_threshold(),
            ai_advisor: true,
        }
    }
}

/// Daemon configuration — background scheduling.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DaemonConfig {
    /// Enable the background daemon.
    #[serde(default)]
    pub enabled: bool,

    /// Check interval in seconds.
    #[serde(default = "default_daemon_interval")]
    pub interval_secs: u64,

    /// Automatically run cleanup when reclaimable space exceeds this (in MB).
    /// 0 = disabled.
    #[serde(default = "default_auto_clean_mb")]
    pub auto_clean_threshold_mb: u64,

    /// Automatically purge memory when pressure exceeds threshold.
    #[serde(default = "default_true")]
    pub auto_purge_memory: bool,

    /// Run maintenance tasks on a schedule.
    #[serde(default = "default_maintenance_interval")]
    pub maintenance_interval_secs: u64,

    /// Collect telemetry snapshots for trend analysis.
    #[serde(default = "default_true")]
    pub collect_telemetry: bool,

    /// Telemetry collection interval in seconds.
    #[serde(default = "default_telemetry_interval")]
    pub telemetry_interval_secs: u64,

    /// PID file location (for single-instance enforcement).
    #[serde(default = "default_pid_file")]
    pub pid_file: PathBuf,
}

fn default_daemon_interval() -> u64 {
    300
}
fn default_auto_clean_mb() -> u64 {
    0
}
fn default_maintenance_interval() -> u64 {
    86400
}
fn default_telemetry_interval() -> u64 {
    60
}
fn default_pid_file() -> PathBuf {
    dirs::cache_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("xmac")
        .join("xmac.pid")
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            interval_secs: default_daemon_interval(),
            auto_clean_threshold_mb: default_auto_clean_mb(),
            auto_purge_memory: true,
            maintenance_interval_secs: default_maintenance_interval(),
            collect_telemetry: true,
            telemetry_interval_secs: default_telemetry_interval(),
            pid_file: default_pid_file(),
        }
    }
}

/// Notification configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationConfig {
    /// Enable macOS notification center alerts.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// Notify on high memory pressure.
    #[serde(default = "default_true")]
    pub memory_pressure: bool,

    /// Notify on large reclaimable space.
    #[serde(default = "default_true")]
    pub reclaimable_space: bool,

    /// Threshold for reclaimable space notifications (in MB).
    #[serde(default = "default_notify_space_mb")]
    pub space_threshold_mb: u64,

    /// Notify on predicted critical pressure.
    #[serde(default = "default_true")]
    pub proactive_warnings: bool,
}

fn default_notify_space_mb() -> u64 {
    500
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            memory_pressure: true,
            reclaimable_space: true,
            space_threshold_mb: default_notify_space_mb(),
            proactive_warnings: true,
        }
    }
}

/// A safe automation rule — runs an action when a condition is met.
/// All rules require explicit user approval in the config file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutomationRule {
    /// Unique name for this rule.
    pub name: String,
    /// Enable this rule.
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Condition that triggers the rule.
    pub condition: AutomationCondition,
    /// Action to take when triggered.
    pub action: AutomationAction,
    /// Minimum cooldown between triggers (in seconds).
    #[serde(default = "default_cooldown")]
    pub cooldown_secs: u64,
    /// Last time this rule fired (epoch seconds, internal).
    #[serde(default, skip)]
    pub last_fired: u64,
}

fn default_cooldown() -> u64 {
    3600
}

/// Conditions that can trigger automation rules.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "params")]
pub enum AutomationCondition {
    /// Memory pressure exceeds threshold (0.0–1.0).
    MemoryPressure { threshold: f64 },
    /// Free disk space drops below threshold (in GB).
    DiskSpaceLow { threshold_gb: u64 },
    /// Reclaimable space exceeds threshold (in MB).
    ReclaimableSpace { threshold_mb: u64 },
    /// A specific process is consuming more than X MB of RAM.
    ProcessMemory {
        process_name: String,
        threshold_mb: u64,
    },
    /// Time-based trigger (cron-like: every N hours).
    Scheduled { interval_hours: u64 },
}

/// Actions that automation rules can take.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", content = "params")]
pub enum AutomationAction {
    /// Purge inactive memory.
    PurgeMemory,
    /// Run a cleanup scan (dry-run only — never auto-delete).
    ScanClean,
    /// Run maintenance tasks.
    RunMaintenance,
    /// Kill a specific process.
    KillProcess { name: String },
    /// Send a notification only (no action).
    Notify { message: String },
}

/// Adaptive learning state — tracks user feedback to adjust behavior.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdaptiveState {
    /// Whether adaptive learning is enabled.
    #[serde(default = "default_true")]
    pub enabled: bool,

    /// User feedback entries (action taken / dismissed).
    #[serde(default)]
    pub feedback: Vec<FeedbackEntry>,

    /// Adjusted weights for recommendation scoring (category → weight).
    #[serde(default)]
    pub category_weights: HashMap<String, f64>,

    /// How many times the user has accepted vs dismissed each category.
    #[serde(default)]
    pub category_acceptance: HashMap<String, (u64, u64)>,

    /// Last time the model was updated (epoch seconds).
    #[serde(default)]
    pub last_updated: u64,
}

/// A single user feedback entry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub timestamp: u64,
    pub category: String,
    pub accepted: bool,
    pub severity: String,
}

#[allow(dead_code)]
impl AdaptiveState {
    pub fn record_feedback(&mut self, category: &str, severity: &str, accepted: bool) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        self.feedback.push(FeedbackEntry {
            timestamp: now,
            category: category.to_string(),
            severity: severity.to_string(),
            accepted,
        });

        // Keep only the last 500 entries.
        if self.feedback.len() > 500 {
            self.feedback.drain(0..self.feedback.len() - 500);
        }

        // Update acceptance counts.
        let entry = self
            .category_acceptance
            .entry(category.to_string())
            .or_insert((0, 0));
        if accepted {
            entry.0 += 1;
        } else {
            entry.1 += 1;
        }

        // Recompute weight: acceptance rate with smoothing.
        let (accepted_n, dismissed_n) = *entry;
        let total = accepted_n + dismissed_n;
        if total > 0 {
            let _rate = accepted_n as f64 / total as f64;
            // Smooth with a prior of 0.5 (5 pseudo-observations).
            let smoothed = (accepted_n as f64 + 2.5) / (total as f64 + 5.0);
            self.category_weights.insert(category.to_string(), smoothed);
        }

        self.last_updated = now;
    }

    /// Get the adjusted weight for a category (default 0.5 if unknown).
    pub fn weight_for(&self, category: &str) -> f64 {
        self.category_weights.get(category).copied().unwrap_or(0.5)
    }

    /// Whether the user tends to accept this category (weight > 0.5).
    pub fn is_preferred(&self, category: &str) -> bool {
        self.weight_for(category) > 0.5
    }
}

/// History retention configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryConfig {
    /// Maximum number of scan snapshots to retain.
    #[serde(default = "default_history_limit")]
    pub max_snapshots: usize,

    /// Maximum number of cleanup transactions to retain.
    #[serde(default = "default_history_limit")]
    pub max_transactions: usize,

    /// History file path.
    #[serde(default = "default_history_path")]
    pub path: PathBuf,
}

fn default_history_limit() -> usize {
    100
}
fn default_history_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("xmac")
        .join("history.json")
}

impl Default for HistoryConfig {
    fn default() -> Self {
        Self {
            max_snapshots: default_history_limit(),
            max_transactions: default_history_limit(),
            path: default_history_path(),
        }
    }
}

/// Logging configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    /// Log level: trace, debug, info, warn, error.
    #[serde(default = "default_log_level")]
    pub level: String,

    /// Write logs to a file in addition to stderr.
    #[serde(default = "default_true")]
    pub file_logging: bool,

    /// Log directory.
    #[serde(default = "default_log_dir")]
    pub dir: PathBuf,

    /// Maximum log file size in MB (rotates when exceeded).
    #[serde(default = "default_log_max_mb")]
    pub max_file_mb: u64,

    /// Number of rotated log files to keep.
    #[serde(default = "default_log_keep")]
    pub keep_files: usize,
}

fn default_log_level() -> String {
    "warn".to_string()
}
fn default_log_dir() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join("xmac")
        .join("logs")
}
fn default_log_max_mb() -> u64 {
    10
}
fn default_log_keep() -> usize {
    5
}

impl Default for LoggingConfig {
    fn default() -> Self {
        Self {
            level: default_log_level(),
            file_logging: true,
            dir: default_log_dir(),
            max_file_mb: default_log_max_mb(),
            keep_files: default_log_keep(),
        }
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Config manager — load, save, merge
// ═══════════════════════════════════════════════════════════════════════

/// Manages loading and saving the X-MaC configuration.
pub struct ConfigManager {
    config: Config,
    path: PathBuf,
}

#[allow(dead_code)]
impl ConfigManager {
    /// Default config path: ~/.config/xmac/config.toml
    pub fn default_config_path() -> PathBuf {
        if let Some(config_dir) = dirs::config_dir() {
            config_dir.join("xmac").join("config.toml")
        } else {
            PathBuf::from("/tmp/xmac-config.toml")
        }
    }

    /// Load config from the default path, creating it with defaults if missing.
    pub fn load() -> Self {
        Self::load_from(&Self::default_config_path())
    }

    /// Load config from a specific path.
    pub fn load_from(path: &Path) -> Self {
        let config = if path.exists() {
            match std::fs::read_to_string(path) {
                Ok(content) => Self::parse(&content).unwrap_or_default(),
                Err(_) => Config::default(),
            }
        } else {
            Config::default()
        };

        Self {
            config,
            path: path.to_path_buf(),
        }
    }

    /// Parse a TOML string into a Config, merging defaults for missing fields.
    pub fn parse(toml_str: &str) -> Result<Config, toml::de::Error> {
        toml::from_str::<Config>(toml_str)
    }

    /// Save the current config to disk.
    pub fn save(&self) -> Result<(), String> {
        self.save_to(&self.path)
    }

    /// Save config to a specific path.
    pub fn save_to(&self, path: &Path) -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("create config dir: {e}"))?;
        }
        let toml_str =
            toml::to_string_pretty(&self.config).map_err(|e| format!("serialize config: {e}"))?;
        std::fs::write(path, toml_str).map_err(|e| format!("write config: {e}"))
    }

    /// Write the default config to disk (creates the file if it doesn't exist).
    pub fn ensure_config_file(&self) -> Result<(), String> {
        if !self.path.exists() {
            self.save()?;
        }
        Ok(())
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn config_mut(&mut self) -> &mut Config {
        &mut self.config
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Set the active optimization profile and adjust related defaults.
    pub fn set_profile(&mut self, profile: OptimizationProfile) {
        self.config.profile = profile;
        // Apply profile-derived defaults to clean config.
        self.config.clean.min_age_days = profile.min_age_days();
    }

    /// Record user feedback for adaptive learning.
    pub fn record_feedback(&mut self, category: &str, severity: &str, accepted: bool) {
        self.config
            .adaptive
            .record_feedback(category, severity, accepted);
    }

    /// Generate a default config TOML string (for `xmac config init`).
    pub fn default_toml() -> String {
        let config = Config::default();
        toml::to_string_pretty(&config).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config_roundtrip() {
        let config = Config::default();
        let toml_str = toml::to_string(&config).expect("serialize");
        let parsed: Config = toml::from_str(&toml_str).expect("deserialize");
        assert_eq!(parsed.clean.min_age_days, config.clean.min_age_days);
        assert_eq!(parsed.optimize.max_processes, config.optimize.max_processes);
        assert_eq!(parsed.daemon.interval_secs, config.daemon.interval_secs);
    }

    #[test]
    fn test_partial_config_uses_defaults() {
        let toml_str = r#"
[clean]
min_age_days = 7
"#;
        let config = ConfigManager::parse(toml_str).expect("parse partial");
        assert_eq!(config.clean.min_age_days, 7);
        // Other fields should use defaults.
        assert_eq!(config.optimize.max_processes, 50);
        assert_eq!(config.daemon.interval_secs, 300);
    }

    #[test]
    fn test_adaptive_learning() {
        let mut state = AdaptiveState::default();
        state.record_feedback("cache", "medium", true);
        state.record_feedback("cache", "medium", true);
        state.record_feedback("cache", "medium", false);

        let weight = state.weight_for("cache");
        // 2 accepted, 1 dismissed, with smoothing prior: (2+2.5)/(3+5) = 0.5625
        assert!(weight > 0.5 && weight < 0.7);
        assert!(state.is_preferred("cache"));
    }

    #[test]
    fn test_profile_presets() {
        let presets = super::super::profiles::ProfilePreset::all();
        assert!(presets.len() >= 6);
    }

    #[test]
    fn test_profile_thresholds() {
        assert!(
            OptimizationProfile::Gaming.pressure_threshold()
                < OptimizationProfile::Balanced.pressure_threshold()
        );
        assert!(
            OptimizationProfile::Conservative.pressure_threshold()
                > OptimizationProfile::Aggressive.pressure_threshold()
        );
    }

    #[test]
    fn test_config_save_load() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let path = tmp.path().join("config.toml");

        let mut mgr = ConfigManager::load_from(&path);
        mgr.config_mut().clean.min_age_days = 42;
        mgr.save_to(&path).expect("save");

        let mgr2 = ConfigManager::load_from(&path);
        assert_eq!(mgr2.config().clean.min_age_days, 42);
    }
}
