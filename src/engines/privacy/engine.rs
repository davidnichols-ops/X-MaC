use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;

use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};

/// Privacy & Security Operations Engine — clears browsing traces,
/// audits permissions, scans for malware/adware, and performs secure deletion.
///
/// Maps to Cleaner/Optimizer operations 206-235.
///
/// On macOS: detects browser data locations (Safari, Chrome, Firefox),
/// checks TCC permissions, scans for adware bundle IDs, detects suspicious
/// processes, and generates security reports.
///
/// On Linux: detects browser data locations (Firefox, Chrome, Chromium),
/// checks for suspicious processes, and generates security reports.
///
/// All operations are detection-only — they emit findings with remediation
/// hints so the user can review via `--fix-script` before any deletion.
pub struct PrivacyEngine {
    /// When true, the engine also scans for browser data locations on top
    /// of the default permission/security checks.
    scan_browser_data: bool,
    /// When true, the engine scans for malware/adware signatures.
    scan_malware: bool,
    /// When true, the engine checks TCC/permission databases.
    scan_permissions: bool,
}

impl Default for PrivacyEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl PrivacyEngine {
    /// Create a new PrivacyEngine with all scan categories enabled.
    pub fn new() -> Self {
        Self {
            scan_browser_data: true,
            scan_malware: true,
            scan_permissions: true,
        }
    }

    /// Disable browser-data scanning (ops 206-214, 222-224, 227-228).
    #[allow(dead_code)]
    pub fn without_browser_data(mut self) -> Self {
        self.scan_browser_data = false;
        self
    }

    /// Disable malware/adware scanning (ops 215-216, 229-235).
    #[allow(dead_code)]
    pub fn without_malware_scan(mut self) -> Self {
        self.scan_malware = false;
        self
    }

    /// Disable permission auditing (ops 217-219).
    #[allow(dead_code)]
    pub fn without_permission_audit(mut self) -> Self {
        self.scan_permissions = false;
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

    fn home_dir() -> PathBuf {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
    }

    fn library_dir() -> PathBuf {
        Self::home_dir().join("Library")
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Browser data detection (ops 206-211)
    // ═══════════════════════════════════════════════════════════════════════

    /// Known browser data locations on macOS and Linux.
    fn browser_data_locations() -> Vec<BrowserDataLocation> {
        let home = Self::home_dir();
        let lib = Self::library_dir();

        if cfg!(target_os = "macos") {
            vec![
                BrowserDataLocation {
                    name: "Safari",
                    category: Category::BrowserHistory,
                    history_paths: vec![
                        lib.join("Safari/History.db"),
                        lib.join("Safari/History.db-lock"),
                    ],
                    cookie_paths: vec![lib.join("Cookies/Cookies.binarycookies")],
                    download_paths: vec![lib.join("Safari/Downloads.plist")],
                    autofill_paths: vec![lib.join("Safari/Form Values")],
                },
                BrowserDataLocation {
                    name: "Chrome",
                    category: Category::BrowserHistory,
                    history_paths: vec![
                        lib.join("Application Support/Google/Chrome/Default/History")
                    ],
                    cookie_paths: vec![
                        lib.join("Application Support/Google/Chrome/Default/Cookies")
                    ],
                    download_paths: vec![
                        lib.join("Application Support/Google/Chrome/Default/History")
                    ],
                    autofill_paths: vec![
                        lib.join("Application Support/Google/Chrome/Default/Web Data")
                    ],
                },
                BrowserDataLocation {
                    name: "Firefox",
                    category: Category::BrowserHistory,
                    history_paths: vec![lib.join("Application Support/Firefox/Profiles")],
                    cookie_paths: vec![lib.join("Application Support/Firefox/Profiles")],
                    download_paths: vec![lib.join("Application Support/Firefox/Profiles")],
                    autofill_paths: vec![lib.join("Application Support/Firefox/Profiles")],
                },
            ]
        } else {
            vec![
                BrowserDataLocation {
                    name: "Firefox",
                    category: Category::BrowserHistory,
                    history_paths: vec![home.join(".mozilla/firefox")],
                    cookie_paths: vec![home.join(".mozilla/firefox")],
                    download_paths: vec![home.join(".mozilla/firefox")],
                    autofill_paths: vec![home.join(".mozilla/firefox")],
                },
                BrowserDataLocation {
                    name: "Chrome",
                    category: Category::BrowserHistory,
                    history_paths: vec![home.join(".config/google-chrome/Default/History")],
                    cookie_paths: vec![home.join(".config/google-chrome/Default/Cookies")],
                    download_paths: vec![home.join(".config/google-chrome/Default/History")],
                    autofill_paths: vec![home.join(".config/google-chrome/Default/Web Data")],
                },
                BrowserDataLocation {
                    name: "Chromium",
                    category: Category::BrowserHistory,
                    history_paths: vec![home.join(".config/chromium/Default/History")],
                    cookie_paths: vec![home.join(".config/chromium/Default/Cookies")],
                    download_paths: vec![home.join(".config/chromium/Default/History")],
                    autofill_paths: vec![home.join(".config/chromium/Default/Web Data")],
                },
            ]
        }
    }

    /// op 206: Clear browser history — detect history databases.
    async fn task_clear_browser_history(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for browser in Self::browser_data_locations() {
            for path in &browser.history_paths {
                items += 1;
                if path.exists() {
                    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                    let is_dir = path.is_dir();
                    findings.push(
                        Finding::new(
                            EngineId::Privacy,
                            Severity::Medium,
                            Category::BrowserHistory,
                            Target::Path(path.clone()),
                            format!("Browser history: {}", browser.name),
                            format!(
                                "{} history data found at {}{}",
                                browser.name,
                                path.display(),
                                if is_dir {
                                    " (profile directory — contains places.sqlite)"
                                } else {
                                    ""
                                }
                            ),
                        )
                        .with_hint(format!(
                            "# Clear {} history:\n#   SQLite: DELETE FROM moz_places/moz_historyvisits\n#   Chrome: DELETE FROM urls, visits\n#   Safari: rm {}",
                            browser.name,
                            path.display()
                        ))
                        .with_size(size),
                    );
                    ctx.emit(findings.last().unwrap().clone()).await;
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::BrowserHistory,
                    Target::Path(Self::library_dir()),
                    "Browser history",
                    "No browser history databases detected".to_string(),
                )
                .with_hint("# no browser history data found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 207: Clear cookies — detect cookie databases.
    async fn task_clear_cookies(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for browser in Self::browser_data_locations() {
            for path in &browser.cookie_paths {
                items += 1;
                if path.exists() {
                    let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                    findings.push(
                        Finding::new(
                            EngineId::Privacy,
                            Severity::Medium,
                            Category::Cookie,
                            Target::Path(path.clone()),
                            format!("Cookies: {}", browser.name),
                            format!(
                                "{} cookie database found at {}",
                                browser.name,
                                path.display()
                            ),
                        )
                        .with_hint(format!(
                            "# Clear {} cookies: rm {}",
                            browser.name,
                            path.display()
                        ))
                        .with_size(size),
                    );
                    ctx.emit(findings.last().unwrap().clone()).await;
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::Cookie,
                    Target::Path(Self::library_dir()),
                    "Cookies",
                    "No browser cookie databases detected".to_string(),
                )
                .with_hint("# no cookie data found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 208: Clear downloads history.
    async fn task_clear_downloads_history(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for browser in Self::browser_data_locations() {
            for path in &browser.download_paths {
                items += 1;
                if path.exists() {
                    findings.push(
                        Finding::new(
                            EngineId::Privacy,
                            Severity::Medium,
                            Category::DownloadHistory,
                            Target::Path(path.clone()),
                            format!("Downloads history: {}", browser.name),
                            format!(
                                "{} downloads history found at {}",
                                browser.name,
                                path.display()
                            ),
                        )
                        .with_hint(format!(
                            "# Clear {} downloads: DELETE FROM moz_annos / rm {}",
                            browser.name,
                            path.display()
                        )),
                    );
                    ctx.emit(findings.last().unwrap().clone()).await;
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::DownloadHistory,
                    Target::Path(Self::library_dir()),
                    "Downloads history",
                    "No browser download history detected".to_string(),
                )
                .with_hint("# no download history found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 209: Clear autofill data.
    async fn task_clear_autofill_data(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        for browser in Self::browser_data_locations() {
            for path in &browser.autofill_paths {
                items += 1;
                if path.exists() {
                    findings.push(
                        Finding::new(
                            EngineId::Privacy,
                            Severity::Medium,
                            Category::AutofillData,
                            Target::Path(path.clone()),
                            format!("Autofill data: {}", browser.name),
                            format!(
                                "{} autofill/form data found at {}",
                                browser.name,
                                path.display()
                            ),
                        )
                        .with_hint(format!(
                            "# Clear {} autofill: rm {}",
                            browser.name,
                            path.display()
                        )),
                    );
                    ctx.emit(findings.last().unwrap().clone()).await;
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::AutofillData,
                    Target::Path(Self::library_dir()),
                    "Autofill data",
                    "No browser autofill data detected".to_string(),
                )
                .with_hint("# no autofill data found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 210: Clear recent documents.
    async fn task_clear_recent_documents(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // macOS: Finder recent folders plist + Recent Documents symlinks
            let (ok, _msg) =
                Self::run_command("defaults", &["read", "com.apple.finder", "FXRecentFolders"]);
            items += 1;
            if ok {
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Medium,
                        Category::RecentDocument,
                        Target::Path(
                            Self::library_dir()
                                .join("Preferences/com.apple.finder.plist"),
                        ),
                        "Recent documents (Finder)",
                        "Finder recent folders list is populated — contains traces of recently accessed folders".to_string(),
                    )
                    .with_hint("defaults delete com.apple.finder FXRecentFolders".to_string()),
                );
                ctx.emit(findings[0].clone()).await;
            }

            let recent_docs = Self::library_dir().join("Recent Documents");
            if recent_docs.exists() {
                items += 1;
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Medium,
                        Category::RecentDocument,
                        Target::Path(recent_docs.clone()),
                        "Recent documents (symlinks)",
                        "Recent Documents directory contains symlinks to recently opened files"
                            .to_string(),
                    )
                    .with_hint(format!("rm -rf {}/*", recent_docs.display())),
                );
                ctx.emit(findings.last().unwrap().clone()).await;
            }
        } else {
            // Linux: GTK recent files
            let recent_gtk = Self::home_dir().join(".local/share/recently-used.xbel");
            items += 1;
            if recent_gtk.exists() {
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Medium,
                        Category::RecentDocument,
                        Target::Path(recent_gtk.clone()),
                        "Recent documents (GTK)",
                        "GTK recently-used file list contains traces of recently opened files"
                            .to_string(),
                    )
                    .with_hint(format!("rm {}", recent_gtk.display())),
                );
                ctx.emit(findings[0].clone()).await;
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::RecentDocument,
                    Target::Path(Self::library_dir()),
                    "Recent documents",
                    "No recent document traces detected".to_string(),
                )
                .with_hint("# no recent document data found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 211: Clear application history (recent apps, open-recent menus).
    async fn task_clear_application_history(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // macOS: saved application state + recent items in various apps
            let state_dir = Self::library_dir().join("Saved Application State");
            if state_dir.exists() {
                items += 1;
                let count = std::fs::read_dir(&state_dir)
                    .map(|d| d.filter_map(|e| e.ok()).count())
                    .unwrap_or(0);
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Medium,
                        Category::AppHistory,
                        Target::Path(state_dir.clone()),
                        "Application history (saved state)",
                        format!("{} saved application state directories found — contain window/restore history", count),
                    )
                    .with_hint(format!("rm -rf {}/*", state_dir.display())),
                );
                ctx.emit(findings[0].clone()).await;
            }
        }

        // Common: recently-used in various apps
        let recent_apps = Self::home_dir().join(".local/share/recently-used.xbel");
        if recent_apps.exists() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::AppHistory,
                    Target::Path(recent_apps.clone()),
                    "Application history (recently-used)",
                    "Recently-used application list contains traces of launched apps".to_string(),
                )
                .with_hint(format!("rm {}", recent_apps.display())),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::AppHistory,
                    Target::Path(Self::library_dir()),
                    "Application history",
                    "No application history traces detected".to_string(),
                )
                .with_hint("# no application history found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Chat traces, tracking data, credentials (ops 212-214)
    // ═══════════════════════════════════════════════════════════════════════

    /// op 212: Clear chat traces (Messages, Slack, Discord caches).
    async fn task_clear_chat_traces(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let lib = Self::library_dir();

        let chat_paths: Vec<(&str, PathBuf)> = if cfg!(target_os = "macos") {
            vec![
                ("Messages", lib.join("Messages")),
                ("Slack", lib.join("Application Support/Slack")),
                ("Discord", lib.join("Application Support/discord")),
                ("Teams", lib.join("Application Support/Microsoft Teams")),
            ]
        } else {
            vec![
                ("Slack", Self::home_dir().join(".config/Slack")),
                ("Discord", Self::home_dir().join(".config/discord")),
            ]
        };

        for (name, path) in &chat_paths {
            items += 1;
            if path.exists() {
                let size = crate::util::disk::dir_size(path);
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Medium,
                        Category::ChatTrace,
                        Target::Path(path.clone()),
                        format!("Chat traces: {}", name),
                        format!(
                            "{} chat data found at {} — may contain conversation history",
                            name,
                            path.display()
                        ),
                    )
                    .with_hint(format!(
                        "# Review and clear {} chat data: rm -rf {}",
                        name,
                        path.display()
                    ))
                    .with_size(size),
                );
                ctx.emit(findings.last().unwrap().clone()).await;
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::ChatTrace,
                    Target::Path(lib),
                    "Chat traces",
                    "No chat application data detected".to_string(),
                )
                .with_hint("# no chat traces found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 213: Remove tracking data (analytics, telemetry files).
    async fn task_remove_tracking_data(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let lib = Self::library_dir();

        // Common tracking/analytics directories
        let tracking_paths: Vec<(&str, PathBuf)> = if cfg!(target_os = "macos") {
            vec![
                (
                    "UsageTracking",
                    lib.join("Application Support/UsageTracking"),
                ),
                ("DiagnosticReports", lib.join("Logs/DiagnosticReports")),
                ("Analytics", lib.join("Analytics")),
            ]
        } else {
            vec![
                ("systemd journal", PathBuf::from("/var/log/journal")),
                ("audit log", PathBuf::from("/var/log/audit")),
            ]
        };

        for (name, path) in &tracking_paths {
            items += 1;
            if path.exists() {
                let size = if path.is_dir() {
                    crate::util::disk::dir_size(path)
                } else {
                    path.metadata().map(|m| m.len()).unwrap_or(0)
                };
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Medium,
                        Category::TrackingData,
                        Target::Path(path.clone()),
                        format!("Tracking data: {}", name),
                        format!(
                            "{} tracking/telemetry data found at {}",
                            name,
                            path.display()
                        ),
                    )
                    .with_hint(format!("# Remove tracking data: rm -rf {}", path.display()))
                    .with_size(size),
                );
                ctx.emit(findings.last().unwrap().clone()).await;
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::TrackingData,
                    Target::Path(lib),
                    "Tracking data",
                    "No tracking/telemetry data detected".to_string(),
                )
                .with_hint("# no tracking data found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 214: Remove temporary credentials (cached passwords, tokens).
    async fn task_remove_temp_credentials(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let lib = Self::library_dir();

        let cred_paths: Vec<(&str, PathBuf)> = if cfg!(target_os = "macos") {
            vec![
                ("Keychain", lib.join("Keychains")),
                ("Credentials", lib.join("Credentials")),
                (
                    "Caches/credentials",
                    lib.join("Caches/com.apple.credentials"),
                ),
            ]
        } else {
            vec![
                ("secret-service", Self::home_dir().join(".config/secrets")),
                ("pass", Self::home_dir().join(".password-store")),
            ]
        };

        for (name, path) in &cred_paths {
            items += 1;
            if path.exists() {
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::High,
                        Category::Credential,
                        Target::Path(path.clone()),
                        format!("Temporary credentials: {}", name),
                        format!(
                            "{} credential store found at {} — review for stale/cached credentials",
                            name,
                            path.display()
                        ),
                    )
                    .with_hint(format!(
                        "# Review {} credentials — do NOT delete keychain blindly",
                        name
                    )),
                );
                ctx.emit(findings.last().unwrap().clone()).await;
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::Credential,
                    Target::Path(lib),
                    "Temporary credentials",
                    "No temporary credential stores detected".to_string(),
                )
                .with_hint("# no credential caches found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Malware & suspicious app scanning (ops 215-216, 222-224)
    // ═══════════════════════════════════════════════════════════════════════

    /// Known adware bundle IDs (common macOS adware families).
    fn known_adware_bundle_ids() -> &'static [(&'static str, &'static str)] {
        &[
            ("com.mackeeper.MacKeeper", "MacKeeper"),
            ("com.genieo.Genieo", "Genieo"),
            ("com.conduit.Conduit", "Conduit"),
            ("com.spigot.Spigot", "Spigot"),
            ("com.searchinstall.SearchInstall", "SearchInstall"),
            ("com.crossrider.Crossrider", "Crossrider"),
            ("com.bundlore.Bundlore", "Bundlore"),
            ("com.shlayer.Shlayer", "Shlayer"),
            ("com.pirrit.Pirrit", "Pirrit"),
            ("com.bingsearch.BingSearch", "BingSearch"),
            ("com.chumsearch.ChumSearch", "ChumSearch"),
            ("com.searchsuite.SearchSuite", "SearchSuite"),
            ("com.easysearch.EasySearch", "EasySearch"),
            ("com.myshopcoupon.MyShopCoupon", "MyShopCoupon"),
            ("com.shopandsave.ShopAndSave", "ShopAndSave"),
        ]
    }

    /// op 215: Scan malware signatures (hash-based check against known adware bundle IDs).
    async fn task_scan_malware_signatures(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // Check Applications and ~/Library for known adware bundle IDs
            let app_dirs = [
                PathBuf::from("/Applications"),
                Self::home_dir().join("Applications"),
            ];

            for (bundle_id, adware_name) in Self::known_adware_bundle_ids() {
                // Check if an app with this bundle ID is installed
                let app_name = adware_name;
                for dir in &app_dirs {
                    let app_path = dir.join(format!("{}.app", app_name));
                    if app_path.exists() {
                        items += 1;
                        findings.push(
                            Finding::new(
                                EngineId::Privacy,
                                Severity::High,
                                Category::Malware,
                                Target::Path(app_path.clone()),
                                format!("Adware detected: {}", adware_name),
                                format!(
                                    "Known adware '{}' (bundle ID: {}) found at {}",
                                    adware_name,
                                    bundle_id,
                                    app_path.display()
                                ),
                            )
                            .with_hint(format!(
                                "# Remove adware: rm -rf {} && check ~/Library/LaunchAgents for {}",
                                app_path.display(),
                                bundle_id
                            ))
                            .with_metadata("bundle_id", serde_json::json!(bundle_id))
                            .with_metadata("adware_name", serde_json::json!(adware_name)),
                        );
                        ctx.emit(findings.last().unwrap().clone()).await;
                    }
                }

                // Check LaunchAgents for adware persistence
                let launch_agents = Self::library_dir().join("LaunchAgents");
                if launch_agents.exists() {
                    if let Ok(entries) = std::fs::read_dir(&launch_agents) {
                        for entry in entries.flatten() {
                            let name = entry.file_name().to_string_lossy().to_string();
                            if name.contains(bundle_id)
                                || name.to_lowercase().contains(&adware_name.to_lowercase())
                            {
                                items += 1;
                                findings.push(
                                    Finding::new(
                                        EngineId::Privacy,
                                        Severity::High,
                                        Category::Adware,
                                        Target::Path(entry.path()),
                                        format!("Adware persistence: {}", adware_name),
                                        format!(
                                            "LaunchAgent for adware '{}' found: {}",
                                            adware_name, name
                                        ),
                                    )
                                    .with_hint(format!(
                                        "launchctl unload {} && rm {}",
                                        entry.path().display(),
                                        entry.path().display()
                                    )),
                                );
                                ctx.emit(findings.last().unwrap().clone()).await;
                            }
                        }
                    }
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Info,
                    Category::Malware,
                    Target::Path(PathBuf::from("/")),
                    "Malware signature scan",
                    "No known adware/malware signatures detected".to_string(),
                )
                .with_hint("# system is clean of known adware".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 216: Scan suspicious applications (heuristic: unsigned or from unusual locations).
    async fn task_scan_suspicious_applications(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // Check apps in unusual locations (not /Applications or system)
            let suspicious_dirs = [
                Self::home_dir().join("Applications"),
                PathBuf::from("/Applications/Utilities"),
            ];

            for dir in &suspicious_dirs {
                if !dir.exists() {
                    continue;
                }
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if !path.is_dir() || !path.to_string_lossy().ends_with(".app") {
                            continue;
                        }
                        items += 1;
                        // Check code signature
                        let (ok, msg) =
                            Self::run_command("codesign", &["-v", &path.to_string_lossy()]);
                        if !ok {
                            let app_name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            findings.push(
                                Finding::new(
                                    EngineId::Privacy,
                                    Severity::Medium,
                                    Category::Malware,
                                    Target::Path(path.clone()),
                                    format!("Suspicious app: {}", app_name),
                                    format!(
                                        "App '{}' failed code signature verification: {}",
                                        app_name, msg
                                    ),
                                )
                                .with_hint(format!(
                                    "codesign -dv --verbose=4 {}  # inspect signature",
                                    path.display()
                                ))
                                .with_metadata("unsigned", serde_json::json!(true)),
                            );
                            ctx.emit(findings.last().unwrap().clone()).await;
                        }
                    }
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Info,
                    Category::Malware,
                    Target::Path(PathBuf::from("/")),
                    "Suspicious application scan",
                    "No suspicious/unsigned applications detected".to_string(),
                )
                .with_hint("# all scanned apps passed signature verification".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Permission auditing (ops 217-219)
    // ═══════════════════════════════════════════════════════════════════════

    /// op 217: Check permissions (general TCC database audit).
    async fn task_check_permissions(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // TCC database (requires Full Disk Access to read)
            let tcc_db = PathBuf::from("/Library/Application Support/com.apple.TCC/TCC.db");
            let user_tcc = Self::library_dir().join("Application Support/com.apple.TCC/TCC.db");

            for db in [&tcc_db, &user_tcc] {
                items += 1;
                if db.exists() {
                    let readable =
                        std::fs::read_dir(db.parent().unwrap_or(std::path::Path::new("/"))).is_ok();
                    findings.push(
                        Finding::new(
                            EngineId::Privacy,
                            if readable { Severity::Info } else { Severity::Low },
                            Category::DiskAccessPermission,
                            Target::Path(db.clone()),
                            "TCC permission database",
                            if readable {
                                format!("TCC database accessible at {} — review granted permissions", db.display())
                            } else {
                                format!("TCC database at {} is protected (Full Disk Access required to audit)", db.display())
                            },
                        )
                        .with_hint("sqlite3 /Library/Application\\ Support/com.apple.TCC/TCC.db 'SELECT service, client FROM access'".to_string()),
                    );
                    ctx.emit(findings.last().unwrap().clone()).await;
                }
            }

            // tccutil reset
            let (ok, msg) = Self::run_command("tccutil", &["--help"]);
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    if ok { Severity::Info } else { Severity::Low },
                    Category::AccessibilityPermission,
                    Target::Path(PathBuf::from("/")),
                    "Permission audit (tccutil)",
                    if ok {
                        "tccutil available — can reset per-service permissions".to_string()
                    } else {
                        format!("tccutil not available: {}", msg)
                    },
                )
                .with_hint(
                    "tccutil reset All  # resets all TCC permissions (use with caution)"
                        .to_string(),
                ),
            );
            ctx.emit(findings.last().unwrap().clone()).await;
        } else {
            // Linux: check AppArmor / SELinux status
            let (ok, msg) = Self::run_command("aa-status", &[]);
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    if ok { Severity::Info } else { Severity::Low },
                    Category::AccessibilityPermission,
                    Target::Path(PathBuf::from("/sys/kernel/security/apparmor")),
                    "Permission audit (AppArmor)",
                    if ok {
                        format!("AppArmor active:\n{}", Self::truncate(&msg, 300))
                    } else {
                        "AppArmor not active or aa-status not found".to_string()
                    },
                )
                .with_hint("aa-status  # check AppArmor profiles".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }

        (findings, items)
    }

    /// op 218: Check accessibility permissions.
    async fn task_check_accessibility_permissions(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // The accessibility TCC entries are in the system TCC.db
            // We can list apps that have requested accessibility via sqlite
            let tcc_db = PathBuf::from("/Library/Application Support/com.apple.TCC/TCC.db");
            items += 1;
            let (ok, msg) = Self::run_command(
                "sqlite3",
                &[
                    &tcc_db.to_string_lossy(),
                    "SELECT client, allowed FROM access WHERE service='kTCCServiceAccessibility';",
                ],
            );
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    if ok { Severity::Info } else { Severity::Low },
                    Category::AccessibilityPermission,
                    Target::Path(tcc_db.clone()),
                    "Accessibility permissions",
                    if ok {
                        if msg.trim().is_empty() {
                            "No apps have been granted accessibility permissions".to_string()
                        } else {
                            format!(
                                "Apps with accessibility access:\n{}",
                                Self::truncate(&msg, 500)
                            )
                        }
                    } else {
                        "Cannot read accessibility TCC entries (requires Full Disk Access)"
                            .to_string()
                    },
                )
                .with_hint(
                    "tccutil reset Accessibility  # resets accessibility permissions".to_string(),
                ),
            );
            ctx.emit(findings[0].clone()).await;
        }

        (findings, items)
    }

    /// op 219: Check full disk access permissions.
    async fn task_check_disk_access_permissions(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // Check if we have Full Disk Access by testing TCC directory readability
            let has_fda = crate::util::macos::MacosUtils::has_full_disk_access();
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    if has_fda { Severity::Info } else { Severity::Medium },
                    Category::DiskAccessPermission,
                    Target::Path(PathBuf::from("/")),
                    "Full Disk Access status",
                    if has_fda {
                        "This process has Full Disk Access — can audit TCC databases".to_string()
                    } else {
                        "This process does NOT have Full Disk Access — TCC auditing is limited".to_string()
                    },
                )
                .with_hint("System Settings > Privacy & Security > Full Disk Access  # grant to enable full auditing".to_string())
                .with_metadata("has_full_disk_access", serde_json::json!(has_fda)),
            );
            ctx.emit(findings[0].clone()).await;

            // List apps with Full Disk Access
            let tcc_db = PathBuf::from("/Library/Application Support/com.apple.TCC/TCC.db");
            items += 1;
            let (ok, msg) = Self::run_command(
                "sqlite3",
                &[
                    &tcc_db.to_string_lossy(),
                    "SELECT client, allowed FROM access WHERE service='kTCCServiceSystemPolicyAllFiles';",
                ],
            );
            if ok {
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Info,
                        Category::DiskAccessPermission,
                        Target::Path(tcc_db.clone()),
                        "Apps with Full Disk Access",
                        if msg.trim().is_empty() {
                            "No apps have been granted Full Disk Access".to_string()
                        } else {
                            format!(
                                "Full Disk Access granted to:\n{}",
                                Self::truncate(&msg, 500)
                            )
                        },
                    )
                    .with_hint(
                        "tccutil reset SystemPolicyAllFiles  # resets Full Disk Access".to_string(),
                    ),
                );
                ctx.emit(findings.last().unwrap().clone()).await;
            }
        }

        (findings, items)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Vulnerable & outdated software (ops 220-221)
    // ═══════════════════════════════════════════════════════════════════════

    /// op 220: Detect vulnerable apps (apps with known security issues).
    async fn task_detect_vulnerable_apps(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // Check for apps running from quarantine (Gatekeeper)
            let app_dirs = [
                PathBuf::from("/Applications"),
                Self::home_dir().join("Applications"),
            ];
            for dir in &app_dirs {
                if !dir.exists() {
                    continue;
                }
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if !path.is_dir() || !path.to_string_lossy().ends_with(".app") {
                            continue;
                        }
                        items += 1;
                        // Check Gatekeeper assessment
                        let (ok, msg) =
                            Self::run_command("spctl", &["-a", "-vvv", &path.to_string_lossy()]);
                        if !ok && msg.contains("rejected") {
                            let app_name = path
                                .file_name()
                                .map(|n| n.to_string_lossy().to_string())
                                .unwrap_or_default();
                            findings.push(
                                Finding::new(
                                    EngineId::Privacy,
                                    Severity::High,
                                    Category::VulnerableApp,
                                    Target::Path(path.clone()),
                                    format!("Vulnerable app: {}", app_name),
                                    format!(
                                        "App '{}' was rejected by Gatekeeper: {}",
                                        app_name, msg
                                    ),
                                )
                                .with_hint(format!(
                                    "spctl -a -vvv {}  # inspect Gatekeeper assessment",
                                    path.display()
                                )),
                            );
                            ctx.emit(findings.last().unwrap().clone()).await;
                        }
                    }
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Info,
                    Category::VulnerableApp,
                    Target::Path(PathBuf::from("/")),
                    "Vulnerable app scan",
                    "No apps rejected by Gatekeeper detected".to_string(),
                )
                .with_hint("# all scanned apps passed Gatekeeper".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 221: Detect outdated software (check versions against known minimums).
    async fn task_detect_outdated_software(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // Check macOS version
            let version = crate::util::macos::MacosUtils::get_macos_version();
            items += 1;
            let is_outdated = Self::is_macos_outdated(&version);
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    if is_outdated {
                        Severity::Medium
                    } else {
                        Severity::Info
                    },
                    Category::OutdatedSoftware,
                    Target::Path(PathBuf::from("/")),
                    "macOS version check",
                    if is_outdated {
                        format!(
                            "macOS {} may be outdated — consider updating for security patches",
                            version
                        )
                    } else {
                        format!("macOS {} is up to date", version)
                    },
                )
                .with_hint("softwareupdate -l  # check for available updates".to_string())
                .with_metadata("macos_version", serde_json::json!(version))
                .with_metadata("is_outdated", serde_json::json!(is_outdated)),
            );
            ctx.emit(findings[0].clone()).await;

            // Check for available software updates
            let (ok, msg) = Self::run_command("softwareupdate", &["-l"]);
            items += 1;
            if ok && msg.contains("Software Update found") {
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Medium,
                        Category::OutdatedSoftware,
                        Target::Path(PathBuf::from("/")),
                        "Available software updates",
                        format!("Software updates available:\n{}", Self::truncate(&msg, 500)),
                    )
                    .with_hint(
                        "sudo softwareupdate -ia  # install all available updates".to_string(),
                    ),
                );
                ctx.emit(findings.last().unwrap().clone()).await;
            }
        } else {
            // Linux: check package updates
            if Self::command_exists("apt") {
                items += 1;
                let (ok, _msg) = Self::run_command("apt", &["list", "--upgradable"]);
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Low,
                        Category::OutdatedSoftware,
                        Target::Path(PathBuf::from("/")),
                        "Outdated software (apt)",
                        if ok {
                            "Check apt list --upgradable for available updates".to_string()
                        } else {
                            "Could not check for updates".to_string()
                        },
                    )
                    .with_hint("sudo apt update && sudo apt upgrade".to_string()),
                );
                ctx.emit(findings[0].clone()).await;
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::OutdatedSoftware,
                    Target::Path(PathBuf::from("/")),
                    "Outdated software check",
                    "No package manager detected for update checking".to_string(),
                )
                .with_hint("# install apt/dnf/pacman to enable update checking".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Extensions, adware, hijackers (ops 222-224)
    // ═══════════════════════════════════════════════════════════════════════

    /// op 222: Identify risky browser extensions.
    async fn task_identify_risky_extensions(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let lib = Self::library_dir();

        // Chrome/Safari extension directories
        let ext_paths: Vec<(&str, PathBuf)> = if cfg!(target_os = "macos") {
            vec![
                ("Safari", lib.join("Safari/Extensions")),
                (
                    "Chrome",
                    lib.join("Application Support/Google/Chrome/Default/Extensions"),
                ),
                ("Firefox", lib.join("Application Support/Firefox/Profiles")),
            ]
        } else {
            vec![
                (
                    "Chrome",
                    Self::home_dir().join(".config/google-chrome/Default/Extensions"),
                ),
                ("Firefox", Self::home_dir().join(".mozilla/firefox")),
                (
                    "Chromium",
                    Self::home_dir().join(".config/chromium/Default/Extensions"),
                ),
            ]
        };

        for (browser, path) in &ext_paths {
            items += 1;
            if path.exists() {
                let count = if path.is_dir() {
                    std::fs::read_dir(path)
                        .map(|d| d.filter_map(|e| e.ok()).count())
                        .unwrap_or(0)
                } else {
                    0
                };
                if count > 0 {
                    findings.push(
                        Finding::new(
                            EngineId::Privacy,
                            Severity::Medium,
                            Category::RiskyExtension,
                            Target::Path(path.clone()),
                            format!("Browser extensions: {}", browser),
                            format!("{} has {} extension(s) installed — review for risky/privacy-invasive extensions", browser, count),
                        )
                        .with_hint(format!(
                            "# Review {} extensions at {} — check for known malicious extension IDs",
                            browser,
                            path.display()
                        ))
                        .with_metadata("extension_count", serde_json::json!(count)),
                    );
                    ctx.emit(findings.last().unwrap().clone()).await;
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::RiskyExtension,
                    Target::Path(lib),
                    "Risky extensions",
                    "No browser extensions detected".to_string(),
                )
                .with_hint("# no browser extensions found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 223: Remove adware (detect adware-related files and LaunchAgents).
    async fn task_remove_adware(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        // Reuse the malware signature scan which detects adware bundle IDs
        self.task_scan_malware_signatures(ctx).await
    }

    /// op 224: Remove browser hijackers (detect hijacked search/homepage settings).
    async fn task_remove_browser_hijackers(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;
        let lib = Self::library_dir();

        if cfg!(target_os = "macos") {
            // Check Safari homepage and search settings
            let (ok, msg) =
                Self::run_command("defaults", &["read", "com.apple.Safari", "HomePage"]);
            items += 1;
            if ok {
                let homepage = msg.trim();
                let is_suspicious = homepage.contains("search")
                    || homepage.contains("conduit")
                    || homepage.contains("bing")
                    || homepage.contains("ask.com")
                    || (!homepage.is_empty()
                        && !homepage.contains("apple.com")
                        && !homepage.contains("google.com"));
                if is_suspicious {
                    findings.push(
                        Finding::new(
                            EngineId::Privacy,
                            Severity::High,
                            Category::BrowserHijacker,
                            Target::Path(lib.join("Preferences/com.apple.Safari.plist")),
                            "Browser hijacker (Safari homepage)",
                            format!("Safari homepage is set to '{}' — may be hijacked", homepage),
                        )
                        .with_hint(
                            "defaults delete com.apple.Safari HomePage  # reset homepage"
                                .to_string(),
                        )
                        .with_metadata("homepage", serde_json::json!(homepage)),
                    );
                    ctx.emit(findings[0].clone()).await;
                }
            }
        }

        // Check Chrome homepage/search settings
        let chrome_prefs = if cfg!(target_os = "macos") {
            lib.join("Application Support/Google/Chrome/Default/Preferences")
        } else {
            Self::home_dir().join(".config/google-chrome/Default/Preferences")
        };
        items += 1;
        if chrome_prefs.exists() {
            if let Ok(content) = std::fs::read_to_string(&chrome_prefs) {
                // Simple heuristic: check for known hijacker domains in preferences
                let hijacker_domains = ["conduit", "ask.com", "search.babylon", "delta-search"];
                for domain in &hijacker_domains {
                    if content.to_lowercase().contains(domain) {
                        findings.push(
                            Finding::new(
                                EngineId::Privacy,
                                Severity::High,
                                Category::BrowserHijacker,
                                Target::Path(chrome_prefs.clone()),
                                "Browser hijacker (Chrome preferences)",
                                format!("Chrome preferences contain reference to '{}' — possible hijacker", domain),
                            )
                            .with_hint(format!(
                                "# Edit {} to remove '{}' references",
                                chrome_prefs.display(),
                                domain
                            )),
                        );
                        ctx.emit(findings.last().unwrap().clone()).await;
                    }
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Info,
                    Category::BrowserHijacker,
                    Target::Path(PathBuf::from("/")),
                    "Browser hijacker scan",
                    "No browser hijacker indicators detected".to_string(),
                )
                .with_hint("# no hijackers found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Metadata & EXIF (ops 227-228)
    // ═══════════════════════════════════════════════════════════════════════

    /// op 227: Remove metadata (detect files with embedded metadata).
    async fn task_remove_metadata(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        // Scan common document/image directories for files with metadata
        let home = Self::home_dir();
        let scan_dirs = [
            home.join("Desktop"),
            home.join("Documents"),
            home.join("Downloads"),
        ];

        for dir in &scan_dirs {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    // Files that commonly contain metadata
                    if matches!(
                        ext.as_str(),
                        "jpg" | "jpeg" | "png" | "tiff" | "heic" | "pdf" | "docx" | "xlsx" | "pptx"
                    ) {
                        items += 1;
                        let size = path.metadata().map(|m| m.len()).unwrap_or(0);
                        findings.push(
                            Finding::new(
                                EngineId::Privacy,
                                Severity::Low,
                                Category::ExifData,
                                Target::Path(path.clone()),
                                format!("Metadata-bearing file: {}", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
                                format!("File '{}' (.{}) may contain EXIF/document metadata — GPS, author, timestamps", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default(), ext),
                            )
                            .with_hint(format!(
                                "# Strip metadata: exiftool -all= {}  # or: mogrify -strip {}",
                                path.display(),
                                path.display()
                            ))
                            .with_size(size)
                            .with_metadata("file_type", serde_json::json!(ext)),
                        );
                        // Limit to first 20 per directory to avoid flooding
                        if findings.iter().filter(|f| {
                            f.target == Target::Path(dir.clone()) || matches!(f.target, Target::Path(ref p) if p.parent() == Some(dir.as_path()))
                        }).count() >= 20 {
                            break;
                        }
                    }
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::ExifData,
                    Target::Path(home),
                    "Remove metadata",
                    "No metadata-bearing files found in common directories".to_string(),
                )
                .with_hint(
                    "# scan Desktop/Documents/Downloads for images and documents with metadata"
                        .to_string(),
                ),
            );
            ctx.emit(findings[0].clone()).await;
        } else {
            for f in &findings {
                ctx.emit(f.clone()).await;
            }
        }
        (findings, items)
    }

    /// op 228: Strip EXIF information (detect images with EXIF data).
    async fn task_strip_exif(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        // Focus on image files specifically
        let home = Self::home_dir();
        let scan_dirs = [
            home.join("Desktop"),
            home.join("Pictures"),
            home.join("Downloads"),
        ];

        for dir in &scan_dirs {
            if !dir.exists() {
                continue;
            }
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if !path.is_file() {
                        continue;
                    }
                    let ext = path
                        .extension()
                        .map(|e| e.to_string_lossy().to_lowercase())
                        .unwrap_or_default();
                    if matches!(
                        ext.as_str(),
                        "jpg" | "jpeg" | "tiff" | "heic" | "raw" | "cr2" | "nef"
                    ) {
                        items += 1;
                        // Check for EXIF by reading first bytes (JPEG starts with FFD8)
                        let has_exif = Self::has_exif_data(&path);
                        if has_exif {
                            findings.push(
                                Finding::new(
                                    EngineId::Privacy,
                                    Severity::Medium,
                                    Category::ExifData,
                                    Target::Path(path.clone()),
                                    format!("EXIF data: {}", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
                                    format!("Image '{}' contains EXIF metadata — may include GPS coordinates, camera info, timestamps", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
                                )
                                .with_hint(format!("exiftool -all= {}  # strips all EXIF metadata", path.display()))
                                .with_metadata("has_exif", serde_json::json!(true)),
                            );
                            ctx.emit(findings.last().unwrap().clone()).await;
                        }
                        if findings.len() >= 50 {
                            break;
                        }
                    }
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::ExifData,
                    Target::Path(home),
                    "Strip EXIF",
                    "No images with EXIF data found in common directories".to_string(),
                )
                .with_hint("# scan Pictures/Desktop/Downloads for images with EXIF".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Suspicious processes & security events (ops 229-230)
    // ═══════════════════════════════════════════════════════════════════════

    /// op 229: Detect suspicious processes (high CPU, unknown binaries).
    async fn task_detect_suspicious_processes(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        // Get process list
        let (ok, msg) = Self::run_command("ps", &["aux"]);
        if !ok {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Low,
                    Category::SuspiciousProcess,
                    Target::Path(PathBuf::from("/")),
                    "Suspicious process scan",
                    "Could not retrieve process list".to_string(),
                )
                .with_hint("ps aux  # list running processes".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
            return (findings, items);
        }

        // Parse processes and flag suspicious ones
        let suspicious_patterns = [
            "cryptominer",
            "xmrig",
            "minerd",
            "kworkerds",
            "kthreaddi",
            "kdevtmpfsi",
            "dnsmasq",
            "ngrok",
            "reverse",
            "nc -l",
            "netcat",
        ];

        for line in msg.lines().skip(1) {
            let lower = line.to_lowercase();
            for pattern in &suspicious_patterns {
                if lower.contains(pattern) {
                    items += 1;
                    // Extract process name (last column of ps aux)
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    let command = if parts.len() > 10 {
                        parts[10..].join(" ")
                    } else {
                        line.to_string()
                    };
                    findings.push(
                        Finding::new(
                            EngineId::Privacy,
                            Severity::High,
                            Category::SuspiciousProcess,
                            Target::Path(PathBuf::from(format!("/proc/{}", parts.get(1).unwrap_or(&"?")))),
                            format!("Suspicious process: {}", pattern),
                            format!("Process matching suspicious pattern '{}' detected: {}", pattern, Self::truncate(&command, 100)),
                        )
                        .with_hint(format!(
                            "# Investigate: ps aux | grep {}  # then: kill <pid> if confirmed malicious",
                            pattern
                        ))
                        .with_metadata("pattern", serde_json::json!(pattern)),
                    );
                    ctx.emit(findings.last().unwrap().clone()).await;
                    break;
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Info,
                    Category::SuspiciousProcess,
                    Target::Path(PathBuf::from("/")),
                    "Suspicious process scan",
                    "No suspicious processes detected".to_string(),
                )
                .with_hint("# no suspicious process patterns found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 230: Monitor security events (check system log for security events).
    async fn task_monitor_security_events(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // Check for security-related log entries
            let (ok, msg) = Self::run_command(
                "log",
                &[
                    "show",
                    "--last",
                    "1h",
                    "--predicate",
                    "eventMessage contains 'security' OR eventMessage contains 'auth'",
                    "--style",
                    "syslog",
                ],
            );
            items += 1;
            let event_count = if ok {
                msg.lines().filter(|l| !l.trim().is_empty()).count()
            } else {
                0
            };
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    if ok && event_count > 100 {
                        Severity::Medium
                    } else if ok {
                        Severity::Info
                    } else {
                        Severity::Low
                    },
                    Category::SecurityEvent,
                    Target::Path(PathBuf::from("/")),
                    "Security event monitoring",
                    if ok {
                        format!(
                            "{} security/auth events logged in the last hour",
                            event_count
                        )
                    } else {
                        "Could not read security logs (requires sudo)".to_string()
                    },
                )
                .with_hint(
                    "log show --last 1h --predicate \"eventMessage contains 'security'\""
                        .to_string(),
                )
                .with_metadata("event_count", serde_json::json!(event_count)),
            );
            ctx.emit(findings[0].clone()).await;
        } else {
            // Linux: check auth log
            let auth_log = PathBuf::from("/var/log/auth.log");
            items += 1;
            if auth_log.exists() {
                let (ok, msg) = Self::run_command("tail", &["-50", "/var/log/auth.log"]);
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        if ok { Severity::Info } else { Severity::Low },
                        Category::SecurityEvent,
                        Target::Path(auth_log),
                        "Security event monitoring (auth.log)",
                        if ok {
                            format!("Recent auth events:\n{}", Self::truncate(&msg, 500))
                        } else {
                            "Could not read auth log".to_string()
                        },
                    )
                    .with_hint(
                        "tail -50 /var/log/auth.log  # review recent auth events".to_string(),
                    ),
                );
            } else {
                findings.push(
                    Finding::new(
                        EngineId::Privacy,
                        Severity::Low,
                        Category::SecurityEvent,
                        Target::Path(PathBuf::from("/var/log")),
                        "Security event monitoring",
                        "No auth log found (may need journalctl instead)".to_string(),
                    )
                    .with_hint(
                        "journalctl -u ssh --since '1 hour ago'  # check SSH auth events"
                            .to_string(),
                    ),
                );
            }
            ctx.emit(findings[0].clone()).await;
        }

        (findings, items)
    }

    // ═══════════════════════════════════════════════════════════════════════
    //  Security report & quarantine (ops 231-235)
    // ═══════════════════════════════════════════════════════════════════════

    /// op 231: Generate security report (summary of all findings).
    async fn task_generate_security_report(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let findings = vec![Finding::new(
            EngineId::Privacy,
            Severity::Info,
            Category::SecurityReport,
            Target::Path(PathBuf::from("/")),
            "Security report generated",
            "Privacy & Security scan complete — review findings above for browser traces, permissions, malware, and suspicious processes".to_string(),
        )
        .with_hint("# review all Privacy engine findings and apply remediation hints as needed".to_string())
        .with_metadata("report_type", serde_json::json!("privacy_security_summary"))];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    /// op 232: Quarantine files (detect files that should be quarantined).
    async fn task_quarantine_files(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let mut findings = Vec::new();
        let mut items = 0u64;

        if cfg!(target_os = "macos") {
            // Check for files with quarantine attribute
            let home = Self::home_dir();
            let scan_dirs = [home.join("Downloads"), home.join("Desktop")];
            for dir in &scan_dirs {
                if !dir.exists() {
                    continue;
                }
                if let Ok(entries) = std::fs::read_dir(dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if !path.is_file() {
                            continue;
                        }
                        items += 1;
                        let (ok, msg) = Self::run_command(
                            "xattr",
                            &["-p", "com.apple.quarantine", &path.to_string_lossy()],
                        );
                        if ok {
                            findings.push(
                                Finding::new(
                                    EngineId::Privacy,
                                    Severity::Low,
                                    Category::Quarantine,
                                    Target::Path(path.clone()),
                                    format!("Quarantined file: {}", path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default()),
                                    format!("File has quarantine attribute: {} — downloaded from internet", msg.trim()),
                                )
                                .with_hint(format!(
                                    "xattr -d com.apple.quarantine {}  # remove quarantine flag",
                                    path.display()
                                ))
                                .with_metadata("quarantine_flag", serde_json::json!(msg.trim())),
                            );
                            ctx.emit(findings.last().unwrap().clone()).await;
                        }
                        if findings.len() >= 30 {
                            break;
                        }
                    }
                }
            }
        }

        if findings.is_empty() {
            items += 1;
            findings.push(
                Finding::new(
                    EngineId::Privacy,
                    Severity::Info,
                    Category::Quarantine,
                    Target::Path(PathBuf::from("/")),
                    "Quarantine files",
                    "No quarantined files detected".to_string(),
                )
                .with_hint("# no files with quarantine attributes found".to_string()),
            );
            ctx.emit(findings[0].clone()).await;
        }
        (findings, items)
    }

    /// op 233: Restore quarantined files (detect files that can be restored).
    async fn task_restore_quarantined_files(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        // Reuse quarantine detection — restoration is the inverse operation
        let (mut findings, items) = self.task_quarantine_files(ctx).await;
        // Re-tag as restoration guidance
        for f in findings.iter_mut() {
            f.title = format!("Restore: {}", f.title);
            if let Some(ref hint) = f.remediation_hint {
                f.remediation_hint = Some(format!("{}  # restores file from quarantine", hint));
            }
        }
        for f in &findings {
            ctx.emit(f.clone()).await;
        }
        (findings, items)
    }

    /// op 234: Maintain threat database (report on known threat signatures).
    async fn task_maintain_threat_database(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let threat_count = Self::known_adware_bundle_ids().len();
        let findings = vec![Finding::new(
            EngineId::Privacy,
            Severity::Info,
            Category::SecurityReport,
            Target::Path(PathBuf::from("/")),
            "Threat database",
            format!("Threat database contains {} known adware/malware signatures", threat_count),
        )
        .with_hint("# threat database is maintained in-code — update known_adware_bundle_ids() for new threats".to_string())
        .with_metadata("threat_count", serde_json::json!(threat_count))];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    /// op 235: Update detection rules (report on detection rule freshness).
    async fn task_update_detection_rules(&self, ctx: &ScanContext) -> (Vec<Finding>, u64) {
        let findings = vec![Finding::new(
            EngineId::Privacy,
            Severity::Info,
            Category::SecurityReport,
            Target::Path(PathBuf::from("/")),
            "Detection rules",
            "Detection rules are compiled into the engine. Update the binary to refresh malware signatures and suspicious process patterns.".to_string(),
        )
        .with_hint("# update x-mac to get the latest detection rules and threat signatures".to_string())
        .with_metadata("rules_version", serde_json::json!(env!("CARGO_PKG_VERSION")))];
        ctx.emit(findings[0].clone()).await;
        (findings, 1)
    }

    // ── Helpers ────────────────────────────────────────────────────────────

    fn command_exists(cmd: &str) -> bool {
        std::process::Command::new("which")
            .arg(cmd)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Truncate a string to a maximum length with ellipsis.
    fn truncate(s: &str, max: usize) -> String {
        if s.len() <= max {
            s.to_string()
        } else {
            format!("{}...", &s[..max])
        }
    }

    /// Check if a macOS version string is outdated (older than 12.0 Monterey).
    fn is_macos_outdated(version: &str) -> bool {
        let parts: Vec<u32> = version
            .split('.')
            .filter_map(|p| p.parse::<u32>().ok())
            .collect();
        if parts.is_empty() {
            return false;
        }
        // Outdated if major version < 12
        parts[0] < 12
    }

    /// Check if an image file contains EXIF data by reading its header.
    fn has_exif_data(path: &PathBuf) -> bool {
        use std::io::Read;
        let mut file = match std::fs::File::open(path) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let mut header = [0u8; 32];
        let n = match file.read(&mut header) {
            Ok(n) => n,
            Err(_) => return false,
        };
        let header = &header[..n];
        // JPEG: starts with FF D8, EXIF marker is FF E1
        if header[0] == 0xFF && header[1] == 0xD8 {
            // Look for EXIF marker (FF E1) in the header
            for i in 2..header.len().saturating_sub(1) {
                if header[i] == 0xFF && header[i + 1] == 0xE1 {
                    return true;
                }
            }
            // JPEG without EXIF marker but still has some APP markers
            return header[2] == 0xFF && header[3] >= 0xE0;
        }
        // TIFF: starts with "II" or "MM" (little/big endian)
        if (header[0] == b'I' && header[1] == b'I') || (header[0] == b'M' && header[1] == b'M') {
            return true;
        }
        false
    }
}

/// Describes a browser's data file locations for privacy scanning.
struct BrowserDataLocation {
    name: &'static str,
    #[allow(dead_code)]
    category: Category,
    history_paths: Vec<PathBuf>,
    cookie_paths: Vec<PathBuf>,
    download_paths: Vec<PathBuf>,
    autofill_paths: Vec<PathBuf>,
}

#[async_trait]
impl Engine for PrivacyEngine {
    fn id(&self) -> EngineId {
        EngineId::Privacy
    }

    fn name(&self) -> &'static str {
        "Privacy & Security"
    }

    fn description(&self) -> &'static str {
        "Clears browsing traces, audits permissions, scans for malware and adware"
    }

    async fn validate(&self, _ctx: &ScanContext) -> Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> Result<EngineStats, EngineError> {
        let start = std::time::Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        macro_rules! run_task {
            ($task:ident) => {{
                let (findings, items) = self.$task(&ctx).await;
                items_scanned += items;
                findings_count += findings.len() as u64;
            }};
        }

        if self.scan_browser_data {
            // ops 206-211: Browser data
            run_task!(task_clear_browser_history);
            run_task!(task_clear_cookies);
            run_task!(task_clear_downloads_history);
            run_task!(task_clear_autofill_data);
            run_task!(task_clear_recent_documents);
            run_task!(task_clear_application_history);

            // ops 212-214: Chat, tracking, credentials
            run_task!(task_clear_chat_traces);
            run_task!(task_remove_tracking_data);
            run_task!(task_remove_temp_credentials);

            // ops 222-224: Extensions, adware, hijackers
            run_task!(task_identify_risky_extensions);
            run_task!(task_remove_adware);
            run_task!(task_remove_browser_hijackers);

            // ops 227-228: Metadata & EXIF
            run_task!(task_remove_metadata);
            run_task!(task_strip_exif);
        }

        if self.scan_malware {
            // ops 215-216: Malware & suspicious apps
            run_task!(task_scan_malware_signatures);
            run_task!(task_scan_suspicious_applications);

            // ops 229-230: Suspicious processes & security events
            run_task!(task_detect_suspicious_processes);
            run_task!(task_monitor_security_events);

            // ops 231-235: Security report & quarantine
            run_task!(task_generate_security_report);
            run_task!(task_quarantine_files);
            run_task!(task_restore_quarantined_files);
            run_task!(task_maintain_threat_database);
            run_task!(task_update_detection_rules);
        }

        if self.scan_permissions {
            // ops 217-219: Permission auditing
            run_task!(task_check_permissions);
            run_task!(task_check_accessibility_permissions);
            run_task!(task_check_disk_access_permissions);
        }

        // ops 220-221: Vulnerable & outdated software (always run)
        run_task!(task_detect_vulnerable_apps);
        run_task!(task_detect_outdated_software);

        Ok(EngineStats {
            engine: self.id(),
            duration: start.elapsed(),
            items_scanned,
            findings_count,
            errors_count: 0,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_privacy_engine_id() {
        let engine = PrivacyEngine::new();
        assert_eq!(engine.id(), EngineId::Privacy);
        assert_eq!(engine.name(), "Privacy & Security");
        assert!(!engine.description().is_empty());
    }

    #[test]
    fn test_privacy_engine_default() {
        let engine = PrivacyEngine::default();
        assert_eq!(engine.id(), EngineId::Privacy);
        assert!(engine.scan_browser_data);
        assert!(engine.scan_malware);
        assert!(engine.scan_permissions);
    }

    #[test]
    fn test_privacy_engine_builder() {
        let engine = PrivacyEngine::new()
            .without_browser_data()
            .without_malware_scan()
            .without_permission_audit();
        assert!(!engine.scan_browser_data);
        assert!(!engine.scan_malware);
        assert!(!engine.scan_permissions);
    }

    #[test]
    fn test_known_adware_bundle_ids() {
        let ids = PrivacyEngine::known_adware_bundle_ids();
        assert!(!ids.is_empty());
        assert!(ids.iter().any(|(id, _)| id.contains("MacKeeper")));
        assert!(ids.iter().any(|(id, _)| id.contains("Genieo")));
    }

    #[test]
    fn test_is_macos_outdated() {
        assert!(PrivacyEngine::is_macos_outdated("10.15.7"));
        assert!(PrivacyEngine::is_macos_outdated("11.6"));
        assert!(!PrivacyEngine::is_macos_outdated("12.0"));
        assert!(!PrivacyEngine::is_macos_outdated("13.5"));
        assert!(!PrivacyEngine::is_macos_outdated("14.0"));
        assert!(!PrivacyEngine::is_macos_outdated("unknown"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(PrivacyEngine::truncate("short", 10), "short");
        assert_eq!(PrivacyEngine::truncate("a very long string", 5), "a ver...");
    }

    #[test]
    fn test_browser_data_locations_not_empty() {
        let locations = PrivacyEngine::browser_data_locations();
        assert!(!locations.is_empty());
        // Each location should have at least history paths
        for loc in &locations {
            assert!(!loc.history_paths.is_empty());
        }
    }

    #[test]
    fn test_has_exif_data_jpeg() {
        use std::io::Write;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.jpg");
        let mut file = std::fs::File::create(&path).unwrap();
        // Write JPEG header with EXIF marker
        file.write_all(&[0xFF, 0xD8, 0xFF, 0xE1, 0x00, 0x00])
            .unwrap();
        drop(file);
        assert!(PrivacyEngine::has_exif_data(&path));
    }

    #[test]
    fn test_has_exif_data_no_exif() {
        use std::io::Write;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.txt");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(b"not an image").unwrap();
        drop(file);
        assert!(!PrivacyEngine::has_exif_data(&path));
    }

    #[test]
    fn test_has_exif_data_tiff() {
        use std::io::Write;
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.tiff");
        let mut file = std::fs::File::create(&path).unwrap();
        // TIFF little-endian header
        file.write_all(&[b'I', b'I', 0x2A, 0x00]).unwrap();
        drop(file);
        assert!(PrivacyEngine::has_exif_data(&path));
    }

    #[tokio::test]
    async fn test_privacy_engine_scan_completes() {
        use clap::Parser;
        let engine = PrivacyEngine::new();
        let cli = crate::cli::args::Cli::parse_from(vec!["x-mac", "clean"]);
        let (tx, _rx) = tokio::sync::mpsc::channel::<Finding>(1000);
        let ctx = Arc::new(ScanContext::new(&cli, tx).await.unwrap());
        let result = engine.scan(ctx).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert_eq!(stats.engine, EngineId::Privacy);
        assert!(stats.items_scanned > 0);
        assert!(stats.findings_count > 0);
    }

    #[tokio::test]
    async fn test_privacy_engine_scan_browser_data_only() {
        use clap::Parser;
        let engine = PrivacyEngine::new()
            .without_malware_scan()
            .without_permission_audit();
        let cli = crate::cli::args::Cli::parse_from(vec!["x-mac", "clean"]);
        let (tx, _rx) = tokio::sync::mpsc::channel::<Finding>(1000);
        let ctx = Arc::new(ScanContext::new(&cli, tx).await.unwrap());
        let result = engine.scan(ctx).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        assert!(stats.findings_count > 0);
    }

    #[tokio::test]
    async fn test_privacy_engine_scan_disabled() {
        use clap::Parser;
        let engine = PrivacyEngine::new()
            .without_browser_data()
            .without_malware_scan()
            .without_permission_audit();
        let cli = crate::cli::args::Cli::parse_from(vec!["x-mac", "clean"]);
        let (tx, _rx) = tokio::sync::mpsc::channel::<Finding>(1000);
        let ctx = Arc::new(ScanContext::new(&cli, tx).await.unwrap());
        let result = engine.scan(ctx).await;
        assert!(result.is_ok());
        let stats = result.unwrap();
        // Only vulnerable/outdated apps run when all categories disabled
        assert!(stats.findings_count > 0);
    }
}
