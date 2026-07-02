//! Privacy-first redaction logic.
//!
//! Ports the MIF Environment Mapper's `DataSanitizer` / `SENSITIVE_PATTERNS`
//! from `mif_env_mapper/config.py` and `sanitizer.py`. When `--redact` is on
//! (the default), every string that flows into a finding's title, description,
//! or metadata is run through [`Redactor::redact`] so that usernames, home
//! directory paths, emails, tokens/keys, IP addresses, UUIDs, and AWS keys
//! are replaced with `[REDACTED_*]` placeholders.

use regex::Regex;

/// A single redaction rule: a compiled regex and the replacement template.
/// Replacement templates use `$1`/`$2` style capture references (the `regex`
/// crate's syntax).
pub struct RedactionRule {
    pub regex: Regex,
    pub replacement: String,
}

/// Privacy-first redactor. Holds the MIF pattern set plus an optional
/// hostname string to scrub when configured.
pub struct Redactor {
    rules: Vec<RedactionRule>,
    hostname: Option<String>,
}

impl Default for Redactor {
    fn default() -> Self {
        Self::new()
    }
}

impl Redactor {
    /// Build the default MIF redactor with the standard sensitive pattern set.
    pub fn new() -> Self {
        Self {
            rules: default_rules(),
            hostname: None,
        }
    }

    /// Build a no-op redactor that passes all strings through unchanged.
    /// Used when `--redact` is disabled.
    pub fn disabled() -> Self {
        Self {
            rules: Vec::new(),
            hostname: None,
        }
    }

    /// Configure a hostname to scrub. When set, every occurrence of the
    /// hostname string is replaced with `[REDACTED_HOST]`.
    pub fn with_hostname(mut self, hostname: impl Into<String>) -> Self {
        let h = hostname.into();
        if !h.is_empty() {
            self.hostname = Some(h);
        }
        self
    }

    /// Number of active redaction rules (excluding the optional hostname rule).
    #[allow(dead_code)]
    pub fn rule_count(&self) -> usize {
        self.rules.len()
    }

    /// Apply every redaction rule sequentially to a raw string. Non-string
    /// data is passed through untouched by callers.
    pub fn redact(&self, text: &str) -> String {
        let mut clean = text.to_string();
        for rule in &self.rules {
            clean = rule.regex.replace_all(&clean, &rule.replacement).to_string();
        }
        if let Some(host) = &self.hostname {
            if !host.is_empty() && clean.contains(host) {
                clean = clean.replace(host, "[REDACTED_HOST]");
            }
        }
        clean
    }

    /// Recursively scrub all string values inside a `serde_json::Value`.
    /// Mirrors MIF's `sanitize_structure`.
    pub fn redact_value(&self, value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::String(s) => serde_json::Value::String(self.redact(&s)),
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(|v| self.redact_value(v)).collect())
            }
            serde_json::Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (k, v) in map {
                    out.insert(k, self.redact_value(v));
                }
                serde_json::Value::Object(out)
            }
            other => other,
        }
    }
}

/// Construct the default MIF sensitive-pattern rule set.
///
/// Order matters: the credential rule must run before the generic
/// connection-string rule so that `password=secret` is caught explicitly.
fn default_rules() -> Vec<RedactionRule> {
    let rules: Vec<(&str, &str)> = vec![
        // Credentials and secrets (passwd/password/secret/token/api_key/...).
        // Matches `name: value` or `name=value`.
        (
            r"(?i)(passwd|password|secret|token|api_key|apikey|auth_token|access_token|private_key)\s*[:=]\s*\S+",
            "${1}=[REDACTED_SECRET]",
        ),
        // macOS user home directories.
        (r"/Users/[a-zA-Z0-9_\-.]+", "/Users/[REDACTED_USER]"),
        // Linux user home directories.
        (r"/home/[a-zA-Z0-9_\-.]+", "/home/[REDACTED_USER]"),
        // SSH / GnuPG key paths.
        (r#"/\.(ssh|gnupg)/[^\s"']+"#, "/.[REDACTED_KEY_PATH]/[REDACTED]"),
        // IPv4 addresses.
        (r"\b(?:\d{1,3}\.){3}\d{1,3}\b", "[REDACTED_IP]"),
        // Email addresses.
        (
            r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}\b",
            "[REDACTED_EMAIL]",
        ),
        // UUIDs.
        (
            r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b",
            "[REDACTED_UUID]",
        ),
        // AWS access key IDs.
        (r"(?i)(AKIA[0-9A-Z]{16})", "[REDACTED_AWS_KEY]"),
        // Generic connection strings with embedded passwords.
        (r"(?i)(://[^:]+:)[^@]+(@)", "${1}[REDACTED]${2}"),
    ];

    rules
        .into_iter()
        .filter_map(|(pat, rep)| {
            let regex = Regex::new(pat).ok()?;
            Some(RedactionRule {
                regex,
                replacement: rep.to_string(),
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_macos_home_path() {
        let r = Redactor::new();
        assert_eq!(
            r.redact("/Users/alice/projects/app"),
            "/Users/[REDACTED_USER]/projects/app"
        );
    }

    #[test]
    fn redacts_linux_home_path() {
        let r = Redactor::new();
        assert_eq!(
            r.redact("/home/bob/code"),
            "/home/[REDACTED_USER]/code"
        );
    }

    #[test]
    fn redacts_email() {
        let r = Redactor::new();
        assert_eq!(
            r.redact("contact me at alice@example.com please"),
            "contact me at [REDACTED_EMAIL] please"
        );
    }

    #[test]
    fn redacts_password_assignment() {
        let r = Redactor::new();
        let out = r.redact("password=hunter2");
        assert!(out.contains("[REDACTED_SECRET]"));
        assert!(!out.contains("hunter2"));
    }

    #[test]
    fn redacts_token_colon_form() {
        let r = Redactor::new();
        let out = r.redact("auth_token: abc123secret");
        assert!(out.contains("[REDACTED_SECRET]"));
        assert!(!out.contains("abc123secret"));
    }

    #[test]
    fn redacts_ipv4() {
        let r = Redactor::new();
        assert_eq!(r.redact("connect to 10.0.0.1 now"), "connect to [REDACTED_IP] now");
    }

    #[test]
    fn redacts_uuid() {
        let r = Redactor::new();
        assert_eq!(
            r.redact("id=550e8400-e29b-41d4-a716-446655440000"),
            "id=[REDACTED_UUID]"
        );
    }

    #[test]
    fn redacts_aws_key() {
        let r = Redactor::new();
        let out = r.redact("key AKIAIOSFODNN7EXAMPLE here");
        assert!(out.contains("[REDACTED_AWS_KEY]"));
        assert!(!out.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn redacts_connection_string_password() {
        let r = Redactor::new();
        let out = r.redact("postgres://user:secretpw@host:5432/db");
        assert!(out.contains("[REDACTED]"));
        assert!(!out.contains("secretpw"));
    }

    #[test]
    fn redacts_ssh_key_path() {
        let r = Redactor::new();
        let out = r.redact("loaded /Users/alice/.ssh/id_rsa key");
        assert!(out.contains("[REDACTED_KEY_PATH]"));
        assert!(!out.contains("id_rsa"));
    }

    #[test]
    fn hostname_redaction_when_configured() {
        let r = Redactor::new().with_hostname("MacBook-Pro.local");
        assert_eq!(
            r.redact("host is MacBook-Pro.local today"),
            "host is [REDACTED_HOST] today"
        );
    }

    #[test]
    fn hostname_redaction_noop_when_not_set() {
        let r = Redactor::new();
        // No hostname configured — string passes through.
        assert_eq!(r.redact("MacBook-Pro.local"), "MacBook-Pro.local");
    }

    #[test]
    fn redact_value_recurses_into_objects_and_arrays() {
        let r = Redactor::new();
        let v = serde_json::json!({
            "path": "/Users/alice/x",
            "nested": ["alice@example.com", 42, {"secret": "password=hunter2"}]
        });
        let out = r.redact_value(v);
        assert_eq!(out["path"], "/Users/[REDACTED_USER]/x");
        assert_eq!(out["nested"][0], "[REDACTED_EMAIL]");
        assert_eq!(out["nested"][1], 42);
        assert!(out["nested"][2]["secret"].as_str().unwrap().contains("[REDACTED_SECRET]"));
    }

    #[test]
    fn non_sensitive_string_unchanged() {
        let r = Redactor::new();
        assert_eq!(r.redact("git 2.43.0"), "git 2.43.0");
    }

    #[test]
    fn default_rule_count_matches_mif() {
        let r = Redactor::new();
        // MIF defines 9 sensitive patterns in SENSITIVE_PATTERNS.
        assert_eq!(r.rule_count(), 9);
    }
}
