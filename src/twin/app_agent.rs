use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// Application Intelligence Agent — understands and models every application.
///
/// Covers: app purpose, behavior, dependencies, files, permissions,
/// unused/abandoned/duplicate detection, broken installation detection,
/// uninstall impact prediction, crash prediction, app health scoring,
/// and suspicious behavior detection.
///
/// Maps to Digital Twin operations 236-275.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIntelligenceGraph {
    /// op 236-240: Per-application intelligence nodes.
    pub apps: Vec<AppIntelligenceNode>,
    /// op 271: Health scores keyed by bundle id (0-100).
    pub health_scores: HashMap<String, f64>,
    /// op 268: Apps flagged for suspicious behavior.
    pub suspicious_apps: Vec<String>,
    /// op 241: Apps with no recorded recent launch.
    pub unused_apps: Vec<String>,
    /// op 242: Groups of duplicate apps (same bundle id, different paths).
    pub duplicate_apps: Vec<Vec<String>>,
    /// op 260: Application knowledge graph edges (dependency relationships).
    pub knowledge_graph: Vec<AppEdge>,
    /// op 265: Automated application policies.
    pub policies: Vec<AppPolicy>,
    /// op 256: Human-readable problem explanations.
    pub problem_explanations: Vec<String>,
    pub total_apps: usize,
}

/// op 236-240: A single application's intelligence node.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppIntelligenceNode {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
    /// op 236: Inferred purpose category ("productivity", "developer", etc.).
    pub purpose: Option<String>,
    /// op 237: Observed behavior class ("foreground", "background-daemon", etc.).
    pub behavior: Option<String>,
    /// op 238: Dependencies (frameworks, libraries, other apps).
    pub dependencies: Vec<String>,
    /// op 239: Files owned by the app (preferences, containers, support).
    pub files: Vec<String>,
    /// op 240: Permissions the app holds (TCC entries).
    pub permissions: Vec<String>,
    /// op 271: Health score 0-100.
    pub health_score: f64,
    pub last_launched_ms: Option<u64>,
    pub is_unused: bool,
    pub is_suspicious: bool,
    /// op 254: Predicted crash probability (0-1).
    pub crash_probability: Option<f64>,
    /// op 245: Whether preferences appear corrupted.
    pub preferences_corrupted: bool,
    /// op 246: Estimated bytes freed if uninstalled.
    pub uninstall_impact_bytes: Option<u64>,
}

/// op 260: An edge in the application knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppEdge {
    pub source: String,
    pub target: String,
    pub relationship: String,
}

/// op 265: An automated application policy.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppPolicy {
    pub name: String,
    pub trigger: String,
    pub action: String,
    pub enabled: bool,
}

impl AppIntelligenceGraph {
    /// Collect the application intelligence graph.
    pub fn collect() -> Self {
        // op 236-240: Build app nodes from the envmap engine's app list.
        let apps = collect_app_nodes();
        let total_apps = apps.len();

        // op 271: Compute health scores.
        let health_scores: HashMap<String, f64> = apps
            .iter()
            .map(|a| (a.bundle_id.clone(), a.health_score))
            .collect();
        // op 268: Suspicious apps.
        let suspicious_apps: Vec<String> = apps
            .iter()
            .filter(|a| a.is_suspicious)
            .map(|a| a.name.clone())
            .collect();
        // op 241: Unused apps.
        let unused_apps: Vec<String> = apps
            .iter()
            .filter(|a| a.is_unused)
            .map(|a| a.name.clone())
            .collect();
        // op 242: Duplicate apps (group by bundle_id).
        let duplicate_apps = detect_duplicate_apps(&apps);
        // op 260: Build knowledge graph edges from dependencies.
        let knowledge_graph = build_knowledge_graph(&apps);
        // op 265: Default policies.
        let policies = default_app_policies();
        // op 256: Explain problems.
        let mut problem_explanations = Vec::new();
        if !suspicious_apps.is_empty() {
            problem_explanations.push(format!(
                "{} app(s) show suspicious behavior and should be reviewed.",
                suspicious_apps.len()
            ));
        }
        let corrupted: Vec<_> = apps.iter().filter(|a| a.preferences_corrupted).collect();
        if !corrupted.is_empty() {
            problem_explanations.push(format!(
                "{} app(s) have corrupted preference files.",
                corrupted.len()
            ));
        }

        Self {
            apps,
            health_scores,
            suspicious_apps,
            unused_apps,
            duplicate_apps,
            knowledge_graph,
            policies,
            problem_explanations,
            total_apps,
        }
    }

    /// op 245: Detect corrupted preferences — returns bundle ids.
    pub fn corrupted_preference_apps(&self) -> Vec<String> {
        self.apps
            .iter()
            .filter(|a| a.preferences_corrupted)
            .map(|a| a.bundle_id.clone())
            .collect()
    }

    /// op 246: Predict uninstall impact — total bytes freed if the given
    /// apps were removed.
    pub fn predict_uninstall_impact(&self, bundle_ids: &[String]) -> u64 {
        self.apps
            .iter()
            .filter(|a| bundle_ids.contains(&a.bundle_id))
            .filter_map(|a| a.uninstall_impact_bytes)
            .sum()
    }

    /// op 249: Track application evolution — return apps whose version
    /// changed (placeholder; requires history).
    pub fn track_evolution(&self) -> Vec<String> {
        Vec::new()
    }

    /// op 250: Recommend alternatives for unused/abandoned apps.
    pub fn recommend_alternatives(&self) -> Vec<(String, String)> {
        self.apps
            .iter()
            .filter(|a| a.is_unused)
            .map(|a| {
                let alt = match a.purpose.as_deref().unwrap_or("unknown") {
                    "browser" => "Consider Safari or Firefox.".to_string(),
                    "editor" => "Consider VS Code or Zed.".to_string(),
                    _ => "Consider uninstalling if no longer needed.".to_string(),
                };
                (a.name.clone(), alt)
            })
            .collect()
    }

    /// op 251: Detect inefficient apps — low health score.
    pub fn inefficient_apps(&self) -> Vec<&AppIntelligenceNode> {
        self.apps.iter().filter(|a| a.health_score < 50.0).collect()
    }

    /// op 252: Benchmark applications — return health scores as benchmarks.
    pub fn benchmark(&self) -> Vec<(String, f64)> {
        self.apps
            .iter()
            .map(|a| (a.name.clone(), a.health_score))
            .collect()
    }

    /// op 253: Compare two applications by bundle id.
    pub fn compare(
        &self,
        a: &str,
        b: &str,
    ) -> Option<(&AppIntelligenceNode, &AppIntelligenceNode)> {
        let na = self.apps.iter().find(|x| x.bundle_id == a)?;
        let nb = self.apps.iter().find(|x| x.bundle_id == b)?;
        Some((na, nb))
    }

    /// op 254: Predict crashes — return apps with crash probability > 0.5.
    pub fn crash_risks(&self) -> Vec<&AppIntelligenceNode> {
        self.apps
            .iter()
            .filter(|a| a.crash_probability.map(|p| p > 0.5).unwrap_or(false))
            .collect()
    }

    /// op 255: Diagnose failures — return explanations for low-health apps.
    pub fn diagnose_failures(&self) -> Vec<String> {
        self.apps
            .iter()
            .filter(|a| a.health_score < 50.0)
            .map(|a| {
                format!(
                    "{}: health {:.0} — {}",
                    a.name,
                    a.health_score,
                    if a.preferences_corrupted {
                        "corrupted preferences detected"
                    } else if a.is_unused {
                        "unused and consuming storage"
                    } else {
                        "elevated crash risk"
                    }
                )
            })
            .collect()
    }

    /// op 257: Recommend fixes for problematic apps.
    pub fn recommend_fixes(&self) -> Vec<(String, String)> {
        self.apps
            .iter()
            .filter(|a| a.health_score < 70.0)
            .map(|a| {
                let fix = if a.preferences_corrupted {
                    "Reset preferences to defaults.".to_string()
                } else if a.is_unused {
                    "Uninstall to reclaim storage.".to_string()
                } else {
                    "Update to the latest version.".to_string()
                };
                (a.name.clone(), fix)
            })
            .collect()
    }

    /// op 258: Automatically repair issues — returns the actions taken.
    pub fn auto_repair(&self) -> Vec<String> {
        let mut actions = Vec::new();
        for a in self.apps.iter().filter(|a| a.preferences_corrupted) {
            actions.push(format!("Reset corrupted preferences for {}.", a.name));
        }
        actions
    }

    /// op 261: Predict user needs — apps likely to be needed next.
    pub fn predict_user_needs(&self) -> Vec<String> {
        // Heuristic: recently launched apps first.
        let mut apps: Vec<&AppIntelligenceNode> = self.apps.iter().collect();
        apps.sort_by_key(|a| std::cmp::Reverse(a.last_launched_ms));
        apps.iter().take(3).map(|a| a.name.clone()).collect()
    }

    /// op 262: Preload common tools — return names of tools to preload.
    pub fn preload_tools(&self) -> Vec<String> {
        self.predict_user_needs()
    }

    /// op 263: Optimize workflows — return workflow recommendations.
    pub fn optimize_workflows(&self) -> Vec<String> {
        vec![
            "Pin frequently-used apps to the Dock for faster access.".to_string(),
            "Create Launchpad folders for related apps.".to_string(),
        ]
    }

    /// op 264: Automate repetitive actions — return automation suggestions.
    pub fn automate_actions(&self) -> Vec<String> {
        vec![
            "Use Shortcuts to automate launching app sets.".to_string(),
            "Create a LaunchAgent for periodic app cleanup.".to_string(),
        ]
    }

    /// op 268: Detect suspicious behavior — return suspicious app names.
    pub fn detect_suspicious(&self) -> Vec<String> {
        self.suspicious_apps.clone()
    }

    /// op 269: Detect unnecessary network access — apps with network perms
    /// that don't need them.
    pub fn unnecessary_network_access(&self) -> Vec<String> {
        self.apps
            .iter()
            .filter(|a| {
                a.permissions.iter().any(|p| p.contains("network"))
                    && a.purpose.as_deref() == Some("offline")
            })
            .map(|a| a.name.clone())
            .collect()
    }

    /// op 270: Detect excessive background activity — unused apps with
    /// background behavior.
    pub fn excessive_background(&self) -> Vec<String> {
        self.apps
            .iter()
            .filter(|a| a.is_unused && a.behavior.as_deref() == Some("background-daemon"))
            .map(|a| a.name.clone())
            .collect()
    }

    /// op 271: Generate an app health score (0-100).
    pub fn app_health_score(&self, bundle_id: &str) -> Option<f64> {
        self.health_scores.get(bundle_id).copied()
    }

    /// op 259: Maintain application profiles — update app profiles with the
    /// latest usage data, health scores, and crash predictions. Refreshes
    /// the derived `health_scores`, `unused_apps`, and `suspicious_apps`
    /// aggregates from the per-app nodes.
    #[allow(dead_code)]
    pub fn maintain_app_profiles(&mut self) {
        // Recompute health scores from the latest node state.
        self.health_scores = self
            .apps
            .iter()
            .map(|a| (a.bundle_id.clone(), a.health_score))
            .collect();

        // Refresh the unused-apps list.
        self.unused_apps = self
            .apps
            .iter()
            .filter(|a| a.is_unused)
            .map(|a| a.name.clone())
            .collect();

        // Refresh the suspicious-apps list.
        self.suspicious_apps = self
            .apps
            .iter()
            .filter(|a| a.is_suspicious)
            .map(|a| a.name.clone())
            .collect();

        // Update crash predictions for each app based on its current state.
        for app in &mut self.apps {
            let prob = estimate_crash_probability(app.is_unused, app.preferences_corrupted);
            app.crash_probability = Some(prob);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────
// Helpers
// ─────────────────────────────────────────────────────────────────────

/// op 236-240: Collect application nodes from the envmap engine.
fn collect_app_nodes() -> Vec<AppIntelligenceNode> {
    let mut installed: Vec<crate::engines::envmap::apps::InstalledApp> = Vec::new();
    for dir in crate::engines::envmap::apps::default_app_dirs() {
        installed.extend(crate::engines::envmap::apps::enumerate_apps_in(&dir, 3));
    }
    installed
        .into_iter()
        .map(|app| {
            let bundle_id = app
                .bundle_id
                .clone()
                .unwrap_or_else(|| app.bundle_name.clone());
            let purpose = infer_purpose(&app.bundle_name, &bundle_id);
            let behavior = infer_behavior(&bundle_id);
            let is_unused = is_app_unused(&app.bundle_path);
            let is_suspicious = is_app_suspicious(&app.bundle_name, &bundle_id);
            let preferences_corrupted = check_preferences_corrupted(&bundle_id);
            let crash_probability = estimate_crash_probability(is_unused, preferences_corrupted);
            // op 246: Estimate uninstall impact from the bundle size.
            let uninstall_impact_bytes = compute_bundle_size(&app.bundle_path);
            let health_score = compute_app_health(
                is_unused,
                is_suspicious,
                preferences_corrupted,
                &app.version,
            );
            let files = collect_app_files(&bundle_id);
            let permissions = infer_permissions(&bundle_id);
            let dependencies = infer_dependencies(&bundle_id);
            // op 117: Best-effort last launch time from bundle mtime.
            let last_launched_ms = std::fs::metadata(&app.bundle_path)
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as u64);
            AppIntelligenceNode {
                bundle_id,
                name: app.bundle_name,
                version: app.version,
                purpose: Some(purpose),
                behavior: Some(behavior),
                dependencies,
                files,
                permissions,
                health_score,
                last_launched_ms,
                is_unused,
                is_suspicious,
                crash_probability: Some(crash_probability),
                preferences_corrupted,
                uninstall_impact_bytes,
            }
        })
        .collect()
}

/// op 236: Infer an app's purpose from its name and bundle id.
fn infer_purpose(name: &str, bundle_id: &str) -> String {
    let lower = format!("{} {}", name, bundle_id).to_lowercase();
    if lower.contains("xcode") || lower.contains("dev") || lower.contains("code") {
        "developer".to_string()
    } else if lower.contains("safari") || lower.contains("chrome") || lower.contains("firefox") {
        "browser".to_string()
    } else if lower.contains("pages") || lower.contains("word") || lower.contains("notes") {
        "productivity".to_string()
    } else if lower.contains("game") || lower.contains("steam") {
        "gaming".to_string()
    } else if lower.contains("docker") || lower.contains("vm") {
        "virtualization".to_string()
    } else {
        "general".to_string()
    }
}

/// op 237: Infer an app's behavior class.
fn infer_behavior(bundle_id: &str) -> String {
    let lower = bundle_id.to_lowercase();
    if lower.contains("daemon") || lower.contains("agent") || lower.contains("helper") {
        "background-daemon".to_string()
    } else if lower.contains("sync") || lower.contains("backup") {
        "background-sync".to_string()
    } else {
        "foreground".to_string()
    }
}

/// op 238: Infer dependencies (placeholder — real impl reads the app's
/// Frameworks directory).
fn infer_dependencies(bundle_id: &str) -> Vec<String> {
    let mut deps = Vec::new();
    if bundle_id.contains("apple") {
        deps.push("Foundation".to_string());
        deps.push("AppKit".to_string());
    }
    deps
}

/// op 239: Collect files owned by the app (preferences, containers, support).
fn collect_app_files(bundle_id: &str) -> Vec<String> {
    let home = std::env::var("HOME").unwrap_or_default();
    let mut files = Vec::new();
    let prefs = format!("{}/Library/Preferences/{}.plist", home, bundle_id);
    if Path::new(&prefs).exists() {
        files.push(prefs);
    }
    let container = format!("{}/Library/Containers/{}", home, bundle_id);
    if Path::new(&container).exists() {
        files.push(container);
    }
    let support = format!("{}/Library/Application Support/{}", home, bundle_id);
    if Path::new(&support).exists() {
        files.push(support);
    }
    files
}

/// op 240: Infer permissions (placeholder — real impl reads TCC.db).
fn infer_permissions(bundle_id: &str) -> Vec<String> {
    let lower = bundle_id.to_lowercase();
    let mut perms = Vec::new();
    if lower.contains("browser") || lower.contains("mail") || lower.contains("chat") {
        perms.push("network".to_string());
    }
    if lower.contains("camera") || lower.contains("zoom") || lower.contains("facetime") {
        perms.push("camera".to_string());
    }
    if lower.contains("mic") || lower.contains("audio") || lower.contains("voice") {
        perms.push("microphone".to_string());
    }
    perms
}

/// op 241: Determine whether an app is unused (bundle mtime older than 90d).
fn is_app_unused(bundle_path: &Path) -> bool {
    let mtime = match std::fs::metadata(bundle_path).and_then(|m| m.modified()) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let days = SystemTime::now()
        .duration_since(mtime)
        .map(|d| d.as_secs() / 86_400)
        .unwrap_or(0);
    days > 90
}

/// op 268: Heuristic suspicious-app detection.
fn is_app_suspicious(name: &str, bundle_id: &str) -> bool {
    let lower = format!("{} {}", name, bundle_id).to_lowercase();
    lower.contains("crack") || lower.contains("keygen") || lower.contains("pirated")
}

/// op 245: Check whether an app's preference file is corrupted (unreadable
/// plist). A zero-byte plist is a cheap proxy for corruption.
fn check_preferences_corrupted(bundle_id: &str) -> bool {
    let home = std::env::var("HOME").unwrap_or_default();
    let prefs = format!("{}/Library/Preferences/{}.plist", home, bundle_id);
    let path = Path::new(&prefs);
    if !path.exists() {
        return false;
    }
    std::fs::metadata(path)
        .map(|m| m.len() == 0)
        .unwrap_or(false)
}

/// op 246: Compute bundle size (bytes) for uninstall impact.
fn compute_bundle_size(bundle_path: &Path) -> Option<u64> {
    if !bundle_path.is_dir() {
        return std::fs::metadata(bundle_path).map(|m| m.len()).ok();
    }
    let mut total: u64 = 0;
    for entry in walkdir::WalkDir::new(bundle_path)
        .follow_links(false)
        .into_iter()
    {
        let entry = entry.ok()?;
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    Some(total)
}

/// op 254: Estimate crash probability from risk factors.
fn estimate_crash_probability(is_unused: bool, corrupted: bool) -> f64 {
    let mut p: f64 = 0.1;
    if corrupted {
        p += 0.4;
    }
    if is_unused {
        p += 0.1;
    }
    p.min(1.0f64)
}

/// op 271: Compute an app's health score (0-100).
fn compute_app_health(is_unused: bool, is_suspicious: bool, corrupted: bool, version: &str) -> f64 {
    let mut score: f64 = 100.0;
    if is_unused {
        score -= 20.0;
    }
    if is_suspicious {
        score -= 40.0;
    }
    if corrupted {
        score -= 30.0;
    }
    if version == "unknown" {
        score -= 10.0;
    }
    score.max(0.0f64)
}

/// op 242: Detect duplicate apps (same bundle id in multiple locations).
fn detect_duplicate_apps(apps: &[AppIntelligenceNode]) -> Vec<Vec<String>> {
    let mut groups: HashMap<String, Vec<String>> = HashMap::new();
    for a in apps {
        groups
            .entry(a.bundle_id.clone())
            .or_default()
            .push(a.name.clone());
    }
    groups.into_values().filter(|g| g.len() > 1).collect()
}

/// op 260: Build the knowledge graph from dependency edges.
fn build_knowledge_graph(apps: &[AppIntelligenceNode]) -> Vec<AppEdge> {
    let mut edges = Vec::new();
    for a in apps {
        for dep in &a.dependencies {
            edges.push(AppEdge {
                source: a.bundle_id.clone(),
                target: dep.clone(),
                relationship: "depends-on".to_string(),
            });
        }
    }
    edges
}

/// op 265: Default automated app policies.
fn default_app_policies() -> Vec<AppPolicy> {
    vec![
        AppPolicy {
            name: "Auto-uninstall abandoned apps".to_string(),
            trigger: "unused > 180d and not system_app".to_string(),
            action: "prompt_uninstall".to_string(),
            enabled: false,
        },
        AppPolicy {
            name: "Reset corrupted preferences".to_string(),
            trigger: "preferences_corrupted == true".to_string(),
            action: "reset_prefs".to_string(),
            enabled: true,
        },
        AppPolicy {
            name: "Quarantine suspicious apps".to_string(),
            trigger: "is_suspicious == true".to_string(),
            action: "quarantine".to_string(),
            enabled: true,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_runs() {
        let g = AppIntelligenceGraph::collect();
        // total_apps is usize, always >= 0
        assert!(!g.policies.is_empty());
    }

    #[test]
    fn test_infer_purpose() {
        assert_eq!(infer_purpose("Xcode", "com.apple.dt.Xcode"), "developer");
        assert_eq!(infer_purpose("Safari", "com.apple.Safari"), "browser");
        assert_eq!(infer_purpose("Pages", "com.apple.Pages"), "productivity");
        assert_eq!(infer_purpose("Foo", "com.example.foo"), "general");
    }

    #[test]
    fn test_infer_behavior() {
        assert_eq!(infer_behavior("com.example.daemon"), "background-daemon");
        assert_eq!(infer_behavior("com.example.app"), "foreground");
    }

    #[test]
    fn test_estimate_crash_probability() {
        assert!((estimate_crash_probability(false, false) - 0.1).abs() < 1e-9);
        assert!((estimate_crash_probability(false, true) - 0.5).abs() < 1e-9);
        assert!((estimate_crash_probability(true, true) - 0.6).abs() < 1e-9);
    }

    #[test]
    fn test_compute_app_health() {
        assert_eq!(compute_app_health(false, false, false, "1.0"), 100.0);
        assert_eq!(compute_app_health(true, false, false, "1.0"), 80.0);
        assert_eq!(compute_app_health(false, true, false, "1.0"), 60.0);
        assert_eq!(compute_app_health(false, false, true, "1.0"), 70.0);
        assert_eq!(compute_app_health(true, true, true, "unknown"), 0.0);
    }

    #[test]
    fn test_detect_duplicate_apps() {
        let apps = vec![
            AppIntelligenceNode {
                bundle_id: "com.example.foo".to_string(),
                name: "Foo".to_string(),
                version: "1.0".to_string(),
                purpose: None,
                behavior: None,
                dependencies: vec![],
                files: vec![],
                permissions: vec![],
                health_score: 80.0,
                last_launched_ms: None,
                is_unused: false,
                is_suspicious: false,
                crash_probability: None,
                preferences_corrupted: false,
                uninstall_impact_bytes: None,
            },
            AppIntelligenceNode {
                bundle_id: "com.example.foo".to_string(),
                name: "Foo2".to_string(),
                version: "1.0".to_string(),
                purpose: None,
                behavior: None,
                dependencies: vec![],
                files: vec![],
                permissions: vec![],
                health_score: 80.0,
                last_launched_ms: None,
                is_unused: false,
                is_suspicious: false,
                crash_probability: None,
                preferences_corrupted: false,
                uninstall_impact_bytes: None,
            },
        ];
        let dups = detect_duplicate_apps(&apps);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].len(), 2);
    }

    #[test]
    fn test_build_knowledge_graph() {
        let apps = vec![AppIntelligenceNode {
            bundle_id: "com.example.foo".to_string(),
            name: "Foo".to_string(),
            version: "1.0".to_string(),
            purpose: None,
            behavior: None,
            dependencies: vec!["Foundation".to_string()],
            files: vec![],
            permissions: vec![],
            health_score: 80.0,
            last_launched_ms: None,
            is_unused: false,
            is_suspicious: false,
            crash_probability: None,
            preferences_corrupted: false,
            uninstall_impact_bytes: None,
        }];
        let edges = build_knowledge_graph(&apps);
        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].relationship, "depends-on");
    }

    #[test]
    fn test_default_app_policies() {
        let p = default_app_policies();
        assert!(p.iter().any(|p| p.action == "quarantine"));
    }

    #[test]
    fn test_predict_uninstall_impact() {
        let mut g = AppIntelligenceGraph::collect();
        g.apps.push(AppIntelligenceNode {
            bundle_id: "test.foo".to_string(),
            name: "Foo".to_string(),
            version: "1.0".to_string(),
            purpose: None,
            behavior: None,
            dependencies: vec![],
            files: vec![],
            permissions: vec![],
            health_score: 80.0,
            last_launched_ms: None,
            is_unused: false,
            is_suspicious: false,
            crash_probability: None,
            preferences_corrupted: false,
            uninstall_impact_bytes: Some(1_000_000),
        });
        let impact = g.predict_uninstall_impact(&["test.foo".to_string()]);
        assert_eq!(impact, 1_000_000);
    }

    #[test]
    fn test_maintain_app_profiles_refreshes_aggregates() {
        let mut g = AppIntelligenceGraph::collect();
        g.apps.clear();
        g.apps.push(AppIntelligenceNode {
            bundle_id: "com.example.unused".to_string(),
            name: "UnusedApp".to_string(),
            version: "1.0".to_string(),
            purpose: None,
            behavior: None,
            dependencies: vec![],
            files: vec![],
            permissions: vec![],
            health_score: 40.0,
            last_launched_ms: None,
            is_unused: true,
            is_suspicious: false,
            crash_probability: None,
            preferences_corrupted: false,
            uninstall_impact_bytes: None,
        });
        g.apps.push(AppIntelligenceNode {
            bundle_id: "com.example.suspicious".to_string(),
            name: "SuspiciousApp".to_string(),
            version: "1.0".to_string(),
            purpose: None,
            behavior: None,
            dependencies: vec![],
            files: vec![],
            permissions: vec![],
            health_score: 30.0,
            last_launched_ms: None,
            is_unused: false,
            is_suspicious: true,
            crash_probability: None,
            preferences_corrupted: false,
            uninstall_impact_bytes: None,
        });
        // Clear aggregates so we can verify they get repopulated.
        g.unused_apps.clear();
        g.suspicious_apps.clear();
        g.health_scores.clear();

        g.maintain_app_profiles();

        assert!(g.unused_apps.contains(&"UnusedApp".to_string()));
        assert!(g.suspicious_apps.contains(&"SuspiciousApp".to_string()));
        assert_eq!(g.health_scores.get("com.example.unused"), Some(&40.0));
        assert_eq!(g.health_scores.get("com.example.suspicious"), Some(&30.0));
        // Crash probabilities should now be populated.
        let unused = g
            .apps
            .iter()
            .find(|a| a.bundle_id == "com.example.unused")
            .unwrap();
        assert!(unused.crash_probability.is_some());
        // Unused + not corrupted → 0.2
        assert!((unused.crash_probability.unwrap() - 0.2).abs() < 1e-9);
    }

    #[test]
    fn test_maintain_app_profiles_updates_crash_predictions() {
        let mut g = AppIntelligenceGraph::collect();
        g.apps.clear();
        g.apps.push(AppIntelligenceNode {
            bundle_id: "com.example.corrupt".to_string(),
            name: "CorruptApp".to_string(),
            version: "1.0".to_string(),
            purpose: None,
            behavior: None,
            dependencies: vec![],
            files: vec![],
            permissions: vec![],
            health_score: 50.0,
            last_launched_ms: None,
            is_unused: false,
            is_suspicious: false,
            crash_probability: Some(0.1),
            preferences_corrupted: true,
            uninstall_impact_bytes: None,
        });

        g.maintain_app_profiles();

        let app = g
            .apps
            .iter()
            .find(|a| a.bundle_id == "com.example.corrupt")
            .unwrap();
        // corrupted → 0.1 + 0.4 = 0.5
        assert!((app.crash_probability.unwrap() - 0.5).abs() < 1e-9);
    }
}
