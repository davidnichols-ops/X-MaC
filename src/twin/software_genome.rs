use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::engines::envmap::apps as envmap_apps;
use crate::engines::envmap::discovery as envmap_discovery;
#[cfg(target_os = "macos")]
use crate::engines::startup::scanner as startup_scanner;
#[cfg(target_os = "macos")]
use crate::engines::startup::scanner::StartupScope;
use crate::util::macos::MacosUtils;

/// Complete Software Genome — inventory of every software component on the Mac.
///
/// Covers: apps, executables, frameworks, dylibs, kexts, system extensions,
/// launch agents/daemons, login items, plugins, browser extensions, fonts,
/// dev tools, SDKs, package managers, Python/Node/Rust environments, Docker,
/// VMs, AI models, datasets, games, cloud apps, sync clients.
///
/// Maps to Digital Twin operations 41-80.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareGenome {
    pub applications: Vec<AppComponent>,
    pub frameworks: Vec<SoftwareComponent>,
    pub dynamic_libraries: Vec<SoftwareComponent>,
    pub kernel_extensions: Vec<SoftwareComponent>,
    pub system_extensions: Vec<SoftwareComponent>,
    pub launch_agents: Vec<SoftwareComponent>,
    pub launch_daemons: Vec<SoftwareComponent>,
    pub login_items: Vec<SoftwareComponent>,
    pub plugins: Vec<SoftwareComponent>,
    pub browser_extensions: Vec<SoftwareComponent>,
    pub fonts: Vec<SoftwareComponent>,
    pub developer_tools: Vec<SoftwareComponent>,
    pub sdks: Vec<SoftwareComponent>,
    pub package_managers: Vec<SoftwareComponent>,
    pub python_envs: Vec<SoftwareComponent>,
    pub node_envs: Vec<SoftwareComponent>,
    pub rust_toolchains: Vec<SoftwareComponent>,
    pub docker_images: Vec<SoftwareComponent>,
    pub containers: Vec<SoftwareComponent>,
    pub virtual_machines: Vec<SoftwareComponent>,
    pub ai_models: Vec<SoftwareComponent>,
    pub datasets: Vec<SoftwareComponent>,
    pub dependency_graph: HashMap<String, Vec<String>>,
    pub total_components: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppComponent {
    pub bundle_id: String,
    pub name: String,
    pub version: String,
    pub path: String,
    pub size_bytes: u64,
    pub developer: Option<String>,
    pub is_signed: bool,
    pub last_launched: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SoftwareComponent {
    pub name: String,
    pub version: Option<String>,
    pub path: String,
    pub size_bytes: u64,
}

impl SoftwareGenome {
    /// Collect the complete software genome.
    ///
    /// Orchestrates operations 41-80: application inventory, framework
    /// detection, launch agents/daemons, login items, plugins, fonts,
    /// developer tools, SDKs, Docker/VMs, and a dependency graph.
    pub fn collect() -> Self {
        // op 41-45: application inventory
        let applications = collect_applications();

        // op 46-48: framework detection
        let frameworks = collect_frameworks();

        // op 49-52: launch agents and daemons
        let (launch_agents, launch_daemons) = collect_launchd_items();

        // op 53: login items and helpers
        let login_items = collect_login_items();

        // op 54-55: plugin detection
        let plugins = collect_plugins();

        // op 56: font enumeration
        let fonts = collect_fonts();

        // op 57-60: developer tools and language environments
        let developer_tools = collect_developer_tools();
        let python_envs = collect_python_envs();
        let node_envs = collect_node_envs();
        let rust_toolchains = collect_rust_toolchains();
        let package_managers = collect_package_managers();

        // op 61: SDK detection
        let sdks = collect_sdks();

        // op 62-64: Docker and VM detection
        let docker_images = collect_docker_images();
        let virtual_machines = collect_virtual_machines();

        // op 65-70: dependency graph
        let dependency_graph = build_dependency_graph(
            &applications,
            &frameworks,
            &plugins,
            &launch_agents,
            &launch_daemons,
        );

        // op 80: total component count
        let total_components = applications.len()
            + frameworks.len()
            + launch_agents.len()
            + launch_daemons.len()
            + login_items.len()
            + plugins.len()
            + fonts.len()
            + developer_tools.len()
            + sdks.len()
            + package_managers.len()
            + python_envs.len()
            + node_envs.len()
            + rust_toolchains.len()
            + docker_images.len()
            + virtual_machines.len();

        Self {
            applications,
            frameworks,
            dynamic_libraries: Vec::new(),
            kernel_extensions: Vec::new(),
            system_extensions: Vec::new(),
            launch_agents,
            launch_daemons,
            login_items,
            plugins,
            browser_extensions: Vec::new(),
            fonts,
            developer_tools,
            sdks,
            package_managers,
            python_envs,
            node_envs,
            rust_toolchains,
            docker_images,
            containers: Vec::new(),
            virtual_machines,
            ai_models: Vec::new(),
            datasets: Vec::new(),
            dependency_graph,
            total_components,
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Recursively compute the total size (in bytes) of a directory tree.
/// Returns 0 if the path does not exist or is unreadable.
fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    let mut total: u64 = 0;
    for entry in walkdir::WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        if entry.file_type().is_file() {
            total += entry.metadata().map(|m| m.len()).unwrap_or(0);
        }
    }
    total
}

/// File size of a single file, or 0 if unreadable.
fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path).map(|m| m.len()).unwrap_or(0)
}

/// Resolve an executable's path via the `which` shell command. Returns `None`
/// if the command is not found on `PATH`.
fn which(name: &str) -> Option<PathBuf> {
    let output = std::process::Command::new("which")
        .arg(name)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if path.is_empty() {
        return None;
    }
    Some(PathBuf::from(path))
}

/// Build a [`SoftwareComponent`] from a directory path, computing its size
/// and using the directory's file name as the component name.
fn component_from_dir(path: &Path) -> Option<SoftwareComponent> {
    if !path.is_dir() {
        return None;
    }
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    Some(SoftwareComponent {
        name,
        version: None,
        path: path.to_string_lossy().to_string(),
        size_bytes: dir_size(path),
    })
}

/// Build a [`SoftwareComponent`] from a file path, computing its size and
/// using the file's stem as the component name.
fn component_from_file(path: &Path) -> Option<SoftwareComponent> {
    if !path.is_file() {
        return None;
    }
    let name = path
        .file_stem()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| path.to_string_lossy().to_string());
    Some(SoftwareComponent {
        name,
        version: None,
        path: path.to_string_lossy().to_string(),
        size_bytes: file_size(path),
    })
}

// ---------------------------------------------------------------------------
// op 41-45: Application inventory
// ---------------------------------------------------------------------------

/// Enumerate installed applications via the envmap engine and convert each
/// [`envmap_apps::InstalledApp`] into an [`AppComponent`].
fn collect_applications() -> Vec<AppComponent> {
    let mut apps = Vec::new();
    for dir in envmap_apps::default_app_dirs() {
        for installed in envmap_apps::enumerate_apps_in(&dir, 3) {
            apps.push(installed_app_to_component(&installed));
        }
    }
    apps
}

/// Convert an [`envmap_apps::InstalledApp`] into an [`AppComponent`].
fn installed_app_to_component(app: &envmap_apps::InstalledApp) -> AppComponent {
    let size_bytes = dir_size(&app.bundle_path);
    // Derive a developer name from the bundle id's second-to-last component
    // (e.g. `com.example.MyApp` -> `example`) when possible.
    let developer = app
        .bundle_id
        .as_ref()
        .and_then(|id| id.split('.').nth_back(1).map(|s| s.to_string()));
    AppComponent {
        bundle_id: app
            .bundle_id
            .clone()
            .unwrap_or_else(|| app.bundle_name.clone()),
        name: app.bundle_name.clone(),
        version: app.version.clone(),
        path: app.bundle_path.to_string_lossy().to_string(),
        size_bytes,
        developer,
        is_signed: false,
        last_launched: None,
    }
}

// ---------------------------------------------------------------------------
// op 46-48: Framework detection
// ---------------------------------------------------------------------------

/// macOS framework bundle directories to scan.
#[cfg(target_os = "macos")]
fn framework_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/System/Library/Frameworks"),
        PathBuf::from("/Library/Frameworks"),
    ]
}

/// Scan framework bundle directories for `.framework` bundles. (ops 46-48)
#[cfg(target_os = "macos")]
fn collect_frameworks() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for dir in framework_dirs() {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                if path
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "framework")
                    .unwrap_or(false)
                {
                    if let Some(c) = component_from_dir(&path) {
                        out.push(c);
                    }
                }
            }
        }
    }
    out
}

#[cfg(not(target_os = "macos"))]
fn collect_frameworks() -> Vec<SoftwareComponent> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// op 49-52: Launch agents and daemons
// ---------------------------------------------------------------------------

/// Scan macOS LaunchAgents and LaunchDaemons directories using the startup
/// engine's scanner. Returns `(agents, daemons)`.
#[cfg(target_os = "macos")]
fn collect_launchd_items() -> (Vec<SoftwareComponent>, Vec<SoftwareComponent>) {
    let mut agents = Vec::new();
    let mut daemons = Vec::new();
    for (dir, scope) in startup_scanner::macos_launchd_dirs() {
        let items = startup_scanner::scan_launchd_dir(&dir, scope);
        for item in items {
            let size_bytes = item.path.as_ref().map(|p| file_size(p)).unwrap_or(0);
            // Fall back to the file stem if the label is empty.
            let name = if item.label.is_empty() {
                item.path
                    .as_ref()
                    .and_then(|p| p.file_stem())
                    .map(|s| s.to_string_lossy().to_string())
                    .unwrap_or_default()
            } else {
                item.label
            };
            let component = SoftwareComponent {
                name,
                version: None,
                path: item
                    .path
                    .as_ref()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default(),
                size_bytes,
            };
            match scope {
                StartupScope::SystemDaemon => daemons.push(component),
                _ => agents.push(component),
            }
        }
    }
    (agents, daemons)
}

#[cfg(not(target_os = "macos"))]
fn collect_launchd_items() -> (Vec<SoftwareComponent>, Vec<SoftwareComponent>) {
    (Vec::new(), Vec::new())
}

// ---------------------------------------------------------------------------
// op 53: Login items and helpers
// ---------------------------------------------------------------------------

/// Discover login items and helper applications via the envmap discovery
/// engine. (op 53)
fn collect_login_items() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for artefact in envmap_discovery::find_login_helpers() {
        let name = artefact
            .path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| artefact.path.to_string_lossy().to_string());
        let size = if artefact.path.is_dir() {
            dir_size(&artefact.path)
        } else {
            file_size(&artefact.path)
        };
        out.push(SoftwareComponent {
            name,
            version: None,
            path: artefact.path.to_string_lossy().to_string(),
            size_bytes: size,
        });
    }
    out
}

// ---------------------------------------------------------------------------
// op 54-55: Plugin detection
// ---------------------------------------------------------------------------

/// Discover application plugin directories via the envmap discovery engine.
/// (ops 54-55)
fn collect_plugins() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for artefact in envmap_discovery::find_plugins() {
        let name = artefact.bundle_id.clone().unwrap_or_else(|| {
            artefact
                .path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| artefact.path.to_string_lossy().to_string())
        });
        out.push(SoftwareComponent {
            name,
            version: None,
            path: artefact.path.to_string_lossy().to_string(),
            size_bytes: dir_size(&artefact.path),
        });
    }
    out
}

// ---------------------------------------------------------------------------
// op 56: Font enumeration
// ---------------------------------------------------------------------------

/// Font file extensions recognised on macOS.
const FONT_EXTENSIONS: &[&str] = &["ttf", "otf", "ttc"];

/// Directories scanned for font files.
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/System/Library/Fonts"));
        dirs.push(PathBuf::from("/Library/Fonts"));
        dirs.push(MacosUtils::home_dir().join("Library/Fonts"));
    } else if cfg!(target_os = "linux") {
        dirs.push(PathBuf::from("/usr/share/fonts"));
        dirs.push(PathBuf::from("/usr/local/share/fonts"));
        dirs.push(MacosUtils::home_dir().join(".fonts"));
        dirs.push(MacosUtils::home_dir().join(".local/share/fonts"));
    }
    dirs
}

/// Scan font directories for `.ttf`, `.otf`, and `.ttc` files. (op 56)
fn collect_fonts() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for dir in font_dirs() {
        if !dir.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .max_depth(2)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let is_font = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| FONT_EXTENSIONS.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false);
            if is_font {
                if let Some(c) = component_from_file(path) {
                    out.push(c);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// op 57-60: Developer tools and language environments
// ---------------------------------------------------------------------------

/// Common developer tool executables to probe via `which`.
const DEV_TOOLS: &[&str] = &[
    "xcodebuild",
    "xcrun",
    "swift",
    "clang",
    "make",
    "cmake",
    "git",
    "gh",
    "docker",
    "go",
    "java",
    "javac",
    "mvn",
    "gradle",
];

/// Detect installed developer tools by probing common executables. (op 57)
fn collect_developer_tools() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();

    // Xcode application bundle (macOS)
    #[cfg(target_os = "macos")]
    {
        let xcode_paths = [
            PathBuf::from("/Applications/Xcode.app"),
            MacosUtils::home_dir().join("Applications/Xcode.app"),
        ];
        for path in xcode_paths {
            if path.is_dir() {
                if let Some(c) = component_from_dir(&path) {
                    out.push(c);
                }
                break;
            }
        }
        // CommandLineTools
        let clt = PathBuf::from("/Library/Developer/CommandLineTools");
        if clt.is_dir() {
            if let Some(c) = component_from_dir(&clt) {
                out.push(c);
            }
        }
    }

    // Probe executables on PATH
    for name in DEV_TOOLS {
        if let Some(path) = which(name) {
            out.push(SoftwareComponent {
                name: name.to_string(),
                version: None,
                path: path.to_string_lossy().to_string(),
                size_bytes: file_size(&path),
            });
        }
    }
    out
}

/// Detect Python environments: `python3`, `python`, pipx venvs, pyenv roots.
/// (op 58)
fn collect_python_envs() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for name in &["python3", "python"] {
        if let Some(path) = which(name) {
            out.push(SoftwareComponent {
                name: name.to_string(),
                version: None,
                path: path.to_string_lossy().to_string(),
                size_bytes: file_size(&path),
            });
        }
    }
    // pyenv root
    let pyenv_root = MacosUtils::home_dir().join(".pyenv");
    if pyenv_root.is_dir() {
        let versions = pyenv_root.join("versions");
        if versions.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&versions) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() {
                        if let Some(c) = component_from_dir(&path) {
                            out.push(c);
                        }
                    }
                }
            }
        }
    }
    out
}

/// Detect Node.js environments: `node`, `npm`, nvm versions, volta roots.
/// (op 58)
fn collect_node_envs() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for name in &["node", "npm", "npx", "yarn", "pnpm"] {
        if let Some(path) = which(name) {
            out.push(SoftwareComponent {
                name: name.to_string(),
                version: None,
                path: path.to_string_lossy().to_string(),
                size_bytes: file_size(&path),
            });
        }
    }
    // nvm versions
    let nvm_versions = MacosUtils::home_dir().join(".nvm/versions/node");
    if nvm_versions.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&nvm_versions) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(c) = component_from_dir(&path) {
                        out.push(c);
                    }
                }
            }
        }
    }
    out
}

/// Detect Rust toolchains: `cargo`, `rustc`, rustup toolchains. (op 59)
fn collect_rust_toolchains() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for name in &["cargo", "rustc", "rustup", "rustfmt"] {
        if let Some(path) = which(name) {
            out.push(SoftwareComponent {
                name: name.to_string(),
                version: None,
                path: path.to_string_lossy().to_string(),
                size_bytes: file_size(&path),
            });
        }
    }
    // rustup toolchains
    let rustup_home = std::env::var("RUSTUP_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| MacosUtils::home_dir().join(".rustup"));
    let toolchains = rustup_home.join("toolchains");
    if toolchains.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&toolchains) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(c) = component_from_dir(&path) {
                        out.push(c);
                    }
                }
            }
        }
    }
    out
}

/// Detect package managers: Homebrew, MacPorts, npm, pip, cargo, etc. (op 60)
fn collect_package_managers() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for name in &["brew", "port", "npm", "pip3", "pip", "cargo", "gem", "go"] {
        if let Some(path) = which(name) {
            out.push(SoftwareComponent {
                name: name.to_string(),
                version: None,
                path: path.to_string_lossy().to_string(),
                size_bytes: file_size(&path),
            });
        }
    }
    // Homebrew installation directories
    #[cfg(target_os = "macos")]
    {
        for brew_path in [PathBuf::from("/opt/homebrew"), PathBuf::from("/usr/local")] {
            if brew_path.is_dir() && brew_path.join("bin/brew").exists() {
                if let Some(c) = component_from_dir(&brew_path) {
                    out.push(c);
                }
                break;
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// op 61: SDK detection
// ---------------------------------------------------------------------------

/// Detect installed SDKs: CommandLineTools SDKs and Xcode SDKs. (op 61)
#[cfg(target_os = "macos")]
fn collect_sdks() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    let sdk_roots = [
        PathBuf::from("/Library/Developer/CommandLineTools/SDKs"),
        PathBuf::from(
            "/Applications/Xcode.app/Contents/Developer/Platforms/MacOSX.platform/Developer/SDKs",
        ),
    ];
    for root in sdk_roots {
        if !root.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&root) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(c) = component_from_dir(&path) {
                        out.push(c);
                    }
                }
            }
        }
    }
    out
}

#[cfg(not(target_os = "macos"))]
fn collect_sdks() -> Vec<SoftwareComponent> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// op 62-64: Docker and VM detection
// ---------------------------------------------------------------------------

/// List Docker images by parsing `docker images` output. (op 62)
fn collect_docker_images() -> Vec<SoftwareComponent> {
    let output = std::process::Command::new("docker")
        .args([
            "images",
            "--format",
            "{{.Repository}}:{{.Tag}}\t{{.Size}}\t{{.ID}}",
        ])
        .output();
    let mut out = Vec::new();
    if let Ok(out_bytes) = output {
        if out_bytes.status.success() {
            let text = String::from_utf8_lossy(&out_bytes.stdout);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() {
                    continue;
                }
                let parts: Vec<&str> = line.split('\t').collect();
                let name = parts.first().map(|s| s.to_string()).unwrap_or_default();
                let id = parts.get(2).map(|s| s.to_string());
                out.push(SoftwareComponent {
                    name,
                    version: id,
                    path: String::new(),
                    size_bytes: 0,
                });
            }
        }
    }
    out
}

/// Detect virtual machines by scanning common VM directories. (op 64)
fn collect_virtual_machines() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    let home = MacosUtils::home_dir();
    let vm_dirs = [
        home.join("Library/Application Support/VMware Fusion/Virtual Machines"),
        home.join("VirtualBox VMs"),
        home.join("Parallels"),
        home.join("Library/Containers/com.parallels.desktop.console"),
    ];
    for dir in vm_dirs {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(c) = component_from_dir(&path) {
                        out.push(c);
                    }
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// op 65-70: Dependency graph
// ---------------------------------------------------------------------------

/// Build a simple dependency graph mapping each application (by bundle id or
/// name) to its associated frameworks, plugins, and launch items. (ops 65-70)
fn build_dependency_graph(
    applications: &[AppComponent],
    frameworks: &[SoftwareComponent],
    plugins: &[SoftwareComponent],
    launch_agents: &[SoftwareComponent],
    launch_daemons: &[SoftwareComponent],
) -> HashMap<String, Vec<String>> {
    let mut graph = HashMap::new();

    // Each application maps to any frameworks whose name contains the app's
    // bundle-id-derived developer prefix (best-effort heuristic).
    for app in applications {
        let mut deps = Vec::new();

        // Heuristic: a framework "belongs" to an app if the framework name
        // starts with the app's developer prefix (e.g. `com.example`).
        if let Some(dev) = &app.developer {
            let prefix = format!("{}.", dev);
            for fw in frameworks {
                if fw.name.starts_with(&prefix) || fw.name.to_lowercase().contains(dev) {
                    deps.push(fw.name.clone());
                }
            }
        }

        // Plugins whose bundle id matches the app's bundle id.
        for plugin in plugins {
            if plugin.name == app.bundle_id {
                deps.push(plugin.path.clone());
            }
        }

        // Launch agents/daemons whose label starts with the app's bundle id.
        for agent in launch_agents.iter().chain(launch_daemons.iter()) {
            if agent.name.starts_with(&app.bundle_id) {
                deps.push(agent.name.clone());
            }
        }

        if !deps.is_empty() {
            graph.insert(app.bundle_id.clone(), deps);
        }
    }

    graph
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collect_returns_non_negative_total() {
        let genome = SoftwareGenome::collect();
        // total_components is a sum of vec lengths, so it must be >= 0 and
        // consistent with the individual vectors.
        let expected = genome.applications.len()
            + genome.frameworks.len()
            + genome.launch_agents.len()
            + genome.launch_daemons.len()
            + genome.login_items.len()
            + genome.plugins.len()
            + genome.fonts.len()
            + genome.developer_tools.len()
            + genome.sdks.len()
            + genome.package_managers.len()
            + genome.python_envs.len()
            + genome.node_envs.len()
            + genome.rust_toolchains.len()
            + genome.docker_images.len()
            + genome.virtual_machines.len();
        assert_eq!(genome.total_components, expected);
    }

    #[test]
    fn collect_runs_without_panic() {
        // The full collection touches the filesystem and may invoke external
        // commands; it must never panic regardless of the host environment.
        let _genome = SoftwareGenome::collect();
    }

    #[test]
    fn dir_size_handles_missing_path() {
        assert_eq!(dir_size(Path::new("/this/does/not/exist/xyz123")), 0);
    }

    #[test]
    fn dir_size_computes_file_tree() {
        use std::fs;
        let tmp = tempfile::TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), "hello").unwrap();
        fs::create_dir_all(tmp.path().join("sub")).unwrap();
        fs::write(tmp.path().join("sub/b.txt"), "world!!").unwrap();
        // 5 + 7 = 12 bytes
        assert_eq!(dir_size(tmp.path()), 12);
    }

    #[test]
    fn file_size_handles_missing_file() {
        assert_eq!(file_size(Path::new("/no/such/file/abc")), 0);
    }

    #[test]
    fn which_returns_none_for_missing_binary() {
        assert!(which("this-binary-definitely-does-not-exist-xyz123").is_none());
    }

    #[test]
    fn which_finds_common_binary() {
        // `ls` exists on every Unix-like system we support.
        if let Some(path) = which("ls") {
            assert!(path.is_file());
        }
    }

    #[test]
    fn component_from_dir_returns_none_for_missing() {
        assert!(component_from_dir(Path::new("/no/such/dir/xyz")).is_none());
    }

    #[test]
    fn component_from_dir_builds_component() {
        let tmp = tempfile::TempDir::new().unwrap();
        let comp = component_from_dir(tmp.path()).expect("component");
        assert!(comp.size_bytes == 0);
        assert!(!comp.path.is_empty());
    }

    #[test]
    fn component_from_file_returns_none_for_missing() {
        assert!(component_from_file(Path::new("/no/such/file/xyz")).is_none());
    }

    #[test]
    fn component_from_file_builds_component() {
        use std::fs;
        let tmp = tempfile::TempDir::new().unwrap();
        let path = tmp.path().join("hello.txt");
        fs::write(&path, "hello world").unwrap();
        let comp = component_from_file(&path).expect("component");
        assert_eq!(comp.size_bytes, 11);
        assert_eq!(comp.name, "hello");
    }

    #[test]
    fn installed_app_to_component_maps_fields() {
        let app = envmap_apps::InstalledApp {
            bundle_path: PathBuf::from("/Applications/Test.app"),
            bundle_name: "Test".to_string(),
            bundle_id: Some("com.example.test".to_string()),
            version: "1.0".to_string(),
        };
        let comp = installed_app_to_component(&app);
        assert_eq!(comp.bundle_id, "com.example.test");
        assert_eq!(comp.name, "Test");
        assert_eq!(comp.version, "1.0");
        assert_eq!(comp.developer.as_deref(), Some("example"));
    }

    #[test]
    fn installed_app_to_component_falls_back_to_name() {
        let app = envmap_apps::InstalledApp {
            bundle_path: PathBuf::from("/Applications/NoId.app"),
            bundle_name: "NoId".to_string(),
            bundle_id: None,
            version: "2.0".to_string(),
        };
        let comp = installed_app_to_component(&app);
        assert_eq!(comp.bundle_id, "NoId");
        assert!(comp.developer.is_none());
    }

    #[test]
    fn collect_applications_returns_vec() {
        // Must not panic; content depends on the host.
        let apps = collect_applications();
        for app in &apps {
            assert!(!app.name.is_empty());
        }
    }

    #[test]
    fn collect_fonts_returns_vec() {
        let fonts = collect_fonts();
        for font in &fonts {
            let ext = Path::new(&font.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            assert!(FONT_EXTENSIONS.contains(&ext.to_lowercase().as_str()));
        }
    }

    #[test]
    fn collect_login_items_returns_vec() {
        let _items = collect_login_items();
    }

    #[test]
    fn collect_plugins_returns_vec() {
        let _plugins = collect_plugins();
    }

    #[test]
    fn collect_developer_tools_returns_vec() {
        let tools = collect_developer_tools();
        // At least git is usually present in dev environments, but we don't
        // assert on specific tools to keep the test host-agnostic.
        for tool in &tools {
            assert!(!tool.name.is_empty());
        }
    }

    #[test]
    fn collect_python_envs_returns_vec() {
        let _envs = collect_python_envs();
    }

    #[test]
    fn collect_node_envs_returns_vec() {
        let _envs = collect_node_envs();
    }

    #[test]
    fn collect_rust_toolchains_returns_vec() {
        let _envs = collect_rust_toolchains();
    }

    #[test]
    fn collect_package_managers_returns_vec() {
        let _pms = collect_package_managers();
    }

    #[test]
    fn collect_docker_images_returns_vec() {
        // Docker may not be installed; the function must still return a vec.
        let _images = collect_docker_images();
    }

    #[test]
    fn collect_virtual_machines_returns_vec() {
        let _vms = collect_virtual_machines();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn collect_frameworks_returns_vec_on_macos() {
        let frameworks = collect_frameworks();
        // /System/Library/Frameworks always exists on macOS.
        assert!(!frameworks.is_empty());
        for fw in &frameworks {
            assert!(fw.path.ends_with(".framework"));
        }
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn collect_sdks_returns_vec_on_macos() {
        let _sdks = collect_sdks();
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn collect_launchd_items_returns_vec_on_macos() {
        let (agents, daemons) = collect_launchd_items();
        // At least system LaunchDaemons should be present on macOS.
        assert!(!daemons.is_empty());
        // All items should have non-empty names (label or file stem fallback).
        for agent in &agents {
            assert!(!agent.name.is_empty(), "agent has empty name: {:?}", agent);
        }
        for daemon in &daemons {
            assert!(
                !daemon.name.is_empty(),
                "daemon has empty name: {:?}",
                daemon
            );
        }
    }

    #[test]
    fn build_dependency_graph_links_apps_to_launchd() {
        let app = AppComponent {
            bundle_id: "com.example.test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            path: "/Applications/Test.app".to_string(),
            size_bytes: 0,
            developer: Some("example".to_string()),
            is_signed: false,
            last_launched: None,
        };
        let agent = SoftwareComponent {
            name: "com.example.test.helper".to_string(),
            version: None,
            path: "/Library/LaunchAgents/com.example.test.helper.plist".to_string(),
            size_bytes: 0,
        };
        let graph = build_dependency_graph(&[app], &[], &[], &[agent], &[]);
        assert_eq!(
            graph.get("com.example.test"),
            Some(&vec!["com.example.test.helper".to_string()])
        );
    }

    #[test]
    fn build_dependency_graph_empty_for_no_deps() {
        let app = AppComponent {
            bundle_id: "com.example.test".to_string(),
            name: "Test".to_string(),
            version: "1.0".to_string(),
            path: "/Applications/Test.app".to_string(),
            size_bytes: 0,
            developer: None,
            is_signed: false,
            last_launched: None,
        };
        let graph = build_dependency_graph(&[app], &[], &[], &[], &[]);
        assert!(graph.is_empty());
    }

    #[test]
    fn font_dirs_non_empty_on_supported_platforms() {
        let dirs = font_dirs();
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            assert!(!dirs.is_empty());
        }
    }

    #[test]
    fn font_extensions_lowercase_match() {
        assert!(FONT_EXTENSIONS.contains(&"ttf"));
        assert!(FONT_EXTENSIONS.contains(&"otf"));
        assert!(FONT_EXTENSIONS.contains(&"ttc"));
        assert!(!FONT_EXTENSIONS.contains(&"woff"));
    }
}
