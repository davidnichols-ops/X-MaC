use anyhow::{bail, Result};
use serde_json::json;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::cli::args::{Cli, OutputFormat, UninstallArgs};
use crate::cli::output::OutputWriter;
use crate::core::types::{Category, EngineId, Finding, Severity, Target};
use crate::engines::envmap::apps::{default_app_dirs, enumerate_apps_in, InstalledApp};
use crate::util::disk::{dir_size, file_size_physical, format_bytes};

/// Run the `uninstall` command: locate an application bundle and its
/// associated user-data leftovers, preview them, and optionally move them
/// to the Trash.
pub async fn run_uninstall(cli: &Cli, args: &UninstallArgs) -> Result<()> {
    let findings = find_uninstall_targets(&args.app, args.app_dir.first())?;
    if findings.is_empty() {
        bail!("No application or leftovers found matching '{}'", args.app);
    }

    if cli.global.format == OutputFormat::Report {
        print_human_report(&findings, &args.app);
    } else {
        let mut writer = OutputWriter::new(&cli.global);
        for finding in &findings {
            writer.write_finding(finding)?;
        }
        writer.flush()?;
    }

    if let Some(path) = &cli.global.fix_script {
        let generator = crate::cli::FixScriptGenerator::new(path.clone());
        let written = generator.write(&findings)?;
        if !cli.global.quiet {
            eprintln!("Wrote uninstall script to {}", written.display());
        }
    }

    if args.yes {
        let policy = crate::cleanup::CleanupPolicy::uninstall();
        let mut executor = crate::cleanup::CleanupExecutor::new(policy, false);
        let plan = executor.plan(&findings);
        let transaction = executor.execute(&plan);

        if cli.global.format == OutputFormat::Json || cli.global.format == OutputFormat::JsonPretty
        {
            let json = match cli.global.format {
                OutputFormat::JsonPretty => serde_json::to_string_pretty(&transaction)?,
                _ => serde_json::to_string(&transaction)?,
            };
            println!("{}", json);
        } else if !cli.global.quiet {
            eprintln!("Uninstall transaction: {}", transaction.id);
            eprintln!("  Succeeded: {}", transaction.successful_count());
            eprintln!(
                "  Failed:    {}",
                transaction.actions.len() - transaction.successful_count()
            );
            eprintln!(
                "  Reclaimed: {}",
                format_bytes(transaction.successful_bytes())
            );
        }
    } else if !cli.global.quiet {
        eprintln!("Dry run: pass --yes to move the listed items to Trash.");
    }

    Ok(())
}

/// Find the application bundle matching `query` plus any user-data leftovers
/// (caches, preferences, saved state, etc.) associated with it.
pub fn find_uninstall_targets(
    query: &str,
    extra_app_dir: Option<&PathBuf>,
) -> Result<Vec<Finding>> {
    let home = crate::util::macos::MacosUtils::home_dir();
    let mut app_dirs = default_app_dirs();
    if let Some(dir) = extra_app_dir {
        app_dirs.push(dir.clone());
    }
    find_uninstall_targets_in(&home, &app_dirs, query)
}

fn find_uninstall_targets_in(
    home: &Path,
    app_dirs: &[PathBuf],
    query: &str,
) -> Result<Vec<Finding>> {
    let query_lower = query.to_lowercase();

    // 1. Locate the application bundle.
    let app = select_app(&query_lower, app_dirs)?;
    let needles = app_needles(&app);

    // 2. Build the list of user-data locations to scan.
    let mut findings = Vec::new();
    let bundle_size = dir_size(&app.bundle_path);
    findings.push(
        Finding::new(
            EngineId::Clean,
            Severity::High,
            Category::InstalledApp,
            Target::Path(app.bundle_path.clone()),
            "Application bundle",
            format!(
                "{} ({}) at {}",
                app.bundle_name,
                app.bundle_id.as_deref().unwrap_or("unknown bundle id"),
                app.bundle_path.display()
            ),
        )
        .with_size(bundle_size)
        .with_metadata("bundle_id".to_string(), json!(app.bundle_id))
        .with_metadata("version".to_string(), json!(app.version))
        .with_hint("Move the .app bundle to Trash to complete the uninstall.".to_string()),
    );

    // 3. Scan standard macOS (and Linux XDG) leftover locations.
    let leftover_locations: &[(&str, &str, Category, Severity)] = &[
        (
            "Library/Application Support",
            "application support",
            Category::OrphanFile,
            Severity::Medium,
        ),
        (
            "Library/Caches",
            "cache directory",
            Category::Cache,
            Severity::Low,
        ),
        (
            "Library/Logs",
            "log directory",
            Category::Log,
            Severity::Low,
        ),
        (
            "Library/Containers",
            "sandbox container",
            Category::OrphanFile,
            Severity::Medium,
        ),
        (
            "Library/Group Containers",
            "group container",
            Category::OrphanFile,
            Severity::Medium,
        ),
        (
            "Library/Application Scripts",
            "application script",
            Category::OrphanFile,
            Severity::Medium,
        ),
        (
            "Library/HTTPStorages",
            "HTTP storage",
            Category::OrphanFile,
            Severity::Medium,
        ),
        (
            "Library/WebKit",
            "WebKit storage",
            Category::OrphanFile,
            Severity::Medium,
        ),
        (
            "Library/Preferences",
            "preference plist",
            Category::OrphanFile,
            Severity::Medium,
        ),
        (
            "Library/Saved Application State",
            "saved application state",
            Category::OrphanFile,
            Severity::Medium,
        ),
        (
            "Library/LaunchAgents",
            "launch agent plist",
            Category::OrphanFile,
            Severity::Medium,
        ),
        // Cross-platform XDG locations for user data.
        (
            ".config",
            "config directory",
            Category::OrphanFile,
            Severity::Medium,
        ),
        (".cache", "cache directory", Category::Cache, Severity::Low),
        (
            ".local/share",
            "local share directory",
            Category::OrphanFile,
            Severity::Medium,
        ),
    ];

    for (rel, desc, category, severity) in leftover_locations {
        let dir = home.join(rel);
        if !dir.is_dir() {
            continue;
        }

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            if name.is_empty() {
                continue;
            }

            // Preferences and LaunchAgents are files; Saved Application State
            // directories have a .savedState extension. Use the stem for
            // matching so bundle IDs like com.example.myapp are compared
            // without the plist/savedState suffix.
            let candidate = if rel.ends_with("Preferences") || rel.ends_with("LaunchAgents") {
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| name.clone())
            } else if rel.ends_with("Saved Application State") {
                path.file_stem()
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_else(|| name.clone())
            } else {
                name.clone()
            };

            if !matches_identifier(&candidate, &needles) && !matches_identifier(&name, &needles) {
                continue;
            }

            let size = if path.is_dir() {
                dir_size(&path)
            } else {
                file_size_physical(&path)
            };

            if size == 0 && path.is_dir() {
                // Empty directory; still report it so it can be removed, but
                // keep the size at zero.
            }

            findings.push(
                Finding::new(
                    EngineId::Clean,
                    *severity,
                    *category,
                    Target::Path(path),
                    format!("{} leftover", desc),
                    format!("Found {} for {}: {}", desc, app.bundle_name, name),
                )
                .with_size(size)
                .with_hint("Move to Trash when uninstalling the application.".to_string()),
            );
        }
    }

    Ok(findings)
}

fn select_app(query_lower: &str, app_dirs: &[PathBuf]) -> Result<InstalledApp> {
    let mut strong = Vec::new();
    let mut candidates = Vec::new();

    for dir in app_dirs {
        // Search a couple of levels deep in user-provided app directories.
        let max_depth = if cfg!(target_os = "macos") { 1 } else { 3 };
        for app in enumerate_apps_in(dir, max_depth) {
            let name = app.bundle_name.to_lowercase();
            let stem = app
                .bundle_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_lowercase())
                .unwrap_or_default();
            let id = app.bundle_id.as_ref().map(|s| s.to_lowercase());

            let is_strong =
                name == *query_lower || stem == *query_lower || id.as_deref() == Some(query_lower);
            let is_candidate = name.contains(query_lower)
                || stem.contains(query_lower)
                || id.as_ref().is_some_and(|s| s.contains(query_lower));

            if is_strong {
                strong.push(app);
            } else if is_candidate {
                candidates.push(app);
            }
        }
    }

    if strong.len() == 1 {
        return Ok(strong.into_iter().next().unwrap());
    }

    if strong.len() > 1 {
        let list = format_app_list(&strong);
        bail!(
            "Multiple applications match '{}':\n{}\nPlease use the full app name or bundle id.",
            query_lower,
            list
        );
    }

    if candidates.len() == 1 {
        return Ok(candidates.into_iter().next().unwrap());
    }

    if candidates.is_empty() {
        bail!("No application found matching '{}'", query_lower);
    }

    let list = format_app_list(&candidates);
    bail!(
        "Multiple applications match '{}':\n{}\nPlease use the full app name or bundle id.",
        query_lower,
        list
    );
}

fn format_app_list(apps: &[InstalledApp]) -> String {
    apps.iter()
        .map(|a| {
            format!(
                "  - {} ({}) at {}",
                a.bundle_name,
                a.bundle_id.as_deref().unwrap_or("unknown bundle id"),
                a.bundle_path.display()
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn app_needles(app: &InstalledApp) -> HashSet<String> {
    let mut needles = HashSet::new();
    needles.insert(app.bundle_name.to_lowercase());
    if let Some(id) = &app.bundle_id {
        let id_lower = id.to_lowercase();
        needles.insert(id_lower.clone());
        // Last component of the bundle id is often the executable/app name.
        if let Some(last) = id.rsplit('.').next() {
            needles.insert(last.to_lowercase());
        }
        // Also add the second-to-last component (the vendor/domain name) for
        // directories that use the full reversed domain.
        let parts: Vec<&str> = id.split('.').collect();
        if parts.len() >= 2 {
            needles.insert(parts[parts.len() - 2].to_lowercase());
        }
    }
    if let Some(stem) = app.bundle_path.file_stem() {
        needles.insert(stem.to_string_lossy().to_lowercase());
    }
    needles
}

fn matches_identifier(name: &str, needles: &HashSet<String>) -> bool {
    let name_lower = name.to_lowercase();
    for needle in needles {
        if name_lower == *needle {
            return true;
        }
        for sep in [".", " ", "-"] {
            if name_lower.starts_with(&format!("{}{}", needle, sep)) {
                return true;
            }
        }
    }
    false
}

fn print_human_report(findings: &[Finding], app_query: &str) {
    let bundle = findings
        .iter()
        .find(|f| f.category == Category::InstalledApp);
    let leftovers: Vec<&Finding> = findings
        .iter()
        .filter(|f| f.category != Category::InstalledApp)
        .collect();
    let total: u64 = findings.iter().filter_map(|f| f.size_bytes).sum();

    if let Some(bundle) = bundle {
        println!(
            "Application bundle for '{}': {}",
            app_query,
            target_path(&bundle.target)
        );
        if let Some(size) = bundle.size_bytes {
            println!("  Size: {}", format_bytes(size));
        }
    }

    if leftovers.is_empty() {
        println!("No user-data leftovers found.");
    } else {
        println!("Leftovers to remove:");
        for f in &leftovers {
            let size = f.size_bytes.unwrap_or(0);
            println!(
                "  [{:?}] {} {}",
                f.category,
                target_path(&f.target),
                format_bytes(size)
            );
        }
    }

    println!("Total reclaimable: {}", format_bytes(total));
}

fn target_path(target: &Target) -> String {
    match target {
        Target::Path(p) => p.display().to_string(),
        Target::Process(pid) => format!("pid:{}", pid),
        Target::Port(p) => format!("port:{}", p),
        Target::EnvironmentVariable(v) => v.clone(),
        Target::LaunchdLabel(l) => l.clone(),
        Target::Package(p) => p.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn write_plist(app: &Path, body: &str) {
        let contents = app.join("Contents");
        fs::create_dir_all(&contents).unwrap();
        fs::write(contents.join("Info.plist"), body).unwrap();
    }

    #[test]
    fn finds_app_bundle_and_leftovers() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let app_dir = home.join("Applications");
        fs::create_dir_all(&app_dir).unwrap();

        let app = app_dir.join("DemoApp.app");
        write_plist(
            &app,
            r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>CFBundleName</key>
    <string>DemoApp</string>
    <key>CFBundleIdentifier</key>
    <string>com.example.demoapp</string>
    <key>CFBundleShortVersionString</key>
    <string>1.0</string>
</dict>
</plist>"#,
        );

        // Create a leftover cache directory and preference file.
        let cache = home.join("Library/Caches/com.example.demoapp");
        fs::create_dir_all(&cache).unwrap();
        fs::write(cache.join("cache.data"), "data").unwrap();

        let prefs = home.join("Library/Preferences");
        fs::create_dir_all(&prefs).unwrap();
        fs::write(prefs.join("com.example.demoapp.plist"), "plist").unwrap();

        let app_dirs = vec![app_dir];
        let findings = find_uninstall_targets_in(home, &app_dirs, "demoapp").unwrap();

        assert!(
            findings
                .iter()
                .any(|f| f.category == Category::InstalledApp),
            "should find the app bundle"
        );
        assert!(
            findings
                .iter()
                .any(|f| f.category == Category::Cache && target_path(&f.target).contains("Caches")),
            "should find the cache directory"
        );
        assert!(
            findings.iter().any(|f| {
                f.category == Category::OrphanFile && target_path(&f.target).contains("Preferences")
            }),
            "should find the preference plist"
        );
    }

    #[test]
    fn no_match_reports_error() {
        let tmp = TempDir::new().unwrap();
        let home = tmp.path();
        let findings = find_uninstall_targets_in(home, &[], "nonexistent");
        assert!(findings.is_err());
    }
}
