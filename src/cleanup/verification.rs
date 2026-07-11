use std::path::{Path, PathBuf};

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
        matches!(self, VerificationResult::Success | VerificationResult::AlreadyGone)
    }
}

/// Verify that a path has been removed or moved to the expected location.
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
pub fn verify_can_cleanup(path: &Path) -> Result<(), String> {
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }
    let metadata = std::fs::metadata(path).map_err(|e| format!("cannot read metadata: {e}"))?;
    if metadata.permissions().readonly() {
        return Err(format!("path is read-only: {}", path.display()));
    }
    Ok(())
}
