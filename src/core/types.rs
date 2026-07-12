use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::SystemTime;

pub type FindingId = String;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(tag = "type", content = "value")]
pub enum Target {
    Path(PathBuf),
    Process(u32),
    Port(u16),
    EnvironmentVariable(String),
    LaunchdLabel(String),
    Package(String),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
pub enum Severity {
    Info,
    Low,
    Medium,
    High,
    Critical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "lowercase")]
pub enum EngineId {
    Clean,
    Conflict,
    Map,
    Depth,
    Envmap,
    All,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    Cache,
    Log,
    XcodeArtifact,
    OrphanFile,
    DuplicateFile,
    PathConflict,
    EnvVarConflict,
    PortConflict,
    ShellConflict,
    PythonEnv,
    NodeEnv,
    ContainerRuntime,
    PackageManager,
    PermissionIssue,
    BrokenSymlink,
    MissingDylib,
    InvalidSignature,
    InstalledApp,
    SystemInfo,
    TempFile,
    BuildArtifact,
    PackageManagerCache,
    BrowserCache,
    MailAttachment,
    IosBackup,
    LanguageFile,
    UniversalBinary,
    LargeFile,
    TrashBin,
    DocumentVersion,
    SystemMaintenance,
    RamOptimization,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: FindingId,
    pub engine: EngineId,
    pub severity: Severity,
    pub category: Category,
    pub target: Target,
    pub title: String,
    pub description: String,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
    pub discovered_at: SystemTime,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remediation_hint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub size_bytes: Option<u64>,
}

impl Finding {
    pub fn new(
        engine: EngineId,
        severity: Severity,
        category: Category,
        target: Target,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("{:x}", uuid::Uuid::now_v7()),
            engine,
            severity,
            category,
            target,
            title: title.into(),
            description: description.into(),
            metadata: HashMap::new(),
            discovered_at: SystemTime::now(),
            remediation_hint: None,
            size_bytes: None,
        }
    }

    pub fn with_hint(mut self, hint: impl Into<String>) -> Self {
        self.remediation_hint = Some(hint.into());
        self
    }

    pub fn with_size(mut self, bytes: u64) -> Self {
        self.size_bytes = Some(bytes);
        self
    }

    pub fn with_metadata(mut self, key: impl Into<String>, value: serde_json::Value) -> Self {
        self.metadata.insert(key.into(), value);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineStats {
    pub engine: EngineId,
    pub duration: std::time::Duration,
    pub items_scanned: u64,
    pub findings_count: u64,
    pub errors_count: u64,
}

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ScanConfig {
    pub concurrency: usize,
    pub include_hidden: bool,
    pub follow_symlinks: bool,
    pub exclude_patterns: Vec<String>,
    pub cache_dir: Option<PathBuf>,
}

impl Default for ScanConfig {
    fn default() -> Self {
        Self {
            concurrency: 4,
            include_hidden: false,
            follow_symlinks: false,
            exclude_patterns: Vec::new(),
            cache_dir: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeverityBreakdown {
    pub info: u64,
    pub low: u64,
    pub medium: u64,
    pub high: u64,
    pub critical: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineBreakdown {
    pub clean: u64,
    pub conflict: u64,
    pub map: u64,
    pub depth: u64,
    pub envmap: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanReport {
    pub scan_id: String,
    pub timestamp: String,
    pub macos_version: String,
    pub apple_silicon: bool,
    pub engines: Vec<EngineStats>,
    pub findings_by_severity: SeverityBreakdown,
    pub findings_by_engine: EngineBreakdown,
    pub findings_by_category: HashMap<String, u64>,
    pub total_reclaimable_bytes: u64,
    pub total_findings: u64,
    pub total_items_scanned: u64,
    pub total_duration_secs: f64,
}

impl ScanReport {
    pub fn from_findings_and_stats(
        findings: &[Finding],
        engine_stats: &[EngineStats],
        macos_version: &str,
        apple_silicon: bool,
        total_duration: std::time::Duration,
    ) -> Self {
        let mut severity = SeverityBreakdown {
            info: 0,
            low: 0,
            medium: 0,
            high: 0,
            critical: 0,
        };
        let mut engine_bd = EngineBreakdown {
            clean: 0,
            conflict: 0,
            map: 0,
            depth: 0,
            envmap: 0,
        };
        let mut category_map: HashMap<String, u64> = HashMap::new();
        let mut reclaimable: u64 = 0;

        for f in findings {
            match f.severity {
                Severity::Info => severity.info += 1,
                Severity::Low => severity.low += 1,
                Severity::Medium => severity.medium += 1,
                Severity::High => severity.high += 1,
                Severity::Critical => severity.critical += 1,
            }
            match f.engine {
                EngineId::Clean => engine_bd.clean += 1,
                EngineId::Conflict => engine_bd.conflict += 1,
                EngineId::Map => engine_bd.map += 1,
                EngineId::Depth => engine_bd.depth += 1,
                EngineId::Envmap => engine_bd.envmap += 1,
                EngineId::All => {}
            }
            let cat = serde_json::to_string(&f.category)
                .unwrap_or_default()
                .trim_matches('"')
                .to_string();
            *category_map.entry(cat).or_insert(0) += 1;
            // Only count reclaimable bytes for categories that represent
            // actual deletable space — not informational categories like
            // SystemInfo (disk usage breakdown) or LargeFile (which are
            // informational pointers, not necessarily deletable).
            if let Some(sz) = f.size_bytes {
                match f.category {
                    Category::SystemInfo | Category::LargeFile => {}
                    _ => reclaimable += sz,
                }
            }
        }

        let total_items_scanned: u64 = engine_stats.iter().map(|s| s.items_scanned).sum();

        Self {
            scan_id: format!("{:x}", uuid::Uuid::now_v7()),
            timestamp: {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                format!("{}", now)
            },
            macos_version: macos_version.to_string(),
            apple_silicon,
            engines: engine_stats.to_vec(),
            findings_by_severity: severity,
            findings_by_engine: engine_bd,
            findings_by_category: category_map,
            total_reclaimable_bytes: reclaimable,
            total_findings: findings.len() as u64,
            total_items_scanned,
            total_duration_secs: total_duration.as_secs_f64(),
        }
    }
}
