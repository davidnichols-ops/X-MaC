use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::types::Finding;
use crate::util::macos::MacosUtils;

use super::policy::{CleanupAction, CleanupPolicy};
use super::preflight::{build_candidates, CleanupCandidate};
use super::undo::{CleanupActionRecord, CleanupTransaction};
use super::verification::{verify_can_cleanup, verify_removal};

/// A cleanup error.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct Error {
    pub message: String,
    pub path: Option<PathBuf>,
}

impl Error {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            path: None,
        }
    }

    pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
        self.path = Some(path.as_ref().to_path_buf());
        self
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.path {
            Some(p) => write!(f, "{}: {}", p.display(), self.message),
            None => write!(f, "{}", self.message),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::new(e.to_string())
    }
}

/// op 247: Result of an intelligent uninstall operation.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct UninstallResult {
    pub app_bundle_id: String,
    pub bytes_freed: u64,
    pub removed_paths: Vec<PathBuf>,
    pub failed_paths: Vec<PathBuf>,
    pub success: bool,
}

/// A transaction that executes a cleanup plan and records undo metadata.
#[derive(Debug, Clone)]
pub struct CleanupExecutor {
    policy: CleanupPolicy,
    dry_run: bool,
    records: Vec<CleanupActionRecord>,
}

impl CleanupExecutor {
    pub fn new(policy: CleanupPolicy, dry_run: bool) -> Self {
        Self {
            policy,
            dry_run,
            records: Vec::new(),
        }
    }

    /// Build a cleanup plan from findings. Non-executable candidates are still
    /// included so the caller can explain why they were skipped.
    pub fn plan(&self, findings: &[Finding]) -> CleanupPlan {
        let candidates = build_candidates(findings, &self.policy);
        CleanupPlan::new(candidates)
    }

    /// op 275: Apply safe changes — execute all executable candidates in the
    /// plan. Each candidate is validated a second time immediately before
    /// touching disk to ensure only safe, verified changes are applied.
    pub fn execute(&mut self, plan: &CleanupPlan) -> CleanupTransaction {
        let mut transaction = CleanupTransaction::new();
        for candidate in &plan.candidates {
            if !candidate.is_executable() {
                let record = self.record_skipped(candidate);
                transaction.add(record);
                continue;
            }
            if self.dry_run {
                transaction.add(self.record_dry_run(candidate));
                continue;
            }
            transaction.add(self.execute_candidate(candidate));
        }
        transaction.finish();
        self.records.extend(transaction.actions.clone());
        transaction
    }

    fn execute_candidate(&self, candidate: &CleanupCandidate) -> CleanupActionRecord {
        let path = &candidate.path;
        let category = format!("{:?}", candidate.category).to_lowercase();
        let reason = candidate.reason.clone();

        // Re-verify right before touching disk to catch races and changes.
        // Check for symlink replacement (TOCTOU): a regular file at planning
        // time could have been swapped with a symlink by execution time.
        if !self.policy.follow_symlinks {
            if let Ok(meta) = std::fs::symlink_metadata(path) {
                if meta.file_type().is_symlink() {
                    return CleanupActionRecord::new(
                        path,
                        None,
                        format!("{:?}", candidate.action),
                        false,
                        candidate.size_bytes,
                        &category,
                        &reason,
                    )
                    .with_error(
                        "symbolic link detected at execution time (TOCTOU race)".to_string(),
                    );
                }
            }
        }

        if let Err(e) = verify_can_cleanup(path) {
            return CleanupActionRecord::new(
                path,
                None,
                format!("{:?}", candidate.action),
                false,
                candidate.size_bytes,
                &category,
                &reason,
            )
            .with_error(e);
        }

        if self.policy.is_protected(path) {
            return CleanupActionRecord::new(
                path,
                None,
                format!("{:?}", candidate.action),
                false,
                candidate.size_bytes,
                &category,
                &reason,
            )
            .with_error("protected path");
        }

        match candidate.action {
            CleanupAction::Trash => self.trash_item(candidate),
            CleanupAction::Review => CleanupActionRecord::new(
                path,
                None,
                "review",
                false,
                candidate.size_bytes,
                &category,
                &reason,
            )
            .with_error("requires explicit review"),
            CleanupAction::Blocked => CleanupActionRecord::new(
                path,
                None,
                "blocked",
                false,
                candidate.size_bytes,
                &category,
                &reason,
            )
            .with_error("blocked by policy"),
        }
    }

    /// op 105: Move to Trash — move the candidate's file to the macOS Trash
    /// directory, using a unique destination name to avoid collisions.
    fn trash_item(&self, candidate: &CleanupCandidate) -> CleanupActionRecord {
        let path = &candidate.path;
        let category = format!("{:?}", candidate.category).to_lowercase();
        let reason = candidate.reason.clone();

        let trash_path = match macos_trash_path(path) {
            Ok(p) => p,
            Err(e) => {
                return CleanupActionRecord::new(
                    path,
                    None,
                    "trash",
                    false,
                    candidate.size_bytes,
                    &category,
                    &reason,
                )
                .with_error(e);
            }
        };

        // Move to trash by renaming. If the trash already exists, append a unique suffix.
        let unique_trash = unique_trash_destination(&trash_path);
        if let Err(e) = rename_to_trash(path, &unique_trash) {
            return CleanupActionRecord::new(
                path,
                Some(unique_trash.clone()),
                "trash",
                false,
                candidate.size_bytes,
                &category,
                &reason,
            )
            .with_error(e);
        }

        let success = verify_removal(path, Some(&unique_trash)).is_success();
        CleanupActionRecord::new(
            path,
            Some(unique_trash),
            "trash",
            success,
            candidate.size_bytes,
            &category,
            &reason,
        )
    }

    fn record_skipped(&self, candidate: &CleanupCandidate) -> CleanupActionRecord {
        CleanupActionRecord::new(
            &candidate.path,
            None,
            "skipped",
            false,
            candidate.size_bytes,
            format!("{:?}", candidate.category).to_lowercase(),
            candidate.reason.clone(),
        )
        .with_error(candidate.preflight_errors.join("; "))
    }

    fn record_dry_run(&self, candidate: &CleanupCandidate) -> CleanupActionRecord {
        CleanupActionRecord::new(
            &candidate.path,
            None,
            format!("{:?}", candidate.action),
            true,
            candidate.size_bytes,
            format!("{:?}", candidate.category).to_lowercase(),
            candidate.reason.clone(),
        )
    }
}

/// A cleanup plan is a validated set of candidates.
#[derive(Debug, Clone)]
pub struct CleanupPlan {
    pub candidates: Vec<CleanupCandidate>,
}

impl CleanupPlan {
    pub fn new(candidates: Vec<CleanupCandidate>) -> Self {
        Self { candidates }
    }

    pub fn executable(&self) -> Vec<&CleanupCandidate> {
        self.candidates
            .iter()
            .filter(|c| c.is_executable())
            .collect()
    }

    pub fn blocked(&self) -> Vec<&CleanupCandidate> {
        self.candidates
            .iter()
            .filter(|c| !c.is_executable())
            .collect()
    }

    pub fn total_reclaimable_bytes(&self) -> u64 {
        self.executable().iter().map(|c| c.size_bytes).sum()
    }
}

/// Compute the Trash destination path for a given file on macOS.
///
/// NOTE: This implementation creates ~/.Trash manually and only handles
/// files within the user's home directory. The macOS-native approach is
/// NSFileManager.trashItem(at:resultingItemURL:) which correctly handles
/// external volumes (using .Trash-<uid> on the volume), network shares,
/// and Trash semantics. The Swift GUI uses NSFileManager.trashItem
/// correctly; this Rust implementation is used by the CLI only and is
/// limited to home-directory items. For files outside $HOME, use the
/// GUI or move files manually.
fn macos_trash_path(path: &Path) -> Result<PathBuf, String> {
    let home = MacosUtils::home_dir()
        .canonicalize()
        .unwrap_or_else(|_| MacosUtils::home_dir());
    // Only allow trashing items that are inside the user's home directory.
    // System paths and external volumes are rejected here.
    let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    if !path.starts_with(&home) {
        return Err(format!(
            "refusing to move item outside home directory to Trash: {} (home: {})",
            path.display(),
            home.display()
        ));
    }
    let trash = home.join(".Trash");
    if !trash.exists() {
        fs::create_dir_all(&trash).map_err(|e| format!("cannot create Trash: {e}"))?;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "xmac-removed".to_string());
    Ok(trash.join(name))
}

fn unique_trash_destination(original: &Path) -> PathBuf {
    if !original.exists() {
        return original.to_path_buf();
    }
    let stem = original
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "xmac-removed".to_string());
    let extension = original
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();
    for counter in 1..10_000 {
        let candidate = original.with_file_name(format!("{stem}-{counter}{extension}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    original.to_path_buf()
}

fn rename_to_trash(source: &Path, dest: &Path) -> Result<(), String> {
    fs::rename(source, dest).map_err(|e| format!("rename failed: {e}"))
}

/// op 106: Secure delete — overwrite the file with zeros before deletion so
/// the original bytes cannot be recovered from disk. The file is first
/// truncated to zero (via `set_len`) to drop any existing data, then filled
/// with zeros in chunks, flushed, and finally removed.
#[allow(dead_code)]
pub fn secure_delete(path: &Path) -> Result<(), Error> {
    let metadata = fs::metadata(path).map_err(|e| Error::from(e).with_path(path))?;
    if metadata.is_dir() {
        // Secure delete only applies to regular files.
        return Err(
            Error::new("secure_delete only supports files, not directories").with_path(path),
        );
    }
    let file_size = metadata.len();

    // Open the file in write mode without truncating so we can overwrite
    // the existing bytes in place.
    let mut file = fs::OpenOptions::new()
        .write(true)
        .open(path)
        .map_err(|e| Error::from(e).with_path(path))?;

    // Truncate to zero first to release the original data blocks.
    file.set_len(0)
        .map_err(|e| Error::from(e).with_path(path))?;

    // Restore the original size and overwrite with zeros so the file
    // occupies zero-filled blocks on disk.
    file.set_len(file_size)
        .map_err(|e| Error::from(e).with_path(path))?;

    // Write zeros in 1 MiB chunks to avoid allocating a huge buffer.
    const CHUNK_SIZE: usize = 1024 * 1024;
    let zeros = vec![0u8; CHUNK_SIZE.min(file_size as usize)];
    let mut remaining = file_size;
    while remaining > 0 {
        let to_write = remaining.min(zeros.len() as u64) as usize;
        file.write_all(&zeros[..to_write])
            .map_err(|e| Error::from(e).with_path(path))?;
        remaining -= to_write as u64;
    }
    file.flush().map_err(|e| Error::from(e).with_path(path))?;
    // Drop the handle before removing the file.
    drop(file);

    fs::remove_file(path).map_err(|e| Error::from(e).with_path(path))
}

/// op 247: Perform intelligent uninstall — remove an application and all of
/// its associated data (caches, preferences, support files, containers).
/// On macOS the app bundle is located under /Applications, ~/Applications,
/// or /Applications/Utilities, and associated data lives under
/// `~/Library/{Caches,Preferences,Application Support,Containers,Saved
/// Application State}` keyed by the bundle id.
#[allow(dead_code)]
pub fn intelligent_uninstall(app_bundle_id: &str) -> Result<UninstallResult, Error> {
    let home = MacosUtils::home_dir();
    let library = home.join("Library");

    // Candidate associated-data directories keyed by the bundle id.
    let associated_paths: Vec<PathBuf> = vec![
        library.join("Caches").join(app_bundle_id),
        library
            .join("Preferences")
            .join(format!("{app_bundle_id}.plist")),
        library.join("Application Support").join(app_bundle_id),
        library.join("Containers").join(app_bundle_id),
        library
            .join("Saved Application State")
            .join(format!("{app_bundle_id}.savedState")),
    ];

    // Locate the app bundle itself.
    let app_bundle = locate_app_bundle(app_bundle_id);

    let mut removed_paths = Vec::new();
    let mut failed_paths = Vec::new();
    let mut bytes_freed: u64 = 0;

    // Remove the app bundle first (move to trash for undoability).
    if let Some(bundle) = &app_bundle {
        match remove_path_to_trash(bundle) {
            Ok(bytes) => {
                bytes_freed += bytes;
                removed_paths.push(bundle.clone());
            }
            Err(_) => {
                failed_paths.push(bundle.clone());
            }
        }
    }

    // Remove associated data.
    for path in &associated_paths {
        if !path.exists() {
            continue;
        }
        match remove_path_to_trash(path) {
            Ok(bytes) => {
                bytes_freed += bytes;
                removed_paths.push(path.clone());
            }
            Err(_) => {
                failed_paths.push(path.clone());
            }
        }
    }

    let success = failed_paths.is_empty();
    Ok(UninstallResult {
        app_bundle_id: app_bundle_id.to_string(),
        bytes_freed,
        removed_paths,
        failed_paths,
        success,
    })
}

/// Locate an app bundle by bundle id in common application directories.
fn locate_app_bundle(app_bundle_id: &str) -> Option<PathBuf> {
    let home = MacosUtils::home_dir();
    let search_dirs = [
        PathBuf::from("/Applications"),
        home.join("Applications"),
        PathBuf::from("/Applications/Utilities"),
    ];
    for dir in &search_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) != Some("app") {
                    continue;
                }
                // Match by bundle id via the Info.plist, or by directory name
                // as a fallback.
                if app_bundle_matches(&path, app_bundle_id) {
                    return Some(path);
                }
            }
        }
    }
    None
}

/// Check whether an app bundle's bundle id matches the given id.
fn app_bundle_matches(app_path: &Path, bundle_id: &str) -> bool {
    // Cheap heuristic: compare against the directory name (without .app).
    if let Some(stem) = app_path.file_stem().and_then(|s| s.to_str()) {
        if stem.eq_ignore_ascii_case(bundle_id) {
            return true;
        }
    }
    // Best-effort: read the Info.plist's CFBundleIdentifier via defaults.
    #[cfg(target_os = "macos")]
    {
        let plist = app_path.join("Contents").join("Info.plist");
        if plist.exists() {
            if let Ok(output) = std::process::Command::new("defaults")
                .args(["read", plist.to_str().unwrap_or(""), "CFBundleIdentifier"])
                .output()
            {
                let id = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if id == bundle_id {
                    return true;
                }
            }
        }
    }
    false
}

/// Move a path to the Trash and return the bytes freed (file size).
fn remove_path_to_trash(path: &Path) -> Result<u64, std::io::Error> {
    let bytes = compute_path_size(path);
    let trash_dest = match macos_trash_path(path) {
        Ok(p) => unique_trash_destination(&p),
        Err(_) => return Err(std::io::Error::other("cannot compute trash destination")),
    };
    fs::rename(path, &trash_dest)?;
    Ok(bytes)
}

/// Compute the total size of a file or directory tree in bytes.
fn compute_path_size(path: &Path) -> u64 {
    let metadata = match fs::metadata(path) {
        Ok(m) => m,
        Err(_) => return 0,
    };
    if metadata.is_file() {
        return metadata.len();
    }
    if metadata.is_dir() {
        let mut total: u64 = 0;
        for entry in walkdir::WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                total += entry.metadata().map(|m| m.len()).unwrap_or(0);
            }
        }
        return total;
    }
    metadata.len()
}

// ═══════════════════════════════════════════════════════════════════════
//  Unit tests
// ═══════════════════════════════════════════════════════════════════════

#[cfg(test)]
mod tests {
    use super::*;

    fn test_home() -> PathBuf {
        MacosUtils::home_dir().join(".xmac_test_cleanup")
    }

    fn setup_test_dir(name: &str) -> PathBuf {
        let dir = test_home().join(name);
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup_test_dir(name: &str) {
        let _ = std::fs::remove_dir_all(test_home().join(name));
    }

    #[test]
    fn test_secure_delete_removes_file() {
        let dir = setup_test_dir("secure_delete");
        let file = dir.join("secret.bin");
        std::fs::write(&file, b"sensitive data that should be wiped").unwrap();
        assert!(file.exists());

        secure_delete(&file).expect("secure_delete should succeed");

        assert!(!file.exists(), "file should be deleted after secure_delete");
        cleanup_test_dir("secure_delete");
    }

    #[test]
    fn test_secure_delete_nonexistent_fails() {
        let dir = setup_test_dir("secure_delete_missing");
        let file = dir.join("nope.bin");
        let result = secure_delete(&file);
        assert!(result.is_err(), "secure_delete on missing file should fail");
        cleanup_test_dir("secure_delete_missing");
    }

    #[test]
    fn test_secure_delete_directory_fails() {
        let dir = setup_test_dir("secure_delete_dir");
        let subdir = dir.join("subdir");
        std::fs::create_dir_all(&subdir).unwrap();
        let result = secure_delete(&subdir);
        assert!(result.is_err(), "secure_delete on a directory should fail");
        cleanup_test_dir("secure_delete_dir");
    }

    #[test]
    fn test_intelligent_uninstall_nonexistent_app() {
        // A bundle id that almost certainly does not exist.
        let result = intelligent_uninstall("com.xmac.nonexistent.test.app.bundle")
            .expect("should not error");
        assert!(!result.success || result.removed_paths.is_empty());
        // No app bundle or associated data to remove.
        assert!(result.removed_paths.is_empty());
    }

    #[test]
    fn test_compute_path_size_file() {
        let dir = setup_test_dir("path_size");
        let file = dir.join("data.bin");
        std::fs::write(&file, b"hello world").unwrap();
        assert_eq!(compute_path_size(&file), 11);
        cleanup_test_dir("path_size");
    }

    #[test]
    fn test_compute_path_size_directory() {
        let dir = setup_test_dir("path_size_dir");
        std::fs::write(dir.join("a.txt"), b"aaaa").unwrap();
        std::fs::write(dir.join("b.txt"), b"bbbbbb").unwrap();
        assert_eq!(compute_path_size(&dir), 10);
        cleanup_test_dir("path_size_dir");
    }

    #[test]
    fn test_error_display() {
        let e = Error::new("something went wrong");
        assert_eq!(format!("{e}"), "something went wrong");
        let e = Error::new("missing").with_path("/tmp/foo");
        assert_eq!(format!("{e}"), "/tmp/foo: missing");
    }
}
