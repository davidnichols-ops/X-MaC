//! Package-manager diagnostics engine.
//!
//! Detects installed package managers (Homebrew, MacPorts, Nix, etc.) and
//! runs their built-in diagnostic commands in the background, capturing the
//! output as findings. This leverages the PM's own health-check tooling
//! rather than reimplementing it.

use async_trait::async_trait;
use std::process::Command;
use std::sync::Arc;
use std::time::Instant;

use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

pub struct DiagEngine {
    /// If true, only run read-only diagnostic commands (no network, no
    /// filesystem mutations). This is the default for the `safe` mode.
    safe_only: bool,
}

impl DiagEngine {
    pub fn new(safe_only: bool) -> Self {
        Self { safe_only }
    }
}

impl Default for DiagEngine {
    fn default() -> Self {
        Self::new(true)
    }
}

#[async_trait]
impl Engine for DiagEngine {
    fn id(&self) -> EngineId {
        // Reuse the Map engine ID since this is a discovery-style engine.
        // We don't add a new EngineId variant to keep the type system stable.
        EngineId::Map
    }

    fn name(&self) -> &'static str {
        "Diagnostics Engine"
    }

    fn description(&self) -> &'static str {
        "Runs built-in diagnostics for detected package managers (brew doctor, etc.)"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut findings_count = 0u64;

        // Detect and run diagnostics for each known package manager.
        let pms = Self::detect_package_managers();
        let items_scanned = pms.len() as u64;

        for pm in &pms {
            let diagnostics = if self.safe_only {
                pm.safe_diagnostics()
            } else {
                pm.all_diagnostics()
            };

            for diag in diagnostics {
                let finding = Self::run_diagnostic(pm, &diag);
                if let Some(f) = finding {
                    findings_count += 1;
                    ctx.emit(f).await;
                }
            }
        }

        Ok(EngineStats {
            engine: self.id(),
            duration: start.elapsed(),
            items_scanned,
            findings_count,
            errors_count: 0,
        })
    }
}

impl DiagEngine {
    /// Detect which package managers are installed by checking for their
    /// binary on PATH or at known locations.
    fn detect_package_managers() -> Vec<PackageManager> {
        let mut pms = Vec::new();

        if Self::command_exists("brew") {
            pms.push(PackageManager {
                name: "Homebrew",
                binary: "brew",
            });
        }
        if Self::command_exists("port") {
            pms.push(PackageManager {
                name: "MacPorts",
                binary: "port",
            });
        }
        if Self::command_exists("nix") {
            pms.push(PackageManager {
                name: "Nix",
                binary: "nix",
            });
        }
        if Self::command_exists("cargo") {
            pms.push(PackageManager {
                name: "Cargo (Rust)",
                binary: "cargo",
            });
        }
        if Self::command_exists("pip3") {
            pms.push(PackageManager {
                name: "pip (Python)",
                binary: "pip3",
            });
        }
        if Self::command_exists("npm") {
            pms.push(PackageManager {
                name: "npm (Node.js)",
                binary: "npm",
            });
        }

        pms
    }

    fn command_exists(cmd: &str) -> bool {
        Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Run a single diagnostic command and produce a finding with the output.
    fn run_diagnostic(pm: &PackageManager, diag: &Diagnostic) -> Option<Finding> {
        let output = Command::new(pm.binary)
            .args(&diag.args)
            .output()
            .ok()?;

        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let combined = if stdout.is_empty() {
            stderr.clone()
        } else if stderr.is_empty() {
            stdout.clone()
        } else {
            format!("{}\n{}", stdout, stderr)
        };

        // `brew doctor` exits non-zero when it finds issues.
        let has_issues = !output.status.success() || diag.issues_from_output(&combined);

        let severity = if has_issues {
            diag.severity_on_issue
        } else {
            Severity::Info
        };

        let title = if has_issues {
            format!("{}: {} found issues", pm.name, diag.label)
        } else {
            format!("{}: {} passed", pm.name, diag.label)
        };

        let description = if combined.is_empty() {
            format!("{} {} completed with no output", pm.binary, diag.args.join(" "))
        } else {
            // Truncate very long output to keep findings manageable.
            let truncated = if combined.len() > 2000 {
                format!("{}...(truncated)", &combined[..2000])
            } else {
                combined
            };
            format!("{} {} output:\n{}", pm.binary, diag.args.join(" "), truncated)
        };

        let finding = Finding::new(
            EngineId::Map,
            severity,
            Category::PackageManager,
            Target::Package(pm.name.to_string()),
            title,
            description,
        )
        .with_metadata("package_manager", serde_json::json!(pm.name))
        .with_metadata("command", serde_json::json!(format!("{} {}", pm.binary, diag.args.join(" "))))
        .with_metadata("exit_code", serde_json::json!(output.status.code().unwrap_or(-1)))
        .with_metadata("has_issues", serde_json::json!(has_issues));

        let finding = if has_issues && !diag.remediation_hint.is_empty() {
            finding.with_hint(diag.remediation_hint.to_string())
        } else {
            finding
        };

        Some(finding)
    }
}

// -- model ------------------------------------------------------------------

struct PackageManager {
    name: &'static str,
    binary: &'static str,
}

struct Diagnostic {
    label: &'static str,
    args: Vec<&'static str>,
    severity_on_issue: Severity,
    remediation_hint: &'static str,
    /// If true, this diagnostic requires network or may modify state —
    /// excluded in safe mode.
    #[allow(dead_code)]
    requires_network: bool,
}

impl Diagnostic {
    fn issues_from_output(&self, _output: &str) -> bool {
        // For now we rely on exit code. `brew doctor` exits non-zero when
        // it finds issues, which is the main one we care about.
        false
    }
}

impl PackageManager {
    /// Read-only diagnostics that don't require network access.
    fn safe_diagnostics(&self) -> Vec<Diagnostic> {
        match self.binary {
            "brew" => vec![
                Diagnostic {
                    label: "brew doctor",
                    args: vec!["doctor"],
                    severity_on_issue: Severity::Medium,
                    remediation_hint: "Run `brew doctor` manually and follow its suggestions",
                    requires_network: false,
                },
                Diagnostic {
                    label: "brew missing",
                    args: vec!["missing"],
                    severity_on_issue: Severity::Medium,
                    remediation_hint: "Run `brew missing` and install the missing dependencies",
                    requires_network: false,
                },
                Diagnostic {
                    label: "brew --version",
                    args: vec!["--version"],
                    severity_on_issue: Severity::Info,
                    remediation_hint: "",
                    requires_network: false,
                },
            ],
            "port" => vec![
                Diagnostic {
                    label: "port version",
                    args: vec!["version"],
                    severity_on_issue: Severity::Info,
                    remediation_hint: "",
                    requires_network: false,
                },
            ],
            "nix" => vec![
                Diagnostic {
                    label: "nix --version",
                    args: vec!["--version"],
                    severity_on_issue: Severity::Info,
                    remediation_hint: "",
                    requires_network: false,
                },
            ],
            "cargo" => vec![
                Diagnostic {
                    label: "cargo --version",
                    args: vec!["--version"],
                    severity_on_issue: Severity::Info,
                    remediation_hint: "",
                    requires_network: false,
                },
            ],
            "pip3" => vec![
                Diagnostic {
                    label: "pip3 --version",
                    args: vec!["--version"],
                    severity_on_issue: Severity::Info,
                    remediation_hint: "",
                    requires_network: false,
                },
            ],
            "npm" => vec![
                Diagnostic {
                    label: "npm --version",
                    args: vec!["--version"],
                    severity_on_issue: Severity::Info,
                    remediation_hint: "",
                    requires_network: false,
                },
                Diagnostic {
                    label: "npm doctor",
                    args: vec!["doctor"],
                    severity_on_issue: Severity::Medium,
                    remediation_hint: "Run `npm doctor` manually and follow its suggestions",
                    requires_network: false,
                },
            ],
            _ => vec![],
        }
    }

    /// All diagnostics including those that may require network.
    fn all_diagnostics(&self) -> Vec<Diagnostic> {
        let mut diags = self.safe_diagnostics();
        match self.binary {
            "brew" => {
                diags.push(Diagnostic {
                    label: "brew outdated",
                    args: vec!["outdated"],
                    severity_on_issue: Severity::Low,
                    remediation_hint: "Run `brew upgrade` to update outdated packages",
                    requires_network: true,
                });
            }
            _ => {}
        }
        diags
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diag_engine_default_is_safe() {
        let engine = DiagEngine::default();
        assert!(engine.safe_only);
    }

    #[test]
    fn test_diag_engine_id_is_map() {
        let engine = DiagEngine::default();
        assert_eq!(engine.id(), EngineId::Map);
    }
}
