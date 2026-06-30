//! Installed application enumeration from `/Applications` and
//! `~/Applications`.
//!
//! Walks each Applications directory for `*.app` bundles, reads each bundle's
//! `Contents/Info.plist`, and extracts the bundle name
//! (`CFBundleName` / `CFBundleExecutable`) and version
//! (`CFBundleShortVersionString`, falling back to `CFBundleVersion`). Uses the
//! `plist` crate — no CoreFoundation linkage required, so this stays
//! cross-platform and unit-testable with a tempdir.

use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// One enumerated macOS application bundle.
#[derive(Debug, Clone)]
pub struct InstalledApp {
    pub bundle_path: PathBuf,
    pub bundle_name: String,
    pub bundle_id: Option<String>,
    pub version: String,
}

/// Default Applications directories scanned on macOS.
pub fn default_app_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![PathBuf::from("/Applications")];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(home).join("Applications"));
    }
    dirs
}

/// Enumerate `.app` bundles under `dir`. `max_depth` bounds the walk so a
/// deeply nested Applications folder stays fast. Returns one `InstalledApp`
/// per bundle whose `Info.plist` could be parsed; bundles missing a plist are
/// skipped silently.
pub fn enumerate_apps_in(dir: &Path, max_depth: usize) -> Vec<InstalledApp> {
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

/// Parse a single `.app` bundle's `Contents/Info.plist` and return its
/// metadata. Returns `None` if the plist is missing or unreadable.
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

    let bundle_id = values
        .get("CFBundleIdentifier")
        .and_then(|v| v.as_string())
        .map(|s| s.to_string());

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
        assert!(dirs.iter().any(|d| d == &PathBuf::from("/Applications")));
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
}
