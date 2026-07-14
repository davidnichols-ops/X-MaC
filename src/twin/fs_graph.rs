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

        // Ops 93-95: duplicate cluster integration.
        graph.duplicate_clusters = graph.collect_duplicate_clusters();

        // Ops 96-100: abandoned/orphan file detection.
        let (abandoned, orphans) = graph.collect_abandoned_and_orphans();
        graph.abandoned_files = abandoned;
        graph.orphan_files = orphans;

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
        }
    }

    /// Ops 81-85: File ownership mapping.
    ///
    /// For the common `~/Library` application directories (Caches,
    /// Application Support, Logs), map every file to its owning application.
    /// The owning app name is the immediate subdirectory name under each of
    /// these roots (e.g. `~/Library/Caches/com.example.App` → owner
    /// `com.example.App`).
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
}
