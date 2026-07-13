use async_trait::async_trait;
use std::sync::Arc;
use std::time::Instant;

use crate::cli::args::{MaintainArgs, RamBoostArgs};
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};
use crate::util::memory::MemoryStats;

/// The maintain engine runs system maintenance tasks.
///
/// On macOS: DNS flush, Spotlight reindex, LaunchServices rebuild, periodic
/// scripts, RAM purge, Quick Look cache, dyld rebuild, disk permissions.
///
/// On Linux: systemd journal vacuum, DNS cache flush (systemd-resolved),
/// apt/dnf cache cleanup, thumbnail cache clear, tmp cleanup, RAM drop caches,
/// systemd-tmpfiles clean, updatedb refresh.
///
/// Tasks that require `sudo` are emitted as findings with the command in the
/// remediation hint, so the user can review via `--fix-script` before running.
pub struct MaintainEngine {
    args: MaintainArgs,
}

impl MaintainEngine {
    pub fn new(args: MaintainArgs) -> Self {
        Self { args }
    }

    /// Apply config overrides. Config disables tasks that CLI left at default-true.
    pub fn with_config(mut self, config: &crate::config::Config) -> Self {
        let mc = &config.maintain;
        if self.args.dns && !mc.dns {
            self.args.dns = false;
        }
        if self.args.spotlight && !mc.spotlight {
            self.args.spotlight = false;
        }
        if self.args.launchservices && !mc.launchservices {
            self.args.launchservices = false;
        }
        if self.args.periodic && !mc.periodic {
            self.args.periodic = false;
        }
        if self.args.purge_ram && !mc.purge_ram {
            self.args.purge_ram = false;
        }
        if self.args.quicklook && !mc.quicklook {
            self.args.quicklook = false;
        }
        // Enable tasks that config turns on (only if CLI left them at default-false)
        if !self.args.repair_permissions && mc.repair_permissions {
            self.args.repair_permissions = true;
        }
        if !self.args.dyld && mc.dyld {
            self.args.dyld = true;
        }
        self
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

    fn command_exists(cmd: &str) -> bool {
        std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Cross-platform / shared tasks
    // ═══════════════════════════════════════════════════════════════════════

    async fn task_flush_dns(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();

        if cfg!(target_os = "macos") {
            let (ok, msg) = Self::run_command("dscacheutil", &["-flushcache"]);
            let _ = Self::run_command("killall", &["-HUP", "mDNSResponder"]);
            findings.push(
                Finding::new(
                    EngineId::All,
                    if ok { Severity::Info } else { Severity::Medium },
                    Category::SystemMaintenance,
                    Target::Path(std::path::PathBuf::from("/")),
                    "DNS cache flush",
                    if ok {
                        "DNS cache flushed successfully".to_string()
                    } else {
                        format!("DNS cache flush failed: {}", msg)
                    },
                )
                .with_hint("dscacheutil -flushcache; killall -HUP mDNSResponder".to_string()),
            );
        } else if cfg!(target_os = "linux") {
            // Try systemd-resolved first, then nscd
            if Self::command_exists("resolvectl") {
                let (ok, msg) = Self::run_command("resolvectl", &["flush-caches"]);
                findings.push(
                    Finding::new(
                        EngineId::All,
                        if ok { Severity::Info } else { Severity::Medium },
                        Category::SystemMaintenance,
                        Target::Path(std::path::PathBuf::from("/")),
                        "DNS cache flush (systemd-resolved)",
                        if ok {
                            "systemd-resolved DNS cache flushed".to_string()
                        } else {
                            format!("DNS flush failed: {}", msg)
                        },
                    )
                    .with_hint("sudo resolvectl flush-caches".to_string()),
                );
            } else if Self::command_exists("nscd") {
                let (ok, msg) = Self::run_command("nscd", &["-i", "hosts"]);
                findings.push(
                    Finding::new(
                        EngineId::All,
                        if ok { Severity::Info } else { Severity::Medium },
                        Category::SystemMaintenance,
                        Target::Path(std::path::PathBuf::from("/")),
                        "DNS cache flush (nscd)",
                        if ok {
                            "nscd DNS cache invalidated".to_string()
                        } else {
                            format!("nscd flush failed: {}", msg)
                        },
                    )
                    .with_hint("sudo nscd -i hosts".to_string()),
                );
            } else {
                findings.push(
                    Finding::new(
                        EngineId::All,
                        Severity::Low,
                        Category::SystemMaintenance,
                        Target::Path(std::path::PathBuf::from("/")),
                        "DNS cache flush",
                        "No DNS cache daemon detected (systemd-resolved or nscd). DNS may be cached by individual applications.".to_string(),
                    )
                    .with_hint("sudo resolvectl flush-caches  # if systemd-resolved is active".to_string()),
                );
            }
        }

        if !findings.is_empty() {
            ctx.emit(findings[0].clone()).await;
        }
        let count = findings.len() as u64;
        (findings, count)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  macOS-only tasks
    // ═══════════════════════════════════════════════════════════════════════

    async fn task_reindex_spotlight(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let (ok, msg) = Self::run_command("mdutil", &["-E", "/"]);
        let needs_sudo =
            !ok && (msg.contains("Try as root") || msg.contains("Operation not permitted"));

        let findings = vec![Finding::new(
            EngineId::All,
            if ok {
                Severity::Info
            } else if needs_sudo {
                Severity::Low
            } else {
                Severity::Medium
            },
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
        .with_hint("sudo mdutil -E /  # requires root".to_string())];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    async fn task_rebuild_launchservices(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let lsregister = "/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister";
        let (ok, msg) = Self::run_command(
            lsregister,
            &[
                "-r", "-domain", "local", "-domain", "system", "-domain", "user",
            ],
        );
        let (ok, msg) = if ok {
            (true, msg)
        } else {
            Self::run_command(
                lsregister,
                &[
                    "-kill", "-r", "-domain", "local", "-domain", "system", "-domain", "user",
                ],
            )
        };

        let findings = vec![Finding::new(
            EngineId::All,
            if ok { Severity::Info } else { Severity::Medium },
            Category::SystemMaintenance,
            Target::Path(std::path::PathBuf::from("/")),
            "LaunchServices database rebuild",
            if ok {
                "LaunchServices database rebuilt".to_string()
            } else {
                format!("LaunchServices rebuild failed: {}", msg)
            },
        )
        .with_hint("lsregister -r -domain local -domain system -domain user".to_string())];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    async fn task_run_periodic(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if !Self::command_exists("periodic") {
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
                    Target::Path(std::path::PathBuf::from(format!(
                        "/etc/periodic/{}",
                        script
                    ))),
                    format!("Periodic script: {}", script),
                    if ok {
                        format!("Periodic {} script completed", script)
                    } else {
                        format!("Periodic {} failed: {}", script, msg)
                    },
                )
                .with_hint(format!("periodic {}", script)),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        }

        (findings, items)
    }

    async fn task_repair_permissions(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
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

        let findings = vec![Finding::new(
            EngineId::All,
            if ok { Severity::Info } else { Severity::Medium },
            Category::SystemMaintenance,
            Target::Path(std::path::PathBuf::from("/")),
            "Purge inactive RAM",
            if ok {
                "Inactive memory purged".to_string()
            } else {
                format!("RAM purge failed (may need sudo): {}", msg)
            },
        )
        .with_hint("purge  # may require sudo".to_string())];
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

        let findings = vec![Finding::new(
            EngineId::All,
            if ok { Severity::Info } else { Severity::Medium },
            Category::SystemMaintenance,
            Target::Path(std::path::PathBuf::from("/")),
            "Clear Quick Look cache",
            if ok {
                "Quick Look thumbnail cache cleared".to_string()
            } else {
                format!("Quick Look cache clear failed: {}", msg)
            },
        )
        .with_hint("qlmanage -r cache".to_string())];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Linux-only tasks
    // ═══════════════════════════════════════════════════════════════════════

    /// Vacuum old systemd journal logs to free disk space.
    async fn task_journal_vacuum(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();

        if !Self::command_exists("journalctl") {
            findings.push(
                Finding::new(
                    EngineId::All,
                    Severity::Low,
                    Category::SystemMaintenance,
                    Target::Path(std::path::PathBuf::from("/var/log/journal")),
                    "Journal vacuum",
                    "journalctl not found — systemd journal not active on this system".to_string(),
                )
                .with_hint("sudo journalctl --vacuum-time=7d  # if systemd is active".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
            return (findings, 1);
        }

        // Try vacuum to 7 days (non-sudo may work if user is in systemd-journal group)
        let (ok, msg) = Self::run_command("journalctl", &["--vacuum-time=7d", "--quiet"]);
        let size_msg = if ok { msg.clone() } else { String::new() };

        findings.push(
            Finding::new(
                EngineId::All,
                if ok { Severity::Info } else { Severity::Low },
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/var/log/journal")),
                "Systemd journal vacuum",
                if ok {
                    if size_msg.is_empty() {
                        "Journal vacuumed to 7 days".to_string()
                    } else {
                        format!("Journal vacuumed: {}", size_msg)
                    }
                } else {
                    format!("Journal vacuum requires sudo: {}", msg)
                },
            )
            .with_hint(
                "sudo journalctl --vacuum-time=7d  # removes journal entries older than 7 days"
                    .to_string(),
            ),
        );
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    /// Clean package manager caches (apt, dnf, pacman, zypper).
    async fn task_pkg_cache_clean(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        // Debian/Ubuntu: apt-get clean + autoremove
        if Self::command_exists("apt-get") {
            items += 1;
            let (ok, msg) = Self::run_command("apt-get", &["clean"]);
            findings.push(
                Finding::new(
                    EngineId::All,
                    if ok { Severity::Info } else { Severity::Low },
                    Category::SystemMaintenance,
                    Target::Path(std::path::PathBuf::from("/var/cache/apt")),
                    "APT cache clean",
                    if ok {
                        "APT package cache cleared (/var/cache/apt/archives/)".to_string()
                    } else {
                        format!("apt clean requires sudo: {}", msg)
                    },
                )
                .with_hint("sudo apt-get clean && sudo apt-get autoremove --yes".to_string()),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        }

        // Fedora/RHEL: dnf clean all
        if Self::command_exists("dnf") {
            items += 1;
            let (ok, msg) = Self::run_command("dnf", &["clean", "all"]);
            findings.push(
                Finding::new(
                    EngineId::All,
                    if ok { Severity::Info } else { Severity::Low },
                    Category::SystemMaintenance,
                    Target::Path(std::path::PathBuf::from("/var/cache/dnf")),
                    "DNF cache clean",
                    if ok {
                        "DNF package cache cleared".to_string()
                    } else {
                        format!("dnf clean requires sudo: {}", msg)
                    },
                )
                .with_hint("sudo dnf clean all".to_string()),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        }

        // Arch: pacman -Sc (clear uninstalled package cache)
        if Self::command_exists("pacman") {
            items += 1;
            let (ok, msg) = Self::run_command("pacman", &["-Sc", "--noconfirm"]);
            findings.push(
                Finding::new(
                    EngineId::All,
                    if ok { Severity::Info } else { Severity::Low },
                    Category::SystemMaintenance,
                    Target::Path(std::path::PathBuf::from("/var/cache/pacman/pkg")),
                    "Pacman cache clean",
                    if ok {
                        "Pacman package cache cleaned (uninstalled packages removed)".to_string()
                    } else {
                        format!("pacman -Sc requires sudo: {}", msg)
                    },
                )
                .with_hint(
                    "sudo pacman -Sc --noconfirm  # clears uninstalled package cache".to_string(),
                ),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        }

        // openSUSE: zypper clean
        if Self::command_exists("zypper") {
            items += 1;
            let (ok, msg) = Self::run_command("zypper", &["clean", "-a"]);
            findings.push(
                Finding::new(
                    EngineId::All,
                    if ok { Severity::Info } else { Severity::Low },
                    Category::SystemMaintenance,
                    Target::Path(std::path::PathBuf::from("/var/cache/zypp")),
                    "Zypper cache clean",
                    if ok {
                        "Zypper package cache cleared".to_string()
                    } else {
                        format!("zypper clean requires sudo: {}", msg)
                    },
                )
                .with_hint("sudo zypper clean -a".to_string()),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        }

        if findings.is_empty() {
            items = 1;
            findings.push(
                Finding::new(
                    EngineId::All,
                    Severity::Low,
                    Category::SystemMaintenance,
                    Target::Path(std::path::PathBuf::from("/var/cache")),
                    "Package manager cache clean",
                    "No supported package manager detected (apt, dnf, pacman, zypper)".to_string(),
                )
                .with_hint("# install apt/dnf/pacman/zypper to enable cache cleanup".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }

        (findings, items)
    }

    /// Clear thumbnail and font caches.
    async fn task_clear_thumbnail_cache(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let home = crate::util::macos::MacosUtils::home_dir();
        let thumb_cache = home.join(".cache/thumbnails");
        let font_cache = std::path::PathBuf::from("/var/cache/fontconfig");

        let mut findings = Vec::new();
        let mut items = 0u64;

        // Thumbnail cache
        if thumb_cache.exists() {
            items += 1;
            let cache_size = crate::util::disk::dir_size(&thumb_cache);
            findings.push(
                Finding::new(
                    EngineId::All,
                    Severity::Info,
                    Category::SystemMaintenance,
                    Target::Path(thumb_cache.clone()),
                    "Thumbnail cache",
                    format!(
                        "Thumbnail cache: {} — safe to clear",
                        crate::util::disk::format_bytes(cache_size)
                    ),
                )
                .with_hint(format!(
                    "rm -rf {}  # frees thumbnail cache",
                    thumb_cache.display()
                )),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        }

        // Font cache
        if font_cache.exists() {
            items += 1;
            let (ok, msg) = Self::run_command("fc-cache", &["-f"]);
            findings.push(
                Finding::new(
                    EngineId::All,
                    if ok { Severity::Info } else { Severity::Low },
                    Category::SystemMaintenance,
                    Target::Path(font_cache.clone()),
                    "Font cache rebuild",
                    if ok {
                        "Font cache rebuilt (fc-cache -f)".to_string()
                    } else {
                        format!("Font cache rebuild failed: {}", msg)
                    },
                )
                .with_hint("fc-cache -f  # rebuilds fontconfig cache".to_string()),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        }

        if findings.is_empty() {
            items = 1;
            findings.push(
                Finding::new(
                    EngineId::All,
                    Severity::Low,
                    Category::SystemMaintenance,
                    Target::Path(home.join(".cache")),
                    "Thumbnail/font cache",
                    "No thumbnail or font caches found".to_string(),
                )
                .with_hint("rm -rf ~/.cache/thumbnails  # if exists".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }

        (findings, items)
    }

    /// Drop kernel caches to free RAM (requires root).
    async fn task_drop_caches(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let findings = vec![
            Finding::new(
                EngineId::All,
                Severity::Medium,
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/proc/sys/vm/drop_caches")),
                "Drop kernel page cache",
                "Writing to /proc/sys/vm/drop_caches frees page cache, dentries, and inodes. Requires root. Safe but may cause temporary I/O slowdown as cache is rebuilt.".to_string(),
            )
            .with_hint("sudo sh -c 'echo 3 > /proc/sys/vm/drop_caches'  # frees page cache + dentries + inodes".to_string()),
        ];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    /// Run systemd-tmpfiles --clean to remove stale temp files.
    async fn task_tmpfiles_clean(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        if !Self::command_exists("systemd-tmpfiles") {
            let findings = vec![Finding::new(
                EngineId::All,
                Severity::Low,
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/tmp")),
                "Tmpfiles clean",
                "systemd-tmpfiles not found — stale temp files not cleaned automatically"
                    .to_string(),
            )
            .with_hint("sudo systemd-tmpfiles --clean  # if systemd is active".to_string())];
            ctx.emit(findings[0].clone()).await;
            return (findings, 1);
        }

        let (ok, msg) = Self::run_command("systemd-tmpfiles", &["--clean"]);
        let findings = vec![Finding::new(
            EngineId::All,
            if ok { Severity::Info } else { Severity::Low },
            Category::SystemMaintenance,
            Target::Path(std::path::PathBuf::from("/tmp")),
            "Systemd tmpfiles clean",
            if ok {
                "Stale temp files cleaned via systemd-tmpfiles".to_string()
            } else {
                format!("systemd-tmpfiles --clean requires sudo: {}", msg)
            },
        )
        .with_hint(
            "sudo systemd-tmpfiles --clean  # removes stale files per tmpfiles.d config"
                .to_string(),
        )];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    /// Refresh the locate database (updatedb).
    async fn task_updatedb(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        if !Self::command_exists("updatedb") {
            let findings = vec![Finding::new(
                EngineId::All,
                Severity::Low,
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/var/lib/mlocate")),
                "Locate database refresh",
                "updatedb not found — install mlocate or plocate to enable fast file search"
                    .to_string(),
            )
            .with_hint("sudo updatedb  # if mlocate/plocate is installed".to_string())];
            ctx.emit(findings[0].clone()).await;
            return (findings, 1);
        }

        let findings = vec![
            Finding::new(
                EngineId::All,
                Severity::Low,
                Category::SystemMaintenance,
                Target::Path(std::path::PathBuf::from("/var/lib/mlocate")),
                "Locate database refresh",
                "updatedb refreshes the file index used by the `locate` command. Requires sudo and may take a few minutes.".to_string(),
            )
            .with_hint("sudo updatedb  # refreshes the locate database".to_string()),
        ];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  RAM Boost — memory optimizer
    // ═══════════════════════════════════════════════════════════════════════

    /// Run the RAM boost pipeline: report → purge → kill processes → report.
    pub async fn run_ram_boost(
        args: RamBoostArgs,
        ctx: Arc<ScanContext>,
    ) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        // ── 1. Before snapshot ──────────────────────────────────────────────
        let before = MemoryStats::collect();
        items_scanned += 1;

        let before_finding = Finding::new(
            EngineId::All,
            Severity::Info,
            Category::RamOptimization,
            Target::Path(std::path::PathBuf::from("/")),
            "Memory report (before)",
            before.report(),
        )
        .with_metadata("phase", serde_json::json!("before"))
        .with_metadata("total_bytes", serde_json::json!(before.total_bytes))
        .with_metadata("used_bytes", serde_json::json!(before.used_bytes))
        .with_metadata("available_bytes", serde_json::json!(before.available_bytes))
        .with_metadata(
            "app_memory_bytes",
            serde_json::json!(before.app_memory_bytes),
        )
        .with_metadata("wired_bytes", serde_json::json!(before.wired_bytes))
        .with_metadata(
            "compressed_bytes",
            serde_json::json!(before.compressed_bytes),
        )
        .with_metadata("swap_used_bytes", serde_json::json!(before.swap_used_bytes))
        .with_metadata(
            "swap_total_bytes",
            serde_json::json!(before.swap_total_bytes),
        )
        .with_metadata(
            "memory_pressure",
            serde_json::json!(format!("{:?}", before.memory_pressure)),
        )
        .with_metadata("top_consumers", serde_json::json!(before.top_consumers));
        ctx.emit(before_finding.clone()).await;
        findings_count += 1;

        if args.report_only {
            return Ok(EngineStats {
                engine: EngineId::All,
                duration: start.elapsed(),
                items_scanned,
                findings_count,
                errors_count: 0,
            });
        }

        // ── 2. Purge inactive memory ────────────────────────────────────────
        let mut purge_succeeded = false;
        let mut purge_message = String::new();
        if args.purge {
            items_scanned += 1;
            let (ok, _msg) = if cfg!(target_os = "macos") {
                if args.privileged {
                    // Use osascript to prompt for admin privileges
                    let (ok_os, msg_os) = Self::run_command(
                        "osascript",
                        &[
                            "-e",
                            "do shell script \"purge\" with administrator privileges",
                        ],
                    );
                    if ok_os {
                        (true, msg_os)
                    } else {
                        (false, format!("osascript purge failed: {}", msg_os))
                    }
                } else {
                    // Try without sudo first, then with sudo -n
                    let (ok1, msg1) = Self::run_command("purge", &[]);
                    if ok1 {
                        (true, msg1)
                    } else {
                        let (ok2, msg2) = Self::run_command("sudo", &["-n", "purge"]);
                        if ok2 {
                            (true, msg2)
                        } else {
                            (false, format!("{} (tried sudo: {})", msg1, msg2))
                        }
                    }
                }
            } else {
                // Linux: try drop_caches (needs root)
                let (ok1, msg1) =
                    Self::run_command("sh", &["-c", "echo 3 > /proc/sys/vm/drop_caches"]);
                if ok1 {
                    (true, msg1)
                } else {
                    let (ok2, msg2) = Self::run_command(
                        "sudo",
                        &["-n", "sh", "-c", "echo 3 > /proc/sys/vm/drop_caches"],
                    );
                    if ok2 {
                        (true, msg2)
                    } else {
                        (false, format!("{} (tried sudo: {})", msg1, msg2))
                    }
                }
            };

            purge_succeeded = ok;
            purge_message = if ok {
                let reclaimable = before.reclaimable_bytes();
                format!(
                    "Inactive memory purged — up to {} could be freed",
                    crate::util::disk::format_bytes(reclaimable)
                )
            } else {
                "Memory purge requires elevated permissions. Run in Terminal: sudo purge"
                    .to_string()
            };

            let purge_finding = Finding::new(
                EngineId::All,
                if ok { Severity::Info } else { Severity::Medium },
                Category::RamOptimization,
                Target::Path(std::path::PathBuf::from("/")),
                "Purge inactive memory",
                purge_message.clone(),
            )
            .with_hint(if cfg!(target_os = "macos") {
                "sudo purge  # run in Terminal to purge inactive RAM".to_string()
            } else {
                "sudo sh -c 'echo 3 > /proc/sys/vm/drop_caches'".to_string()
            })
            .with_metadata("purge_success", serde_json::json!(ok))
            .with_metadata("purge_message", serde_json::json!(purge_message));
            ctx.emit(purge_finding).await;
            findings_count += 1;
        }

        // ── 3. Kill memory-hungry processes ─────────────────────────────────
        let mut killed = 0u64;
        if args.kill_top > 0 {
            let candidates = Self::select_processes_to_kill(&before, args.kill_top, &args);
            for proc in &candidates {
                items_scanned += 1;
                let success = Self::kill_process(proc.pid, args.force);
                killed += if success { 1 } else { 0 };

                let kill_finding = Finding::new(
                    EngineId::All,
                    if success {
                        Severity::Info
                    } else {
                        Severity::Medium
                    },
                    Category::RamOptimization,
                    Target::Process(proc.pid),
                    format!("Kill process: {} (PID {})", proc.name, proc.pid),
                    if success {
                        format!(
                            "Terminated {} (PID {}) — was using {} ({:.1}%)",
                            proc.name,
                            proc.pid,
                            crate::util::disk::format_bytes(proc.rss_bytes),
                            proc.percent
                        )
                    } else {
                        format!(
                            "Failed to kill {} (PID {}) — may need elevated permissions",
                            proc.name, proc.pid
                        )
                    },
                )
                .with_hint(format!(
                    "kill -{} {}  # {}",
                    if args.force { "9" } else { "15" },
                    proc.pid,
                    proc.name
                ));
                ctx.emit(kill_finding).await;
                findings_count += 1;
            }
        }

        // Kill by name
        if let Some(names) = &args.kill_name {
            for name in names.split(',') {
                let name = name.trim();
                if name.is_empty() {
                    continue;
                }
                let pids = Self::find_pids_by_name(name);
                for pid in pids {
                    items_scanned += 1;
                    let success = Self::kill_process(pid, args.force);
                    killed += if success { 1 } else { 0 };

                    let kill_finding = Finding::new(
                        EngineId::All,
                        if success {
                            Severity::Info
                        } else {
                            Severity::Medium
                        },
                        Category::RamOptimization,
                        Target::Process(pid),
                        format!("Kill process by name: {} (PID {})", name, pid),
                        if success {
                            format!("Terminated '{}' (PID {})", name, pid)
                        } else {
                            format!("Failed to kill '{}' (PID {})", name, pid)
                        },
                    );
                    ctx.emit(kill_finding).await;
                    findings_count += 1;
                }
            }
        }

        // ── 4. After snapshot ───────────────────────────────────────────────
        // Small delay to let the OS reclaim freed pages
        tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        let after = MemoryStats::collect();
        items_scanned += 1;

        let freed = before.used_bytes.saturating_sub(after.used_bytes);
        let freed_swap = before.swap_used_bytes.saturating_sub(after.swap_used_bytes);

        let summary = format!(
            "{}\n  ── After Boost ──\n{}\n  Freed: {} RAM, {} swap\n  Processes killed: {}",
            "─".repeat(40),
            after.report(),
            crate::util::disk::format_bytes(freed),
            crate::util::disk::format_bytes(freed_swap),
            killed,
        );

        let after_finding = Finding::new(
            EngineId::All,
            if freed > 0 {
                Severity::Info
            } else {
                Severity::Low
            },
            Category::RamOptimization,
            Target::Path(std::path::PathBuf::from("/")),
            "Memory report (after)",
            summary,
        )
        .with_metadata("phase", serde_json::json!("after"))
        .with_metadata("total_bytes", serde_json::json!(after.total_bytes))
        .with_metadata("used_bytes", serde_json::json!(after.used_bytes))
        .with_metadata("available_bytes", serde_json::json!(after.available_bytes))
        .with_metadata(
            "app_memory_bytes",
            serde_json::json!(after.app_memory_bytes),
        )
        .with_metadata("wired_bytes", serde_json::json!(after.wired_bytes))
        .with_metadata(
            "compressed_bytes",
            serde_json::json!(after.compressed_bytes),
        )
        .with_metadata("swap_used_bytes", serde_json::json!(after.swap_used_bytes))
        .with_metadata(
            "swap_total_bytes",
            serde_json::json!(after.swap_total_bytes),
        )
        .with_metadata("freed_bytes", serde_json::json!(freed))
        .with_metadata("freed_swap_bytes", serde_json::json!(freed_swap))
        .with_metadata("processes_killed", serde_json::json!(killed))
        .with_metadata("purge_success", serde_json::json!(purge_succeeded))
        .with_metadata("purge_message", serde_json::json!(purge_message))
        .with_metadata(
            "memory_pressure",
            serde_json::json!(format!("{:?}", after.memory_pressure)),
        )
        .with_metadata("top_consumers", serde_json::json!(after.top_consumers));
        ctx.emit(after_finding).await;
        findings_count += 1;

        Ok(EngineStats {
            engine: EngineId::All,
            duration: start.elapsed(),
            items_scanned,
            findings_count,
            errors_count: 0,
        })
    }

    /// Select processes to kill based on memory usage and safety rules.
    fn select_processes_to_kill<'a>(
        stats: &'a MemoryStats,
        count: usize,
        args: &RamBoostArgs,
    ) -> Vec<&'a crate::util::memory::ProcessMemory> {
        let min_rss = args.min_rss_mb * 1024 * 1024;
        let protected = [
            "kernel_task",
            "launchd",
            "WindowServer",
            "loginwindow",
            "Finder",
            "Dock",
            "SystemUIServer",
            "coreaudiod",
            "bluetoothd",
            "systemd",
            "init",
            "udev",
        ];

        stats
            .top_consumers
            .iter()
            .filter(|p| p.rss_bytes >= min_rss)
            .filter(|p| {
                if !args.protect_system {
                    return true;
                }
                !protected.iter().any(|&name| p.name.contains(name))
            })
            .take(count)
            .collect()
    }

    /// Send a signal to a process. Returns true if the signal was sent.
    fn kill_process(pid: u32, force: bool) -> bool {
        let signal = if force { "9" } else { "15" };
        Self::run_command(
            "kill",
            ["-".to_string() + signal, pid.to_string()]
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .as_slice(),
        )
        .0
    }

    /// Find PIDs by process name using pgrep.
    fn find_pids_by_name(name: &str) -> Vec<u32> {
        let output = std::process::Command::new("pgrep")
            .arg("-x")
            .arg(name)
            .output();
        match output {
            Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
                .lines()
                .filter_map(|l| l.trim().parse::<u32>().ok())
                .collect(),
            _ => Vec::new(),
        }
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
        "Runs system maintenance: DNS flush, cache cleanup, journal vacuum, RAM purge, tmpfiles clean"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        if cfg!(target_os = "macos") {
            // macOS tasks
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
        } else if cfg!(target_os = "linux") {
            // Linux tasks — map the same flags to Linux equivalents
            if self.args.dns {
                let (f, i) = self.task_flush_dns(&ctx).await;
                items_scanned += i;
                findings_count += f.len() as u64;
            }
            // spotlight → journal vacuum
            if self.args.spotlight {
                let (f, i) = self.task_journal_vacuum(&ctx).await;
                items_scanned += i;
                findings_count += f.len() as u64;
            }
            // launchservices → pkg cache clean
            if self.args.launchservices {
                let (f, i) = self.task_pkg_cache_clean(&ctx).await;
                items_scanned += i;
                findings_count += f.len() as u64;
            }
            // periodic → tmpfiles clean
            if self.args.periodic {
                let (f, i) = self.task_tmpfiles_clean(&ctx).await;
                items_scanned += i;
                findings_count += f.len() as u64;
            }
            // repair_permissions → updatedb refresh
            if self.args.repair_permissions {
                let (f, i) = self.task_updatedb(&ctx).await;
                items_scanned += i;
                findings_count += f.len() as u64;
            }
            // purge_ram → drop caches
            if self.args.purge_ram {
                let (f, i) = self.task_drop_caches(&ctx).await;
                items_scanned += i;
                findings_count += f.len() as u64;
            }
            // dyld → thumbnail/font cache clean
            if self.args.dyld {
                let (f, i) = self.task_clear_thumbnail_cache(&ctx).await;
                items_scanned += i;
                findings_count += f.len() as u64;
            }
            // quicklook → also thumbnail cache (already handled by dyld flag above)
            if self.args.quicklook && !self.args.dyld {
                let (f, i) = self.task_clear_thumbnail_cache(&ctx).await;
                items_scanned += i;
                findings_count += f.len() as u64;
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
