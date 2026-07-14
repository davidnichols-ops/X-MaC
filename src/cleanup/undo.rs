use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use super::history::CleanupHistory;
use super::transaction::Error;

/// op 107: Record of a single cleanup action that was executed, with enough
/// metadata (original + trashed paths) to undo the deletion by moving the
/// item back from Trash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupActionRecord {
    pub id: String,
    pub original_path: PathBuf,
    pub trashed_path: Option<PathBuf>,
    pub action: String,
    pub success: bool,
    pub size_bytes: u64,
    pub category: String,
    pub reason: String,
    pub timestamp: u64,
    pub error: Option<String>,
}

impl CleanupActionRecord {
    pub fn new(
        original_path: impl AsRef<Path>,
        trashed_path: Option<PathBuf>,
        action: impl Into<String>,
        success: bool,
        size_bytes: u64,
        category: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            id: format!("{:x}", Uuid::now_v7()),
            original_path: original_path.as_ref().to_path_buf(),
            trashed_path,
            action: action.into(),
            success,
            size_bytes,
            category: category.into(),
            reason: reason.into(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            error: None,
        }
    }

    pub fn with_error(mut self, error: impl Into<String>) -> Self {
        self.error = Some(error.into());
        self
    }
}

/// A transaction bundles multiple cleanup action records together.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupTransaction {
    pub id: String,
    pub started_at: u64,
    pub finished_at: Option<u64>,
    pub actions: Vec<CleanupActionRecord>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl CleanupTransaction {
    pub fn new() -> Self {
        Self {
            id: format!("{:x}", Uuid::now_v7()),
            started_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
            finished_at: None,
            actions: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    pub fn add(&mut self, record: CleanupActionRecord) {
        self.actions.push(record);
    }

    pub fn finish(&mut self) {
        self.finished_at = Some(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs(),
        );
    }

    pub fn successful_bytes(&self) -> u64 {
        self.actions
            .iter()
            .filter(|a| a.success)
            .map(|a| a.size_bytes)
            .sum()
    }

    pub fn successful_count(&self) -> usize {
        self.actions.iter().filter(|a| a.success).count()
    }
}

impl Default for CleanupTransaction {
    fn default() -> Self {
        Self::new()
    }
}

/// op 248: Restore removed applications — restore a previously uninstalled
/// app from Trash by searching the cleanup history for action records that
/// match the given app id (matched against the original path or category)
/// and moving each trashed item back to its original location.
#[allow(dead_code)]
pub fn restore_app(history: &CleanupHistory, app_id: &str) -> Result<(), Error> {
    let mut restored_any = false;
    for transaction in &history.transactions {
        for action in &transaction.actions {
            // Match by the original path containing the app id, or by the
            // category field containing the app id.
            let matches = action.original_path.to_string_lossy().contains(app_id)
                || action.category.contains(app_id);
            if !matches {
                continue;
            }
            // Only restore items that were moved to Trash (have a trashed path).
            if let Some(trashed) = &action.trashed_path {
                if !trashed.exists() {
                    continue;
                }
                // Restore to the original location, recreating parent dirs.
                if let Some(parent) = action.original_path.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                std::fs::rename(trashed, &action.original_path)
                    .map_err(|e| Error::from(e).with_path(&action.original_path))?;
                restored_any = true;
            }
        }
    }
    if !restored_any {
        return Err(Error::new(format!(
            "no trashed items matching '{app_id}' found in history"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cleanup::history::ScanSnapshot;

    #[test]
    fn test_restore_app_no_history() {
        let history = CleanupHistory::new();
        let result = restore_app(&history, "com.example.nonexistent");
        assert!(result.is_err(), "restore with no history should fail");
    }

    #[test]
    fn test_restore_app_empty_history_no_panic() {
        let mut history = CleanupHistory::new();
        history.add_snapshot(ScanSnapshot::new(1000, 500, 3));
        // No transactions, so restore should fail gracefully.
        let result = restore_app(&history, "com.example.app");
        assert!(result.is_err());
    }

    #[test]
    fn test_restore_app_missing_trash_path() {
        let mut history = CleanupHistory::new();
        let mut txn = CleanupTransaction::new();
        txn.add(CleanupActionRecord::new(
            "/Applications/Foo.app",
            Some(PathBuf::from("/tmp/nonexistent-trash/Foo.app")),
            "trash",
            true,
            1000,
            "com.example.app",
            "test",
        ));
        txn.finish();
        history.add_transaction(txn);
        // The trashed path does not exist, so nothing can be restored.
        let result = restore_app(&history, "com.example.app");
        assert!(
            result.is_err(),
            "restore with missing trash path should fail"
        );
    }
}
