use std::path::PathBuf;

use crate::core::types::Category;
use crate::util::macos::MacosUtils;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct CleanRule {
    pub name: &'static str,
    pub pattern: &'static str,
    pub description: &'static str,
    pub category: Category,
}

#[allow(dead_code)]
pub struct CleanRules {
    pub rules: Vec<CleanRule>,
}

impl Default for CleanRules {
    fn default() -> Self {
        Self {
            rules: vec![
                CleanRule {
                    name: "xcode_derived_data",
                    pattern: "Library/Developer/Xcode/DerivedData/*",
                    description: "Xcode build artifacts",
                    category: Category::XcodeArtifact,
                },
                CleanRule {
                    name: "xcode_archives",
                    pattern: "Library/Developer/Xcode/Archives/*",
                    description: "Xcode archives",
                    category: Category::XcodeArtifact,
                },
                CleanRule {
                    name: "xcode_device_support",
                    pattern: "Library/Developer/Xcode/iOS DeviceSupport/*",
                    description: "iOS device support files",
                    category: Category::XcodeArtifact,
                },
                CleanRule {
                    name: "home_caches",
                    pattern: "Library/Caches/*",
                    description: "User cache files",
                    category: Category::Cache,
                },
                CleanRule {
                    name: "system_caches",
                    pattern: "/Library/Caches/*",
                    description: "System cache files",
                    category: Category::Cache,
                },
                CleanRule {
                    name: "home_logs",
                    pattern: "Library/Logs/*",
                    description: "User log files",
                    category: Category::Log,
                },
                CleanRule {
                    name: "system_logs",
                    pattern: "/var/log/*",
                    description: "System log files",
                    category: Category::Log,
                },
            ],
        }
    }
}

impl CleanRules {
    pub fn cache_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        vec![
            home.join("Library/Caches"),
            PathBuf::from("/Library/Caches"),
        ]
    }

    pub fn xcode_paths(&self) -> Vec<PathBuf> {
        let home = MacosUtils::home_dir();
        vec![
            home.join("Library/Developer/Xcode/DerivedData"),
            home.join("Library/Developer/Xcode/Archives"),
            home.join("Library/Developer/Xcode/iOS DeviceSupport"),
        ]
    }
}
