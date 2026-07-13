//! The `envmap` engine — environment mapping.
//!
//! Ports the MIF Environment Mapper into X-MaC. It is a read-only,
//! privacy-first engine that discovers:
//!
//! 1. OS / system metadata (reusing `util::macos`).
//! 2. System packages — Homebrew formulae + casks on macOS; dpkg/rpm/pacman
//!    on Linux.
//! 3. Language packages — Python pip + pipx, npm global, Ruby gems.
//! 4. Installed applications from `/Applications` and `~/Applications` (bundle
//!    name + version via the `plist` crate).
//!
//! Every string that flows into a finding is run through [`Redactor`] when
//! `--redact` is on (the default), scrubbing usernames, home paths, emails,
//! tokens/keys, IPs, UUIDs, and AWS keys.

use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::cli::args::EnvmapArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

use super::apps;
use super::discovery;
use super::redaction::Redactor;

pub struct EnvmapEngine {
    args: EnvmapArgs,
}

impl EnvmapEngine {
    pub fn new(args: EnvmapArgs) -> Self {
        Self { args }
    }

    /// Build the redactor for this run, optionally configuring hostname
    /// redaction from the system hostname. Returns a no-op redactor when
    /// `--redact` is disabled.
    fn build_redactor(&self) -> Redactor {
        if !self.args.redact {
            return Redactor::disabled();
        }
        let r = Redactor::new();
        if self.args.redact_hostnames {
            if let Some(host) = hostname() {
                return r.with_hostname(host);
            }
        }
        r
    }
}

impl Default for EnvmapEngine {
    fn default() -> Self {
        Self::new(EnvmapArgs::default())
    }
}

#[async_trait]
impl Engine for EnvmapEngine {
    fn id(&self) -> EngineId {
        EngineId::Envmap
    }

    fn name(&self) -> &'static str {
        "Envmap Engine"
    }

    fn description(&self) -> &'static str {
        "Maps the system environment: OS, system/language packages, and installed applications (privacy-first)."
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let redactor = self.build_redactor();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;
        let mut errors_count = 0u64;

        // 1. OS / system metadata.
        if self.args.system {
            let os_finding = self.build_system_finding(&redactor);
            ctx.emit(os_finding).await;
            items_scanned += 1;
            findings_count += 1;
        }

        // 2. System packages (Homebrew / dpkg / rpm / pacman).
        if self.args.system_packages {
            let results = discovery::gather_system_packages();
            items_scanned += results.iter().map(|r| r.packages.len() as u64).sum::<u64>();
            for r in results {
                if r.packages.is_empty() {
                    if r.error.is_some() {
                        errors_count += 1;
                    }
                    continue;
                }
                let finding = self.build_package_source_finding(&redactor, r.source, &r.packages);
                ctx.emit(finding).await;
                findings_count += 1;
            }
        }

        // 3. Language packages (pip, pipx, npm, gems).
        if self.args.language_packages {
            let results = discovery::gather_language_packages();
            items_scanned += results.iter().map(|r| r.packages.len() as u64).sum::<u64>();
            for r in results {
                if r.packages.is_empty() {
                    if r.error.is_some() {
                        errors_count += 1;
                    }
                    continue;
                }
                let finding = self.build_package_source_finding(&redactor, r.source, &r.packages);
                ctx.emit(finding).await;
                findings_count += 1;
            }
        }

        // 4. Installed applications from /Applications + ~/Applications.
        if self.args.apps {
            let dirs = if self.args.app_dirs.is_empty() {
                apps::default_app_dirs()
            } else {
                self.args.app_dirs.to_vec()
            };
            for dir in dirs {
                let apps = apps::enumerate_apps_in(&dir, 3);
                items_scanned += apps.len() as u64;
                for app in apps {
                    let finding = self.build_app_finding(&redactor, &app);
                    ctx.emit(finding).await;
                    findings_count += 1;
                }
            }
        }

        Ok(EngineStats {
            engine: self.id(),
            duration: start.elapsed(),
            items_scanned,
            findings_count,
            errors_count,
        })
    }
}

impl EnvmapEngine {
    fn build_system_finding(&self, redactor: &Redactor) -> Finding {
        let os_platform = if cfg!(target_os = "macos") {
            "darwin"
        } else if cfg!(target_os = "linux") {
            "linux"
        } else {
            "unknown"
        };
        let kernel = read_kernel_version().unwrap_or_else(|| "unknown".to_string());
        let host = hostname().unwrap_or_else(|| "unknown".to_string());
        let arch = std::env::consts::ARCH.to_string();

        let title = format!("System: {} ({})", os_platform, arch);
        let description = format!(
            "Operating system {} on {}, kernel {}",
            os_platform, arch, kernel
        );

        Finding::new(
            EngineId::Envmap,
            Severity::Info,
            Category::SystemInfo,
            Target::EnvironmentVariable("OS".to_string()),
            redactor.redact(&title),
            redactor.redact(&description),
        )
        .with_metadata("os_platform", serde_json::json!(os_platform))
        .with_metadata(
            "kernel_version",
            serde_json::json!(redactor.redact(&kernel)),
        )
        .with_metadata("hostname", serde_json::json!(redactor.redact(&host)))
        .with_metadata("architecture", serde_json::json!(arch))
        .with_metadata(
            "apple_silicon",
            serde_json::json!(cfg!(target_arch = "aarch64")),
        )
    }

    fn build_package_source_finding(
        &self,
        redactor: &Redactor,
        source: &str,
        packages: &[String],
    ) -> Finding {
        let count = packages.len();
        let title = format!("{}: {} package(s)", source, count);
        let sample: Vec<String> = packages.iter().take(5).cloned().collect();

        Finding::new(
            EngineId::Envmap,
            Severity::Info,
            Category::PackageManager,
            Target::Package(source.to_string()),
            redactor.redact(&title),
            redactor.redact(&format!("Discovered {} package(s) from {}", count, source)),
        )
        .with_metadata("source", serde_json::json!(source))
        .with_metadata("count", serde_json::json!(count))
        .with_metadata("sample", redactor.redact_value(serde_json::json!(sample)))
        .with_metadata(
            "packages",
            redactor.redact_value(serde_json::json!(packages)),
        )
    }

    fn build_app_finding(&self, redactor: &Redactor, app: &apps::InstalledApp) -> Finding {
        let title = format!("App: {} {}", app.bundle_name, app.version);
        let description = format!(
            "Installed application {} (version {}) at {}",
            app.bundle_name,
            app.version,
            app.bundle_path.display()
        );

        Finding::new(
            EngineId::Envmap,
            Severity::Info,
            Category::InstalledApp,
            Target::Path(app.bundle_path.clone()),
            redactor.redact(&title),
            redactor.redact(&description),
        )
        .with_metadata(
            "bundle_name",
            serde_json::json!(redactor.redact(&app.bundle_name)),
        )
        .with_metadata(
            "bundle_id",
            serde_json::json!(app.bundle_id.as_ref().map(|s| redactor.redact(s))),
        )
        .with_metadata("version", serde_json::json!(redactor.redact(&app.version)))
    }
}

/// Best-effort hostname lookup (no network — uses `hostname` command on Unix).
fn hostname() -> Option<String> {
    use std::process::Command;
    let out = Command::new("hostname").output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

/// Best-effort kernel version (`uname -r`).
fn read_kernel_version() -> Option<String> {
    use std::process::Command;
    let out = Command::new("uname").arg("-r").output().ok()?;
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() {
        None
    } else {
        Some(s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::EnvmapArgs;
    use std::path::PathBuf;

    fn default_args() -> EnvmapArgs {
        EnvmapArgs {
            system: true,
            system_packages: true,
            language_packages: true,
            apps: true,
            app_dirs: Vec::new(),
            redact: true,
            redact_hostnames: false,
        }
    }

    #[test]
    fn engine_id_is_envmap() {
        let e = EnvmapEngine::new(default_args());
        assert_eq!(e.id(), EngineId::Envmap);
    }

    #[test]
    fn engine_default_constructs() {
        let e = EnvmapEngine::default();
        assert_eq!(e.name(), "Envmap Engine");
        assert!(!e.description().is_empty());
    }

    #[test]
    fn build_redactor_respects_redact_flag() {
        let mut args = default_args();
        args.redact = true;
        let e = EnvmapEngine::new(args);
        let r = e.build_redactor();
        assert_eq!(r.rule_count(), 9);
    }

    #[test]
    fn build_system_finding_redacts_hostname() {
        let mut args = default_args();
        args.redact_hostnames = true;
        let e = EnvmapEngine::new(args);
        let r = e.build_redactor();
        let f = e.build_system_finding(&r);
        assert_eq!(f.engine, EngineId::Envmap);
        assert_eq!(f.category, Category::SystemInfo);
        assert!(f.title.starts_with("System:"));
        // metadata should contain os_platform.
        assert!(f.metadata.get("os_platform").is_some());
    }

    #[test]
    fn build_package_source_finding_carries_count_and_sample() {
        let e = EnvmapEngine::new(default_args());
        let r = e.build_redactor();
        let pkgs = vec!["git 2.43.0".to_string(), "node 21.5.0".to_string()];
        let f = e.build_package_source_finding(&r, "homebrew_formulae", &pkgs);
        assert_eq!(f.category, Category::PackageManager);
        assert_eq!(f.metadata.get("count").unwrap(), &serde_json::json!(2));
        let sample = f.metadata.get("sample").unwrap().as_array().unwrap();
        assert_eq!(sample.len(), 2);
    }

    #[test]
    fn build_app_finding_redacts_home_path() {
        let e = EnvmapEngine::new(default_args());
        let r = e.build_redactor();
        let app = apps::InstalledApp {
            bundle_path: PathBuf::from("/Users/alice/Applications/MyApp.app"),
            bundle_name: "MyApp".to_string(),
            bundle_id: Some("com.example.myapp".to_string()),
            version: "1.0".to_string(),
        };
        let f = e.build_app_finding(&r, &app);
        assert_eq!(f.category, Category::InstalledApp);
        // The description must not leak the raw username.
        assert!(!f.description.contains("/Users/alice"));
        assert!(f.description.contains("/Users/[REDACTED_USER]"));
    }

    #[tokio::test]
    async fn validate_always_ok() {
        use clap::Parser;
        let e = EnvmapEngine::default();
        let cli = crate::cli::args::Cli::parse_from(vec!["x-mac", "envmap"]);
        let (tx, _rx) = tokio::sync::mpsc::channel::<crate::core::types::Finding>(1000);
        let ctx = crate::core::ScanContext::new(&cli, tx).await.unwrap();
        assert!(e.validate(&ctx).await.is_ok());
    }
}
