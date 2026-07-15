// Bundled rules — loaded at compile time from the rules/ directory
//
// Rules are embedded in the binary so there are no runtime file dependencies.
// The SafetyEngine loads from the `rules/` directory at runtime for development,
// and from embedded strings for release builds.

/// Get all bundled rule YAML content as (filename, content) pairs.
pub fn bundled_rules() -> Vec<(&'static str, &'static str)> {
    vec![
        ("developer.yaml", include_str!("../../rules/developer.yaml")),
        ("apps.yaml", include_str!("../../rules/apps.yaml")),
        ("system.yaml", include_str!("../../rules/system.yaml")),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::rule_engine::{SafetyEngine, SafetyRating};

    #[test]
    fn test_bundled_rules_load() {
        let rules_content = bundled_rules();
        assert!(rules_content.len() >= 3);

        let mut all_rules = Vec::new();
        for (_, yaml) in rules_content {
            let parsed: Vec<crate::safety::rule_engine::SafetyRule> =
                serde_yaml::from_str(yaml).expect("failed to parse bundled rules");
            all_rules.extend(parsed);
        }
        assert!(
            all_rules.len() >= 20,
            "should have at least 20 rules, got {}",
            all_rules.len()
        );
    }

    #[test]
    fn test_engine_from_bundled_rules() {
        let rules_content = bundled_rules();
        let mut all_rules = Vec::new();
        for (_, yaml) in rules_content {
            let parsed: Vec<crate::safety::rule_engine::SafetyRule> =
                serde_yaml::from_str(yaml).unwrap();
            all_rules.extend(parsed);
        }

        let engine = SafetyEngine::new(all_rules, vec![]).unwrap();

        // Xcode DerivedData should be safe.
        let result = engine
            .classify("~/Library/Developer/Xcode/DerivedData/MyApp/build")
            .unwrap();
        assert_eq!(result.rating, SafetyRating::Safe);

        // Keychain should be protected.
        let result = engine
            .classify("~/Library/Keychains/login.keychain-db")
            .unwrap();
        assert_eq!(result.rating, SafetyRating::Protected);

        // Slack cache should be review.
        let result = engine
            .classify("~/Library/Application Support/Slack/Cache/data")
            .unwrap();
        assert_eq!(result.rating, SafetyRating::Review);
    }
}
