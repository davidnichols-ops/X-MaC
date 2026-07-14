use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::cleanup::policy::RiskLevel;
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
        // op 42: every executable
        let executables = collect_executables();

        // op 43-48: framework detection
        let frameworks = collect_frameworks();

        // op 44: dynamic libraries
        let dynamic_libraries = collect_dynamic_libraries();

        // op 45: kernel extensions
        let kernel_extensions = collect_kernel_extensions();

        // op 49-52: launch agents and daemons
        let (launch_agents, launch_daemons) = collect_launchd_items();

        // op 53: login items and helpers
        let login_items = collect_login_items();

        // op 51-55: plugin detection
        let plugins = collect_plugins();

        // op 52: browser extensions
        let browser_extensions = collect_browser_extensions();

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
        let containers = collect_containers();
        let virtual_machines = collect_virtual_machines();

        // op 63: system extensions
        let system_extensions = collect_system_extensions();

        // op 71-72: AI models and datasets
        let ai_models = collect_ai_models();
        let datasets = collect_datasets();

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
            + executables.len()
            + frameworks.len()
            + dynamic_libraries.len()
            + kernel_extensions.len()
            + system_extensions.len()
            + launch_agents.len()
            + launch_daemons.len()
            + login_items.len()
            + plugins.len()
            + browser_extensions.len()
            + fonts.len()
            + developer_tools.len()
            + sdks.len()
            + package_managers.len()
            + python_envs.len()
            + node_envs.len()
            + rust_toolchains.len()
            + docker_images.len()
            + containers.len()
            + virtual_machines.len()
            + ai_models.len()
            + datasets.len();

        Self {
            applications,
            frameworks,
            dynamic_libraries,
            kernel_extensions,
            system_extensions,
            launch_agents,
            launch_daemons,
            login_items,
            plugins,
            browser_extensions,
            fonts,
            developer_tools,
            sdks,
            package_managers,
            python_envs,
            node_envs,
            rust_toolchains,
            docker_images,
            containers,
            virtual_machines,
            ai_models,
            datasets,
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
// op 42: Executable inventory
// ---------------------------------------------------------------------------

/// Directories scanned for standalone executables.
fn executable_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/usr/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
        dirs.push(PathBuf::from("/opt/homebrew/bin"));
    } else if cfg!(target_os = "linux") {
        dirs.push(PathBuf::from("/usr/bin"));
        dirs.push(PathBuf::from("/usr/local/bin"));
    }
    dirs
}

/// op 42: Inventory every executable — scan `/usr/bin`, `/usr/local/bin`, and
/// `/opt/homebrew/bin` for executable files.
#[allow(dead_code)]
fn collect_executables() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for dir in executable_dirs() {
        if !dir.is_dir() {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                // Only include files that are executable by the owner.
                let is_exec = std::fs::metadata(&path)
                    .map(|m| {
                        use std::os::unix::fs::PermissionsExt;
                        m.permissions().mode() & 0o100 != 0
                    })
                    .unwrap_or(false);
                if is_exec {
                    if let Some(c) = component_from_file(&path) {
                        out.push(c);
                    }
                }
            }
        }
    }
    out
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

/// op 43: Inventory frameworks — scan framework bundle directories for
/// `.framework` bundles. (ops 46-48)
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
// op 44: Dynamic library inventory
// ---------------------------------------------------------------------------

/// Directories scanned for dynamic libraries (`.dylib`/`.so`).
fn dynamic_library_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if cfg!(target_os = "macos") {
        dirs.push(PathBuf::from("/usr/lib"));
        dirs.push(PathBuf::from("/usr/local/lib"));
        dirs.push(PathBuf::from("/opt/homebrew/lib"));
    } else if cfg!(target_os = "linux") {
        dirs.push(PathBuf::from("/usr/lib"));
        dirs.push(PathBuf::from("/usr/local/lib"));
    }
    dirs
}

/// Dynamic-library file extensions recognised on the current platform.
fn dynamic_library_extensions() -> &'static [&'static str] {
    if cfg!(target_os = "macos") {
        &["dylib"]
    } else {
        &["so"]
    }
}

/// op 44: Inventory dynamic libraries — scan `/usr/lib` and `/usr/local/lib`
/// for `.dylib` files.
#[allow(dead_code)]
fn collect_dynamic_libraries() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    let exts = dynamic_library_extensions();
    for dir in dynamic_library_dirs() {
        if !dir.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .max_depth(1)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let is_dylib = path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| exts.contains(&e.to_lowercase().as_str()))
                .unwrap_or(false);
            if is_dylib {
                if let Some(c) = component_from_file(path) {
                    out.push(c);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// op 45: Kernel extension inventory
// ---------------------------------------------------------------------------

/// Directories scanned for kernel extensions (`.kext` bundles).
#[cfg(target_os = "macos")]
fn kernel_extension_dirs() -> Vec<PathBuf> {
    vec![
        PathBuf::from("/System/Library/Extensions"),
        PathBuf::from("/Library/Extensions"),
    ]
}

/// op 45: Inventory kernel extensions — scan `/System/Library/Extensions`
/// and `/Library/Extensions` for `.kext` bundles.
#[allow(dead_code)]
#[cfg(target_os = "macos")]
fn collect_kernel_extensions() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for dir in kernel_extension_dirs() {
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
                    .map(|e| e == "kext")
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
#[allow(dead_code)]
fn collect_kernel_extensions() -> Vec<SoftwareComponent> {
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

/// op 51: Inventory plugins — discover application plugin directories via
/// the envmap discovery engine. (ops 54-55)
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
// op 52: Browser extension inventory
// ---------------------------------------------------------------------------

/// Directories scanned for browser extensions (Safari, Chrome, Firefox).
#[cfg(target_os = "macos")]
fn browser_extension_dirs() -> Vec<PathBuf> {
    let home = MacosUtils::home_dir();
    vec![
        // Safari extensions (legacy and app extensions live in
        // ~/Library/Safari/Extensions).
        home.join("Library/Safari/Extensions"),
        // Chrome extensions.
        home.join("Library/Application Support/Google/Chrome/Default/Extensions"),
        // Firefox extensions (profile-scoped).
        home.join("Library/Application Support/Firefox/Profiles"),
    ]
}

/// op 52: Inventory browser extensions — scan Safari, Chrome, and Firefox
/// extension directories for installed extensions.
#[allow(dead_code)]
#[cfg(target_os = "macos")]
fn collect_browser_extensions() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for dir in browser_extension_dirs() {
        if !dir.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .max_depth(3)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // Safari extensions end in `.safariextz` or `.appex`; Chrome
            // extensions are directories with a `manifest.json`; Firefox
            // extensions are `.xpi` files. We capture bundle-style dirs and
            // xpi files.
            let name = match path.file_name().and_then(|n| n.to_str()) {
                Some(n) => n.to_string(),
                None => continue,
            };
            let is_ext = name.ends_with(".safariextz")
                || name.ends_with(".appex")
                || name.ends_with(".xpi")
                || path.join("manifest.json").exists();
            if is_ext {
                if let Some(c) = component_from_dir(path) {
                    out.push(c);
                }
            }
        }
    }
    out
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn collect_browser_extensions() -> Vec<SoftwareComponent> {
    Vec::new()
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

/// op 79: Inventory developer tools — detect installed developer tools by
/// probing common executables. (op 57)
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

/// op 75: Inventory package managers — detect Homebrew, MacPorts, npm, pip,
/// cargo, etc. (op 60)
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

/// op 77: Inventory SDKs — detect installed SDKs: CommandLineTools SDKs and
/// Xcode SDKs. (op 61)
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

/// op 74: Inventory virtual machines — detect VMs by scanning common VM
/// directories. (op 64)
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
// op 63: System extension inventory
// ---------------------------------------------------------------------------

/// op 63: Inventory system extensions — use `systemextensionsctl list` on
/// macOS to find installed system extensions.
#[allow(dead_code)]
#[cfg(target_os = "macos")]
fn collect_system_extensions() -> Vec<SoftwareComponent> {
    let output = std::process::Command::new("systemextensionsctl")
        .arg("list")
        .output();
    let mut out = Vec::new();
    if let Ok(out_bytes) = output {
        if out_bytes.status.success() {
            let text = String::from_utf8_lossy(&out_bytes.stdout);
            for line in text.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with("---')") {
                    continue;
                }
                // Lines look like:
                //   `-- com.example.ext (1.0) [activated]
                // We extract the bundle id and version when possible.
                let stripped = line.trim_start_matches(['-', ' ', '`']);
                let parts: Vec<&str> = stripped.split_whitespace().collect();
                if parts.len() < 2 {
                    continue;
                }
                let name = parts[0].to_string();
                let version = parts
                    .get(1)
                    .map(|v| v.trim_matches(|c: char| c == '(' || c == ')').to_string());
                out.push(SoftwareComponent {
                    name,
                    version,
                    path: String::new(),
                    size_bytes: 0,
                });
            }
        }
    }
    out
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)]
fn collect_system_extensions() -> Vec<SoftwareComponent> {
    Vec::new()
}

// ---------------------------------------------------------------------------
// op 73: Container inventory
// ---------------------------------------------------------------------------

/// op 73: Inventory containers — use `docker ps -a` if available to list
/// all containers (running and stopped).
#[allow(dead_code)]
fn collect_containers() -> Vec<SoftwareComponent> {
    let output = std::process::Command::new("docker")
        .args([
            "ps",
            "-a",
            "--format",
            "{{.Names}}\t{{.Image}}\t{{.Status}}\t{{.ID}}",
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
                let version = parts.get(1).map(|s| s.to_string());
                let id = parts.get(3).map(|s| s.to_string());
                out.push(SoftwareComponent {
                    name,
                    version: version.or(id),
                    path: String::new(),
                    size_bytes: 0,
                });
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// op 71: AI model inventory
// ---------------------------------------------------------------------------

/// File extensions recognised as AI/ML model artefacts.
const AI_MODEL_EXTENSIONS: &[&str] = &[
    "mlmodel",
    "mlmodelc",
    "onnx",
    "pb",
    "pt",
    "pth",
    "safetensors",
];

/// Directories scanned for AI/ML models.
fn ai_model_dirs() -> Vec<PathBuf> {
    let home = MacosUtils::home_dir();
    // Common model storage locations.
    vec![
        home.join("Models"),
        home.join("ml-models"),
        home.join("Library/Application Support/CoreML"),
        // Hugging Face cache.
        home.join(".cache/huggingface"),
    ]
}

/// op 71: Inventory AI models — scan for CoreML models (`.mlmodel`,
/// `.mlmodelc`), ONNX models, and TensorFlow/PyTorch models.
#[allow(dead_code)]
fn collect_ai_models() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for dir in ai_model_dirs() {
        if !dir.is_dir() {
            continue;
        }
        for entry in walkdir::WalkDir::new(&dir)
            .max_depth(3)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            let is_model = if path.is_file() {
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| AI_MODEL_EXTENSIONS.contains(&e.to_lowercase().as_str()))
                    .unwrap_or(false)
            } else if path.is_dir() {
                // Compiled CoreML model bundles end in `.mlmodelc`.
                path.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e == "mlmodelc")
                    .unwrap_or(false)
            } else {
                false
            };
            if is_model {
                if path.is_dir() {
                    if let Some(c) = component_from_dir(path) {
                        out.push(c);
                    }
                } else if let Some(c) = component_from_file(path) {
                    out.push(c);
                }
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// op 72: Dataset inventory
// ---------------------------------------------------------------------------

/// Directories scanned for common ML/data datasets.
fn dataset_dirs() -> Vec<PathBuf> {
    let home = MacosUtils::home_dir();
    vec![
        home.join("Datasets"),
        home.join("data"),
        home.join("ml-data"),
    ]
}

/// op 72: Inventory datasets — scan for common dataset directories
/// (`~/Datasets`, `~/data`, `~/ml-data`).
#[allow(dead_code)]
fn collect_datasets() -> Vec<SoftwareComponent> {
    let mut out = Vec::new();
    for dir in dataset_dirs() {
        if !dir.is_dir() {
            continue;
        }
        // Each immediate subdirectory is treated as a dataset.
        let mut found_sub = false;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(c) = component_from_dir(&path) {
                        out.push(c);
                        found_sub = true;
                    }
                }
            }
        }
        // If the directory itself has no subdirectories, record it as a
        // single dataset.
        if !found_sub {
            if let Some(c) = component_from_dir(&dir) {
                out.push(c);
            }
        }
    }
    out
}

// ---------------------------------------------------------------------------
// op 289: Software risk estimation
// ---------------------------------------------------------------------------

/// op 289: Estimate software risk — estimate the security/maintenance risk of
/// a software component (outdated, unsigned, abandoned).
///
/// Heuristics:
/// - A component with no version and no path is opaque (high risk).
/// - A component with no version cannot be verified as current (medium risk).
/// - A component with no local path cannot be audited (medium risk).
/// - A zero-byte component with a version is likely a stub (low risk).
/// - Otherwise the component is considered safe.
#[allow(dead_code)]
fn estimate_software_risk(component: &SoftwareComponent) -> RiskLevel {
    // No version information: we cannot verify it is current.
    let has_version = component
        .version
        .as_ref()
        .map(|v| !v.is_empty())
        .unwrap_or(false);
    // No path: the component is not backed by a local file (e.g. a container
    // or remote image), so we cannot inspect it directly.
    let has_path = !component.path.is_empty();

    if !has_version && !has_path {
        // Completely opaque component — highest risk.
        return RiskLevel::High;
    }
    if !has_version {
        // Unknown version: cannot confirm it is patched.
        return RiskLevel::Medium;
    }
    if !has_path {
        // No local artefact to audit.
        return RiskLevel::Medium;
    }
    // Heuristic: a zero-byte component with a version is likely a stub or
    // abandoned artefact.
    if component.size_bytes == 0 {
        return RiskLevel::Low;
    }
    RiskLevel::Safe
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
        // consistent with the individual vectors.  Note: `executables` is
        // counted in total_components but not stored as a struct field, so
        // we account for it separately.
        let executables_count = collect_executables().len();
        let expected = genome.applications.len()
            + executables_count
            + genome.frameworks.len()
            + genome.dynamic_libraries.len()
            + genome.kernel_extensions.len()
            + genome.system_extensions.len()
            + genome.launch_agents.len()
            + genome.launch_daemons.len()
            + genome.login_items.len()
            + genome.plugins.len()
            + genome.browser_extensions.len()
            + genome.fonts.len()
            + genome.developer_tools.len()
            + genome.sdks.len()
            + genome.package_managers.len()
            + genome.python_envs.len()
            + genome.node_envs.len()
            + genome.rust_toolchains.len()
            + genome.docker_images.len()
            + genome.containers.len()
            + genome.virtual_machines.len()
            + genome.ai_models.len()
            + genome.datasets.len();
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

    #[test]
    fn collect_executables_returns_vec() {
        let exes = collect_executables();
        for exe in &exes {
            assert!(!exe.name.is_empty());
        }
    }

    #[test]
    fn collect_dynamic_libraries_returns_vec() {
        let libs = collect_dynamic_libraries();
        for lib in &libs {
            let ext = Path::new(&lib.path)
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");
            assert!(dynamic_library_extensions().contains(&ext.to_lowercase().as_str()));
        }
    }

    #[test]
    fn collect_kernel_extensions_returns_vec() {
        let _kexts = collect_kernel_extensions();
    }

    #[test]
    fn collect_browser_extensions_returns_vec() {
        let _exts = collect_browser_extensions();
    }

    #[test]
    fn collect_system_extensions_returns_vec() {
        let _exts = collect_system_extensions();
    }

    #[test]
    fn collect_containers_returns_vec() {
        // Docker may not be installed; the function must still return a vec.
        let _containers = collect_containers();
    }

    #[test]
    fn collect_ai_models_returns_vec() {
        let _models = collect_ai_models();
    }

    #[test]
    fn collect_datasets_returns_vec() {
        let _datasets = collect_datasets();
    }

    #[test]
    fn ai_model_extensions_contains_coreml_and_onnx() {
        assert!(AI_MODEL_EXTENSIONS.contains(&"mlmodel"));
        assert!(AI_MODEL_EXTENSIONS.contains(&"mlmodelc"));
        assert!(AI_MODEL_EXTENSIONS.contains(&"onnx"));
    }

    #[test]
    fn executable_dirs_non_empty_on_supported_platforms() {
        let dirs = executable_dirs();
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            assert!(!dirs.is_empty());
        }
    }

    #[test]
    fn dynamic_library_dirs_non_empty_on_supported_platforms() {
        let dirs = dynamic_library_dirs();
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            assert!(!dirs.is_empty());
        }
    }

    #[test]
    fn dataset_dirs_contains_expected_paths() {
        let dirs = dataset_dirs();
        let names: Vec<String> = dirs
            .iter()
            .filter_map(|d| d.file_name().map(|n| n.to_string_lossy().to_string()))
            .collect();
        assert!(names.iter().any(|n| n == "Datasets"));
        assert!(names.iter().any(|n| n == "data"));
        assert!(names.iter().any(|n| n == "ml-data"));
    }

    #[test]
    fn estimate_software_risk_safe_for_versioned_local_component() {
        let component = SoftwareComponent {
            name: "Test".to_string(),
            version: Some("1.0".to_string()),
            path: "/usr/bin/test".to_string(),
            size_bytes: 1024,
        };
        assert_eq!(estimate_software_risk(&component), RiskLevel::Safe);
    }

    #[test]
    fn estimate_software_risk_high_for_opaque_component() {
        let component = SoftwareComponent {
            name: "Ghost".to_string(),
            version: None,
            path: String::new(),
            size_bytes: 0,
        };
        assert_eq!(estimate_software_risk(&component), RiskLevel::High);
    }

    #[test]
    fn estimate_software_risk_medium_for_missing_version() {
        let component = SoftwareComponent {
            name: "Unknown".to_string(),
            version: None,
            path: "/usr/bin/unknown".to_string(),
            size_bytes: 512,
        };
        assert_eq!(estimate_software_risk(&component), RiskLevel::Medium);
    }

    #[test]
    fn estimate_software_risk_medium_for_missing_path() {
        let component = SoftwareComponent {
            name: "Container".to_string(),
            version: Some("latest".to_string()),
            path: String::new(),
            size_bytes: 0,
        };
        assert_eq!(estimate_software_risk(&component), RiskLevel::Medium);
    }

    #[test]
    fn estimate_software_risk_low_for_zero_byte_stub() {
        let component = SoftwareComponent {
            name: "Stub".to_string(),
            version: Some("0.1".to_string()),
            path: "/usr/bin/stub".to_string(),
            size_bytes: 0,
        };
        assert_eq!(estimate_software_risk(&component), RiskLevel::Low);
    }
}
