use std::collections::HashMap;
use std::path::PathBuf;
use std::fs;

use crate::core::context::ScanContext;
use crate::core::types::{Category, EngineId, Finding, Severity, Target};
use crate::util::macos::MacosUtils;

pub struct EnvConflictScanner;

impl EnvConflictScanner {
    pub fn new() -> Self {
        Self
    }

    pub async fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();
        let mut var_map: HashMap<String, Vec<(String, String)>> = HashMap::new();

        let shell_configs = Self::get_shell_config_paths();

        for (config_path, shell_type) in &shell_configs {
            if config_path.exists() {
                let vars = Self::parse_shell_config(config_path, shell_type);
                for (var_name, var_value) in vars {
                    var_map.entry(var_name).or_default().push((config_path.to_string_lossy().to_string(), var_value));
                }
            }
        }

        for (var_name, occurrences) in var_map {
            if occurrences.len() > 1 {
                let unique_values: Vec<String> = occurrences.iter()
                    .map(|(_, v)| v.clone())
                    .collect();

                if unique_values.iter().all(|v| v == &unique_values[0]) {
                    continue;
                }

                let descriptions: Vec<String> = occurrences.iter()
                    .map(|(path, value)| format!("{}={}", path.split('/').last().unwrap_or(""), value))
                    .collect();

                findings.push(
                    Finding::new(
                        EngineId::Conflict,
                        Severity::Medium,
                        Category::EnvVarConflict,
                        Target::EnvironmentVariable(var_name.clone()),
                        format!("Environment variable conflict: {}", var_name),
                        format!("Variable set to different values: {}", descriptions.join("; ")),
                    )
                    .with_metadata("conflicting_values".to_string(), serde_json::json!(occurrences.iter().map(|(p, v)| format!("{}: {}", p, v)).collect::<Vec<_>>()))
                    .with_hint("Check your shell configuration files to resolve conflicts".to_string()),
                );
            }
        }

        Ok(findings)
    }

    fn get_shell_config_paths() -> Vec<(PathBuf, &'static str)> {
        let home = MacosUtils::home_dir();
        vec![
            (home.join(".bash_profile"), "bash"),
            (home.join(".bashrc"), "bash"),
            (home.join(".zshrc"), "zsh"),
            (home.join(".zshenv"), "zsh"),
            (home.join(".zprofile"), "zsh"),
            (home.join(".config/fish/config.fish"), "fish"),
        ]
    }

    pub fn parse_shell_config(path: &PathBuf, shell_type: &str) -> HashMap<String, String> {
        let mut vars = HashMap::new();

        if let Ok(content) = fs::read_to_string(path) {
            match shell_type {
                "bash" | "zsh" => {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.starts_with("export ") {
                            let _line = line.trim_start_matches("export ");
                        }
                        if line.contains('=') {
                            if let Some((key, value)) = line.split_once('=') {
                                let key = key.trim();
                                let value = value.trim().trim_matches('"').trim_matches('\'');
                                vars.insert(key.to_string(), value.to_string());
                            }
                        }
                    }
                }
                "fish" => {
                    for line in content.lines() {
                        let line = line.trim();
                        if line.starts_with("set -x ") || line.starts_with("set -gx ") {
                            let line = line.trim_start_matches("set -x ")
                                .trim_start_matches("set -gx ");
                            if let Some((key, value)) = line.split_once(' ') {
                                vars.insert(key.trim().to_string(), value.trim().to_string());
                            }
                        }
                    }
                }
                _ => {}
            }
        }

        vars
    }

}

impl Default for EnvConflictScanner {
    fn default() -> Self {
        Self::new()
    }
}
