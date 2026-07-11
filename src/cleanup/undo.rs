use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

/// Record of a single cleanup action that was executed.
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
