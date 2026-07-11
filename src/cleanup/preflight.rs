use std::path::PathBuf;

use crate::core::types::{Category, Finding, Target};

use super::policy::{CleanupAction, CleanupPolicy, RiskLevel};
use super::verification::verify_can_cleanup;

/// A validated cleanup candidate ready for execution.
#[derive(Debug, Clone)]
pub struct CleanupCandidate {
    pub finding_id: String,
    pub path: PathBuf,
    pub category: Category,
    pub size_bytes: u64,
    pub action: CleanupAction,
    pub risk: RiskLevel,
    pub reason: String,
    pub preflight_errors: Vec<String>,
    pub preflight_warnings: Vec<String>,
    pub blocked: bool,
}

impl CleanupCandidate {
    pub fn is_executable(&self) -> bool {
        !self.blocked && self.preflight_errors.is_empty() && self.action != CleanupAction::Blocked
    }
}

/// Build a list of cleanup candidates from findings, applying policy and
/// preflight validation.
pub fn build_candidates(findings: &[Finding], policy: &CleanupPolicy) -> Vec<CleanupCandidate> {
    let mut candidates = Vec::new();
    for finding in findings {
        let path = match &finding.target {
            Target::Path(p) => p.clone(),
            _ => continue,
        };
        let action = policy.action_for(&finding.category);
        let risk = super::policy::risk_for_category(&finding.category);
        let mut candidate = CleanupCandidate {
            finding_id: finding.id.clone(),
            path,
            category: finding.category,
            size_bytes: finding.size_bytes.unwrap_or(0),
            action,
            risk,
            reason: finding.description.clone(),
            preflight_errors: Vec::new(),
            preflight_warnings: Vec::new(),
            blocked: false,
        };
        apply_preflight(&mut candidate, policy);
        candidates.push(candidate);
    }
    candidates
}

fn apply_preflight(candidate: &mut CleanupCandidate, policy: &CleanupPolicy) {
    if policy.is_protected(&candidate.path) {
        candidate.blocked = true;
        candidate.preflight_errors.push("protected path".to_string());
    }

    if candidate.action == CleanupAction::Blocked {
        candidate.blocked = true;
        candidate.preflight_errors.push("category blocked by policy".to_string());
    }

    if candidate.path.is_symlink() && !policy.follow_symlinks {
        candidate.preflight_errors.push("symbolic link refused without explicit follow".to_string());
    }

    if let Err(e) = verify_can_cleanup(&candidate.path) {
        candidate.preflight_errors.push(e);
    }

    if candidate.path == std::env::current_dir().unwrap_or_default() {
        candidate.preflight_warnings.push("path is the current working directory".to_string());
    }
}
