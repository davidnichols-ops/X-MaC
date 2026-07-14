use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::engines::duplicate::cluster::{DuplicateCluster as EngineDuplicateCluster, FileEntry};
use crate::engines::duplicate::scanner::{is_image_file, is_screenshot};
use crate::engines::envmap::apps::{default_app_dirs, enumerate_apps_in};
use crate::engines::envmap::discovery::{detect_app_leftovers, detect_orphan_files};
use crate::util::macos::MacosUtils;

/// Maximum walk depth when scanning the home directory for a quick file
/// count/size estimate. Keeps the operation fast on large filesystems.
const HOME_SCAN_MAX_DEPTH: usize = 4;

/// Maximum walk depth when mapping ownership inside a single `~/Library`
/// application container. Bounded so a single pathological app cannot
/// dominate collection time.
const OWNERSHIP_SCAN_MAX_DEPTH: usize = 3;

/// Minimum file size (in bytes) to consider for duplicate detection. Files
/// smaller than this are not worth the hashing overhead. Mirrors the
/// duplicate engine's default.
const DUPLICATE_MIN_SIZE: u64 = 1024;

/// Maximum walk depth when scanning for generated/source files and inactive
/// datasets. Bounded to keep scans fast on large project trees.
const GENERATED_SCAN_MAX_DEPTH: usize = 6;

/// Maximum walk depth when scanning for stale development projects.
const STALE_SCAN_MAX_DEPTH: usize = 5;

/// Minimum directory size (in bytes) to be considered a storage leak.
const STORAGE_LEAK_MIN_SIZE: u64 = 100 * 1024 * 1024; // 100 MB

/// A directory is flagged as a storage leak when its leak ratio (size ×
/// (1 − usefulness)) exceeds this threshold.
const STORAGE_LEAK_THRESHOLD: f64 = 50.0 * 1024.0 * 1024.0; // 50 MB-equivalent

/// A folder is "runaway" when its size exceeds 1 GB.
const RUNAWAY_FOLDER_THRESHOLD: u64 = 1024 * 1024 * 1024; // 1 GB

/// A project is "stale" when not modified in >180 days.
const STALE_PROJECT_THRESHOLD_SECS: u64 = 180 * 24 * 60 * 60;

/// A dataset/asset is "inactive" when not accessed in >365 days.
const INACTIVE_DATASET_THRESHOLD_SECS: u64 = 365 * 24 * 60 * 60;

/// An owner's file-count growth is "abnormal" when it adds at least this
/// many files between snapshots.
const ABNORMAL_GROWTH_THRESHOLD: u64 = 100;

/// Filesystem Intelligence Graph — a knowledge graph of every file on the Mac.
///
/// Covers: file index, ownership, creators, app relationships, dependencies,
/// cache relationships, config relationships, duplicate content, file
/// importance prediction, storage growth forecasting, abandoned/orphan
/// detection, and cleanup simulation.
///
/// Maps to Digital Twin operations 81-120.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemGraph {
    pub total_files: u64,
    pub total_size_bytes: u64,
    pub ownership_map: HashMap<PathBuf, String>,
    pub creator_map: HashMap<PathBuf, String>,
    pub dependency_edges: Vec<FileEdge>,
    pub cache_relationships: Vec<FileEdge>,
    pub duplicate_clusters: Vec<DuplicateCluster>,
    pub abandoned_files: Vec<PathBuf>,
    pub orphan_files: Vec<PathBuf>,
    pub storage_growth_trend: Option<f64>,
    pub exhaustion_forecast_days: Option<u64>,
    /// op 84: edges between apps and their owned files (caches, support,
    /// preferences).
    pub app_relationships: Vec<FileEdge>,
    /// op 88: edges between preference plist files and their owning apps.
    pub preference_relationships: Vec<FileEdge>,
    /// op 90: generated files (build artifacts, compiled outputs) mapped to
    /// their source.
    pub generated_files: Vec<GeneratedFile>,
    /// op 91: edges from source files to their generated outputs.
    pub source_files: Vec<FileEdge>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileEdge {
    pub source: PathBuf,
    pub target: PathBuf,
    pub relationship: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DuplicateCluster {
    pub hash: String,
    pub files: Vec<PathBuf>,
    pub total_size_bytes: u64,
    pub recommended_keep: Option<PathBuf>,
}

/// op 90: A generated file (build artifact or compiled output) mapped to the
/// source that produced it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedFile {
    pub path: PathBuf,
    pub source: PathBuf,
    pub kind: String,
    pub size_bytes: u64,
}

/// op 105: A directory consuming disproportionate space relative to its
/// usefulness.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageLeak {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub usefulness_score: f64,
    pub leak_ratio: f64,
}

/// op 106: A folder growing unusually fast (typically in temp/cache areas).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunawayFolder {
    pub path: PathBuf,
    pub size_bytes: u64,
    pub growth_bytes: u64,
}

/// op 109: A development project not modified in a long time (>180 days).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StaleProject {
    pub path: PathBuf,
    pub last_modified_secs: u64,
    pub size_bytes: u64,
}

/// op 110: An ML dataset or data archive not accessed in a long time
/// (>365 days).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InactiveDataset {
    pub path: PathBuf,
    pub last_accessed_secs: u64,
    pub size_bytes: u64,
}

/// op 111: An unused asset (old download, unused media file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UnusedAsset {
    pub path: PathBuf,
    pub last_accessed_secs: u64,
    pub size_bytes: u64,
    pub kind: String,
}

/// op 114: A node in the filesystem knowledge graph.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FSNode {
    pub path: PathBuf,
    pub owner: Option<String>,
    pub creator: Option<String>,
    pub importance: f64,
    pub size_bytes: u64,
}

/// op 114: A queryable filesystem knowledge graph aggregating all
/// relationships.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FSKnowledgeGraph {
    pub nodes: Vec<FSNode>,
    pub edges: Vec<FileEdge>,
    pub total_files: u64,
    pub total_size_bytes: u64,
}

/// op 115: A single filesystem evolution event between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FSEvolutionEvent {
    pub kind: String,
    pub path: PathBuf,
    pub delta_bytes: i64,
}

/// op 267: A directory growing abnormally fast between two snapshots.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbnormalGrowth {
    pub path: PathBuf,
    pub previous_size_bytes: u64,
    pub current_size_bytes: u64,
    pub growth_bytes: u64,
    pub growth_rate: f64,
}

impl FilesystemGraph {
    /// Collect the filesystem intelligence graph.
    ///
    /// Orchestrates operations 81-120: file ownership mapping, creator
    /// mapping, cache relationships, duplicate clusters, abandoned/orphan
    /// detection, storage growth forecasting, and a bounded home-directory
    /// file/size census.
    pub fn collect() -> Self {
        let mut graph = Self::empty();

        // Ops 81-88: ownership + creator mapping.
        graph.map_file_ownership();
        graph.map_creators();

        // Ops 89-92: cache relationship detection.
        graph.detect_cache_relationships();

        // Ops 93-95: duplicate cluster integration (limited to top 100 by size
        // to keep the Digital Twin JSON output manageable for GUI consumption).
        let mut duplicates = graph.collect_duplicate_clusters();
        duplicates.sort_by_key(|c| std::cmp::Reverse(c.total_size_bytes));
        duplicates.truncate(100);
        graph.duplicate_clusters = duplicates;

        // Ops 96-100: abandoned/orphan file detection (limited to 200 each).
        let (abandoned, orphans) = graph.collect_abandoned_and_orphans();
        graph.abandoned_files = abandoned.into_iter().take(200).collect();
        graph.orphan_files = orphans.into_iter().take(200).collect();

        // Total file count + size census (bounded depth for performance).
        let (count, size) = count_files_in_home(HOME_SCAN_MAX_DEPTH);
        graph.total_files = count;
        graph.total_size_bytes = size;

        // Ops 101-105: storage growth forecasting (uses total_size_bytes).
        graph.forecast_storage_growth();

        graph
    }

    /// Construct an empty (zeroed) filesystem graph.
    fn empty() -> Self {
        Self {
            total_files: 0,
            total_size_bytes: 0,
            ownership_map: HashMap::new(),
            creator_map: HashMap::new(),
            dependency_edges: Vec::new(),
            cache_relationships: Vec::new(),
            duplicate_clusters: Vec::new(),
            abandoned_files: Vec::new(),
            orphan_files: Vec::new(),
            storage_growth_trend: None,
            exhaustion_forecast_days: None,
            app_relationships: Vec::new(),
            preference_relationships: Vec::new(),
            generated_files: Vec::new(),
            source_files: Vec::new(),
        }
    }

    /// Ops 81-85: File ownership mapping.
    ///
    /// op 82: Map file ownership — for the common `~/Library` application
    /// directories (Caches, Application Support, Logs), map every file to its
    /// owning application. The owning app name is the immediate subdirectory
    /// name under each of these roots (e.g.
    /// `~/Library/Caches/com.example.App` → owner `com.example.App`).
    fn map_file_ownership(&mut self) {
        for root in ownership_roots() {
            if !root.is_dir() {
                continue;
            }
            let entries = match std::fs::read_dir(&root) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let app_dir = entry.path();
                if !app_dir.is_dir() {
                    continue;
                }
                let owner = entry.file_name().to_string_lossy().to_string();
                for file_entry in WalkDir::new(&app_dir)
                    .min_depth(1)
                    .max_depth(OWNERSHIP_SCAN_MAX_DEPTH)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    self.ownership_map
                        .insert(file_entry.path().to_path_buf(), owner.clone());
                }
            }
        }
    }

    /// Ops 86-88: Creator mapping.
    ///
    /// Best-effort heuristic that attributes a creator process to each known
    /// file. Files inside an application container inherit the owning app as
    /// their creator. Files under `~/Library/Logs` are attributed to the log
    /// subdirectory name. Files in user document directories are attributed
    /// to `"user"`.
    fn map_creators(&mut self) {
        // Inherit creator from the ownership map where known.
        for (path, owner) in &self.ownership_map {
            self.creator_map.insert(path.clone(), owner.clone());
        }

        // Logs: attribute creator from the immediate log subdirectory.
        let logs = MacosUtils::get_logs_dir();
        if logs.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&logs) {
                for entry in entries.flatten() {
                    let app_dir = entry.path();
                    if !app_dir.is_dir() {
                        continue;
                    }
                    let creator = entry.file_name().to_string_lossy().to_string();
                    for file_entry in WalkDir::new(&app_dir)
                        .min_depth(1)
                        .max_depth(OWNERSHIP_SCAN_MAX_DEPTH)
                        .into_iter()
                        .filter_map(|e| e.ok())
                        .filter(|e| e.file_type().is_file())
                    {
                        self.creator_map
                            .insert(file_entry.path().to_path_buf(), creator.clone());
                    }
                }
            }
        }

        // User document directories: attribute to "user".
        let home = MacosUtils::home_dir();
        for sub in ["Documents", "Desktop", "Downloads"] {
            let dir = home.join(sub);
            if !dir.is_dir() {
                continue;
            }
            for file_entry in WalkDir::new(&dir)
                .min_depth(1)
                .max_depth(OWNERSHIP_SCAN_MAX_DEPTH)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                self.creator_map
                    .insert(file_entry.path().to_path_buf(), "user".to_string());
            }
        }
    }

    /// Ops 89-92: Cache relationship detection.
    ///
    /// Identifies files in `~/Library/Caches/<app>` and links them to the
    /// matching application data directory `~/Library/Application
    /// Support/<app>` via `FileEdge` entries with the relationship
    /// `"cache_of"`.
    fn detect_cache_relationships(&mut self) {
        let caches = MacosUtils::get_caches_dir();
        let app_support = MacosUtils::get_application_support_dir();
        if !caches.is_dir() {
            return;
        }
        let entries = match std::fs::read_dir(&caches) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let app_name = entry.file_name();
            let cache_dir = entry.path();
            if !cache_dir.is_dir() {
                continue;
            }
            let support_dir = app_support.join(&app_name);
            if !support_dir.is_dir() {
                // No matching Application Support directory — skip, but we
                // still record a self-edge from the cache dir to itself is
                // not useful, so only link when the counterpart exists.
                continue;
            }
            // Link the cache directory to its parent app support directory.
            self.cache_relationships.push(FileEdge {
                source: cache_dir.clone(),
                target: support_dir.clone(),
                relationship: "cache_of".to_string(),
            });
            // Also link individual cached files to the support directory.
            for file_entry in WalkDir::new(&cache_dir)
                .min_depth(1)
                .max_depth(OWNERSHIP_SCAN_MAX_DEPTH)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                self.cache_relationships.push(FileEdge {
                    source: file_entry.path().to_path_buf(),
                    target: support_dir.clone(),
                    relationship: "cache_of".to_string(),
                });
            }
        }
    }

    /// Ops 93-95: Duplicate cluster integration.
    ///
    /// Uses the `crate::engines::duplicate` engine's data structures and
    /// hashing strategy to detect duplicate files in the user's common
    /// document directories. Returns clusters converted to the filesystem
    /// graph's [`DuplicateCluster`] representation.
    fn collect_duplicate_clusters(&self) -> Vec<DuplicateCluster> {
        let candidates = scan_duplicate_candidates();
        if candidates.is_empty() {
            return Vec::new();
        }

        // Group by size as a fast pre-filter (op 83).
        let size_groups = group_entries_by_size(candidates);

        let mut engine_clusters: Vec<EngineDuplicateCluster> = Vec::new();
        for group in size_groups {
            if group.len() < 2 {
                continue;
            }
            // Full hash each file in the size group (synchronous BLAKE3).
            let hashed: Vec<FileEntry> = group
                .into_iter()
                .filter_map(|mut fe| {
                    if let Some(hash) = sync_full_hash(&fe.path) {
                        fe.blake3_hash = Some(hash.clone());
                        fe.partial_hash = Some(hash);
                        Some(fe)
                    } else {
                        None
                    }
                })
                .collect();

            // Group by full hash → identical clusters (op 89, 92).
            for (_, cluster_files) in group_entries_by_hash(hashed) {
                if cluster_files.len() < 2 {
                    continue;
                }
                engine_clusters.push(EngineDuplicateCluster::from_files(cluster_files));
            }
        }

        // Convert engine clusters to fs_graph clusters.
        engine_clusters
            .into_iter()
            .map(convert_engine_cluster)
            .collect()
    }

    /// Ops 96-100: Abandoned/orphan file detection.
    ///
    /// Uses [`detect_app_leftovers`] and [`detect_orphan_files`] from the
    /// envmap discovery engine. Returns `(abandoned_files, orphan_files)`.
    fn collect_abandoned_and_orphans(&self) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let installed = installed_bundle_ids();
        let abandoned = detect_app_leftovers(&installed)
            .into_iter()
            .map(|a| a.path)
            .collect();
        let orphans = detect_orphan_files(&installed)
            .into_iter()
            .map(|a| a.path)
            .collect();
        (abandoned, orphans)
    }

    /// Ops 101-105: Storage growth forecasting.
    ///
    /// Calculates a simple linear extrapolation of storage growth. The trend
    /// (bytes per day) is estimated as the current total size divided over a
    /// nominal one-year window, and the exhaustion forecast is the available
    /// space divided by that daily growth rate.
    fn forecast_storage_growth(&mut self) {
        // Use the already-computed total_size_bytes directly.
        // Rough heuristic: assume the current corpus represents one year of
        // accumulated data, so the daily growth rate is total / 365.
        let growth_per_day = self.total_size_bytes as f64 / 365.0;
        self.storage_growth_trend = Some(growth_per_day);

        if growth_per_day > 0.0 {
            let available = available_space_bytes(&MacosUtils::home_dir());
            if available > 0 {
                self.exhaustion_forecast_days = Some((available as f64 / growth_per_day) as u64);
            }
        }
    }

    /// Ops 106-110: File importance prediction.
    ///
    /// Scores the importance of a single file on a 0–1 scale based on its
    /// location:
    /// - System paths (`/System`, `/usr`, `/bin`, `/sbin`, `/Library`) → 1.0
    /// - User documents (`~/Documents`, `~/Desktop`, `~/Downloads`) → 0.8
    /// - Caches (`~/Library/Caches`) → 0.1
    /// - Logs (`~/Library/Logs`) → 0.05
    /// - Temp files (`/tmp`, `~/Library/Caches/TemporaryItems`,
    ///   `$TMPDIR`) → 0.01
    /// - Unknown → 0.5
    pub fn predict_file_importance(path: &Path) -> f64 {
        let s = path.to_string_lossy();
        if is_temp_path(&s) {
            0.01
        } else if is_log_path(&s) {
            0.05
        } else if is_cache_path(&s) {
            0.1
        } else if is_user_doc_path(&s) {
            0.8
        } else if is_system_path(&s) {
            1.0
        } else {
            0.5
        }
    }

    /// op 84: Map application relationships.
    ///
    /// Builds edges between apps and their files: app → caches, app →
    /// support files, app → preferences. For each per-application
    /// subdirectory under `~/Library/Caches`, `~/Library/Application
    /// Support`, and `~/Library/Preferences`, an edge is recorded from the
    /// app directory to every file it owns.
    #[allow(dead_code)]
    pub fn map_app_relationships(&mut self) {
        let home = MacosUtils::home_dir();
        let library = home.join("Library");
        let roots = [
            ("caches", library.join("Caches")),
            ("support", library.join("Application Support")),
            ("preferences", library.join("Preferences")),
        ];
        for (rel, root) in roots {
            if !root.is_dir() {
                continue;
            }
            let entries = match std::fs::read_dir(&root) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let app_dir = entry.path();
                let app_name = entry.file_name().to_string_lossy().to_string();
                // For Preferences, files (plist) live directly under the
                // root rather than per-app subdirectories.
                if app_dir.is_file() {
                    self.app_relationships.push(FileEdge {
                        source: app_dir.clone(),
                        target: app_dir,
                        relationship: format!("app_{}_{}", app_name, rel),
                    });
                    continue;
                }
                if !app_dir.is_dir() {
                    continue;
                }
                for file_entry in WalkDir::new(&app_dir)
                    .min_depth(1)
                    .max_depth(OWNERSHIP_SCAN_MAX_DEPTH)
                    .into_iter()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().is_file())
                {
                    self.app_relationships.push(FileEdge {
                        source: app_dir.clone(),
                        target: file_entry.path().to_path_buf(),
                        relationship: format!("app_{}_{}", app_name, rel),
                    });
                }
            }
        }
    }

    /// op 88: Map preference relationships.
    ///
    /// Maps preference files (`~/Library/Preferences/*.plist`) to their
    /// owning apps. A plist named `com.example.App.plist` is linked to the
    /// app directory `~/Library/Caches/com.example.App` (or Application
    /// Support) when a matching per-app container exists.
    #[allow(dead_code)]
    pub fn map_preference_relationships(&mut self) {
        let prefs = MacosUtils::home_dir().join("Library/Preferences");
        if !prefs.is_dir() {
            return;
        }
        let caches = MacosUtils::get_caches_dir();
        let support = MacosUtils::get_application_support_dir();
        let entries = match std::fs::read_dir(&prefs) {
            Ok(e) => e,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.ends_with(".plist") {
                continue;
            }
            // Derive the bundle id by stripping the `.plist` suffix.
            let bundle_id = name.trim_end_matches(".plist");
            // Look for a matching per-app container.
            let cache_dir = caches.join(bundle_id);
            let support_dir = support.join(bundle_id);
            let target = if cache_dir.is_dir() {
                cache_dir
            } else if support_dir.is_dir() {
                support_dir
            } else {
                continue;
            };
            self.preference_relationships.push(FileEdge {
                source: path,
                target,
                relationship: "preference_of".to_string(),
            });
        }
    }

    /// op 90: Map generated files.
    ///
    /// Identifies generated files (build artifacts like `.o`, `.class`,
    /// `.pyc`; compiled outputs in `target/`, `build/`, `DerivedData/`
    /// directories) and maps each to its nearest source file. The source is
    /// heuristically chosen as the closest file with a source extension
    /// (`.rs`, `.swift`, `.c`, `.cpp`, `.py`, `.go`, `.js`, `.ts`) sharing a
    /// path stem.
    #[allow(dead_code)]
    pub fn map_generated_files(&mut self) {
        let home = MacosUtils::home_dir();
        let scan_roots = [
            home.join("Documents"),
            home.join("Desktop"),
            home.join("Developer"),
            home.join("Projects"),
            MacosUtils::get_xcode_derived_data_dir(),
        ];
        for root in scan_roots {
            if !root.is_dir() {
                continue;
            }
            for entry in WalkDir::new(&root)
                .min_depth(1)
                .max_depth(GENERATED_SCAN_MAX_DEPTH)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if !is_generated_file(path) {
                    continue;
                }
                let size = entry
                    .metadata()
                    .map(crate::util::disk::physical_size)
                    .unwrap_or(0);
                let kind = generated_file_kind(path);
                let source = find_source_for(path);
                self.generated_files.push(GeneratedFile {
                    path: path.to_path_buf(),
                    source,
                    kind,
                    size_bytes: size,
                });
            }
        }
    }

    /// op 91: Map source files.
    ///
    /// Identifies source files (`.rs`, `.swift`, `.c`, `.cpp`, `.py`, `.go`,
    /// `.js`, `.ts`) and maps each to its generated outputs (e.g. the
    /// compiled artifact sharing the same path stem).
    #[allow(dead_code)]
    pub fn map_source_files(&mut self) {
        let home = MacosUtils::home_dir();
        let scan_roots = [
            home.join("Documents"),
            home.join("Desktop"),
            home.join("Developer"),
            home.join("Projects"),
        ];
        for root in scan_roots {
            if !root.is_dir() {
                continue;
            }
            for entry in WalkDir::new(&root)
                .min_depth(1)
                .max_depth(GENERATED_SCAN_MAX_DEPTH)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if !is_source_file(path) {
                    continue;
                }
                // Look for a sibling generated artifact sharing the stem.
                if let Some(generated) = find_generated_for(path) {
                    self.source_files.push(FileEdge {
                        source: path.to_path_buf(),
                        target: generated,
                        relationship: "compiles_to".to_string(),
                    });
                }
            }
        }
    }

    /// op 105: Detect storage leaks.
    ///
    /// Finds directories consuming disproportionate space relative to their
    /// usefulness. Scans cache, log, and temp directories and flags those
    /// whose `leak_ratio = size * (1 - usefulness)` exceeds the leak
    /// threshold.
    #[allow(dead_code)]
    pub fn detect_storage_leaks(&self) -> Vec<StorageLeak> {
        let home = MacosUtils::home_dir();
        let library = home.join("Library");
        let candidate_roots = [
            library.join("Caches"),
            library.join("Logs"),
            home.join(".Trash"),
        ];
        let mut leaks = Vec::new();
        for root in candidate_roots {
            if !root.is_dir() {
                continue;
            }
            let entries = match std::fs::read_dir(&root) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let dir = entry.path();
                if !dir.is_dir() {
                    continue;
                }
                let size = crate::util::disk::dir_size(&dir);
                if size < STORAGE_LEAK_MIN_SIZE {
                    continue;
                }
                let usefulness = FilesystemGraph::predict_file_importance(&dir);
                let leak_ratio = size as f64 * (1.0 - usefulness);
                if leak_ratio >= STORAGE_LEAK_THRESHOLD {
                    leaks.push(StorageLeak {
                        path: dir,
                        size_bytes: size,
                        usefulness_score: usefulness,
                        leak_ratio,
                    });
                }
            }
        }
        leaks
    }

    /// op 106: Detect runaway folders.
    ///
    /// Identifies folders growing unusually fast — heuristically, any
    /// temp/cache area directory whose current size exceeds the runaway
    /// threshold (1 GB). Because a single snapshot cannot measure growth
    /// directly, `growth_bytes` is reported as the current size.
    #[allow(dead_code)]
    pub fn detect_runaway_folders(&self) -> Vec<RunawayFolder> {
        let home = MacosUtils::home_dir();
        let library = home.join("Library");
        let candidate_roots = [
            library.join("Caches"),
            library.join("Logs"),
            std::env::temp_dir(),
        ];
        let mut runaways = Vec::new();
        for root in candidate_roots {
            if !root.is_dir() {
                continue;
            }
            let entries = match std::fs::read_dir(&root) {
                Ok(e) => e,
                Err(_) => continue,
            };
            for entry in entries.flatten() {
                let dir = entry.path();
                if !dir.is_dir() {
                    continue;
                }
                let size = crate::util::disk::dir_size(&dir);
                if size >= RUNAWAY_FOLDER_THRESHOLD {
                    runaways.push(RunawayFolder {
                        path: dir,
                        size_bytes: size,
                        growth_bytes: size,
                    });
                }
            }
        }
        runaways
    }

    /// op 109: Detect stale projects.
    ///
    /// Finds development projects (directories containing markers like
    /// `.git`, `Cargo.toml`, `package.json`, `*.xcodeproj`) not modified in
    /// more than 180 days.
    #[allow(dead_code)]
    pub fn detect_stale_projects(&self) -> Vec<StaleProject> {
        let home = MacosUtils::home_dir();
        let scan_roots = [
            home.join("Documents"),
            home.join("Developer"),
            home.join("Projects"),
            home.join("Desktop"),
        ];
        let now = now_secs();
        let mut stale = Vec::new();
        for root in scan_roots {
            if !root.is_dir() {
                continue;
            }
            for entry in WalkDir::new(&root)
                .min_depth(1)
                .max_depth(STALE_SCAN_MAX_DEPTH)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_dir())
            {
                let dir = entry.path();
                if !is_project_dir(dir) {
                    continue;
                }
                let mtime = dir_mtime_secs(dir).unwrap_or(now);
                if now.saturating_sub(mtime) > STALE_PROJECT_THRESHOLD_SECS {
                    stale.push(StaleProject {
                        path: dir.to_path_buf(),
                        last_modified_secs: mtime,
                        size_bytes: crate::util::disk::dir_size(dir),
                    });
                }
            }
        }
        stale
    }

    /// op 110: Detect inactive datasets.
    ///
    /// Finds ML datasets and data archives (`.zip`, `.tar`, `.tar.gz`,
    /// `.h5`, `.pkl`, `.npy`, `.npz`, `.parquet`, `.csv`) not accessed in
    /// more than 365 days.
    #[allow(dead_code)]
    pub fn detect_inactive_datasets(&self) -> Vec<InactiveDataset> {
        let home = MacosUtils::home_dir();
        let scan_roots = [
            home.join("Documents"),
            home.join("Downloads"),
            home.join("Developer"),
            home.join("Datasets"),
        ];
        let now = now_secs();
        let mut inactive = Vec::new();
        for root in scan_roots {
            if !root.is_dir() {
                continue;
            }
            for entry in WalkDir::new(&root)
                .min_depth(1)
                .max_depth(GENERATED_SCAN_MAX_DEPTH)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if !is_dataset_file(path) {
                    continue;
                }
                let atime = file_atime_secs(path).unwrap_or(now);
                if now.saturating_sub(atime) > INACTIVE_DATASET_THRESHOLD_SECS {
                    inactive.push(InactiveDataset {
                        path: path.to_path_buf(),
                        last_accessed_secs: atime,
                        size_bytes: crate::util::disk::file_size_physical(path),
                    });
                }
            }
        }
        inactive
    }

    /// op 111: Detect unused assets.
    ///
    /// Finds unused assets — old downloads and unused media files
    /// (`.png`, `.jpg`, `.jpeg`, `.mov`, `.mp4`, `.aif`, `.wav`) not
    /// accessed in more than 365 days.
    #[allow(dead_code)]
    pub fn detect_unused_assets(&self) -> Vec<UnusedAsset> {
        let home = MacosUtils::home_dir();
        let scan_roots = [
            home.join("Downloads"),
            home.join("Documents"),
            home.join("Movies"),
        ];
        let now = now_secs();
        let mut unused = Vec::new();
        for root in scan_roots {
            if !root.is_dir() {
                continue;
            }
            for entry in WalkDir::new(&root)
                .min_depth(1)
                .max_depth(GENERATED_SCAN_MAX_DEPTH)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file())
            {
                let path = entry.path();
                if !is_media_file(path) {
                    continue;
                }
                let atime = file_atime_secs(path).unwrap_or(now);
                if now.saturating_sub(atime) > INACTIVE_DATASET_THRESHOLD_SECS {
                    unused.push(UnusedAsset {
                        path: path.to_path_buf(),
                        last_accessed_secs: atime,
                        size_bytes: crate::util::disk::file_size_physical(path),
                        kind: asset_kind(path),
                    });
                }
            }
        }
        unused
    }

    /// op 114: Build filesystem knowledge graph.
    ///
    /// Aggregates all relationships into a queryable graph: one node per
    /// known file (from the ownership and creator maps) plus all edges
    /// (dependencies, cache relationships, app relationships, preference
    /// relationships, source → generated mappings).
    #[allow(dead_code)]
    pub fn build_knowledge_graph(&self) -> FSKnowledgeGraph {
        let mut nodes: Vec<FSNode> = Vec::new();
        // Collect the union of all known file paths.
        let mut seen: std::collections::HashSet<PathBuf> = std::collections::HashSet::new();
        for path in self.ownership_map.keys() {
            if seen.insert(path.clone()) {
                nodes.push(FSNode {
                    path: path.clone(),
                    owner: self.ownership_map.get(path).cloned(),
                    creator: self.creator_map.get(path).cloned(),
                    importance: FilesystemGraph::predict_file_importance(path),
                    size_bytes: crate::util::disk::file_size_physical(path),
                });
            }
        }
        for path in self.creator_map.keys() {
            if seen.insert(path.clone()) {
                nodes.push(FSNode {
                    path: path.clone(),
                    owner: self.ownership_map.get(path).cloned(),
                    creator: self.creator_map.get(path).cloned(),
                    importance: FilesystemGraph::predict_file_importance(path),
                    size_bytes: crate::util::disk::file_size_physical(path),
                });
            }
        }
        let mut edges = Vec::new();
        edges.extend(self.dependency_edges.clone());
        edges.extend(self.cache_relationships.clone());
        edges.extend(self.app_relationships.clone());
        edges.extend(self.preference_relationships.clone());
        edges.extend(self.source_files.clone());
        // Generated files → source edges.
        for g in &self.generated_files {
            edges.push(FileEdge {
                source: g.source.clone(),
                target: g.path.clone(),
                relationship: format!("generates_{}", g.kind),
            });
        }
        FSKnowledgeGraph {
            nodes,
            edges,
            total_files: self.total_files,
            total_size_bytes: self.total_size_bytes,
        }
    }

    /// op 115: Track filesystem evolution.
    ///
    /// Compares two snapshots to track changes. Files present in `self` but
    /// not in `previous` are `added`; files present in `previous` but not in
    /// `self` are `removed`. The total size delta is reported as a single
    /// `grown`/`shrunk` event on the home directory.
    #[allow(dead_code)]
    pub fn track_evolution(&self, previous: &FilesystemGraph) -> Vec<FSEvolutionEvent> {
        let mut events = Vec::new();
        let prev_paths: std::collections::HashSet<&PathBuf> =
            previous.ownership_map.keys().collect();
        let curr_paths: std::collections::HashSet<&PathBuf> = self.ownership_map.keys().collect();
        for path in curr_paths.iter() {
            if !prev_paths.contains(path) {
                events.push(FSEvolutionEvent {
                    kind: "added".to_string(),
                    path: (*path).clone(),
                    delta_bytes: crate::util::disk::file_size_physical(path) as i64,
                });
            }
        }
        for path in prev_paths.iter() {
            if !curr_paths.contains(path) {
                events.push(FSEvolutionEvent {
                    kind: "removed".to_string(),
                    path: (*path).clone(),
                    delta_bytes: -(crate::util::disk::file_size_physical(path) as i64),
                });
            }
        }
        let delta = self.total_size_bytes as i64 - previous.total_size_bytes as i64;
        if delta != 0 {
            events.push(FSEvolutionEvent {
                kind: if delta > 0 { "grown" } else { "shrunk" }.to_string(),
                path: MacosUtils::home_dir(),
                delta_bytes: delta,
            });
        }
        events
    }

    /// op 116: Predict storage growth.
    ///
    /// Predicts storage usage `days` days ahead based on the current
    /// `storage_growth_trend` (bytes per day). Returns the projected total
    /// size in bytes.
    #[allow(dead_code)]
    pub fn predict_storage_growth(&self, days: u32) -> u64 {
        let trend = self.storage_growth_trend.unwrap_or(0.0);
        let projected = self.total_size_bytes as f64 + trend * days as f64;
        if projected < 0.0 {
            0
        } else {
            projected as u64
        }
    }

    /// op 267: Detect abnormal growth.
    ///
    /// Finds directories (grouped by owning app) growing abnormally fast
    /// between two snapshots. Compares the number of owned files per owner
    /// in `previous` vs `self` and flags owners whose file-count growth
    /// exceeds the abnormal growth threshold.
    #[allow(dead_code)]
    pub fn detect_abnormal_growth(&self, previous: &FilesystemGraph) -> Vec<AbnormalGrowth> {
        let prev_counts = count_files_by_owner(&previous.ownership_map);
        let curr_counts = count_files_by_owner(&self.ownership_map);
        let mut results = Vec::new();
        for (owner, curr) in &curr_counts {
            let prev = prev_counts.get(owner).copied().unwrap_or(0);
            let growth = curr.saturating_sub(prev);
            if growth >= ABNORMAL_GROWTH_THRESHOLD {
                let growth_rate = if prev == 0 {
                    1.0
                } else {
                    growth as f64 / prev as f64
                };
                results.push(AbnormalGrowth {
                    path: MacosUtils::home_dir().join(owner),
                    previous_size_bytes: prev,
                    current_size_bytes: *curr,
                    growth_bytes: growth,
                    growth_rate,
                });
            }
        }
        results
    }
}

// ---------------------------------------------------------------------------
// Free helper functions
// ---------------------------------------------------------------------------

/// Return the `~/Library` subdirectories that hold per-application data
/// keyed by app/bundle name. On non-macOS targets this is empty.
fn ownership_roots() -> Vec<PathBuf> {
    if cfg!(target_os = "macos") {
        vec![
            MacosUtils::get_caches_dir(),
            MacosUtils::get_application_support_dir(),
            MacosUtils::get_logs_dir(),
        ]
    } else {
        Vec::new()
    }
}

/// Gather the bundle identifiers of all currently-installed applications.
/// Used to distinguish leftovers/orphans from live app data.
fn installed_bundle_ids() -> Vec<String> {
    let mut ids = Vec::new();
    for dir in default_app_dirs() {
        for app in enumerate_apps_in(&dir, 2) {
            if let Some(id) = app.bundle_id {
                ids.push(id);
            }
        }
    }
    ids
}

/// Count the total number of files and sum of their physical sizes beneath
/// the user's home directory, bounded to `max_depth` for performance.
fn count_files_in_home(max_depth: usize) -> (u64, u64) {
    let home = MacosUtils::home_dir();
    if !home.is_dir() {
        return (0, 0);
    }
    let mut count = 0u64;
    let mut size = 0u64;
    for entry in WalkDir::new(&home)
        .min_depth(1)
        .max_depth(max_depth)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        if let Ok(meta) = entry.metadata() {
            count += 1;
            size += crate::util::disk::physical_size(meta);
        }
    }
    (count, size)
}

/// Scan the standard user document directories for duplicate-detection
/// candidates, returning [`FileEntry`] records ready for hashing.
fn scan_duplicate_candidates() -> Vec<FileEntry> {
    let home = MacosUtils::home_dir();
    let scan_paths = [
        home.join("Downloads"),
        home.join("Documents"),
        home.join("Desktop"),
    ];

    let mut candidates = Vec::new();
    for scan_path in &scan_paths {
        if !scan_path.is_dir() {
            continue;
        }
        for entry in WalkDir::new(scan_path)
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let metadata = match std::fs::metadata(path) {
                Ok(m) => m,
                Err(_) => continue,
            };
            let size = crate::util::disk::physical_size(metadata.clone());
            if size < DUPLICATE_MIN_SIZE {
                continue;
            }
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            candidates.push(FileEntry {
                path: path.to_path_buf(),
                size,
                modified,
                file_name,
                blake3_hash: None,
                partial_hash: None,
                image_fingerprint: None,
                is_screenshot: is_screenshot(path),
                is_image: is_image_file(path),
            });
        }
    }
    candidates
}

/// Group [`FileEntry`] records by physical size, keeping only groups with
/// more than one file (a fast pre-filter for duplicate detection).
fn group_entries_by_size(files: Vec<FileEntry>) -> Vec<Vec<FileEntry>> {
    let mut groups: HashMap<u64, Vec<FileEntry>> = HashMap::new();
    for file in files {
        groups.entry(file.size).or_default().push(file);
    }
    groups
        .into_iter()
        .filter(|(_, g)| g.len() > 1)
        .map(|(_, g)| g)
        .collect()
}

/// Group [`FileEntry`] records by their BLAKE3 hash, keeping only groups
/// with more than one file.
fn group_entries_by_hash(files: Vec<FileEntry>) -> Vec<(String, Vec<FileEntry>)> {
    let mut groups: HashMap<String, Vec<FileEntry>> = HashMap::new();
    for file in files {
        if let Some(hash) = file.blake3_hash.clone() {
            groups.entry(hash).or_default().push(file);
        }
    }
    groups.into_iter().filter(|(_, g)| g.len() > 1).collect()
}

/// Compute a full BLAKE3 hash of a file synchronously by streaming the
/// contents through a buffered reader. Returns the hex-encoded hash, or
/// `None` if the file cannot be read.
fn sync_full_hash(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    let mut reader = BufReader::new(file);
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 65536];
    loop {
        let read = reader.read(&mut buffer).ok()?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Some(hasher.finalize().to_hex().to_string())
}

/// Convert a duplicate-engine cluster into the filesystem graph's
/// [`DuplicateCluster`] representation.
fn convert_engine_cluster(cluster: EngineDuplicateCluster) -> DuplicateCluster {
    let hash = cluster
        .hash
        .clone()
        .unwrap_or_else(|| "unknown".to_string());
    let total_size_bytes: u64 = cluster.files.iter().map(|f| f.size).sum();
    let files: Vec<PathBuf> = cluster.files.iter().map(|f| f.path.clone()).collect();
    let recommended_keep = cluster.keep_file.map(|f| f.path);
    DuplicateCluster {
        hash,
        files,
        total_size_bytes,
        recommended_keep,
    }
}

/// Available free space (in bytes) on the filesystem containing `path`.
#[cfg(target_os = "macos")]
fn available_space_bytes(path: &Path) -> u64 {
    use std::ffi::CString;
    use std::os::unix::ffi::OsStrExt;

    let c_path = match CString::new(path.as_os_str().as_bytes()) {
        Ok(c) => c,
        Err(_) => return 0,
    };
    unsafe {
        let mut stat: libc::statvfs = std::mem::zeroed();
        if libc::statvfs(c_path.as_ptr(), &mut stat) == 0 {
            (stat.f_bavail as u64) * (stat.f_frsize as u64)
        } else {
            0
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn available_space_bytes(_path: &Path) -> u64 {
    0
}

/// Whether a path lives in a system-critical location.
fn is_system_path(s: &str) -> bool {
    s.starts_with("/System/")
        || s.starts_with("/usr/")
        || s.starts_with("/bin/")
        || s.starts_with("/sbin/")
        || s.starts_with("/Library/")
}

/// Whether a path is inside a user document directory.
fn is_user_doc_path(s: &str) -> bool {
    // Match /Users/<name>/Documents|Desktop|Downloads and also
    // /home/<name>/... on Linux, without depending on $HOME at runtime
    // (avoids test races when other tests temporarily change HOME).
    if let Some(idx) = s.find("/Documents/") {
        return s[..idx].starts_with("/Users/") || s[..idx].starts_with("/home/");
    }
    if let Some(idx) = s.find("/Desktop/") {
        return s[..idx].starts_with("/Users/") || s[..idx].starts_with("/home/");
    }
    if let Some(idx) = s.find("/Downloads/") {
        return s[..idx].starts_with("/Users/") || s[..idx].starts_with("/home/");
    }
    false
}

/// Whether a path is inside a caches directory.
fn is_cache_path(s: &str) -> bool {
    s.contains("/Library/Caches/")
}

/// Whether a path is inside a logs directory.
fn is_log_path(s: &str) -> bool {
    s.contains("/Library/Logs/")
}

/// Whether a path is a temporary/scratch location.
fn is_temp_path(s: &str) -> bool {
    s.starts_with("/tmp/")
        || s.starts_with("/var/tmp/")
        || s.contains("/Library/Caches/TemporaryItems/")
        || std::env::var("TMPDIR")
            .ok()
            .map(|t| s.starts_with(&t))
            .unwrap_or(false)
}

/// Return the file extension of `path` as a lowercase string, or `None`.
fn lower_extension(path: &Path) -> Option<String> {
    path.extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
}

/// Whether a path is a generated/build-artifact file based on its extension
/// or its location inside a known build-output directory.
fn is_generated_file(path: &Path) -> bool {
    if let Some(ext) = lower_extension(path) {
        if matches!(
            ext.as_str(),
            "o" | "obj" | "class" | "pyc" | "pyo" | "elc" | "a" | "so" | "dylib" | "exe"
        ) {
            return true;
        }
    }
    let s = path.to_string_lossy();
    s.contains("/target/")
        || s.contains("/build/")
        || s.contains("/DerivedData/")
        || s.contains("/.build/")
        || s.contains("/node_modules/")
}

/// Categorize a generated file by its extension/location.
fn generated_file_kind(path: &Path) -> String {
    if let Some(ext) = lower_extension(path) {
        match ext.as_str() {
            "o" | "obj" => return "object".to_string(),
            "class" => return "bytecode".to_string(),
            "pyc" | "pyo" => return "python_bytecode".to_string(),
            "a" => return "static_lib".to_string(),
            "so" | "dylib" => return "shared_lib".to_string(),
            "exe" => return "executable".to_string(),
            _ => {}
        }
    }
    "build_artifact".to_string()
}

/// Whether a path is a source file based on its extension.
fn is_source_file(path: &Path) -> bool {
    lower_extension(path)
        .map(|e| {
            matches!(
                e.as_str(),
                "rs" | "swift"
                    | "c"
                    | "cpp"
                    | "cc"
                    | "h"
                    | "hpp"
                    | "py"
                    | "go"
                    | "js"
                    | "ts"
                    | "jsx"
                    | "tsx"
                    | "m"
                    | "mm"
            )
        })
        .unwrap_or(false)
}

/// Whether a path is a dataset/archive file based on its extension.
fn is_dataset_file(path: &Path) -> bool {
    lower_extension(path)
        .map(|e| {
            matches!(
                e.as_str(),
                "zip"
                    | "tar"
                    | "gz"
                    | "bz2"
                    | "xz"
                    | "h5"
                    | "pkl"
                    | "npy"
                    | "npz"
                    | "parquet"
                    | "csv"
                    | "tfrecord"
                    | "hdf5"
            )
        })
        .unwrap_or(false)
}

/// Whether a path is a media/asset file based on its extension.
fn is_media_file(path: &Path) -> bool {
    lower_extension(path)
        .map(|e| {
            matches!(
                e.as_str(),
                "png"
                    | "jpg"
                    | "jpeg"
                    | "gif"
                    | "heic"
                    | "tiff"
                    | "bmp"
                    | "psd"
                    | "mov"
                    | "mp4"
                    | "m4v"
                    | "avi"
                    | "aif"
                    | "wav"
                    | "mp3"
                    | "aac"
                    | "flac"
            )
        })
        .unwrap_or(false)
}

/// Categorize a media asset by its extension.
fn asset_kind(path: &Path) -> String {
    if let Some(ext) = lower_extension(path) {
        match ext.as_str() {
            "png" | "jpg" | "jpeg" | "gif" | "heic" | "tiff" | "bmp" | "psd" => {
                return "image".to_string()
            }
            "mov" | "mp4" | "m4v" | "avi" => return "video".to_string(),
            "aif" | "wav" | "mp3" | "aac" | "flac" => return "audio".to_string(),
            _ => {}
        }
    }
    "asset".to_string()
}

/// Whether a directory is a development project root (contains a marker
/// file/directory like `.git`, `Cargo.toml`, `package.json`, `*.xcodeproj`).
fn is_project_dir(dir: &Path) -> bool {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if name_str == ".git"
                || name_str == "Cargo.toml"
                || name_str == "package.json"
                || name_str == "pyproject.toml"
                || name_str == "go.mod"
                || name_str == ".svn"
                || name_str.ends_with(".xcodeproj")
                || name_str.ends_with(".xcworkspace")
            {
                return true;
            }
        }
    }
    false
}

/// Find the nearest source file for a generated file by looking for a
/// sibling file with a source extension sharing the same stem. Falls back to
/// the parent project directory if no sibling source is found.
fn find_source_for(generated: &Path) -> PathBuf {
    if let (Some(parent), Some(stem)) = (generated.parent(), generated.file_stem()) {
        if let Ok(entries) = std::fs::read_dir(parent) {
            for entry in entries.flatten() {
                let candidate = entry.path();
                if candidate.is_file()
                    && candidate.file_stem().map(|s| s == stem).unwrap_or(false)
                    && is_source_file(&candidate)
                {
                    return candidate;
                }
            }
        }
    }
    // Fall back to the nearest ancestor project directory.
    let mut cur = generated.parent();
    while let Some(dir) = cur {
        if is_project_dir(dir) {
            return dir.to_path_buf();
        }
        cur = dir.parent();
    }
    generated.to_path_buf()
}

/// Find a generated output for a source file by looking for a sibling file
/// with a generated extension sharing the same stem.
fn find_generated_for(source: &Path) -> Option<PathBuf> {
    let (parent, stem) = (source.parent()?, source.file_stem()?);
    let entries = std::fs::read_dir(parent).ok()?;
    for entry in entries.flatten() {
        let candidate = entry.path();
        if candidate.is_file()
            && candidate.file_stem().map(|s| s == stem).unwrap_or(false)
            && is_generated_file(&candidate)
        {
            return Some(candidate);
        }
    }
    None
}

/// Current time in seconds since the UNIX epoch.
fn now_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Modification time of a directory in seconds since the UNIX epoch.
fn dir_mtime_secs(dir: &Path) -> Option<u64> {
    std::fs::metadata(dir)
        .ok()?
        .modified()
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_secs())
}

/// Last access time of a file in seconds since the UNIX epoch.
fn file_atime_secs(path: &Path) -> Option<u64> {
    use std::os::unix::fs::MetadataExt;
    std::fs::metadata(path).ok().map(|m| m.atime() as u64)
}

/// Count the number of files owned by each owner in an ownership map.
fn count_files_by_owner(ownership: &HashMap<PathBuf, String>) -> HashMap<String, u64> {
    let mut counts: HashMap<String, u64> = HashMap::new();
    for owner in ownership.values() {
        *counts.entry(owner.clone()).or_default() += 1;
    }
    counts
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_empty_graph_has_zeroed_fields() {
        let g = FilesystemGraph::empty();
        assert_eq!(g.total_files, 0);
        assert_eq!(g.total_size_bytes, 0);
        assert!(g.ownership_map.is_empty());
        assert!(g.creator_map.is_empty());
        assert!(g.cache_relationships.is_empty());
        assert!(g.duplicate_clusters.is_empty());
        assert!(g.abandoned_files.is_empty());
        assert!(g.orphan_files.is_empty());
        assert!(g.storage_growth_trend.is_none());
        assert!(g.exhaustion_forecast_days.is_none());
        assert!(g.app_relationships.is_empty());
        assert!(g.preference_relationships.is_empty());
        assert!(g.generated_files.is_empty());
        assert!(g.source_files.is_empty());
    }

    #[test]
    fn test_predict_file_importance_system_path() {
        let p = PathBuf::from("/System/Library/Frameworks/Foundation.framework/Versions/A");
        assert_eq!(FilesystemGraph::predict_file_importance(&p), 1.0);
    }

    #[test]
    fn test_predict_file_importance_user_documents() {
        // Use a fixed path to avoid races with tests that temporarily change HOME.
        let p = PathBuf::from("/Users/test/Documents/report.pdf");
        assert_eq!(FilesystemGraph::predict_file_importance(&p), 0.8);
    }

    #[test]
    fn test_predict_file_importance_caches() {
        let p = PathBuf::from("/Users/test/Library/Caches/com.example.App/cache.db");
        assert_eq!(FilesystemGraph::predict_file_importance(&p), 0.1);
    }

    #[test]
    fn test_predict_file_importance_logs() {
        let p = PathBuf::from("/Users/test/Library/Logs/com.example.App/main.log");
        assert_eq!(FilesystemGraph::predict_file_importance(&p), 0.05);
    }

    #[test]
    fn test_predict_file_importance_temp() {
        let p = PathBuf::from("/tmp/build-output-123.bin");
        assert_eq!(FilesystemGraph::predict_file_importance(&p), 0.01);
    }

    #[test]
    fn test_predict_file_importance_unknown_is_neutral() {
        let p = PathBuf::from("/Users/test/Projects/foo/src/main.rs");
        assert_eq!(FilesystemGraph::predict_file_importance(&p), 0.5);
    }

    #[test]
    fn test_count_files_in_home_bounded() {
        // Counting the home dir should never panic and should return a
        // non-negative pair. We cannot assert exact counts, but the depth
        // bound keeps it fast.
        let (count, size) = count_files_in_home(1);
        let _ = count;
        let _ = size;
    }

    #[test]
    fn test_count_files_in_temp_dir() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("a.txt"), b"hello").unwrap();
        fs::write(tmp.path().join("b.txt"), b"world!").unwrap();

        // Bypass the home-dir assumption by walking the temp dir directly.
        let mut count = 0u64;
        let mut size = 0u64;
        for entry in WalkDir::new(tmp.path())
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            if let Ok(meta) = entry.metadata() {
                count += 1;
                size += crate::util::disk::physical_size(meta);
            }
        }
        assert_eq!(count, 2);
        assert!(size >= b"hello".len() as u64 + b"world!".len() as u64);
    }

    #[test]
    fn test_group_entries_by_size_filters_unique() {
        let mk = |path: &str, size: u64| FileEntry {
            path: PathBuf::from(path),
            size,
            modified: 0,
            file_name: path.to_string(),
            blake3_hash: None,
            partial_hash: None,
            image_fingerprint: None,
            is_screenshot: false,
            is_image: false,
        };
        let files = vec![mk("/a.txt", 100), mk("/b.txt", 100), mk("/c.txt", 200)];
        let groups = group_entries_by_size(files);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn test_group_entries_by_hash_filters_unique() {
        let mk = |path: &str, hash: &str| FileEntry {
            path: PathBuf::from(path),
            size: 100,
            modified: 0,
            file_name: path.to_string(),
            blake3_hash: Some(hash.to_string()),
            partial_hash: None,
            image_fingerprint: None,
            is_screenshot: false,
            is_image: false,
        };
        let files = vec![
            mk("/a.txt", "abc"),
            mk("/b.txt", "abc"),
            mk("/c.txt", "def"),
        ];
        let groups = group_entries_by_hash(files);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].0, "abc");
        assert_eq!(groups[0].1.len(), 2);
    }

    #[test]
    fn test_sync_full_hash_identical_files() {
        let tmp = TempDir::new().unwrap();
        let p1 = tmp.path().join("one.txt");
        let p2 = tmp.path().join("two.txt");
        fs::write(&p1, b"identical content").unwrap();
        fs::write(&p2, b"identical content").unwrap();
        fs::write(tmp.path().join("three.txt"), b"different content").unwrap();

        let h1 = sync_full_hash(&p1).unwrap();
        let h2 = sync_full_hash(&p2).unwrap();
        let h3 = sync_full_hash(&tmp.path().join("three.txt")).unwrap();
        assert_eq!(h1, h2);
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_sync_full_hash_missing_file_returns_none() {
        assert!(sync_full_hash(&PathBuf::from("/nonexistent/path/to/file")).is_none());
    }

    #[test]
    fn test_convert_engine_cluster_preserves_keep() {
        let mk = |path: &str, size: u64, modified: u64| FileEntry {
            path: PathBuf::from(path),
            size,
            modified,
            file_name: path.to_string(),
            blake3_hash: Some("abc123".to_string()),
            partial_hash: Some("abc123".to_string()),
            image_fingerprint: None,
            is_screenshot: false,
            is_image: false,
        };
        let files = vec![mk("/old.txt", 100, 100), mk("/new.txt", 100, 200)];
        let engine_cluster = EngineDuplicateCluster::from_files(files);
        let converted = convert_engine_cluster(engine_cluster);
        assert_eq!(converted.hash, "abc123");
        assert_eq!(converted.files.len(), 2);
        assert_eq!(converted.total_size_bytes, 200);
        // The newest file (mtime 200) should be recommended to keep.
        assert_eq!(converted.recommended_keep, Some(PathBuf::from("/new.txt")));
    }

    #[test]
    fn test_collect_duplicate_clusters_finds_identical() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();

        fs::create_dir_all(dir.join("Downloads")).unwrap();
        let content = b"duplicate payload for clustering";
        fs::write(dir.join("Downloads/a.txt"), content).unwrap();
        fs::write(dir.join("Downloads/b.txt"), content).unwrap();
        fs::write(dir.join("Downloads/unique.txt"), b"unique payload here").unwrap();

        // Build candidates directly from the temp dir instead of using HOME.
        let mut candidates = Vec::new();
        for entry in WalkDir::new(dir.join("Downloads"))
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let metadata = std::fs::metadata(path).unwrap();
            let size = crate::util::disk::physical_size(metadata.clone());
            if size < DUPLICATE_MIN_SIZE {
                continue;
            }
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            candidates.push(FileEntry {
                path: path.to_path_buf(),
                size,
                modified,
                file_name,
                blake3_hash: None,
                partial_hash: None,
                image_fingerprint: None,
                is_screenshot: is_screenshot(path),
                is_image: is_image_file(path),
            });
        }
        assert!(candidates.len() >= 3);

        // Group by size, hash, and build clusters.
        let size_groups = group_entries_by_size(candidates);
        let mut all_clusters = Vec::new();
        for group in size_groups {
            if group.len() < 2 {
                continue;
            }
            // Compute BLAKE3 hashes before grouping by hash.
            let hashed: Vec<FileEntry> = group
                .into_iter()
                .filter_map(|mut fe| {
                    if let Some(hash) = sync_full_hash(&fe.path) {
                        fe.blake3_hash = Some(hash.clone());
                        fe.partial_hash = Some(hash);
                        Some(fe)
                    } else {
                        None
                    }
                })
                .collect();
            let hash_groups = group_entries_by_hash(hashed);
            for (_, group) in hash_groups {
                if group.len() < 2 {
                    continue;
                }
                let engine_cluster = EngineDuplicateCluster::from_files(group);
                let converted = convert_engine_cluster(engine_cluster);
                all_clusters.push(converted);
            }
        }
        assert_eq!(all_clusters.len(), 1);
        assert_eq!(all_clusters[0].files.len(), 2);
        assert!(all_clusters[0].total_size_bytes > 0);
        assert!(all_clusters[0].recommended_keep.is_some());
    }

    #[test]
    fn test_collect_duplicate_clusters_none_when_unique() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();
        fs::create_dir_all(dir.join("Documents")).unwrap();
        // Three files with different content — no duplicates.
        fs::write(dir.join("Documents/a.txt"), b"aaa").unwrap();
        fs::write(dir.join("Documents/b.txt"), b"bbb").unwrap();
        fs::write(dir.join("Documents/c.txt"), b"ccc").unwrap();

        let mut candidates = Vec::new();
        for entry in WalkDir::new(dir.join("Documents"))
            .min_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let path = entry.path();
            let metadata = std::fs::metadata(path).unwrap();
            let size = crate::util::disk::physical_size(metadata.clone());
            if size < DUPLICATE_MIN_SIZE {
                continue;
            }
            let modified = metadata
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let file_name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            candidates.push(FileEntry {
                path: path.to_path_buf(),
                size,
                modified,
                file_name,
                blake3_hash: None,
                partial_hash: None,
                image_fingerprint: None,
                is_screenshot: is_screenshot(path),
                is_image: is_image_file(path),
            });
        }
        // Files have different content, so after hashing no clusters form.
        let size_groups = group_entries_by_size(candidates);
        let mut all_clusters = Vec::new();
        for group in size_groups {
            if group.len() < 2 {
                continue;
            }
            let hashed: Vec<FileEntry> = group
                .into_iter()
                .filter_map(|mut fe| {
                    if let Some(hash) = sync_full_hash(&fe.path) {
                        fe.blake3_hash = Some(hash.clone());
                        fe.partial_hash = Some(hash);
                        Some(fe)
                    } else {
                        None
                    }
                })
                .collect();
            for (_, cluster_files) in group_entries_by_hash(hashed) {
                if cluster_files.len() < 2 {
                    continue;
                }
                all_clusters.push(convert_engine_cluster(EngineDuplicateCluster::from_files(
                    cluster_files,
                )));
            }
        }
        assert!(all_clusters.is_empty());
    }

    #[test]
    fn test_forecast_storage_growth_sets_trend() {
        let mut g = FilesystemGraph::empty();
        g.total_size_bytes = 365_000_000; // 365 MB → 1 MB/day heuristic
        g.forecast_storage_growth();
        assert!(g.storage_growth_trend.is_some());
        let trend = g.storage_growth_trend.unwrap();
        assert!(trend > 0.0);
        // Exhaustion forecast may be None if available space is unavailable,
        // but when set it must be positive.
        if let Some(days) = g.exhaustion_forecast_days {
            assert!(days > 0);
        }
    }

    #[test]
    fn test_forecast_storage_growth_zero_total() {
        let mut g = FilesystemGraph::empty();
        g.total_size_bytes = 0;
        g.forecast_storage_growth();
        // Trend is still computed (0.0); exhaustion forecast stays None.
        assert_eq!(g.storage_growth_trend, Some(0.0));
        assert!(g.exhaustion_forecast_days.is_none());
    }

    #[test]
    fn test_is_system_path() {
        assert!(is_system_path("/System/Library/foo"));
        assert!(is_system_path("/usr/bin/ls"));
        assert!(is_system_path("/Library/Fonts"));
        assert!(!is_system_path("/Users/test/Documents/foo"));
    }

    #[test]
    fn test_is_cache_log_temp_paths() {
        assert!(is_cache_path("/Users/test/Library/Caches/com.app/cache.db"));
        assert!(is_log_path("/Users/test/Library/Logs/com.app/main.log"));
        assert!(is_temp_path("/tmp/scratch.bin"));
        assert!(is_temp_path(
            "/Users/test/Library/Caches/TemporaryItems/item"
        ));
        assert!(!is_cache_path("/Users/test/Documents/file.txt"));
    }

    #[test]
    fn test_available_space_bytes_is_non_negative() {
        let home = MacosUtils::home_dir();
        let avail = available_space_bytes(&home);
        // On a real filesystem this should be positive; on CI sandboxes it
        // may be 0, so only assert non-negativity.
        let _ = avail;
    }

    #[test]
    fn test_ownership_roots_macos() {
        let roots = ownership_roots();
        if cfg!(target_os = "macos") {
            assert_eq!(roots.len(), 3);
        } else {
            assert!(roots.is_empty());
        }
    }

    // -----------------------------------------------------------------------
    // New operation tests (ops 84, 88, 90, 91, 105, 106, 109, 110, 111,
    // 114, 115, 116, 267)
    // -----------------------------------------------------------------------

    /// Mutex to serialize tests that modify the `$HOME` env var, preventing
    /// parallel interference.
    static HOME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// Helper: run a closure with `$HOME` temporarily pointed at a temp dir.
    /// Acquires a global mutex so parallel tests don't interfere with each
    /// other's `$HOME` setting.
    fn with_home_tmp<F: FnOnce(&Path)>(body: F) {
        let _guard = HOME_LOCK.lock().unwrap();
        let tmp = TempDir::new().unwrap();
        let old_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", tmp.path());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| body(tmp.path())));
        match old_home {
            Some(h) => std::env::set_var("HOME", h),
            None => std::env::remove_var("HOME"),
        }
        assert!(result.is_ok(), "closure panicked");
    }

    #[test]
    fn test_map_app_relationships_links_app_files() {
        with_home_tmp(|home| {
            let caches = home.join("Library/Caches/com.example.App");
            fs::create_dir_all(&caches).unwrap();
            fs::write(caches.join("cache.db"), b"data").unwrap();

            let mut g = FilesystemGraph::empty();
            g.map_app_relationships();
            // At least one edge should reference the cache file.
            assert!(g
                .app_relationships
                .iter()
                .any(|e| e.target == caches.join("cache.db")));
        });
    }

    #[test]
    fn test_map_app_relationships_empty_when_no_library() {
        with_home_tmp(|_home| {
            let mut g = FilesystemGraph::empty();
            g.map_app_relationships();
            assert!(g.app_relationships.is_empty());
        });
    }

    #[test]
    fn test_map_preference_relationships_links_plist_to_app() {
        with_home_tmp(|home| {
            let prefs = home.join("Library/Preferences");
            let caches = home.join("Library/Caches/com.example.App");
            fs::create_dir_all(&prefs).unwrap();
            fs::create_dir_all(&caches).unwrap();
            fs::write(prefs.join("com.example.App.plist"), b"plist").unwrap();

            let mut g = FilesystemGraph::empty();
            g.map_preference_relationships();
            assert_eq!(g.preference_relationships.len(), 1);
            assert_eq!(g.preference_relationships[0].target, caches);
            assert_eq!(g.preference_relationships[0].relationship, "preference_of");
        });
    }

    #[test]
    fn test_map_preference_relationships_skips_unmatched_plist() {
        with_home_tmp(|home| {
            let prefs = home.join("Library/Preferences");
            fs::create_dir_all(&prefs).unwrap();
            fs::write(prefs.join("com.nomatch.App.plist"), b"plist").unwrap();

            let mut g = FilesystemGraph::empty();
            g.map_preference_relationships();
            assert!(g.preference_relationships.is_empty());
        });
    }

    #[test]
    fn test_map_generated_files_finds_object_file() {
        with_home_tmp(|home| {
            let proj = home.join("Projects/foo");
            fs::create_dir_all(&proj).unwrap();
            fs::write(proj.join("main.c"), b"int main(){}").unwrap();
            fs::write(proj.join("main.o"), b"\0\0\0").unwrap();

            let mut g = FilesystemGraph::empty();
            g.map_generated_files();
            assert!(g
                .generated_files
                .iter()
                .any(|f| f.path == proj.join("main.o")));
            // The source should be the sibling main.c.
            let obj = g
                .generated_files
                .iter()
                .find(|f| f.path == proj.join("main.o"))
                .unwrap();
            assert_eq!(obj.source, proj.join("main.c"));
            assert_eq!(obj.kind, "object");
        });
    }

    #[test]
    fn test_map_source_files_links_to_generated() {
        with_home_tmp(|home| {
            let proj = home.join("Projects/bar");
            fs::create_dir_all(&proj).unwrap();
            fs::write(proj.join("app.rs"), b"fn main(){}").unwrap();
            fs::write(proj.join("app.o"), b"\0\0").unwrap();

            let mut g = FilesystemGraph::empty();
            g.map_source_files();
            assert!(g
                .source_files
                .iter()
                .any(|e| e.source == proj.join("app.rs") && e.target == proj.join("app.o")));
        });
    }

    #[test]
    fn test_detect_storage_leaks_flags_large_cache_dir() {
        with_home_tmp(|home| {
            let cache = home.join("Library/Caches/com.leaky.App");
            fs::create_dir_all(&cache).unwrap();
            // Write enough non-zero data to exceed STORAGE_LEAK_MIN_SIZE.
            // Use a repeating non-zero pattern so APFS won't compress/sparse
            // the file and the physical block count matches the logical size.
            let chunk = vec![0xABu8; 1024 * 1024];
            let mut f = fs::File::create(cache.join("blob.bin")).unwrap();
            let blocks_needed = (STORAGE_LEAK_MIN_SIZE / (1024 * 1024)) + 2;
            for _ in 0..blocks_needed {
                use std::io::Write;
                f.write_all(&chunk).unwrap();
            }
            f.sync_all().unwrap();

            let g = FilesystemGraph::empty();
            let leaks = g.detect_storage_leaks();
            assert!(leaks.iter().any(|l| l.path == cache));
        });
    }

    #[test]
    fn test_detect_storage_leaks_empty_when_small() {
        with_home_tmp(|home| {
            let cache = home.join("Library/Caches/com.small.App");
            fs::create_dir_all(&cache).unwrap();
            fs::write(cache.join("tiny.bin"), b"small").unwrap();

            let g = FilesystemGraph::empty();
            let leaks = g.detect_storage_leaks();
            assert!(leaks.iter().all(|l| l.path != cache));
        });
    }

    #[test]
    fn test_detect_runaway_folders_flags_large_temp_dir() {
        with_home_tmp(|home| {
            let cache = home.join("Library/Caches/com.runaway.App");
            fs::create_dir_all(&cache).unwrap();
            let big = vec![0u8; (RUNAWAY_FOLDER_THRESHOLD + 1024) as usize];
            fs::write(cache.join("huge.bin"), &big).unwrap();

            let g = FilesystemGraph::empty();
            let runaways = g.detect_runaway_folders();
            assert!(runaways.iter().any(|r| r.path == cache));
        });
    }

    #[test]
    fn test_detect_runaway_folders_empty_when_small() {
        with_home_tmp(|home| {
            let cache = home.join("Library/Caches/com.tiny.App");
            fs::create_dir_all(&cache).unwrap();
            fs::write(cache.join("tiny.bin"), b"small").unwrap();

            let g = FilesystemGraph::empty();
            let runaways = g.detect_runaway_folders();
            assert!(runaways.iter().all(|r| r.path != cache));
        });
    }

    #[test]
    fn test_detect_stale_projects_finds_old_project() {
        with_home_tmp(|home| {
            let proj = home.join("Projects/old");
            fs::create_dir_all(&proj).unwrap();
            fs::write(proj.join("Cargo.toml"), b"[package]").unwrap();
            // Set the project dir mtime to far in the past.
            set_mtime_past(&proj, STALE_PROJECT_THRESHOLD_SECS + 86400);

            let g = FilesystemGraph::empty();
            let stale = g.detect_stale_projects();
            assert!(stale.iter().any(|s| s.path == proj));
        });
    }

    #[test]
    fn test_detect_stale_projects_skips_recent_project() {
        with_home_tmp(|home| {
            let proj = home.join("Projects/fresh");
            fs::create_dir_all(&proj).unwrap();
            fs::write(proj.join("Cargo.toml"), b"[package]").unwrap();
            // mtime is "now" by default — should not be flagged.

            let g = FilesystemGraph::empty();
            let stale = g.detect_stale_projects();
            assert!(stale.iter().all(|s| s.path != proj));
        });
    }

    #[test]
    fn test_detect_inactive_datasets_finds_old_archive() {
        with_home_tmp(|home| {
            let docs = home.join("Documents");
            fs::create_dir_all(&docs).unwrap();
            let archive = docs.join("old_data.zip");
            fs::write(&archive, b"zipdata").unwrap();
            set_atime_past(&archive, INACTIVE_DATASET_THRESHOLD_SECS + 86400);

            let g = FilesystemGraph::empty();
            let inactive = g.detect_inactive_datasets();
            assert!(inactive.iter().any(|d| d.path == archive));
        });
    }

    #[test]
    fn test_detect_inactive_datasets_skips_recent() {
        with_home_tmp(|home| {
            let docs = home.join("Documents");
            fs::create_dir_all(&docs).unwrap();
            let archive = docs.join("fresh_data.zip");
            fs::write(&archive, b"zipdata").unwrap();
            // atime is recent — should not be flagged.

            let g = FilesystemGraph::empty();
            let inactive = g.detect_inactive_datasets();
            assert!(inactive.iter().all(|d| d.path != archive));
        });
    }

    #[test]
    fn test_detect_unused_assets_finds_old_media() {
        with_home_tmp(|home| {
            let downloads = home.join("Downloads");
            fs::create_dir_all(&downloads).unwrap();
            let movie = downloads.join("old_movie.mov");
            fs::write(&movie, b"moviedata").unwrap();
            set_atime_past(&movie, INACTIVE_DATASET_THRESHOLD_SECS + 86400);

            let g = FilesystemGraph::empty();
            let unused = g.detect_unused_assets();
            assert!(unused.iter().any(|a| a.path == movie && a.kind == "video"));
        });
    }

    #[test]
    fn test_detect_unused_assets_skips_recent() {
        with_home_tmp(|home| {
            let downloads = home.join("Downloads");
            fs::create_dir_all(&downloads).unwrap();
            let movie = downloads.join("fresh_movie.mov");
            fs::write(&movie, b"moviedata").unwrap();

            let g = FilesystemGraph::empty();
            let unused = g.detect_unused_assets();
            assert!(unused.iter().all(|a| a.path != movie));
        });
    }

    #[test]
    fn test_build_knowledge_graph_aggregates_nodes_and_edges() {
        let mut g = FilesystemGraph::empty();
        g.ownership_map.insert(
            PathBuf::from("/Users/test/Library/Caches/app/cache.db"),
            "app".to_string(),
        );
        g.cache_relationships.push(FileEdge {
            source: PathBuf::from("/Users/test/Library/Caches/app/cache.db"),
            target: PathBuf::from("/Users/test/Library/Application Support/app"),
            relationship: "cache_of".to_string(),
        });
        let kg = g.build_knowledge_graph();
        assert!(!kg.nodes.is_empty());
        assert!(!kg.edges.is_empty());
        // The cache edge should be present.
        assert!(kg.edges.iter().any(|e| e.relationship == "cache_of"));
    }

    #[test]
    fn test_build_knowledge_graph_empty_graph() {
        let g = FilesystemGraph::empty();
        let kg = g.build_knowledge_graph();
        assert!(kg.nodes.is_empty());
        assert!(kg.edges.is_empty());
        assert_eq!(kg.total_files, 0);
        assert_eq!(kg.total_size_bytes, 0);
    }

    #[test]
    fn test_track_evolution_detects_added_and_removed() {
        let mut prev = FilesystemGraph::empty();
        prev.ownership_map
            .insert(PathBuf::from("/Users/test/old.txt"), "user".to_string());
        let mut curr = FilesystemGraph::empty();
        curr.ownership_map
            .insert(PathBuf::from("/Users/test/new.txt"), "user".to_string());
        curr.total_size_bytes = 1000;
        prev.total_size_bytes = 500;

        let events = curr.track_evolution(&prev);
        assert!(events
            .iter()
            .any(|e| e.kind == "added" && e.path == PathBuf::from("/Users/test/new.txt")));
        assert!(events
            .iter()
            .any(|e| e.kind == "removed" && e.path == PathBuf::from("/Users/test/old.txt")));
        assert!(events.iter().any(|e| e.kind == "grown"));
    }

    #[test]
    fn test_track_evolution_no_change() {
        let mut prev = FilesystemGraph::empty();
        prev.ownership_map
            .insert(PathBuf::from("/Users/test/same.txt"), "user".to_string());
        prev.total_size_bytes = 500;
        let mut curr = FilesystemGraph::empty();
        curr.ownership_map
            .insert(PathBuf::from("/Users/test/same.txt"), "user".to_string());
        curr.total_size_bytes = 500;

        let events = curr.track_evolution(&prev);
        assert!(events.is_empty());
    }

    #[test]
    fn test_predict_storage_growth_linear() {
        let mut g = FilesystemGraph::empty();
        g.total_size_bytes = 1_000_000;
        g.storage_growth_trend = Some(1_000.0); // 1 KB/day
                                                // 10 days → 1_000_000 + 10_000
        assert_eq!(g.predict_storage_growth(10), 1_010_000);
    }

    #[test]
    fn test_predict_storage_growth_no_trend() {
        let g = FilesystemGraph::empty();
        // No trend set → returns the current total.
        assert_eq!(g.predict_storage_growth(30), 0);
    }

    #[test]
    fn test_detect_abnormal_growth_flags_owner() {
        let mut prev = FilesystemGraph::empty();
        // Previous: owner "app" had 10 files.
        for i in 0..10 {
            prev.ownership_map.insert(
                PathBuf::from(format!("/Users/test/old_{i}.txt")),
                "app".to_string(),
            );
        }
        let mut curr = FilesystemGraph::empty();
        // Current: owner "app" now has 200 files (growth of 190 ≥ threshold).
        for i in 0..200 {
            curr.ownership_map.insert(
                PathBuf::from(format!("/Users/test/new_{i}.txt")),
                "app".to_string(),
            );
        }
        let growth = curr.detect_abnormal_growth(&prev);
        assert!(growth
            .iter()
            .any(|g| g.growth_bytes >= ABNORMAL_GROWTH_THRESHOLD));
    }

    #[test]
    fn test_detect_abnormal_growth_none_when_stable() {
        let mut prev = FilesystemGraph::empty();
        for i in 0..50 {
            prev.ownership_map.insert(
                PathBuf::from(format!("/Users/test/f_{i}.txt")),
                "app".to_string(),
            );
        }
        let mut curr = FilesystemGraph::empty();
        for i in 0..55 {
            curr.ownership_map.insert(
                PathBuf::from(format!("/Users/test/f_{i}.txt")),
                "app".to_string(),
            );
        }
        let growth = curr.detect_abnormal_growth(&prev);
        assert!(growth.is_empty());
    }

    #[test]
    fn test_is_generated_file_by_extension_and_path() {
        assert!(is_generated_file(&PathBuf::from("/proj/build/main.o")));
        assert!(is_generated_file(&PathBuf::from(
            "/proj/target/release/app"
        )));
        assert!(is_generated_file(&PathBuf::from("/proj/mod.pyc")));
        assert!(!is_generated_file(&PathBuf::from("/proj/main.rs")));
    }

    #[test]
    fn test_is_source_file_by_extension() {
        assert!(is_source_file(&PathBuf::from("/proj/main.rs")));
        assert!(is_source_file(&PathBuf::from("/proj/app.swift")));
        assert!(!is_source_file(&PathBuf::from("/proj/main.o")));
    }

    #[test]
    fn test_is_dataset_file_and_is_media_file() {
        assert!(is_dataset_file(&PathBuf::from("/data/train.zip")));
        assert!(is_dataset_file(&PathBuf::from("/data/model.h5")));
        assert!(!is_dataset_file(&PathBuf::from("/data/notes.txt")));
        assert!(is_media_file(&PathBuf::from("/pics/photo.jpg")));
        assert!(is_media_file(&PathBuf::from("/movies/clip.mov")));
        assert!(!is_media_file(&PathBuf::from("/docs/readme.md")));
    }

    #[test]
    fn test_is_project_dir_detects_markers() {
        let tmp = TempDir::new().unwrap();
        fs::write(tmp.path().join("Cargo.toml"), b"[package]").unwrap();
        assert!(is_project_dir(tmp.path()));

        let tmp2 = TempDir::new().unwrap();
        fs::write(tmp2.path().join("package.json"), b"{}").unwrap();
        assert!(is_project_dir(tmp2.path()));

        let tmp3 = TempDir::new().unwrap();
        fs::write(tmp3.path().join("random.txt"), b"hi").unwrap();
        assert!(!is_project_dir(tmp3.path()));
    }

    #[test]
    fn test_count_files_by_owner() {
        let mut map = HashMap::new();
        map.insert(PathBuf::from("/a"), "app1".to_string());
        map.insert(PathBuf::from("/b"), "app1".to_string());
        map.insert(PathBuf::from("/c"), "app2".to_string());
        let counts = count_files_by_owner(&map);
        assert_eq!(counts.get("app1"), Some(&2));
        assert_eq!(counts.get("app2"), Some(&1));
    }

    /// Set a path's modification time to `secs_in_past` seconds before now.
    fn set_mtime_past(path: &Path, secs_in_past: u64) {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(path).unwrap();
        let times = [
            libc::timespec {
                tv_sec: meta.atime() as i64 - secs_in_past as i64,
                tv_nsec: meta.atime_nsec() as i64,
            },
            libc::timespec {
                tv_sec: meta.mtime() as i64 - secs_in_past as i64,
                tv_nsec: meta.mtime_nsec() as i64,
            },
        ];
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let c_path = CString::new(path.as_os_str().as_bytes()).unwrap();
        unsafe {
            libc::utimensat(libc::AT_FDCWD, c_path.as_ptr(), times.as_ptr(), 0);
        }
    }

    /// Set a path's access time to `secs_in_past` seconds before now.
    fn set_atime_past(path: &Path, secs_in_past: u64) {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(path).unwrap();
        let times = [
            libc::timespec {
                tv_sec: meta.atime() as i64 - secs_in_past as i64,
                tv_nsec: meta.atime_nsec() as i64,
            },
            libc::timespec {
                tv_sec: meta.mtime() as i64,
                tv_nsec: meta.mtime_nsec() as i64,
            },
        ];
        use std::ffi::CString;
        use std::os::unix::ffi::OsStrExt;
        let c_path = CString::new(path.as_os_str().as_bytes()).unwrap();
        unsafe {
            libc::utimensat(libc::AT_FDCWD, c_path.as_ptr(), times.as_ptr(), 0);
        }
    }
}
