use std::process::Command;

use crate::core::context::ScanContext;
use crate::core::types::{Category, EngineId, Finding, Severity, Target};

pub struct PortConflictScanner {
    ports: Vec<u16>,
}

impl PortConflictScanner {
    pub fn new(ports: Vec<u16>) -> Self {
        Self { ports }
    }

    pub async fn scan(&self, _ctx: &ScanContext) -> anyhow::Result<Vec<Finding>> {
        let mut findings = Vec::new();

        for &port in &self.ports {
            if let Some(process_info) = self.check_port(port).await? {
                findings.push(
                    Finding::new(
                        EngineId::Conflict,
                        Severity::Medium,
                        Category::PortConflict,
                        Target::Port(port),
                        format!("Port {} is in use", port),
                        format!(
                            "Process {} (PID {}) is listening on port {}",
                            process_info.name, process_info.pid, port
                        ),
                    )
                    .with_metadata("pid".to_string(), serde_json::json!(process_info.pid))
                    .with_hint(format!(
                        "Kill process {} with 'kill {}' or use a different port",
                        process_info.pid, process_info.pid
                    )),
                );
            }
        }

        Ok(findings)
    }

    async fn check_port(&self, port: u16) -> anyhow::Result<Option<ProcessInfo>> {
        let output = Command::new("lsof")
            .args(["-i", &format!(":{}", port), "-P", "-n"])
            .output();

        match output {
            Ok(out) => {
                let output_str = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = output_str.lines().collect();

                if let Some(line) = lines.get(1) {
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if let Some(pid_str) = parts.get(1) {
                        let pid = pid_str.parse::<u32>().unwrap_or(0);
                        let name = parts[0].to_string();

                        return Ok(Some(ProcessInfo { pid, name }));
                    }
                }
                Ok(None)
            }
            Err(_) => Ok(None),
        }
    }
}

struct ProcessInfo {
    pid: u32,
    name: String,
}

impl Default for PortConflictScanner {
    fn default() -> Self {
        Self {
            ports: vec![3000, 5000, 8000, 8080, 9000],
        }
    }
}
