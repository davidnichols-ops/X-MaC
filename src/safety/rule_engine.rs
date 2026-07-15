// SafetyEngine — loads YAML rules and classifies files by risk level
//
// Three-tier classification (same model as Gargantua):
//   Safe      — disposable, regenerates on demand. Preselected in cleanup.
//   Review    — might hold state worth a look. Shown but never auto-selected.
//   Protected — system impact or data-loss risk. Hard-blocked from deletion.
//
// The engine loads rules from the bundled `rules/` directory at startup.
// Rules are immutable once loaded — no runtime mutation, no remote rules.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

// ═══════════════════════════════════════════════════════════════════════
//  Rule Definitions
// ═══════════════════════════════════════════════════════════════════════

/// Three-tier safety rating. A model can explain a rating, never lower it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SafetyRating {
    Safe,
    Review,
    Protected,
}

impl SafetyRating {
    pub fn can_delete(&self) -> bool {
        matches!(self, SafetyRating::Safe)
    }

    pub fn is_protected(&self) -> bool {
        matches!(self, SafetyRating::Protected)
    }

    pub fn label(&self) -> &'static str {
        match self {
            SafetyRating::Safe => "safe",
            SafetyRating::Review => "review",
            SafetyRating::Protected => "protected",
        }
    }

    pub fn color_hex(&self) -> &'static str {
        match self {
            SafetyRating::Safe => "#34c759",      // green
            SafetyRating::Review => "#ff9f0a",    // orange
            SafetyRating::Protected => "#ff453a", // red
        }
    }
}

/// A single cleanup rule loaded from YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyRule {
    /// Unique rule name (e.g., "xcode_derived_data").
    pub name: String,
    /// Human-readable explanation of why this rule exists.
    pub description: String,
    /// The safety rating assigned to matching files.
    pub rating: SafetyRating,
    /// Glob patterns matching file paths (supports ~, **, *).
    pub paths: Vec<String>,
    /// Confidence score 0-100.
    #[serde(default = "default_confidence")]
    pub confidence: u8,
    /// Upstream commit this rule was imported from.
    #[serde(default)]
    pub upstream_commit: Option<String>,
    /// Category for grouping (e.g., "developer", "apps", "system").
    #[serde(default)]
    pub category: Option<String>,
}

fn default_confidence() -> u8 {
    80
}

/// The result of classifying a file path against the rule set.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyClassification {
    pub path: String,
    pub rating: SafetyRating,
    pub rule_name: String,
    pub rule_description: String,
    pub confidence: u8,
    pub category: Option<String>,
    /// Whether the file is preselected for deletion (only Safe rating).
    pub preselected: bool,
}

impl SafetyClassification {
    /// Plain-language explanation for the GUI.
    pub fn explanation(&self) -> String {
        format!(
            "[{}] {} (confidence: {}/100, rule: {})",
            self.rating.label(),
            self.rule_description,
            self.confidence,
            self.rule_name,
        )
    }
}

// ═══════════════════════════════════════════════════════════════════════
//  Safety Engine
// ═══════════════════════════════════════════════════════════════════════

/// The safety engine. Loads rules once at startup and classifies paths.
pub struct SafetyEngine {
    rules: Vec<SafetyRule>,
    /// Compiled glob patterns for fast matching.
    compiled: Vec<(glob::Pattern, usize)>, // (pattern, rule_index)
    /// Protected-roots paths that are hard-blocked regardless of rules.
    protected_roots: Vec<String>,
}

impl SafetyEngine {
    /// Load rules from the bundled `rules/` directory.
    pub fn load_default() -> anyhow::Result<Self> {
        let rules_dir = Path::new("rules");
        Self::load_from_dir(rules_dir)
    }

    /// Load rules from a specific directory.
    pub fn load_from_dir(dir: &Path) -> anyhow::Result<Self> {
        let mut rules = Vec::new();
        if dir.exists() {
            for entry in std::fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|e| e.to_str()) == Some("yaml")
                    || path.extension().and_then(|e| e.to_str()) == Some("yml")
                {
                    let contents = std::fs::read_to_string(&path)?;
                    let parsed: Vec<SafetyRule> = serde_yaml::from_str(&contents)
                        .map_err(|e| anyhow::anyhow!("parse {}: {}", path.display(), e))?;
                    rules.extend(parsed);
                }
            }
        }

        // Always include built-in protected roots.
        let protected_roots = vec![
            "~/Library/Keychains/**".to_string(),
            "~/Library/Preferences/**".to_string(),
            "/System/**".to_string(),
            "/usr/**".to_string(),
            "/bin/**".to_string(),
            "/sbin/**".to_string(),
            "~/.ssh/**".to_string(),
        ];

        Self::new(rules, protected_roots)
    }

    /// Create a new engine from explicit rules.
    pub fn new(rules: Vec<SafetyRule>, protected_roots: Vec<String>) -> anyhow::Result<Self> {
        let mut compiled = Vec::new();
        for (idx, rule) in rules.iter().enumerate() {
            for pattern_str in &rule.paths {
                let expanded = expand_path(pattern_str);
                if let Ok(pattern) = glob::Pattern::new(&expanded) {
                    compiled.push((pattern, idx));
                }
            }
        }
        Ok(Self {
            rules,
            compiled,
            protected_roots,
        })
    }

    /// Classify a single file path. Returns None if no rule matches.
    pub fn classify(&self, path: &str) -> Option<SafetyClassification> {
        let expanded = expand_path(path);

        // First check protected roots — these override everything.
        for root in &self.protected_roots {
            let root_expanded = expand_path(root);
            if let Ok(pattern) = glob::Pattern::new(&root_expanded) {
                if pattern.matches(&expanded) {
                    return Some(SafetyClassification {
                        path: path.to_string(),
                        rating: SafetyRating::Protected,
                        rule_name: "protected_roots".to_string(),
                        rule_description: "Protected by bundled protected-roots policy".to_string(),
                        confidence: 100,
                        category: Some("system".to_string()),
                        preselected: false,
                    });
                }
            }
        }

        // Then check rules — first match wins (rules should be ordered
        // most-specific to least-specific).
        for (pattern, rule_idx) in &self.compiled {
            if pattern.matches(&expanded) || pattern.matches(path) {
                let rule = &self.rules[*rule_idx];
                return Some(SafetyClassification {
                    path: path.to_string(),
                    rating: rule.rating,
                    rule_name: rule.name.clone(),
                    rule_description: rule.description.clone(),
                    confidence: rule.confidence,
                    category: rule.category.clone(),
                    preselected: rule.rating.can_delete(),
                });
            }
        }

        None
    }

    /// Classify multiple paths at once.
    pub fn classify_batch(&self, paths: &[String]) -> Vec<SafetyClassification> {
        paths.iter().filter_map(|p| self.classify(p)).collect()
    }

    /// Check if a path is protected (hard-blocked from deletion).
    pub fn is_protected(&self, path: &str) -> bool {
        self.classify(path)
            .map(|c| c.rating.is_protected())
            .unwrap_or(false)
    }

    /// Get all loaded rules.
    pub fn rules(&self) -> &[SafetyRule] {
        &self.rules
    }

    /// Get rule count by rating.
    pub fn rule_counts(&self) -> HashMap<String, usize> {
        let mut counts = HashMap::new();
        for rule in &self.rules {
            *counts.entry(rule.rating.label().to_string()).or_insert(0) += 1;
        }
        counts
    }

    /// Get a rule by name.
    pub fn get_rule(&self, name: &str) -> Option<&SafetyRule> {
        self.rules.iter().find(|r| r.name == name)
    }
}

/// Expand ~ in a path to the home directory.
fn expand_path(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return format!("{}/{}", home.display(), rest);
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_engine() -> SafetyEngine {
        let rules = vec![
            SafetyRule {
                name: "xcode_derived_data".to_string(),
                description: "Xcode build intermediates".to_string(),
                rating: SafetyRating::Safe,
                paths: vec!["~/Library/Developer/Xcode/DerivedData/**".to_string()],
                confidence: 98,
                upstream_commit: Some("a7c19e4".to_string()),
                category: Some("developer".to_string()),
            },
            SafetyRule {
                name: "slack_cache".to_string(),
                description: "Slack cached workspace data".to_string(),
                rating: SafetyRating::Review,
                paths: vec!["~/Library/Application Support/Slack/Cache/**".to_string()],
                confidence: 71,
                upstream_commit: None,
                category: Some("apps".to_string()),
            },
        ];
        let protected = vec![
            "~/Library/Keychains/**".to_string(),
            "/System/**".to_string(),
        ];
        SafetyEngine::new(rules, protected).unwrap()
    }

    #[test]
    fn test_classify_safe() {
        let engine = make_engine();
        let result = engine
            .classify("~/Library/Developer/Xcode/DerivedData/MyApp-abc123/build")
            .unwrap();
        assert_eq!(result.rating, SafetyRating::Safe);
        assert!(result.preselected);
        assert_eq!(result.rule_name, "xcode_derived_data");
    }

    #[test]
    fn test_classify_review() {
        let engine = make_engine();
        let result = engine
            .classify("~/Library/Application Support/Slack/Cache/something")
            .unwrap();
        assert_eq!(result.rating, SafetyRating::Review);
        assert!(!result.preselected);
    }

    #[test]
    fn test_classify_protected_overrides_rules() {
        let engine = make_engine();
        let result = engine
            .classify("~/Library/Keychains/login.keychain-db")
            .unwrap();
        assert_eq!(result.rating, SafetyRating::Protected);
        assert_eq!(result.rule_name, "protected_roots");
        assert!(!result.preselected);
    }

    #[test]
    fn test_classify_no_match() {
        let engine = make_engine();
        let result = engine.classify("~/Documents/myfile.txt");
        assert!(result.is_none());
    }

    #[test]
    fn test_is_protected() {
        let engine = make_engine();
        assert!(engine.is_protected("/System/Library/CoreServices"));
        assert!(!engine.is_protected("~/Library/Caches/com.apple.Safari"));
    }

    #[test]
    fn test_explanation_format() {
        let engine = make_engine();
        let result = engine
            .classify("~/Library/Developer/Xcode/DerivedData/test")
            .unwrap();
        let explanation = result.explanation();
        assert!(explanation.contains("safe"));
        assert!(explanation.contains("xcode_derived_data"));
        assert!(explanation.contains("98"));
    }

    #[test]
    fn test_rule_counts() {
        let engine = make_engine();
        let counts = engine.rule_counts();
        assert_eq!(counts.get("safe"), Some(&1));
        assert_eq!(counts.get("review"), Some(&1));
    }

    #[test]
    fn test_rating_can_delete() {
        assert!(SafetyRating::Safe.can_delete());
        assert!(!SafetyRating::Review.can_delete());
        assert!(!SafetyRating::Protected.can_delete());
    }
}
