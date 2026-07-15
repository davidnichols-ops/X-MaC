use std::path::PathBuf;

use crate::core::types::{Category, Finding, Target};

use super::policy::{CleanupAction, CleanupPolicy, RiskLevel};
use super::verification::verify_can_cleanup;

/// A validated cleanup candidate ready for execution.
#[derive(Debug, Clone)]
#[allow(dead_code)]
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
    /// Safety rating from the rule engine (safe/review/protected).
    pub safety_rating: Option<String>,
    /// Human-readable explanation of why this is safe/risky.
    pub safety_explanation: Option<String>,
    /// The rule that matched (if any).
    pub safety_rule: Option<String>,
    /// Confidence 0-100.
    pub safety_confidence: Option<u8>,
}

impl CleanupCandidate {
    pub fn is_executable(&self) -> bool {
        !self.blocked
            && self.preflight_errors.is_empty()
            && self.action != CleanupAction::Blocked
            && self.safety_rating.as_deref() != Some("protected")
    }
}

/// Enrich findings with safety classifications from the rule engine.
/// This should be called before `build_candidates` so the safety info
/// is available in both the findings (for GUI display) and the candidates
/// (for execution decisions).
#[allow(dead_code)]
pub fn enrich_with_safety(findings: &mut [Finding]) {
    let engine = match crate::safety::rule_engine::SafetyEngine::load_default() {
        Ok(e) => e,
        Err(_) => return, // No rules available — skip enrichment.
    };

    for finding in findings.iter_mut() {
        if let Target::Path(ref path) = finding.target {
            if let Some(classification) = engine.classify(&path.to_string_lossy()) {
                finding.safety_rating = Some(classification.rating.label().to_string());
                finding.safety_rule = Some(classification.rule_name.clone());
                finding.safety_explanation = Some(classification.explanation());
                finding.safety_confidence = Some(classification.confidence);
            }
        }
    }
}

/// Build a list of cleanup candidates from findings, applying policy and
/// preflight validation. Also applies safety classifications from the
/// rule engine to block protected paths and provide explanations.
pub fn build_candidates(findings: &[Finding], policy: &CleanupPolicy) -> Vec<CleanupCandidate> {
    // Load safety engine for classification.
    let safety_engine = crate::safety::rule_engine::SafetyEngine::load_default().ok();

    let mut candidates = Vec::new();
    for finding in findings {
        let path = match &finding.target {
            Target::Path(p) => p.clone(),
            _ => continue,
        };

        // Get safety classification — prefer finding's existing one, else classify.
        let classification = if finding.safety_rating.is_some() {
            // Already enriched.
            None
        } else if let Some(ref engine) = safety_engine {
            engine.classify(&path.to_string_lossy())
        } else {
            None
        };

        // Determine action: safety rating overrides policy for protected.
        let action = if classification
            .as_ref()
            .map(|c| c.rating.is_protected())
            .unwrap_or(false)
            || finding.safety_rating.as_deref() == Some("protected")
        {
            CleanupAction::Blocked
        } else {
            policy.action_for(&finding.category)
        };

        let risk = super::policy::risk_for_category(&finding.category);
        let reason = finding
            .safety_explanation
            .clone()
            .unwrap_or_else(|| finding.description.clone());

        let mut candidate = CleanupCandidate {
            finding_id: finding.id.clone(),
            path,
            category: finding.category,
            size_bytes: finding.size_bytes.unwrap_or(0),
            action,
            risk,
            reason,
            preflight_errors: Vec::new(),
            preflight_warnings: Vec::new(),
            blocked: false,
            safety_rating: finding.safety_rating.clone().or_else(|| {
                classification
                    .as_ref()
                    .map(|c| c.rating.label().to_string())
            }),
            safety_explanation: finding
                .safety_explanation
                .clone()
                .or_else(|| classification.as_ref().map(|c| c.explanation())),
            safety_rule: finding
                .safety_rule
                .clone()
                .or_else(|| classification.as_ref().map(|c| c.rule_name.clone())),
            safety_confidence: finding
                .safety_confidence
                .or_else(|| classification.as_ref().map(|c| c.confidence)),
        };
        apply_preflight(&mut candidate, policy);
        candidates.push(candidate);
    }
    candidates
}

/// op 312: Test before execution — apply preflight validation checks to a
/// candidate before any destructive action is taken. Verifies protected
/// paths, blocked categories, symlink safety, and writability.
fn apply_preflight(candidate: &mut CleanupCandidate, policy: &CleanupPolicy) {
    if policy.is_protected(&candidate.path) {
        candidate.blocked = true;
        candidate
            .preflight_errors
            .push("protected path".to_string());
    }

    if candidate.action == CleanupAction::Blocked {
        candidate.blocked = true;
        candidate
            .preflight_errors
            .push("category blocked by policy".to_string());
    }

    if candidate.path.is_symlink() && !policy.follow_symlinks {
        candidate
            .preflight_errors
            .push("symbolic link refused without explicit follow".to_string());
    }

    if let Err(e) = verify_can_cleanup(&candidate.path) {
        candidate.preflight_errors.push(e);
    }

    if candidate.path == std::env::current_dir().unwrap_or_default() {
        candidate
            .preflight_warnings
            .push("path is the current working directory".to_string());
    }
}
