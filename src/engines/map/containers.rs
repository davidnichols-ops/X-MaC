use std::path::PathBuf;
use std::process::Command;

use crate::core::context::ScanContext;
use crate::core::types::{Category, EngineId, Finding, Severity, Target};
use crate::util::macos::MacosUtils;
use crate::util::disk;

pub struct ContainerScanner;

impl ContainerScanner {
    pub fn new() -> Self {
        Self
    }

    pub async fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();

        if Self::is_docker_installed() {
            let docker_info = Self::get_docker_info().await;

            findings.push(
                Finding::new(
                    EngineId::Map,
                    Severity::Info,
                    Category::ContainerRuntime,
                    Target::Path(PathBuf::from("/var/run/docker.sock")),
                    "Docker installed",
                    "Docker is installed on this system",
                )
                .with_metadata("runtime".to_string(), serde_json::json!("docker"))
                .with_metadata("running".to_string(), serde_json::json!(docker_info.as_ref().map(|i| i.running).unwrap_or(false))),
            );

            if let Some(info) = docker_info {
                if info.running {
                    let containers = Self::get_docker_containers().await;
                    let images_size = Self::get_docker_images_size().await;

                    findings.push(
                        Finding::new(
                            EngineId::Map,
                            Severity::Info,
                            Category::ContainerRuntime,
                            Target::Path(PathBuf::from("/var/run/docker.sock")),
                            format!("Docker running with {} container(s)", containers.len()),
                            format!("Active containers: {}", containers.join(", ")),
                        )
                        .with_metadata("containers".to_string(), serde_json::json!(containers))
                        .with_size(images_size),
                    );
                }
            }
        }

        if Self::is_colima_installed() {
            findings.push(
                Finding::new(
                    EngineId::Map,
                    Severity::Info,
                    Category::ContainerRuntime,
                    Target::Path(MacosUtils::home_dir().join(".colima")),
                    "Colima installed",
                    "Colima container runtime is installed",
                )
                .with_metadata("runtime".to_string(), serde_json::json!("colima")),
            );
        }

        if Self::is_lima_installed() {
            let lima_instances = Self::get_lima_instances();

            findings.push(
                Finding::new(
                    EngineId::Map,
                    Severity::Info,
                    Category::ContainerRuntime,
                    Target::Path(MacosUtils::home_dir().join(".lima")),
                    format!("Lima installed ({} instance(s))", lima_instances.len()),
                    "Lima container runtime is installed",
                )
                .with_metadata("runtime".to_string(), serde_json::json!("lima"))
                .with_metadata("instances".to_string(), serde_json::json!(lima_instances)),
            );
        }

        if Self::is_orbstack_installed() {
            findings.push(
                Finding::new(
                    EngineId::Map,
                    Severity::Info,
                    Category::ContainerRuntime,
                    Target::Path(MacosUtils::home_dir().join(".orbstack")),
                    "OrbStack installed",
                    "OrbStack container runtime is installed",
                )
                .with_metadata("runtime".to_string(), serde_json::json!("orbstack")),
            );
        }

        if Self::is_podman_installed() {
            findings.push(
                Finding::new(
                    EngineId::Map,
                    Severity::Info,
                    Category::ContainerRuntime,
                    Target::Path(MacosUtils::home_dir().join(".local/share/containers")),
                    "Podman installed",
                    "Podman container runtime is installed",
                )
                .with_metadata("runtime".to_string(), serde_json::json!("podman")),
            );
        }

        Ok(findings)
    }

    fn is_docker_running() -> bool {
        std::path::Path::new("/var/run/docker.sock").exists()
    }

    fn is_docker_installed() -> bool {
        let output = Command::new("docker").arg("--version").output();
        output.is_ok()
    }

    async fn get_docker_info() -> Option<DockerInfo> {
        let output = Command::new("docker")
            .args(["info", "--format", "{{.ServerVersion}}"])
            .output()
            .ok()?;

        if output.status.success() {
            Some(DockerInfo {
                version: String::from_utf8_lossy(&output.stdout).trim().to_string(),
                running: Self::is_docker_running(),
            })
        } else {
            None
        }
    }

    async fn get_docker_containers() -> Vec<String> {
        let output = Command::new("docker")
            .args(["ps", "--format", "{{.Names}}"])
            .output()
            .ok();

        match output {
            Some(out) if out.status.success() => {
                String::from_utf8_lossy(&out.stdout)
                    .lines()
                    .map(|s| s.to_string())
                    .collect()
            }
            _ => Vec::new(),
        }
    }

    async fn get_docker_images_size() -> u64 {
        let output = Command::new("docker")
            .args(["system", "df", "--format", "{{.Size}}"])
            .output()
            .ok();

        match output {
            Some(out) if out.status.success() => {
                let size_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                Self::parse_docker_size(&size_str)
            }
            _ => 0,
        }
    }

    fn parse_docker_size(size_str: &str) -> u64 {
        let size_str = size_str.replace("GB", "").replace("MB", "").trim().to_string();
        size_str.parse::<f64>().unwrap_or(0.0) as u64 * 1024 * 1024 * 1024
    }

    fn is_colima_installed() -> bool {
        MacosUtils::home_dir().join(".colima").exists()
    }

    fn is_lima_installed() -> bool {
        MacosUtils::home_dir().join(".lima").exists()
    }

    fn get_lima_instances() -> Vec<String> {
        let lima_dir = MacosUtils::home_dir().join(".lima");
        let mut instances = Vec::new();

        if let Ok(entries) = std::fs::read_dir(&lima_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy().to_string();
                        if !name_str.starts_with('.') && name_str != "_config" {
                            instances.push(name_str);
                        }
                    }
                }
            }
        }

        instances
    }

    fn is_orbstack_installed() -> bool {
        MacosUtils::home_dir().join(".orbstack").exists()
    }

    fn is_podman_installed() -> bool {
        MacosUtils::home_dir().join(".local/share/containers").exists()
    }
}

struct DockerInfo {
    version: String,
    running: bool,
}

impl Default for ContainerScanner {
    fn default() -> Self {
        Self::new()
    }
}
