use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::cli::args::MaintainArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

/// The maintain engine runs macOS system maintenance tasks. Unlike other
/// engines, these tasks may actually execute commands (not just scan) — but
/// each task is emitted as a finding with the command in the remediation
/// hint, so the user can review via `--fix-script` before running.
///
/// Tasks that require `sudo` (repair permissions, dyld rebuild) are always
/// emitted as findings with the command commented out in fix scripts.
pub struct MaintainEngine {
    args: MaintainArgs,
}

impl MaintainEngine {
    pub fn new(args: MaintainArgs) -> Self {
        Self { args }
    }

    fn run_command(cmd: &str, args: &[&str]) -> (bool, String) {
        match std::process::Command::new(cmd).args(args).output() {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                let msg = if stdout.is_empty() { stderr } else { stdout };
                (output.status.success(), msg)
            }
            Err(e) => (false, e.to_string()),
        }
    }

    async fn task_flush_dns(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let (ok, msg) = Self::run_command("dscacheutil", &["-flushcache"]);
        let _ = Self::run_command("killall", &["-HUP", "mDNSResponder"]);

        findings.push(
            Finding::new(
                EngineId::All,
                if ok { Severity::Info } else { Severity::Medium },
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/")),
                "DNS cache flush",
                if ok { "DNS cache flushed successfully".to_string() } else { format!("DNS cache flush failed: {}", msg) },
            )
            .with_hint("dscacheutil -flushcache; killall -HUP mDNSResponder".to_string()),
        );
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    async fn task_reindex_spotlight(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        // mdutil -E / requires root on most macOS versions.
        // Try without sudo first; if it fails, emit as a sudo-required finding.
        let (ok, msg) = Self::run_command("mdutil", &["-E", "/"]);
        let needs_sudo = !ok && (msg.contains("Try as root") || msg.contains("Operation not permitted"));

        findings.push(
            Finding::new(
                EngineId::All,
                if ok { Severity::Info } else if needs_sudo { Severity::Low } else { Severity::Medium },
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/")),
                "Spotlight reindex",
                if ok {
                    "Spotlight reindex initiated for /".to_string()
                } else if needs_sudo {
                    "Spotlight reindex requires sudo — skipped (run manually if needed)".to_string()
                } else {
                    format!("Spotlight reindex failed: {}", msg)
                },
            )
            .with_hint("sudo mdutil -E /  # requires root".to_string()),
        );
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    async fn task_rebuild_launchservices(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        // Rebuild the LaunchServices database via lsregister.
        // The -kill flag was removed in macOS 15+. Use -r -domain args without -kill.
        let lsregister = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
        // Try without -kill first (works on macOS 15+), fall back to -kill for older macOS
        let (ok, msg) = Self::run_command(lsregister, &["-r", "-domain", "local", "-domain", "system", "-domain", "user"]);
        let (ok, msg) = if ok {
            (true, msg)
        } else {
            // Try with -kill for older macOS versions
            Self::run_command(lsregister, &["-kill", "-r", "-domain", "local", "-domain", "system", "-domain", "user"])
        };

        findings.push(
            Finding::new(
                EngineId::All,
                if ok { Severity::Info } else { Severity::Medium },
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/")),
                "LaunchServices database rebuild",
                if ok { "LaunchServices database rebuilt".to_string() } else { format!("LaunchServices rebuild failed: {}", msg) },
            )
            .with_hint("lsregister -r -domain local -domain system -domain user".to_string()),
        );
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    async fn task_run_periodic(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        // Check if the periodic command exists at all
        let periodic_exists = std::process::Command::new("which")
            .arg("periodic")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if !periodic_exists {
            // periodic not available — emit a single informational finding
            items = 1;
            findings.push(
                Finding::new(
                    EngineId::All,
                    Severity::Low,
                    Category::SystemMaintenance,
                    Target::Path(std::path::PathBuf::from("/etc/periodic")),
                    "Periodic scripts",
                    "The `periodic` command is not available on this system. Maintenance scripts may be handled by launchd instead.".to_string(),
                )
                .with_hint("sudo periodic daily weekly monthly  # if available".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
            return (findings, items);
        }

        for script in &["daily", "weekly", "monthly"] {
            items += 1;
            let (ok, msg) = Self::run_command("periodic", &[script]);
            findings.push(
                Finding::new(
                    EngineId::All,
                    if ok { Severity::Info } else { Severity::Medium },
                    Category::SystemMaintenance,
                    Target::Path(std::path::PathBuf::from(format!("/etc/periodic/{}", script))),
                    format!("Periodic script: {}", script),
                    if ok { format!("Periodic {} script completed", script) } else { format!("Periodic {} failed: {}", script, msg) },
                )
                .with_hint(format!("periodic {}", script)),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        }

        (findings, items)
    }

    async fn task_repair_permissions(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        // diskutil repairPermissions is deprecated on APFS but still works
        // on HFS+ volumes. We emit it as a finding with the command.
        let findings = vec![
            Finding::new(
                EngineId::All,
                Severity::Medium,
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/")),
                "Repair disk permissions",
                "Disk permission repair requires sudo and is only applicable to HFS+ volumes (not APFS)".to_string(),
            )
            .with_hint("sudo diskutil repairPermissions /  # only for HFS+ volumes".to_string()),
        ];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    async fn task_purge_ram(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let (ok, msg) = Self::run_command("purge", &[]);

        let findings = vec![
            Finding::new(
                EngineId::All,
                if ok { Severity::Info } else { Severity::Medium },
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/")),
                "Purge inactive RAM",
                if ok { "Inactive memory purged".to_string() } else { format!("RAM purge failed (may need sudo): {}", msg) },
            )
            .with_hint("purge  # may require sudo".to_string()),
        ];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    async fn task_rebuild_dyld(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let findings = vec![
            Finding::new(
                EngineId::All,
                Severity::High,
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/var/db/dyld")),
                "Rebuild dyld shared cache",
                "Rebuilding the dyld shared cache requires sudo and can slow down app launches temporarily. Use with caution.".to_string(),
            )
            .with_hint("sudo update_dyld_shared_cache  # requires sudo, use with caution".to_string()),
        ];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    async fn task_clear_quicklook(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let (ok, msg) = Self::run_command("qlmanage", &["-r", "cache"]);

        let findings = vec![
            Finding::new(
                EngineId::All,
                if ok { Severity::Info } else { Severity::Medium },
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/")),
                "Clear Quick Look cache",
                if ok { "Quick Look thumbnail cache cleared".to_string() } else { format!("Quick Look cache clear failed: {}", msg) },
            )
            .with_hint("qlmanage -r cache".to_string()),
        ];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }
}

#[async_trait]
impl Engine for MaintainEngine {
    fn id(&self) -> EngineId {
        EngineId::All
    }

    fn name(&self) -> &'static str {
        "Maintain Engine"
    }

    fn description(&self) -> &'static str {
        "Runs macOS system maintenance: DNS flush, Spotlight reindex, LaunchServices rebuild, periodic scripts, RAM purge"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        if self.args.dns {
            let (f, i) = self.task_flush_dns(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
        }

        if self.args.spotlight {
            let (f, i) = self.task_reindex_spotlight(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
        }

        if self.args.launchservices {
            let (f, i) = self.task_rebuild_launchservices(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
        }

        if self.args.periodic {
            let (f, i) = self.task_run_periodic(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
        }

        if self.args.repair_permissions {
            let (f, i) = self.task_repair_permissions(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
        }

        if self.args.purge_ram {
            let (f, i) = self.task_purge_ram(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
        }

        if self.args.dyld {
            let (f, i) = self.task_rebuild_dyld(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
        }

        if self.args.quicklook {
            let (f, i) = self.task_clear_quicklook(&ctx).await;
            items_scanned += i;
            findings_count += f.len() as u64;
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

impl Default for MaintainEngine {
    fn default() -> Self {
        Self::new(MaintainArgs {
            dns: true,
            spotlight: true,
            launchservices: true,
            periodic: true,
            repair_permissions: false,
            purge_ram: true,
            dyld: false,
            quicklook: true,
        })
    }
}
