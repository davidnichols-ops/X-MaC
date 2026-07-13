use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::core::types::{Category, Severity};

/// Action types the cleanup engine may apply to a candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CleanupAction {
    /// Move to the macOS Trash; reversible.
    Trash,
    /// Remove only if the user explicitly overrides safety policy.
    Review,
    /// Do not remove; show to the user for manual inspection.
    Blocked,
}

/// Risk level assigned to a cleanup candidate based on category and context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskLevel {
    Safe,
    Low,
    Medium,
    High,
}

/// A safety policy decides which categories are allowed, which require review,
/// and which are blocked by default.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CleanupPolicy {
    pub default_action: CleanupAction,
    pub allow_permanent_delete: bool,
    pub category_actions: HashMap<String, CleanupAction>,
    pub protected_paths: Vec<PathBuf>,
    pub protected_path_prefixes: Vec<PathBuf>,
    pub allow_trash_overrides: bool,
    pub follow_symlinks: bool,
}

impl Default for CleanupPolicy {
    fn default() -> Self {
        Self::safe()
    }
}

impl CleanupPolicy {
    /// Conservative default: every destructive action uses Trash, no permanent
    /// delete, and high-risk categories are blocked until reviewed.
    pub fn safe() -> Self {
        let mut category_actions = HashMap::new();
        for cat in [
            "cache",
            "temp_file",
            "build_artifact",
            "package_manager_cache",
            "browser_cache",
            "log",
            "trash_bin",
            "document_version",
            "language_file",
            "xcode_artifact",
        ] {
            category_actions.insert(cat.to_string(), CleanupAction::Trash);
        }
        for cat in [
            "orphan_file",
            "mail_attachment",
            "ios_backup",
            "large_file",
            "duplicate_file",
            "universal_binary",
        ] {
            category_actions.insert(cat.to_string(), CleanupAction::Review);
        }
        for cat in [
            "system_info",
            "system_maintenance",
            "ram_optimization",
            "permission_issue",
            "broken_symlink",
            "missing_dylib",
            "invalid_signature",
            "installed_app",
            "path_conflict",
            "env_var_conflict",
            "port_conflict",
            "shell_conflict",
            "python_env",
            "node_env",
            "container_runtime",
            "package_manager",
        ] {
            category_actions.insert(cat.to_string(), CleanupAction::Blocked);
        }

        // Add Time Machine / backup volumes as protected paths so the
        // cleanup executor will never delete anything on them, even if a
        // scan somehow produced a finding pointing into a backup.
        let mut protected_paths = default_protected_paths();
        protected_paths.extend(crate::util::backup::backup_volumes());
        let mut protected_prefixes = default_protected_prefixes();
        protected_prefixes.extend(crate::util::backup::backup_volumes());

        Self {
            default_action: CleanupAction::Trash,
            allow_permanent_delete: false,
            category_actions,
            protected_paths,
            protected_path_prefixes: protected_prefixes,
            allow_trash_overrides: true,
            follow_symlinks: false,
        }
    }

    /// Returns the action for a given category, falling back to the default.
    pub fn action_for(&self, category: &Category) -> CleanupAction {
        let key = serde_json::to_string(category)
            .unwrap_or_default()
            .trim_matches('"')
            .to_string();
        self.category_actions
            .get(&key)
            .copied()
            .unwrap_or(self.default_action)
    }

    /// Decide whether a path is protected. The check is case-insensitive on
    /// macOS and resolves the path before comparing it.
    pub fn is_protected(&self, path: &Path) -> bool {
        let resolved = match path.canonicalize() {
            Ok(p) => p,
            Err(_) => path.to_path_buf(),
        };

        let path_lower = resolved.to_string_lossy().to_lowercase();
        for protected in &self.protected_paths {
            let protected_lower = protected.to_string_lossy().to_lowercase();
            if path_lower == protected_lower {
                return true;
            }
        }
        for prefix in &self.protected_path_prefixes {
            let prefix_lower = prefix.to_string_lossy().to_lowercase();
            if path_lower == prefix_lower || path_lower.starts_with(&format!("{}/", prefix_lower)) {
                return true;
            }
        }
        false
    }
}

fn default_protected_paths() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/"),
        PathBuf::from("/System"),
        PathBuf::from("/Applications"),
        PathBuf::from("/Library"),
        PathBuf::from("/usr"),
        PathBuf::from("/bin"),
        PathBuf::from("/sbin"),
        PathBuf::from("/private/var/db"),
        PathBuf::from("/private/var/root"),
        PathBuf::from("/dev"),
        PathBuf::from("/etc"),
        PathBuf::from("/Volumes"),
    ]
}

fn default_protected_prefixes() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/System"),
        PathBuf::from("/usr"),
        PathBuf::from("/bin"),
        PathBuf::from("/sbin"),
        PathBuf::from("/Library"),
        PathBuf::from("/Applications"),
        PathBuf::from("/private/var/db"),
        PathBuf::from("/private/var/root"),
        PathBuf::from("/dev"),
        PathBuf::from("/etc"),
        PathBuf::from("/Volumes"),
    ]
}

/// Decide whether a given severity should force review.
#[allow(dead_code)]
pub fn severity_requires_review(severity: Severity) -> bool {
    matches!(severity, Severity::High | Severity::Critical)
}

/// Compute a simple risk level for a category.
pub fn risk_for_category(category: &Category) -> RiskLevel {
    match category {
        Category::Cache
        | Category::TempFile
        | Category::BuildArtifact
        | Category::PackageManagerCache
        | Category::BrowserCache
        | Category::TrashBin
        | Category::Log => RiskLevel::Safe,
        Category::XcodeArtifact
        | Category::DocumentVersion
        | Category::LanguageFile
        | Category::UniversalBinary => RiskLevel::Low,
        Category::OrphanFile
        | Category::MailAttachment
        | Category::IosBackup
        | Category::DuplicateFile => RiskLevel::Medium,
        Category::LargeFile | Category::InstalledApp => RiskLevel::High,
        _ => RiskLevel::High,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_policy_blocks_system_paths() {
        let policy = CleanupPolicy::safe();
        assert!(policy.is_protected(Path::new("/")));
        assert!(policy.is_protected(Path::new("/System")));
        assert!(policy.is_protected(Path::new("/usr/local/bin")));
        assert!(policy.is_protected(Path::new("/Applications")));
    }

    #[test]
    fn home_child_is_not_protected() {
        let policy = CleanupPolicy::safe();
        assert!(!policy.is_protected(Path::new("/Users/david/Library/Caches")));
    }

    #[test]
    fn caches_default_to_trash() {
        let policy = CleanupPolicy::safe();
        assert_eq!(policy.action_for(&Category::Cache), CleanupAction::Trash);
    }

    #[test]
    fn large_files_default_to_review() {
        let policy = CleanupPolicy::safe();
        assert_eq!(
            policy.action_for(&Category::LargeFile),
            CleanupAction::Review
        );
    }

    #[test]
    fn system_info_is_blocked() {
        let policy = CleanupPolicy::safe();
        assert_eq!(
            policy.action_for(&Category::SystemInfo),
            CleanupAction::Blocked
        );
    }
}
