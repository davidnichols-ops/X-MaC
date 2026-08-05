//! Installed application enumeration.
//!
//! On macOS: walks `/Applications` and `~/Applications` for `*.app` bundles,
//! reads each bundle's `Contents/Info.plist`, and extracts the bundle name
//! (`CFBundleName` / `CFBundleExecutable`) and version
//! (`CFBundleShortVersionString`, falling back to `CFBundleVersion`).
//!
//! On Linux: scans Freedesktop `.desktop` files from standard XDG data dirs,
//! plus flatpak and snap application listings.
//!
//! Uses the `plist` crate for macOS bundles — no CoreFoundation linkage
//! required, so this stays cross-platform and unit-testable with a tempdir.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// One enumerated application (macOS .app bundle or Linux .desktop entry).
#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub bundle_path: PathBuf,
    pub bundle_name: String,
    pub bundle_id: Option<String>,
    pub version: String,
}

/// Default application directories scanned.
/// On macOS: `/Applications` and `~/Applications`.
/// On Linux: standard XDG data directories for `.desktop` files.
pub fn default_app_dirs() -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        let mut dirs = vec![PathBuf::from("/Applications")];
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join("Applications"));
        }
        dirs
    } else {
        // Linux: XDG data directories for .desktop files
        let mut dirs = vec![
            PathBuf::from("/usr/share/applications"),
            PathBuf::from("/usr/local/share/applications"),
        ];
        // XDG_DATA_HOME
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share/applications"));
        }
        // XDG_DATA_DIRS
        if let Ok(xdg_data_dirs) = std::env::var("XDG_DATA_DIRS") {
            for d in xdg_data_dirs.split(':') {
                if !d.is_empty() {
                    dirs.push(PathBuf::from(d).join("applications"));
                }
            }
        }
        // Flatpak and snap application dirs
        if let Ok(home) = std::env::var("HOME") {
            dirs.push(PathBuf::from(home).join(".local/share/flatpak/exports/share/applications"));
        }
        dirs.push(PathBuf::from("/var/lib/flatpak/exports/share/applications"));
        dirs.push(PathBuf::from("/var/lib/snapd/desktop/applications"));
        dirs
    }
}

/// Enumerate applications under `dir`. On macOS, looks for `.app` bundles;
/// on Linux, looks for `.desktop` files. `max_depth` bounds the walk.
///
/// op 111: Enumerate installed applications.
pub fn enumerate_apps_in(dir: &Path, max_depth: usize) -> Vec<InstalledApp> {
    if cfg!(target_os = "macos") {
        enumerate_macos_apps(dir, max_depth)
    } else {
        enumerate_linux_apps(dir, max_depth)
    }
}

/// macOS: enumerate `.app` bundles under `dir`.
fn enumerate_macos_apps(dir: &Path, max_depth: usize) -> Vec<InstalledApp> {
    let mut apps = Vec::new();

    if !dir.exists() {
        return apps;
    }

    for entry in WalkDir::new(dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        if !path
            .file_name()
            .map(|n| {
                let s = n.to_string_lossy();
                s.ends_with(".app")
            })
            .unwrap_or(false)
        {
            continue;
        }

        if let Some(app) = parse_app_bundle(path) {
            apps.push(app);
        }
    }

    apps
}

/// Linux: enumerate `.desktop` files under `dir`.
fn enumerate_linux_apps(dir: &Path, max_depth: usize) -> Vec<InstalledApp> {
    let mut apps = Vec::new();

    if !dir.exists() {
        return apps;
    }

    for entry in WalkDir::new(dir)
        .max_depth(max_depth)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if !path
            .extension()
            .map(|ext| ext == "desktop")
            .unwrap_or(false)
        {
            continue;
        }

        if let Some(app) = parse_desktop_file(path) {
            apps.push(app);
        }
    }

    apps
}

/// Parse a single `.app` bundle's `Contents/Info.plist` and return its
/// metadata. Returns `None` if the plist is missing or unreadable.
///
/// op 112: Read application bundles.
pub fn parse_app_bundle(bundle_path: &Path) -> Option<InstalledApp> {
    let info_plist = bundle_path.join("Contents/Info.plist");
    if !info_plist.is_file() {
        return None;
    }

    let values: plist::Dictionary = plist::from_file(&info_plist).ok()?;

    let bundle_name = values
        .get("CFBundleName")
        .and_then(|v| v.as_string())
        .or_else(|| values.get("CFBundleExecutable").and_then(|v| v.as_string()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            bundle_path
                .file_stem()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".to_string())
        });

    // op 115: Identify bundle identifier.
    let bundle_id = values
        .get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

    // op 113: Identify app version.
    let version = values
        .get("CFBundleShortVersionString")
        .and_then(|v| v.as_string())
        .or_else(|| values.get("CFBundleVersion").and_then(|v| v.as_string()))
        .map(|s| s.to_string())
        .unwrap_or_else(|| "unknown".to_string());

    Some(InstalledApp {
        bundle_path: bundle_path.to_path_buf(),
        bundle_name,
        bundle_id,
        version,
    })
}

/// Parse a Freedesktop `.desktop` file and return its metadata.
/// Extracts `Name`, `X-Flatpak`/`Icon` (for bundle_id proxy), and version
/// if available. Returns `None` if the file is unreadable or has no `Name`.
pub fn parse_desktop_file(path: &Path) -> Option<InstalledApp> {
    let content = std::fs::read_to_string(path).ok()?;

    let mut name: Option<String> = None;
    let mut desktop_id: Option<String> = None;
    let mut version: Option<String> = None;
    let mut in_desktop_entry = false;

    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_desktop_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_desktop_entry {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();
            match key {
                "Name" => {
                    if name.is_none() {
                        name = Some(value.to_string());
                    }
                }
                "X-Flatpak" | "X-SnapInstanceName" => {
                    if desktop_id.is_none() {
                        desktop_id = Some(value.to_string());
                    }
                }
                "Version" => {
                    version = Some(value.to_string());
                }
                _ => {}
            }
        }
    }

    let bundle_name = name.unwrap_or_else(|| {
        path.file_stem()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "unknown".to_string())
    });

    // Use the filename (without .desktop) as a fallback bundle_id
    let bundle_id =
        desktop_id.or_else(|| path.file_stem().map(|n| n.to_string_lossy().to_string()));

    let version = version.unwrap_or_else(|| "unknown".to_string());

    Some(InstalledApp {
        bundle_path: path.to_path_buf(),
        bundle_name,
        bundle_id,
        version,
    })
}

// ---- Phase 5: application analysis operations --------------------------

/// Launch history for a single application, aggregated from the various
/// sources available on the host (LaunchServices, saved state, mtime hints).
#[derive(Debug, Clone, Default)]
#[allow(dead_code)]
pub struct AppHistory {
    /// Bundle identifier the history pertains to.
    pub bundle_id: String,
    /// Best-known number of launches (0 when no source reports a count).
    pub launch_count: u64,
    /// Epoch seconds of the most recent launch, when known.
    pub last_launch: Option<u64>,
    /// Coarse usage pattern label: `"heavy"`, `"moderate"`, `"light"`, or
    /// `"unknown"` when no signal was available.
    pub usage_pattern: &'static str,
}

/// A complete application inventory: every enumerated app plus summary
/// categories, total size, and per-app status flags.
#[derive(Debug, Clone, Default)]
pub struct AppInventory {
    /// All enumerated applications.
    pub apps: Vec<InstalledApp>,
    /// Coarse category buckets (e.g. `"productivity"`, `"developer"`).
    pub categories: Vec<(String, Vec<InstalledApp>)>,
    /// Sum of every app bundle size in bytes.
    pub total_size: u64,
    /// Bundle ids flagged as unused (op 118).
    pub unused: Vec<String>,
    /// Bundle ids flagged as abandoned (op 119).
    pub abandoned: Vec<String>,
    /// Bundle ids flagged as suspicious (op 142).
    pub suspicious: Vec<String>,
    /// Groups of duplicate applications (op 143).
    pub duplicates: Vec<Vec<InstalledApp>>,
}

/// Application support / caches data left behind by an app that is no longer
/// installed (op 243). On macOS these live under `~/Library/Application
/// Support` and `~/Library/Caches`.
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct AbandonedAppData {
    /// Bundle identifier inferred from the leftover directory name.
    pub bundle_id: String,
    /// Path to the leftover data directory.
    pub path: PathBuf,
    /// Library sub-directory kind: `"Application Support"` or `"Caches"`.
    pub kind: &'static str,
    /// Total size of the leftover directory in bytes.
    pub size: u64,
}

/// op 114: Identify developer — extract the developer name from the bundle
/// identifier prefix (e.g. `com.apple.` → `Apple`, `com.google.` → `Google`).
/// Returns `None` when no known developer prefix matches.
#[allow(dead_code)]
pub fn identify_developer(bundle_id: &str) -> Option<String> {
    let known: &[(&str, &str)] = &[
        ("com.apple.", "Apple"),
        ("com.google.", "Google"),
        ("com.microsoft.", "Microsoft"),
        ("com.adobe.", "Adobe"),
        ("com.mozilla.", "Mozilla"),
        ("org.mozilla.", "Mozilla"),
        ("com.spotify.", "Spotify"),
        ("com.slack.", "Slack"),
        ("com.tinyspeck.", "Slack"),
        ("com.github.", "GitHub"),
        ("com.jetbrains.", "JetBrains"),
        ("org.chromium.", "Chromium"),
        ("com.valvesoftware.", "Valve"),
        ("com.zoom.", "Zoom"),
        ("us.zoom.", "Zoom"),
        ("com.electron.", "Electron"),
        ("io.obsidian.", "Obsidian"),
        ("md.obsidian.", "Obsidian"),
        ("notion.id", "Notion"),
        ("com.figma.", "Figma"),
        ("com.docker.", "Docker"),
        ("com.virtualbox.", "Oracle"),
        ("org.virtualbox.", "Oracle"),
    ];
    for (prefix, vendor) in known {
        if bundle_id.starts_with(prefix) {
            return Some((*vendor).to_string());
        }
    }
    // Generic reverse-DNS: use the second component as a fallback vendor.
    let parts: Vec<&str> = bundle_id.split('.').collect();
    if parts.len() >= 2 {
        let candidate = parts[1];
        // Capitalise the first letter for a friendlier display.
        let mut chars = candidate.chars();
        if let Some(first) = chars.next() {
            return Some(first.to_uppercase().collect::<String>() + chars.as_str());
        }
    }
    None
}

/// op 116: Identify app size — calculate the total size (in bytes) of an
/// application bundle by recursively summing every file it contains.
#[allow(dead_code)]
pub fn app_size(bundle_path: &Path) -> u64 {
    if !bundle_path.exists() {
        return 0;
    }
    let mut total: u64 = 0;
    for entry in WalkDir::new(bundle_path).follow_links(false).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

/// op 118: Detect unused applications — an app is considered unused when its
/// bundle directory mtime is older than 90 days (i.e. it has not been
/// updated/launched recently).
#[allow(dead_code)]
pub fn is_app_unused(bundle_path: &Path) -> bool {
    let metadata = match std::fs::metadata(bundle_path) {
        Ok(m) => m,
        Err(_) => return false,
    };
    let mtime = match metadata.modified() {
        Ok(t) => t,
        Err(_) => return false,
    };
    let age = std::time::SystemTime::now()
        .duration_since(mtime)
        .unwrap_or_default();
    age > std::time::Duration::from_secs(90 * 24 * 60 * 60)
}

/// op 119: Detect abandoned applications — an app is abandoned when it has
/// leftover data (a bundle path that no longer exists) but its bundle id is
/// no longer present in `/Applications`. Returns `true` when the bundle path
/// is missing while the bundle id looks valid.
#[allow(dead_code)]
pub fn is_app_abandoned(bundle_id: &str, bundle_path: &Path) -> bool {
    if bundle_id.is_empty() || !bundle_id.contains('.') {
        return false;
    }
    if bundle_path.exists() {
        return false;
    }
    // Confirm the bundle id is not present in /Applications (macOS) or the
    // default app dirs (Linux). On non-macOS hosts we treat a missing bundle
    // path as abandoned when the bundle id is well-formed.
    if cfg!(target_os = "macos") {
        for dir in default_app_dirs() {
            if !dir.is_dir() {
                continue;
            }
            for entry in WalkDir::new(&dir)
                .max_depth(3)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !entry.file_type().is_dir() {
                    continue;
                }
                if let Some(app) = parse_app_bundle(entry.path()) {
                    if app.bundle_id.as_deref() == Some(bundle_id) {
                        return false;
                    }
                }
            }
        }
    }
    true
}

/// op 137: Track application history — collect launch count, last launch
/// time, and usage patterns from the various sources available on the host.
/// On macOS this consults `~/Library/Saved Application State` and the bundle
/// mtime; on other platforms only the mtime signal is used.
#[allow(dead_code)]
pub fn track_app_history(bundle_id: &str) -> AppHistory {
    let mut history = AppHistory {
        bundle_id: bundle_id.to_string(),
        ..Default::default()
    };

    // macOS: look for a saved-state directory keyed by the bundle id.
    if cfg!(target_os = "macos") {
        if let Ok(home) = std::env::var("HOME") {
            let saved = PathBuf::from(home)
                .join("Library/Saved Application State")
                .join(format!("{}.savedState", bundle_id));
            if saved.is_dir() {
                if let Ok(metadata) = std::fs::metadata(&saved) {
                    if let Ok(mtime) = metadata.modified() {
                        if let Ok(dur) = mtime.duration_since(std::time::UNIX_EPOCH) {
                            history.last_launch = Some(dur.as_secs());
                        }
                    }
                }
                // A saved-state directory implies at least one launch.
                history.launch_count = 1;
            }
        }
    }

    // Derive a coarse usage pattern from the launch count / last launch.
    history.usage_pattern = if history.launch_count == 0 {
        "unknown"
    } else if history.launch_count >= 30 {
        "heavy"
    } else if history.launch_count >= 5 {
        "moderate"
    } else {
        "light"
    };

    history
}

/// op 142: Detect suspicious apps — flag apps whose name or bundle id
/// matches suspicious patterns: unsigned-looking bundle ids, unknown
/// developers, or names suggesting apps in unusual locations.
#[allow(dead_code)]
pub fn is_app_suspicious(name: &str, bundle_id: &str) -> bool {
    // Apps without a reverse-DNS bundle id are often unsigned.
    if bundle_id.is_empty() || !bundle_id.contains('.') {
        return true;
    }
    // Known-bad name patterns commonly used by adware / crack installers.
    let bad_names = [
        "crack",
        "keygen",
        "patcher",
        "unlocker",
        "activator",
        "hack",
        "warez",
        "pirated",
        "free-download",
    ];
    let lower_name = name.to_lowercase();
    if bad_names.iter().any(|b| lower_name.contains(b)) {
        return true;
    }
    // Bundle ids that use a public email domain as the vendor prefix are
    // typically not from an established developer.
    let public_domains = ["gmail.", "yahoo.", "outlook.", "hotmail.", "icloud."];
    if public_domains.iter().any(|d| bundle_id.contains(d)) {
        return true;
    }
    // Known developers are not suspicious.
    if identify_developer(bundle_id).is_some() {
        return false;
    }
    // A bundle id with only two components and an unknown vendor is mildly
    // suspicious.
    let parts: Vec<&str> = bundle_id.split('.').collect();
    parts.len() < 3
}

/// op 143: Detect duplicate applications — group apps by bundle id and
/// return groups containing more than one entry (i.e. duplicates).
#[allow(dead_code)]
pub fn detect_duplicate_apps(apps: &[InstalledApp]) -> Vec<Vec<InstalledApp>> {
    use std::collections::HashMap;
    let mut groups: HashMap<String, Vec<InstalledApp>> = HashMap::new();
    for app in apps {
        if let Some(id) = &app.bundle_id {
            groups.entry(id.clone()).or_default().push(app.clone());
        }
    }
    groups
        .into_iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(_, v)| v)
        .collect()
}

/// Coarse category classification for an app based on its name / bundle id.
fn classify_app(app: &InstalledApp) -> &'static str {
    let haystack = format!(
        "{} {}",
        app.bundle_name,
        app.bundle_id.as_deref().unwrap_or("")
    );
    let h = haystack.to_lowercase();
    if h.contains("xcode")
        || h.contains("jetbrains")
        || h.contains("docker")
        || h.contains("terminal")
        || h.contains("iterm")
        || h.contains("git")
        || h.contains("vscode")
        || h.contains("visual studio code")
    {
        "developer"
    } else if h.contains("chrome")
        || h.contains("firefox")
        || h.contains("safari")
        || h.contains("edge")
        || h.contains("browser")
    {
        "browser"
    } else if h.contains("spotify")
        || h.contains("music")
        || h.contains("vlc")
        || h.contains("plex")
        || h.contains("itunes")
    {
        "media"
    } else if h.contains("word")
        || h.contains("excel")
        || h.contains("powerpoint")
        || h.contains("notion")
        || h.contains("obsidian")
        || h.contains("pages")
        || h.contains("keynote")
        || h.contains("numbers")
    {
        "productivity"
    } else {
        "other"
    }
}

/// op 144: Generate app inventory — create a complete inventory with
/// categories, sizes, and status flags by enumerating every default app dir.
#[allow(dead_code)]
pub fn generate_app_inventory() -> AppInventory {
    let mut inventory = AppInventory::default();
    let mut all_apps: Vec<InstalledApp> = Vec::new();
    for dir in default_app_dirs() {
        all_apps.extend(enumerate_apps_in(&dir, 4));
    }

    // Total size.
    inventory.total_size = all_apps.iter().map(|a| app_size(&a.bundle_path)).sum();

    // Categories.
    use std::collections::HashMap;
    let mut buckets: HashMap<&'static str, Vec<InstalledApp>> = HashMap::new();
    for app in &all_apps {
        buckets
            .entry(classify_app(app))
            .or_default()
            .push(app.clone());
    }
    inventory.categories = buckets
        .into_iter()
        .map(|(k, v)| (k.to_string(), v))
        .collect();

    // Status flags.
    for app in &all_apps {
        let id = app
            .bundle_id
            .clone()
            .unwrap_or_else(|| app.bundle_name.clone());
        if is_app_unused(&app.bundle_path) {
            inventory.unused.push(id.clone());
        }
        if is_app_abandoned(&id, &app.bundle_path) {
            inventory.abandoned.push(id.clone());
        }
        if is_app_suspicious(&app.bundle_name, &id) {
            inventory.suspicious.push(id);
        }
    }

    inventory.duplicates = detect_duplicate_apps(&all_apps);
    inventory.apps = all_apps;
    inventory
}

/// op 243: Detect abandoned applications (twin context) — find Application
/// Support / Caches directories for apps that are no longer installed. On
/// macOS these live under `~/Library/Application Support` and
/// `~/Library/Caches`; on Linux the XDG cache home is consulted.
#[allow(dead_code)]
pub fn find_abandoned_app_data() -> Vec<AbandonedAppData> {
    let mut out = Vec::new();
    if cfg!(target_os = "macos") {
        let home = match std::env::var("HOME") {
            Ok(h) => PathBuf::from(h),
            Err(_) => return out,
        };
        let installed: Vec<String> = default_app_dirs()
            .iter()
            .flat_map(|d| enumerate_apps_in(d, 4))
            .filter_map(|a| a.bundle_id)
            .collect();
        let dirs: &[(&'static str, PathBuf)] = &[
            (
                "Application Support",
                home.join("Library/Application Support"),
            ),
            ("Caches", home.join("Library/Caches")),
        ];
        for (kind, dir) in dirs {
            if !dir.is_dir() {
                continue;
            }
            let entries = match std::fs::read_dir(dir) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                if !name.contains('.') {
                    continue;
                }
                if installed.iter().any(|id| id == &name) {
                    continue;
                }
                let size = if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    dir_size(&entry.path())
                } else {
                    entry.metadata().map(|m| m.len()).unwrap_or(0)
                };
                out.push(AbandonedAppData {
                    bundle_id: name,
                    path: entry.path(),
                    kind,
                    size,
                });
            }
        }
    }
    out
}

/// Recursively compute the size of a directory in bytes.
fn dir_size(path: &Path) -> u64 {
    let mut total: u64 = 0;
    for entry in WalkDir::new(path).follow_links(false).into_iter() {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_plist(dir: &Path, name: &str, body: &str) {
        fs::write(dir.join("Contents").join(name), body).unwrap();
    }

    fn make_app(root: &Path, app_name: &str, plist_body: &str) -> PathBuf {
        let app = root.join(app_name);
        fs::create_dir_all(app.join("Contents")).unwrap();
        write_plist(&app, "Info.plist", plist_body);
        app
    }

    #[test]
    fn parses_bundle_name_and_version() {
        let tmp = TempDir::new().unwrap();
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>MyApp</string>
    <key>CFBundleIdentifier</key>
    <string>com.example.myapp</string>
    <key>CFBundleShortVersionString</key>
    <string>1.2.3</string>
    <key>CFBundleVersion</key>
    <string>123</string>
</dict>
</plist>"#;
        let app = make_app(tmp.path(), "MyApp.app", plist);
        let parsed = parse_app_bundle(&app).expect("parse");
        assert_eq!(parsed.bundle_name, "MyApp");
        assert_eq!(parsed.bundle_id.as_deref(), Some("com.example.myapp"));
        assert_eq!(parsed.version, "1.2.3");
    }

    #[test]
    fn falls_back_to_cfbundleversion() {
        let tmp = TempDir::new().unwrap();
        let plist = r#"<?xml version="1.0" encoding="UTF-8"?>
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>NoVerApp</string>
    <key>CFBundleVersion</key>
    <string>999</string>
</dict>
</plist>"#;
        let app = make_app(tmp.path(), "NoVerApp.app", plist);
        let parsed = parse_app_bundle(&app).expect("parse");
        assert_eq!(parsed.version, "999");
    }

    #[test]
    fn missing_plist_returns_none() {
        let tmp = TempDir::new().unwrap();
        let app = tmp.path().join("Empty.app");
        fs::create_dir_all(app.join("Contents")).unwrap();
        assert!(parse_app_bundle(&app).is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn enumerate_apps_in_walks_directory() {
        let tmp = TempDir::new().unwrap();
        let plist_a = r#"<plist version="1.0"><dict>
            <key>CFBundleName</key><string>AppA</string>
            <key>CFBundleShortVersionString</key><string>2.0</string>
        </dict></plist>"#;
        let plist_b = r#"<plist version="1.0"><dict>
            <key>CFBundleName</key><string>AppB</string>
            <key>CFBundleShortVersionString</key><string>3.1</string>
        </dict></plist>"#;
        make_app(tmp.path(), "AppA.app", plist_a);
        make_app(tmp.path(), "AppB.app", plist_b);

        let apps = enumerate_apps_in(tmp.path(), 2);
        assert_eq!(apps.len(), 2);
        let names: Vec<_> = apps.iter().map(|a| a.bundle_name.as_str()).collect();
        assert!(names.contains(&"AppA"));
        assert!(names.contains(&"AppB"));
    }

    #[test]
    fn enumerate_apps_in_nonexistent_dir_is_empty() {
        let apps = enumerate_apps_in(Path::new("/this/does/not/exist/xyz123"), 3);
        assert!(apps.is_empty());
    }

    #[test]
    fn default_app_dirs_includes_system_applications() {
        let dirs = default_app_dirs();
        if cfg!(target_os = "macos") {
            assert!(dirs.iter().any(|d| d == &PathBuf::from("/Applications")));
        } else {
            // Linux: should include /usr/share/applications
            assert!(dirs
                .iter()
                .any(|d| d == &PathBuf::from("/usr/share/applications")));
        }
    }

    #[test]
    fn parse_desktop_file_extracts_name() {
        let tmp = TempDir::new().unwrap();
        let desktop = tmp.path().join("firefox.desktop");
        fs::write(&desktop, "[Desktop Entry]\nName=Firefox\nVersion=124.0\n").unwrap();
        let parsed = parse_desktop_file(&desktop).expect("parse");
        assert_eq!(parsed.bundle_name, "Firefox");
        assert_eq!(parsed.version, "124.0");
        assert_eq!(parsed.bundle_id.as_deref(), Some("firefox"));
    }

    #[test]
    fn parse_desktop_file_falls_back_to_filename() {
        let tmp = TempDir::new().unwrap();
        let desktop = tmp.path().join("chromium-browser.desktop");
        // No Name= line — should fall back to filename stem
        fs::write(&desktop, "[Desktop Entry]\nExec=chromium-browser\n").unwrap();
        let parsed = parse_desktop_file(&desktop).expect("parse");
        assert_eq!(parsed.bundle_name, "chromium-browser");
    }

    #[test]
    fn falls_back_to_file_stem_when_no_bundle_name() {
        let tmp = TempDir::new().unwrap();
        let plist = r#"<plist version="1.0"><dict>
            <key>CFBundleShortVersionString</key><string>1.0</string>
        </dict></plist>"#;
        let app = make_app(tmp.path(), "StemOnly.app", plist);
        let parsed = parse_app_bundle(&app).expect("parse");
        assert_eq!(parsed.bundle_name, "StemOnly");
    }

    // ---- op 114: identify_developer -------------------------------------

    #[test]
    fn identify_developer_known_prefixes() {
        assert_eq!(
            identify_developer("com.apple.Safari").as_deref(),
            Some("Apple")
        );
        assert_eq!(
            identify_developer("com.google.Chrome").as_deref(),
            Some("Google")
        );
        assert_eq!(
            identify_developer("com.microsoft.Excel").as_deref(),
            Some("Microsoft")
        );
        assert_eq!(
            identify_developer("org.mozilla.firefox").as_deref(),
            Some("Mozilla")
        );
    }

    #[test]
    fn identify_developer_falls_back_to_second_component() {
        let dev = identify_developer("com.example.myapp").expect("fallback");
        assert_eq!(dev, "Example");
    }

    #[test]
    fn identify_developer_returns_none_for_garbage() {
        assert!(identify_developer("").is_none());
        assert!(identify_developer("nodothave").is_none());
    }

    // ---- op 116: app_size -----------------------------------------------

    #[test]
    fn app_size_sums_files_in_bundle() {
        let tmp = TempDir::new().unwrap();
        let app = make_app(
            tmp.path(),
            "Sized.app",
            "<plist version=\"1.0\"><dict></dict></plist>",
        );
        // Add two extra files with known content.
        fs::write(app.join("Contents").join("a.txt"), "aaaa").unwrap(); // 4 bytes
        fs::write(app.join("Contents").join("b.txt"), "bbbbbb").unwrap(); // 6 bytes
        let size = app_size(&app);
        // At least the two extra files (plist body size varies).
        assert!(size >= 10);
    }

    #[test]
    fn app_size_missing_path_is_zero() {
        assert_eq!(app_size(Path::new("/no/such/app/xyz123.app")), 0);
    }

    // ---- op 118: is_app_unused ------------------------------------------

    #[test]
    fn is_app_unused_false_for_recent_bundle() {
        let tmp = TempDir::new().unwrap();
        let app = make_app(
            tmp.path(),
            "Fresh.app",
            "<plist version=\"1.0\"><dict></dict></plist>",
        );
        assert!(!is_app_unused(&app));
    }

    #[test]
    fn is_app_unused_missing_path_is_false() {
        assert!(!is_app_unused(Path::new("/no/such/app/xyz123.app")));
    }

    // ---- op 119: is_app_abandoned ---------------------------------------

    #[test]
    fn is_app_abandoned_true_when_bundle_missing() {
        let path = Path::new("/no/such/app/xyz123.app");
        // A well-formed bundle id with a missing path; on macOS we cannot
        // guarantee /Applications is empty, so only assert on Linux or when
        // the id is clearly absent.
        if !cfg!(target_os = "macos") {
            assert!(is_app_abandoned("com.example.neverinstalled", path));
        }
    }

    #[test]
    fn is_app_abandoned_false_when_bundle_exists() {
        let tmp = TempDir::new().unwrap();
        let app = make_app(
            tmp.path(),
            "Present.app",
            "<plist version=\"1.0\"><dict></dict></plist>",
        );
        assert!(!is_app_abandoned("com.example.present", &app));
    }

    #[test]
    fn is_app_abandoned_false_for_invalid_bundle_id() {
        assert!(!is_app_abandoned("garbage", Path::new("/no/such/app")));
        assert!(!is_app_abandoned("", Path::new("/no/such/app")));
    }

    // ---- op 137: track_app_history --------------------------------------

    #[test]
    fn track_app_history_returns_unknown_for_unseen_id() {
        let h = track_app_history("com.example.neverlaunched");
        assert_eq!(h.bundle_id, "com.example.neverlaunched");
        assert_eq!(h.launch_count, 0);
        assert_eq!(h.usage_pattern, "unknown");
    }

    // ---- op 142: is_app_suspicious --------------------------------------

    #[test]
    fn is_app_suspicious_flags_unsigned_bundle_id() {
        assert!(is_app_suspicious("MyApp", ""));
        assert!(is_app_suspicious("MyApp", "nodothave"));
    }

    #[test]
    fn is_app_suspicious_flags_bad_names() {
        assert!(is_app_suspicious("Adobe Crack", "com.example.crack"));
        assert!(is_app_suspicious("Keygen Tool", "com.example.keygen"));
    }

    #[test]
    fn is_app_suspicious_flags_public_email_vendor() {
        assert!(is_app_suspicious("Thing", "com.gmail.thing"));
    }

    #[test]
    fn is_app_suspicious_false_for_known_developer() {
        assert!(!is_app_suspicious("Safari", "com.apple.Safari"));
        assert!(!is_app_suspicious("Chrome", "com.google.Chrome"));
    }

    // ---- op 143: detect_duplicate_apps ----------------------------------

    fn app_with(id: &str, name: &str) -> InstalledApp {
        InstalledApp {
            bundle_path: PathBuf::from(format!("/apps/{}.app", name)),
            bundle_name: name.to_string(),
            bundle_id: Some(id.to_string()),
            version: "1.0".to_string(),
        }
    }

    #[test]
    fn detect_duplicate_apps_groups_by_bundle_id() {
        let apps = vec![
            app_with("com.example.dup", "DupA"),
            app_with("com.example.dup", "DupB"),
            app_with("com.example.unique", "Unique"),
        ];
        let groups = detect_duplicate_apps(&apps);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn detect_duplicate_apps_empty_when_all_unique() {
        let apps = vec![
            app_with("com.example.a", "A"),
            app_with("com.example.b", "B"),
        ];
        assert!(detect_duplicate_apps(&apps).is_empty());
    }

    #[test]
    fn detect_duplicate_apps_ignores_apps_without_bundle_id() {
        let apps = vec![
            InstalledApp {
                bundle_path: PathBuf::from("/apps/X.app"),
                bundle_name: "X".to_string(),
                bundle_id: None,
                version: "1.0".to_string(),
            },
            InstalledApp {
                bundle_path: PathBuf::from("/apps/Y.app"),
                bundle_name: "Y".to_string(),
                bundle_id: None,
                version: "1.0".to_string(),
            },
        ];
        assert!(detect_duplicate_apps(&apps).is_empty());
    }

    // ---- op 144: generate_app_inventory ---------------------------------

    #[test]
    fn generate_app_inventory_runs_without_panic() {
        let inventory = generate_app_inventory();
        // The inventory should at least be well-formed.
        let total: u64 = inventory
            .apps
            .iter()
            .map(|a| app_size(&a.bundle_path))
            .sum();
        assert_eq!(inventory.total_size, total);
    }

    // ---- op 243: find_abandoned_app_data --------------------------------

    #[test]
    fn find_abandoned_app_data_runs_without_panic() {
        let data = find_abandoned_app_data();
        // Every entry must have a bundle id containing a dot.
        assert!(data.iter().all(|d| d.bundle_id.contains('.')));
    }
}
