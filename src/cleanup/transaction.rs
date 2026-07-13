use std::fs;
use std::path::{Path, PathBuf};

use crate::core::types::Finding;
use crate::util::macos::MacosUtils;

use super::policy::{CleanupAction, CleanupPolicy};
use super::preflight::{build_candidates, CleanupCandidate};
use super::undo::{CleanupActionRecord, CleanupTransaction};
use super::verification::{verify_can_cleanup, verify_removal};

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

    /// Execute all executable candidates in the plan. Each candidate is validated
    /// a second time immediately before touching disk.
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
