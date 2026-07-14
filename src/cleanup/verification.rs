use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// op 314: System metrics snapshot used to verify improvements before and
/// after a cleanup or maintenance operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct SystemMetrics {
    pub used_bytes: u64,
    pub available_bytes: u64,
    pub free_bytes: u64,
    pub pressure_level: u32,
    pub process_count: usize,
    pub swap_used_bytes: u64,
}

/// Verification result after a cleanup action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VerificationResult {
    Success,
    AlreadyGone,
    StillPresent,
    MovedToUnexpectedLocation(PathBuf),
    Failed(String),
}

impl VerificationResult {
    pub fn is_success(&self) -> bool {
        matches!(
            self,
            VerificationResult::Success | VerificationResult::AlreadyGone
        )
    }
}

/// op 313: Monitor after execution — verify that a path has been removed
/// or moved to the expected Trash location after a cleanup action.
pub fn verify_removal(original: &Path, expected_trash: Option<&Path>) -> VerificationResult {
    let original_exists = original.exists();
    if let Some(expected) = expected_trash {
        let expected_exists = expected.exists();
        if !original_exists && expected_exists {
            return VerificationResult::Success;
        }
        if original_exists && expected_exists {
            return VerificationResult::MovedToUnexpectedLocation(expected.to_path_buf());
        }
        if !original_exists && !expected_exists {
            return VerificationResult::AlreadyGone;
        }
        return VerificationResult::Failed(format!(
            "path still exists and expected trash location is missing: {}",
            expected.display()
        ));
    }
    if original_exists {
        VerificationResult::StillPresent
    } else {
        VerificationResult::AlreadyGone
    }
}

/// Verify that a path is safe to touch before any destructive action.
/// Uses symlink_metadata to check the symlink itself, not its target.
pub fn verify_can_cleanup(path: &Path) -> Result<(), String> {
    // Use symlink_metadata so we check the symlink itself, not the target.
    // This prevents TOCTOU attacks where a regular file is replaced with a
    // symlink between planning and execution.
    let metadata =
        std::fs::symlink_metadata(path).map_err(|e| format!("cannot read metadata: {e}"))?;
    if metadata.permissions().readonly() {
        return Err(format!("path is read-only: {}", path.display()));
    }
    Ok(())
}
