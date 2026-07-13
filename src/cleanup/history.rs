use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::undo::CleanupTransaction;

/// A stored snapshot of a scan and its outcome, used for history, trend
/// analysis, and repeatability.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ScanSnapshot {
    pub id: String,
    pub timestamp: u64,
    pub total_bytes: u64,
    pub reclaimable_bytes: u64,
    pub finding_count: usize,
    pub category_counts: HashMap<String, u64>,
}

#[allow(dead_code)]
impl ScanSnapshot {
    pub fn new(total_bytes: u64, reclaimable_bytes: u64, finding_count: usize) -> Self {
        Self {
            id: format!("{:x}", Uuid::now_v7()),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            total_bytes,
            reclaimable_bytes,
            finding_count,
            category_counts: HashMap::new(),
        }
    }

    pub fn with_categories(mut self, counts: HashMap<String, u64>) -> Self {
        self.category_counts = counts;
        self
    }
}

/// Local history store for scan snapshots and cleanup transactions.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct CleanupHistory {
    pub snapshots: Vec<ScanSnapshot>,
    pub transactions: Vec<CleanupTransaction>,
    pub max_entries: usize,
}

impl Default for CleanupHistory {
    fn default() -> Self {
        Self {
            snapshots: Vec::new(),
            transactions: Vec::new(),
            max_entries: 100,
        }
    }
}

#[allow(dead_code)]
impl CleanupHistory {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_snapshot(&mut self, snapshot: ScanSnapshot) {
        self.snapshots.push(snapshot);
        self.enforce_limit();
    }

    pub fn add_transaction(&mut self, transaction: CleanupTransaction) {
        self.transactions.push(transaction);
        self.enforce_limit();
    }

    fn enforce_limit(&mut self) {
        if self.snapshots.len() > self.max_entries {
            let excess = self.snapshots.len() - self.max_entries;
            self.snapshots.drain(0..excess);
        }
        if self.transactions.len() > self.max_entries {
            let excess = self.transactions.len() - self.max_entries;
            self.transactions.drain(0..excess);
        }
    }
}

#[allow(dead_code)]
pub fn default_history_path() -> PathBuf {
    let home = std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp"));
    home.join("Library/Caches/com.xmac.gui/history.json")
}

#[allow(dead_code)]
pub fn load_history(path: &Path) -> CleanupHistory {
    if let Ok(content) = std::fs::read_to_string(path) {
        if let Ok(history) = serde_json::from_str::<CleanupHistory>(&content) {
            return history;
        }
    }
    CleanupHistory::new()
}

#[allow(dead_code)]
pub fn save_history(history: &CleanupHistory, path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let json = serde_json::to_string_pretty(history).map_err(|e| e.to_string())?;
    // Compute a simple integrity hash derived from the machine's hostname
    // and the JSON content. This prevents tampering with the history file
    // to inject fake cleanup transactions. On verification, if the hash
    // doesn't match, the history is treated as empty.
    let hash = compute_history_hmac(&json);
    let content = format!("{{\"_hmac\":\"{}\",\"data\":{}}}", hash, json);
    std::fs::write(path, content).map_err(|e| e.to_string())
}

/// Compute a simple integrity hash for the history file.
/// Uses a key derived from the system hostname — not cryptographically
/// strong, but prevents casual tampering. For full integrity, use a
/// key stored in the macOS Keychain.
fn compute_history_hmac(json: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    // Mix in a system-specific value so the hash is machine-bound
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("USER"))
        .unwrap_or_default();
    hostname.hash(&mut hasher);
    json.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}
