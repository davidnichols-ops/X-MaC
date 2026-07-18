use serde::{Deserialize, Serialize};

/// App-specific optimization profiles that tune cleanup and memory
/// behavior for different usage scenarios.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum OptimizationProfile {
    /// Balanced everyday use — safe defaults.
    #[default]
    Balanced,
    /// Maximize free RAM and CPU for games.
    Gaming,
    /// Clear build artifacts aggressively, keep dev tools responsive.
    Development,
    /// Free maximum memory for video/audio editing.
    VideoEditing,
    /// Conservative — minimal intervention, maximum safety.
    Conservative,
    /// Aggressive cleanup — reclaim every possible byte.
    Aggressive,
    /// User-defined custom profile.
    Custom,
}

#[allow(dead_code)]
impl OptimizationProfile {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Balanced => "Balanced",
            Self::Gaming => "Gaming",
            Self::Development => "Development",
            Self::VideoEditing => "Video Editing",
            Self::Conservative => "Conservative",
            Self::Aggressive => "Aggressive",
            Self::Custom => "Custom",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Balanced => "Safe defaults for everyday use — cleans caches, purges inactive memory, runs light maintenance.",
            Self::Gaming => "Maximizes free RAM and CPU for games. Kills non-essential background apps, purges memory aggressively.",
            Self::Development => "Clears build artifacts (node_modules, target, __pycache__) aggressively. Keeps dev tools fast.",
            Self::VideoEditing => "Frees maximum memory for video/audio editing. Purges caches and inactive memory aggressively.",
            Self::Conservative => "Minimal intervention — only the safest operations. Ideal for production machines.",
            Self::Aggressive => "Reclaims every possible byte of disk and RAM. Use when you need maximum space.",
            Self::Custom => "User-defined settings configured via config.toml.",
        }
    }

    /// Recommended min age for file cleanup (days).
    pub fn min_age_days(&self) -> u64 {
        match self {
            Self::Conservative => 90,
            Self::Balanced => 30,
            Self::Development | Self::Aggressive => 7,
            Self::Gaming | Self::VideoEditing | Self::Custom => 14,
        }
    }

    /// Whether to purge inactive memory aggressively.
    pub fn aggressive_purge(&self) -> bool {
        matches!(self, Self::Gaming | Self::VideoEditing | Self::Aggressive)
    }

    /// Whether to kill non-essential background processes.
    pub fn kill_background(&self) -> bool {
        matches!(self, Self::Gaming | Self::VideoEditing)
    }

    /// Whether to clear build artifacts.
    pub fn clear_build_artifacts(&self) -> bool {
        matches!(self, Self::Development | Self::Aggressive)
    }

    /// Daemon check interval in seconds.
    pub fn daemon_interval_secs(&self) -> u64 {
        match self {
            Self::Gaming | Self::VideoEditing => 60,
            Self::Aggressive => 120,
            Self::Balanced | Self::Development => 300,
            Self::Conservative | Self::Custom => 900,
        }
    }

    /// Memory pressure threshold (0.0–1.0) to trigger proactive action.
    pub fn pressure_threshold(&self) -> f64 {
        match self {
            Self::Conservative => 0.90,
            Self::Balanced => 0.80,
            Self::Development => 0.75,
            Self::Gaming | Self::VideoEditing => 0.65,
            Self::Aggressive => 0.60,
            Self::Custom => 0.80,
        }
    }

    /// Minimum file size in bytes for duplicate detection.
    pub fn dedup_min_size(&self) -> u64 {
        match self {
            Self::Conservative => 10 * 1024 * 1024,
            Self::Balanced | Self::Development | Self::Gaming | Self::VideoEditing | Self::Custom => {
                1024
            }
            Self::Aggressive => 1024,
        }
    }

    /// Whether duplicate detection is enabled by default for this profile.
    pub fn dedup_enabled(&self) -> bool {
        match self {
            Self::Conservative => false,
            Self::Balanced | Self::Development | Self::Gaming | Self::VideoEditing
            | Self::Aggressive | Self::Custom => true,
        }
    }

    /// Whether similar-image detection (perceptual hashing) is enabled.
    pub fn dedup_similar_images(&self) -> bool {
        matches!(self, Self::Aggressive)
    }
}

/// A named preset that can be applied to the config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfilePreset {
    pub name: String,
    pub profile: OptimizationProfile,
    pub description: String,
}

impl ProfilePreset {
    pub fn all() -> Vec<ProfilePreset> {
        vec![
            ProfilePreset {
                name: "Balanced".to_string(),
                profile: OptimizationProfile::Balanced,
                description: OptimizationProfile::Balanced.description().to_string(),
            },
            ProfilePreset {
                name: "Gaming".to_string(),
                profile: OptimizationProfile::Gaming,
                description: OptimizationProfile::Gaming.description().to_string(),
            },
            ProfilePreset {
                name: "Development".to_string(),
                profile: OptimizationProfile::Development,
                description: OptimizationProfile::Development.description().to_string(),
            },
            ProfilePreset {
                name: "Video Editing".to_string(),
                profile: OptimizationProfile::VideoEditing,
                description: OptimizationProfile::VideoEditing.description().to_string(),
            },
            ProfilePreset {
                name: "Conservative".to_string(),
                profile: OptimizationProfile::Conservative,
                description: OptimizationProfile::Conservative.description().to_string(),
            },
            ProfilePreset {
                name: "Aggressive".to_string(),
                profile: OptimizationProfile::Aggressive,
                description: OptimizationProfile::Aggressive.description().to_string(),
            },
        ]
    }
}
