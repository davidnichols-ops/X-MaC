use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

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

/// A snapshot of a file's metadata at scan time, used to verify the file
/// hasn't been modified between scan and deletion (TOCTOU protection).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub size_bytes: u64,
    pub modified_secs: u64,
}

impl FileSnapshot {
    /// Capture a snapshot of the given path's metadata. Returns None if
    /// metadata cannot be read (file may have been removed already).
    pub fn capture(path: &Path) -> Option<Self> {
        let metadata = std::fs::symlink_metadata(path).ok()?;
        let modified_secs = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Some(Self {
            size_bytes: metadata.len(),
            modified_secs,
        })
    }

    /// Verify that the current metadata of `path` matches this snapshot.
    /// Returns Ok(()) if the file is unchanged, or an error message if the
    /// file has been modified or removed.
    pub fn verify(&self, path: &Path) -> Result<(), String> {
        let metadata = std::fs::symlink_metadata(path)
            .map_err(|e| format!("cannot read metadata: {e}"))?;
        if metadata.len() != self.size_bytes {
            return Err(format!(
                "size changed: {} -> {} (file was modified after scan)",
                self.size_bytes,
                metadata.len()
            ));
        }
        let modified_secs = metadata
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        if modified_secs != self.modified_secs {
            return Err(format!(
                "modification time changed: {} -> {} (file was modified after scan)",
                self.modified_secs, modified_secs
            ));
        }
        Ok(())
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;

    #[test]
    fn test_file_snapshot_capture_and_verify() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"hello world").unwrap();

        let snapshot = FileSnapshot::capture(tmp.path()).unwrap();
        // Immediately verify — should match.
        assert!(snapshot.verify(tmp.path()).is_ok());
    }

    #[test]
    fn test_file_snapshot_detects_size_change() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"hello").unwrap();
        let snapshot = FileSnapshot::capture(tmp.path()).unwrap();

        // Modify the file — size changes.
        std::fs::write(tmp.path(), b"hello world, modified!").unwrap();
        let result = snapshot.verify(tmp.path());
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("size changed"));
    }

    #[test]
    fn test_file_snapshot_detects_missing_file() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"data").unwrap();
        let snapshot = FileSnapshot::capture(tmp.path()).unwrap();

        // Delete the file.
        drop(tmp);
        let result = snapshot.verify(Path::new("/nonexistent/path/xyz"));
        assert!(result.is_err());
    }
}
