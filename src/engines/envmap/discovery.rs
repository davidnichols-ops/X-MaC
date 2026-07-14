//! System and language package discovery.
//!
//! Ports the MIF `DiscoveryEngine.gather_packages` into Rust. macOS is
//! first-class (Homebrew formulae + casks); Linux degrades gracefully by
//! probing `dpkg-query` / `rpm` / `pacman`. Language runtimes (Python pip /
//! pipx, npm global, Ruby gems) are queried on every platform on a best-effort
//! basis — missing tools simply yield empty lists.

use std::path::PathBuf;
use std::process::Command;
use std::time::Duration;

use crate::util::macos::MacosUtils;

/// One discovered package: a display string (e.g. `git 2.43.0` or
/// `numpy==1.26.2`) plus the source that found it.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct DiscoveredPackage {
    pub source: &'static str,
    pub display: String,
}

/// Result of probing a single package source.
#[derive(Debug, Clone)]
pub struct SourceResult {
    pub source: &'static str,
    pub packages: Vec<String>,
    /// Non-fatal error message if the source could not be queried.
    pub error: Option<String>,
}

impl SourceResult {
    pub fn ok(source: &'static str, packages: Vec<String>) -> Self {
        Self {
            source,
            packages,
            error: None,
        }
    }

    #[allow(dead_code)]
    pub fn err(source: &'static str, msg: impl Into<String>) -> Self {
        Self {
            source,
            packages: Vec::new(),
            error: Some(msg.into()),
        }
    }
}

/// Run a command defensively, returning its stripped stdout lines. Mirrors
/// MIF's `execute_command`: never uses a shell, swallows all failures into an
/// empty result. The `timeout` argument is accepted for API symmetry with
/// [`run_command_timed`] but not enforced here (use [`run_command_timed`] for
/// a hard deadline).
pub fn run_command(args: &[&str], _timeout: Option<Duration>) -> Vec<String> {
    if args.is_empty() {
        return Vec::new();
    }
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let output = match cmd.output() {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    if !output.status.success() {
        return Vec::new();
    }

    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Run a command with a hard timeout using `wait` with a polling fallback.
/// Returns stripped stdout lines on success, empty on any failure or timeout.
#[allow(dead_code)]
pub fn run_command_timed(args: &[&str], timeout: Duration) -> Vec<String> {
    if args.is_empty() {
        return Vec::new();
    }
    let mut cmd = Command::new(args[0]);
    cmd.args(&args[1..]);
    cmd.stdout(std::process::Stdio::piped());
    cmd.stderr(std::process::Stdio::piped());

    let mut child = match cmd.spawn() {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let deadline = std::time::Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                if !status.success() {
                    return Vec::new();
                }
                let mut output = match child.stdout.take() {
                    Some(s) => s,
                    None => return Vec::new(),
                };
                use std::io::Read;
                let mut buf = Vec::new();
                let _ = output.read_to_end(&mut buf);
                return String::from_utf8_lossy(&buf)
                    .lines()
                    .map(|l| l.trim().to_string())
                    .filter(|l| !l.is_empty())
                    .collect();
            }
            Ok(None) => {
                if std::time::Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Vec::new();
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(_) => return Vec::new(),
        }
    }
}

/// Whether a given executable is available on PATH.
#[allow(dead_code)]
pub fn command_available(name: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for dir in path.split(':') {
            let candidate = std::path::Path::new(dir).join(name);
            if candidate.is_file() {
                return true;
            }
        }
    }
    false
}

/// Gather system-level packages (Homebrew formulae + casks on macOS; on Linux
/// probe dpkg/rpm/pacman and return the first source that yields output).
pub fn gather_system_packages() -> Vec<SourceResult> {
    let mut results = Vec::new();

    if cfg!(target_os = "macos") {
        // Homebrew formulae.
        let formulae = run_command(&["brew", "list", "--versions"], None);
        results.push(SourceResult::ok("homebrew_formulae", formulae));

        // Homebrew casks.
        let casks = run_command(&["brew", "list", "--casks", "--versions"], None);
        results.push(SourceResult::ok("homebrew_casks", casks));
    } else if cfg!(target_os = "linux") {
        // Try dpkg-query first, then rpm, then pacman.
        let dpkg = run_command(&["dpkg-query", "-W", "-f=${Package} ${Version}\n"], None);
        if !dpkg.is_empty() {
            results.push(SourceResult::ok("dpkg", dpkg));
        } else {
            let rpm = run_command(
                &["rpm", "-qa", "--queryformat", "%{NAME} %{VERSION}\n"],
                None,
            );
            if !rpm.is_empty() {
                results.push(SourceResult::ok("rpm", rpm));
            } else {
                let pacman = run_command(&["pacman", "-Q"], None);
                results.push(SourceResult::ok("pacman", pacman));
            }
        }
    }

    results
}

/// Gather language-runtime packages: Python (pip freeze + pipx list), Node.js
/// (npm global), Ruby gems. Each source is best-effort.
pub fn gather_language_packages() -> Vec<SourceResult> {
    let mut results = Vec::new();

    // Python pip freeze — prefer `python3 -m pip freeze`.
    let pip = run_command(&["python3", "-m", "pip", "freeze"], None);
    results.push(SourceResult::ok("pip", pip));

    // pipx installed apps.
    let pipx = run_pipx_list();
    results.push(SourceResult::ok("pipx", pipx));

    // npm global packages (parseable form).
    let npm = run_command(&["npm", "list", "-g", "--depth=0", "--parseable"], None);
    // npm --parseable emits paths; reduce to basenames for readability.
    let npm = npm
        .into_iter()
        .map(|p| {
            std::path::Path::new(&p)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or(p)
        })
        .collect::<Vec<_>>();
    results.push(SourceResult::ok("npm_global", npm));

    // Ruby gems (local).
    let gems = run_command(&["gem", "list", "--local"], None);
    results.push(SourceResult::ok("ruby_gems", gems));

    results
}

/// `pipx list` emits a multi-line human report; extract the `package X.Y.Z`
/// style lines into a compact list.
fn run_pipx_list() -> Vec<String> {
    let raw = run_command(&["pipx", "list", "--short"], None);
    if !raw.is_empty() {
        return raw;
    }
    // Fallback: parse the verbose `pipx list` output for "package <name> <ver>".
    let verbose = run_command(&["pipx", "list"], None);
    let mut out = Vec::new();
    for line in verbose {
        let trimmed = line.trim();
        if trimmed.starts_with("package ") {
            out.push(trimmed.to_string());
        }
    }
    out
}

/// Flatten a slice of `SourceResult` into a single list of `DiscoveredPackage`.
#[allow(dead_code)]
pub fn flatten(results: &[SourceResult]) -> Vec<DiscoveredPackage> {
    let mut out = Vec::new();
    for r in results {
        for pkg in &r.packages {
            out.push(DiscoveredPackage {
                source: r.source,
                display: pkg.clone(),
            });
        }
    }
    out
}

// ---- Phase 5: application-related discovery ----------------------------

/// One application-related artefact discovered outside a `.app` bundle
/// (leftover, saved state, plugin, extension, login helper, orphan file).
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct AppArtefact {
    /// Category label matching a [`crate::core::types::Category`] variant.
    pub kind: &'static str,
    /// Path to the artefact on disk.
    pub path: PathBuf,
    /// Bundle identifier this artefact is associated with, when known.
    pub bundle_id: Option<String>,
}

/// Standard macOS `~/Library` subdirectories that hold application leftovers
/// keyed by bundle identifier (op 120).
#[cfg(target_os = "macos")]
pub fn leftover_search_dirs() -> Vec<(&'static str, PathBuf)> {
    let home = MacosUtils::home_dir();
    vec![
        ("Preferences", home.join("Library/Preferences")),
        ("Containers", home.join("Library/Containers")),
        (
            "Application Support",
            home.join("Library/Application Support"),
        ),
        ("Caches", home.join("Library/Caches")),
        (
            "Saved Application State",
            home.join("Library/Saved Application State"),
        ),
        ("HTTPStorages", home.join("Library/HTTPStorages")),
        ("WebKit", home.join("Library/WebKit")),
        ("Logs", home.join("Library/Logs")),
        ("Group Containers", home.join("Library/Group Containers")),
    ]
}

#[cfg(not(target_os = "macos"))]
pub fn leftover_search_dirs() -> Vec<(&'static str, PathBuf)> {
    Vec::new()
}

/// Detect application leftovers — files/directories in `~/Library` whose
/// names reference a bundle identifier that is no longer installed (op 120).
/// `installed_bundle_ids` is the set of currently-installed bundle IDs.
#[allow(dead_code)]
pub fn detect_app_leftovers(installed_bundle_ids: &[String]) -> Vec<AppArtefact> {
    let mut out = Vec::new();
    for (_label, dir) in leftover_search_dirs() {
        if !dir.is_dir() {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            // Leftover candidates are typically named after a bundle id
            // (e.g. `com.example.app.plist` or `com.example.app`).
            let candidate = name.strip_suffix(".plist").unwrap_or(&name).to_string();
            if looks_like_bundle_id(&candidate)
                && !installed_bundle_ids.iter().any(|id| id == &candidate)
            {
                out.push(AppArtefact {
                    kind: "app_leftover",
                    path: entry.path(),
                    bundle_id: Some(candidate),
                });
            }
        }
    }
    out
}

/// Whether a string looks like a reverse-DNS bundle identifier.
fn looks_like_bundle_id(s: &str) -> bool {
    s.contains('.') && s.split('.').count() >= 2
}

/// Find saved application state directories (op 124). On macOS these live
/// under `~/Library/Saved Application State/<bundle-id>.savedState`.
#[allow(dead_code)]
pub fn find_saved_states() -> Vec<AppArtefact> {
    find_library_subdir("Saved Application State", "saved_app_state", ".savedState")
}

/// Find application plugin directories (op 125). On macOS plugins live under
/// `~/Library/Application Support/<bundle>/Plugins` and
/// `/Library/Application Support/<bundle>/Plugins`.
#[allow(dead_code)]
pub fn find_plugins() -> Vec<AppArtefact> {
    let mut out = Vec::new();
    let home = MacosUtils::home_dir();
    let roots = [
        home.join("Library/Application Support"),
        PathBuf::from("/Library/Application Support"),
    ];
    for root in &roots {
        if !root.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let plugins = entry.path().join("Plugins");
                if plugins.is_dir() {
                    out.push(AppArtefact {
                        kind: "app_plugin",
                        path: plugins,
                        bundle_id: Some(entry.file_name().to_string_lossy().to_string()),
                    });
                }
            }
        }
    }
    out
}

/// Find application extension directories (op 126). On macOS app extensions
/// live under `~/Library/Application Support/<bundle>/Extensions` and the
/// system-wide `/Library/Extensions`.
#[allow(dead_code)]
pub fn find_extensions() -> Vec<AppArtefact> {
    let mut out = Vec::new();
    let home = MacosUtils::home_dir();
    let roots = [
        home.join("Library/Application Support"),
        PathBuf::from("/Library/Application Support"),
    ];
    for root in &roots {
        if !root.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(root) {
            for entry in entries.flatten() {
                let exts = entry.path().join("Extensions");
                if exts.is_dir() {
                    out.push(AppArtefact {
                        kind: "app_extension",
                        path: exts,
                        bundle_id: Some(entry.file_name().to_string_lossy().to_string()),
                    });
                }
            }
        }
    }
    // System-wide extensions directory.
    let sys_ext = PathBuf::from("/Library/Extensions");
    if sys_ext.is_dir() {
        out.push(AppArtefact {
            kind: "app_extension",
            path: sys_ext,
            bundle_id: None,
        });
    }
    out
}

/// Find login helper applications (op 127). On macOS these live inside app
/// bundles under `Contents/Library/LoginItems` and `Contents/Helpers`, plus
/// the system `~/Library/LoginItems` directory.
#[allow(dead_code)]
pub fn find_login_helpers() -> Vec<AppArtefact> {
    let mut out = Vec::new();
    let home = MacosUtils::home_dir();
    let user_login = home.join("Library/LoginItems");
    if user_login.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&user_login) {
            for entry in entries.flatten() {
                out.push(AppArtefact {
                    kind: "login_helper",
                    path: entry.path(),
                    bundle_id: None,
                });
            }
        }
    }
    out
}

/// Detect orphan files — files in `~/Library` containers/caches whose
/// owning bundle identifier is no longer installed (op 130). This overlaps
/// with [`detect_app_leftovers`] but also flags empty container directories.
#[allow(dead_code)]
pub fn detect_orphan_files(installed_bundle_ids: &[String]) -> Vec<AppArtefact> {
    detect_app_leftovers(installed_bundle_ids)
        .into_iter()
        .map(|a| AppArtefact {
            kind: "orphan_file",
            path: a.path,
            bundle_id: a.bundle_id,
        })
        .collect()
}

/// Detect broken uninstallations — application containers/support directories
/// that still exist after the app bundle has been removed (op 131). A
/// broken uninstall is signalled by a leftover container directory that is
/// non-empty (i.e. the uninstaller left data behind).
#[allow(dead_code)]
pub fn detect_broken_uninstalls(installed_bundle_ids: &[String]) -> Vec<AppArtefact> {
    let mut out = Vec::new();
    for (_label, dir) in leftover_search_dirs() {
        if !dir.is_dir() {
            continue;
        }
        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => continue,
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            let candidate = name.strip_suffix(".plist").unwrap_or(&name).to_string();
            if !looks_like_bundle_id(&candidate)
                || installed_bundle_ids.iter().any(|id| id == &candidate)
            {
                continue;
            }
            // Only flag as a broken uninstall when the leftover directory is
            // non-empty (a real data residue, not just a stale plist).
            if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                if let Ok(inner) = std::fs::read_dir(entry.path()) {
                    if inner.count() > 0 {
                        out.push(AppArtefact {
                            kind: "broken_uninstall",
                            path: entry.path(),
                            bundle_id: Some(candidate),
                        });
                    }
                }
            }
        }
    }
    out
}

/// Helper: scan a `~/Library/<subdir>` directory for entries matching a
/// suffix and emit [`AppArtefact`]s with the bundle id extracted from the
/// filename.
fn find_library_subdir(subdir: &str, kind: &'static str, suffix: &str) -> Vec<AppArtefact> {
    let home = MacosUtils::home_dir();
    let dir = home.join("Library").join(subdir);
    let mut out = Vec::new();
    if !dir.is_dir() {
        return out;
    }
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(id) = name.strip_suffix(suffix) {
                out.push(AppArtefact {
                    kind,
                    path: entry.path(),
                    bundle_id: Some(id.to_string()),
                });
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_result_ok_carries_packages() {
        let r = SourceResult::ok("test", vec!["a".to_string(), "b".to_string()]);
        assert_eq!(r.source, "test");
        assert_eq!(r.packages.len(), 2);
        assert!(r.error.is_none());
    }

    #[test]
    fn source_result_err_has_message_and_empty_packages() {
        let r = SourceResult::err("test", "boom");
        assert_eq!(r.packages.len(), 0);
        assert_eq!(r.error.as_deref(), Some("boom"));
    }

    #[test]
    fn flatten_preserves_source() {
        let results = vec![
            SourceResult::ok("a", vec!["x".to_string()]),
            SourceResult::ok("b", vec!["y".to_string(), "z".to_string()]),
        ];
        let flat = flatten(&results);
        assert_eq!(flat.len(), 3);
        assert_eq!(flat[0].source, "a");
        assert_eq!(flat[1].source, "b");
        assert_eq!(flat[2].display, "z");
    }

    #[test]
    fn run_command_empty_args_returns_empty() {
        let out = run_command(&[], None);
        assert!(out.is_empty());
    }

    #[test]
    fn run_command_missing_binary_returns_empty() {
        let out = run_command(&["this-binary-definitely-does-not-exist-xyz123"], None);
        assert!(out.is_empty());
    }

    #[test]
    fn run_command_timed_missing_binary_returns_empty() {
        let out = run_command_timed(
            &["this-binary-definitely-does-not-exist-xyz123"],
            Duration::from_secs(1),
        );
        assert!(out.is_empty());
    }

    #[test]
    fn run_command_timed_empty_args_returns_empty() {
        let out = run_command_timed(&[], Duration::from_secs(1));
        assert!(out.is_empty());
    }

    #[test]
    fn gather_packages_returns_some_results() {
        // On any host these calls are best-effort; they must at least return
        // a vector (possibly with empty package lists) without panicking.
        let sys = gather_system_packages();
        let lang = gather_language_packages();
        // Flatten should never panic regardless of contents.
        let _ = flatten(&sys);
        let _ = flatten(&lang);
    }

    // ---- Phase 5 tests --------------------------------------------------

    #[test]
    fn looks_like_bundle_id_detects_reverse_dns() {
        assert!(looks_like_bundle_id("com.example.app"));
        assert!(!looks_like_bundle_id("README"));
        assert!(!looks_like_bundle_id("single"));
    }

    #[test]
    fn detect_app_leftovers_flags_uninstalled_ids() {
        // This test relies on the real ~/Library which may not contain the
        // fixture ids; we only assert the function runs without panicking
        // and returns a vector. On non-macOS hosts the search dirs are empty.
        let installed = vec!["com.example.installed".to_string()];
        let leftovers = detect_app_leftovers(&installed);
        // The fixture id must never appear in the results.
        assert!(leftovers
            .iter()
            .all(|a| a.bundle_id.as_deref() != Some("com.example.installed")));
    }

    #[test]
    fn detect_orphan_files_runs_without_panic() {
        let installed = vec!["com.example.installed".to_string()];
        let orphans = detect_orphan_files(&installed);
        assert!(orphans.iter().all(|a| a.kind == "orphan_file"));
    }

    #[test]
    fn detect_broken_uninstalls_runs_without_panic() {
        let installed = vec!["com.example.installed".to_string()];
        let broken = detect_broken_uninstalls(&installed);
        assert!(broken.iter().all(|a| a.kind == "broken_uninstall"));
    }

    #[test]
    fn find_saved_states_runs_without_panic() {
        let states = find_saved_states();
        assert!(states.iter().all(|a| a.kind == "saved_app_state"));
    }

    #[test]
    fn find_plugins_runs_without_panic() {
        let plugins = find_plugins();
        assert!(plugins.iter().all(|a| a.kind == "app_plugin"));
    }

    #[test]
    fn find_extensions_runs_without_panic() {
        let exts = find_extensions();
        assert!(exts.iter().all(|a| a.kind == "app_extension"));
    }

    #[test]
    fn find_login_helpers_runs_without_panic() {
        let helpers = find_login_helpers();
        assert!(helpers.iter().all(|a| a.kind == "login_helper"));
    }
}
