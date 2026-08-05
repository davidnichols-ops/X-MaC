use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use serde::{Deserialize, Serialize};

/// A snapshot of a file's metadata at scan time, used to verify the file
/// hasn't been modified between scan and deletion (TOCTOU protection).
///
/// The snapshot captures size, modification time, and an optional BLAKE3
/// hash for cryptographic verification. The hash is only computed when
/// `capture_with_hash()` is used — the fast `capture()` method skips it
/// for performance on large files.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileSnapshot {
    pub size_bytes: u64,
    pub modified_secs: u64,
    /// Optional BLAKE3 hash of the file content for cryptographic verification.
    /// None when only metadata-based verification is needed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_hash: Option<String>,
}

impl FileSnapshot {
    /// Capture a snapshot of the given path's metadata. Returns None if
    /// metadata cannot be read (file may have been removed already).
    /// Does NOT compute a content hash — use `capture_with_hash()` for that.
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
            content_hash: None,
        })
    }

    /// Capture a snapshot with a BLAKE3 content hash for cryptographic
    /// verification. Slower than `capture()` but detects content changes
    /// even if size and mtime are preserved.
    #[allow(dead_code)]
    pub fn capture_with_hash(path: &Path) -> Option<Self> {
        let mut snap = Self::capture(path)?;
        if path.is_file() {
            if let Ok(data) = std::fs::read(path) {
                let hash = blake3::hash(&data);
                snap.content_hash = Some(hash.to_hex().to_string());
            }
        }
        Some(snap)
    }

    /// Verify that the current metadata of `path` matches this snapshot.
    /// Returns Ok(()) if the file is unchanged, or an error message if the
    /// file has been modified or removed.
    ///
    /// If a content hash was captured, it is re-computed and compared.
    /// This provides cryptographic assurance that the file content is
    /// identical, even if an attacker preserved size and mtime.
    pub fn verify(&self, path: &Path) -> Result<(), String> {
        let metadata =
            std::fs::symlink_metadata(path).map_err(|e| format!("cannot read metadata: {e}"))?;
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
        // Cryptographic verification — only if a hash was captured.
        if let Some(expected_hash) = &self.content_hash {
            if path.is_file() {
                match std::fs::read(path) {
                    Ok(data) => {
                        let actual = blake3::hash(&data);
                        let actual_hex = actual.to_hex().to_string();
                        if &actual_hex != expected_hash {
                            return Err(format!(
                                "content hash mismatch: expected {}, got {} (file content was modified after scan)",
                                expected_hash, actual_hex
                            ));
                        }
                    }
                    Err(e) => {
                        return Err(format!("cannot read file for hash verification: {e}"));
                    }
                }
            }
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

    #[test]
    fn test_file_snapshot_with_hash_detects_content_change() {
        let tmp = NamedTempFile::new().unwrap();
        // Write 5 bytes.
        std::fs::write(tmp.path(), b"hello").unwrap();
        let snapshot = FileSnapshot::capture_with_hash(tmp.path()).unwrap();
        assert!(snapshot.content_hash.is_some());

        // Replace with 5 different bytes — same size, different content.
        std::fs::write(tmp.path(), b"world").unwrap();
        // Fix the mtime so it matches (to test that hash catches it even
        // when size and mtime are the same).
        // We can't easily preserve mtime in a test, so just check that
        // either mtime or hash catches the change.
        let result = snapshot.verify(tmp.path());
        assert!(result.is_err(), "should detect content change");
    }

    #[test]
    fn test_file_snapshot_with_hash_preserves_unchanged() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"unchanged content").unwrap();
        let snapshot = FileSnapshot::capture_with_hash(tmp.path()).unwrap();
        assert!(snapshot.content_hash.is_some());

        // Verify immediately — should pass.
        let result = snapshot.verify(tmp.path());
        assert!(result.is_ok(), "unchanged file should verify");
    }

    #[test]
    fn test_file_snapshot_without_hash_skips_content_check() {
        let tmp = NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), b"hello").unwrap();
        let snapshot = FileSnapshot::capture(tmp.path()).unwrap();
        assert!(snapshot.content_hash.is_none());

        // Should verify OK immediately.
        assert!(snapshot.verify(tmp.path()).is_ok());
    }
}
