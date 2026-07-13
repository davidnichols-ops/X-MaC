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

/// Envelope used for integrity-protected history files.
/// Format: {"_hmac":"<hash>","data":{...CleanupHistory...}}
#[derive(Deserialize)]
struct HistoryEnvelope {
    _hmac: String,
    data: CleanupHistory,
}

#[allow(dead_code)]
pub fn load_history(path: &Path) -> CleanupHistory {
    if let Ok(content) = std::fs::read_to_string(path) {
        // Try the integrity-protected envelope format first.
        // We deserialize into a typed struct so that re-serialization
        // for HMAC verification produces the same output as save_history.
        if let Ok(envelope) = serde_json::from_str::<HistoryEnvelope>(&content) {
            // Re-serialize the data to compute the expected HMAC.
            // This matches what save_history does (compact JSON via to_string).
            if let Ok(data_json) = serde_json::to_string(&envelope.data) {
                let expected_hmac = compute_history_hmac(&data_json);
                if envelope._hmac == expected_hmac {
                    return envelope.data;
                }
            }
            // HMAC mismatch or parse failure — treat as tampered/empty
            return CleanupHistory::new();
        }
        // Fall back to plain JSON (legacy format without HMAC)
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
    // Use compact JSON so the HMAC computed here matches what load_history
    // computes from the parsed Value's to_string() representation.
    let json = serde_json::to_string(history).map_err(|e| e.to_string())?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn save_load_roundtrip_preserves_data() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("history.json");

        let mut history = CleanupHistory::new();
        history.add_snapshot(ScanSnapshot::new(1000, 500, 3));
        save_history(&history, &path).unwrap();

        let loaded = load_history(&path);
        assert_eq!(loaded.snapshots.len(), 1);
        assert_eq!(loaded.snapshots[0].total_bytes, 1000);
        assert_eq!(loaded.snapshots[0].reclaimable_bytes, 500);
        assert_eq!(loaded.snapshots[0].finding_count, 3);
    }

    #[test]
    fn tampered_history_returns_empty() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("history.json");

        let mut history = CleanupHistory::new();
        history.add_snapshot(ScanSnapshot::new(1000, 500, 3));
        save_history(&history, &path).unwrap();

        // Tamper with the file — modify the data without updating the HMAC
        let content = std::fs::read_to_string(&path).unwrap();
        let tampered = content.replace("\"total_bytes\":1000", "\"total_bytes\":99999");
        std::fs::write(&path, tampered).unwrap();

        let loaded = load_history(&path);
        // HMAC mismatch → empty history
        assert!(loaded.snapshots.is_empty());
    }

    #[test]
    fn legacy_plain_json_still_loads() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("history.json");
        std::fs::write(&path, r#"{"snapshots":[],"transactions":[],"max_entries":100}"#).unwrap();

        let loaded = load_history(&path);
        assert_eq!(loaded.max_entries, 100);
    }
}
