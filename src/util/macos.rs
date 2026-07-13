use std::path::{Path, PathBuf};
use std::process::Command;

pub struct MacosUtils;

#[allow(dead_code)]
impl MacosUtils {
    pub fn new() -> Self {
        Self
    }

    pub fn resolve_firmlink(path: &Path) -> PathBuf {
        let path_str = path.to_string_lossy();
        if path_str.starts_with("/var") {
            PathBuf::from(path_str.replace("/var", "/private/var"))
        } else if path_str.starts_with("/tmp") {
            PathBuf::from(path_str.replace("/tmp", "/private/tmp"))
        } else {
            path.to_path_buf()
        }
    }

    pub fn has_full_disk_access() -> bool {
        let test_path = PathBuf::from("/Library/Application Support/com.apple.TCC");
        std::fs::read_dir(&test_path).is_ok()
    }

    pub fn get_macos_version() -> String {
        if cfg!(target_os = "macos") {
            let output = Command::new("sw_vers").arg("-productVersion").output();

            match output {
                Ok(out) => {
                    let version = String::from_utf8_lossy(&out.stdout).trim().to_string();
                    if version.is_empty() {
                        "unknown".to_string()
                    } else {
                        version
                    }
                }
                Err(_) => "unknown".to_string(),
            }
        } else if cfg!(target_os = "linux") {
            // Read /etc/os-release for the Linux distribution version
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if let Some(v) = line.strip_prefix("PRETTY_NAME=") {
                        return v.trim_matches('"').to_string();
                    }
                }
                for line in content.lines() {
                    if let Some(v) = line.strip_prefix("VERSION=") {
                        return v.trim_matches('"').to_string();
                    }
                }
            }
            "Linux (unknown distro)".to_string()
        } else {
            "unknown".to_string()
        }
    }

    pub fn is_apple_silicon() -> bool {
        cfg!(target_os = "macos") && cfg!(target_arch = "aarch64")
    }

    pub fn home_dir() -> PathBuf {
        std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("/tmp"))
    }

    pub fn get_library_dir() -> PathBuf {
        Self::home_dir().join("Library")
    }

    pub fn get_application_support_dir() -> PathBuf {
        Self::get_library_dir().join("Application Support")
    }

    pub fn get_caches_dir() -> PathBuf {
        Self::get_library_dir().join("Caches")
    }

    pub fn get_logs_dir() -> PathBuf {
        Self::get_library_dir().join("Logs")
    }

    pub fn get_xcode_derived_data_dir() -> PathBuf {
        Self::get_library_dir().join("Developer/Xcode/DerivedData")
    }

    pub fn get_xcode_archives_dir() -> PathBuf {
        Self::get_library_dir().join("Developer/Xcode/Archives")
    }

    pub fn get_xcode_device_support_dir() -> PathBuf {
        Self::get_library_dir().join("Developer/Xcode/iOS DeviceSupport")
    }
}

impl Default for MacosUtils {
    fn default() -> Self {
        Self::new()
    }
}
