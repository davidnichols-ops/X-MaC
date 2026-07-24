use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::Metadata;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use walkdir::WalkDir;

use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};
use crate::util::backup;

use super::cluster::{DuplicateCluster, FileEntry};
use super::hasher;
use super::scanner::{is_image_file, is_screenshot, ImageFingerprint};

/// Default minimum file size to consider for duplicate detection (1 KB).
/// Files smaller than this are not worth the hashing overhead.
const DEFAULT_MIN_SIZE: u64 = 1024;

/// Default partial-hash sample size: hash first and last 4 KB of each file.
const PARTIAL_SAMPLE_SIZE: u64 = 4096;

/// Default maximum number of files to hash concurrently.
const DEFAULT_CONCURRENCY: usize = 8;

/// Whitelist of path substrings that should never be flagged as duplicates.
const DEFAULT_WHITELIST: &[&str] = &[
    "/.Trash/",
    "/Library/",
    "/.git/",
    "/.npm/",
    "/node_modules/",
    "/.cache/",
];

/// Duplicate Detection Engine — finds duplicate files using BLAKE3 hashing
/// and perceptual fingerprinting for images.
///
/// Implements Cleaner/Optimizer operations 81–110:
///   - 81:  Scan file candidates
///   - 82:  Compare filenames (name-based pre-filter)
///   - 83:  Compare file sizes (size-based grouping)
///   - 84:  Generate hashes (BLAKE3)
///   - 85:  Generate partial hashes (first/last N bytes)
///   - 86:  Generate full hashes (full file BLAKE3)
///   - 87:  Compare hashes
///   - 88:  Compare metadata
///   - 89:  Detect identical files (hash match)
///   - 90:  Detect renamed duplicates (same hash, different name)
///   - 91:  Detect copied folders (folder-level hash comparison)
///   - 92:  Group duplicate clusters (union-find clustering)
///   - 93:  Detect duplicate photos (image-specific detection)
///   - 94:  Detect similar photos (perceptual fingerprint comparison)
///   - 95:  Generate image fingerprints (byte-level fingerprint)
///   - 96:  Compare perceptual hashes (Hamming distance)
///   - 97:  Detect screenshots (filename pattern detection)
///   - 98:  Detect blurry images (placeholder — requires image decoding)
///   - 99:  Rank duplicate confidence (confidence scoring)
///   - 100: Select safest deletion candidate (keep newest/largest/original)
///   - 101: Preserve newest file (mtime-based selection)
///   - 102: Preserve highest-resolution image (dimension heuristic)
///   - 103: Preserve original path (path priority)
///   - 104: Create deletion queue (emit findings with deletion metadata)
///   - 108: Generate duplicate reports (findings with rich metadata)
///   - 109: Ignore duplicate whitelist (user-configured whitelist)
///   - 110: Maintain scan database (hash map returned in metadata)
pub struct DuplicateEngine {
    /// Minimum file size in bytes to consider for duplicate detection.
    min_size: u64,
    /// Partial hash sample size (first/last N bytes).
    partial_sample_size: u64,
    /// Maximum number of files to hash concurrently.
    concurrency: usize,
    /// Paths to scan for duplicates. Defaults to ~/Downloads and ~/Documents.
    scan_paths: Vec<PathBuf>,
    /// Whitelist of path substrings to exclude from duplicate detection.
    whitelist: Vec<String>,
    /// Whether to detect similar images via perceptual hashing.
    similar_images: bool,
    /// Whether duplicate detection is enabled.
    enabled: bool,
}

impl Default for DuplicateEngine {
    fn default() -> Self {
        let home = crate::util::macos::MacosUtils::home_dir();
        Self {
            min_size: DEFAULT_MIN_SIZE,
            partial_sample_size: PARTIAL_SAMPLE_SIZE,
            concurrency: DEFAULT_CONCURRENCY,
            scan_paths: vec![
                home.join("Downloads"),
                home.join("Documents"),
                home.join("Desktop"),
            ],
            whitelist: DEFAULT_WHITELIST.iter().map(|s| s.to_string()).collect(),
            similar_images: false,
            enabled: true,
        }
    }
}

impl DuplicateEngine {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
    }

    /// Apply config and profile overrides to the engine. Config values act
    /// as defaults; the active optimization profile further tunes behavior.
    #[allow(dead_code)]
    pub fn with_config(mut self, config: &crate::config::Config) -> Self {
        let dc = &config.duplicate;
        let profile = config.profile;

        self.min_size = dc.min_size;
        self.enabled = dc.enabled;
        self.similar_images = dc.similar_images;

        let profile_min_size = profile.dedup_min_size();
        if profile_min_size > self.min_size {
            self.min_size = profile_min_size;
        }
        if !dc.enabled {
            self.enabled = profile.dedup_enabled();
        }
        if profile.dedup_similar_images() {
            self.similar_images = true;
        }

        self
    }

    /// Set custom scan paths.
    #[allow(dead_code)]
    pub fn with_scan_paths(mut self, paths: Vec<PathBuf>) -> Self {
        self.scan_paths = paths;
        self
    }

    /// Set minimum file size for duplicate consideration.
    #[allow(dead_code)]
    pub fn with_min_size(mut self, min_size: u64) -> Self {
        self.min_size = min_size;
        self
    }

    /// Set the partial hash sample size.
    #[allow(dead_code)]
    pub fn with_partial_sample_size(mut self, size: u64) -> Self {
        self.partial_sample_size = size;
        self
    }

    /// Set the concurrency level for hashing.
    #[allow(dead_code)]
    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency.max(1);
        self
    }

    /// Add a whitelist entry (path substring to exclude).
    #[allow(dead_code)]
    pub fn with_whitelist_entry(mut self, entry: impl Into<String>) -> Self {
        self.whitelist.push(entry.into());
        self
    }

    /// Check if a path is whitelisted (should be excluded from duplicate
    /// detection).
    fn is_whitelisted(&self, path: &Path) -> bool {
        let path_str = path.to_string_lossy();
        self.whitelist.iter().any(|w| path_str.contains(w.as_str()))
    }

    /// Check if a path is inside a backup volume (never scan these).
    fn is_backup(&self, path: &Path) -> bool {
        backup::is_backup_path(path)
    }

    /// Determine if a file should be excluded from duplicate scanning.
    /// Excludes: whitelisted paths, backup volumes, hidden files (by name).
    ///
    /// Note: only the file's own name (last path component) is checked for
    /// the hidden-file test. Parent directory names are not checked here —
    /// hidden directories are pruned during the walk instead.
    fn should_exclude(&self, path: &Path, include_hidden: bool) -> bool {
        if self.is_whitelisted(path) || self.is_backup(path) {
            return true;
        }
        if !include_hidden {
            // Check if the file's own name (last component) starts with '.'
            if path
                .file_name()
                .map(|n| n.to_string_lossy().starts_with('.'))
                .unwrap_or(false)
            {
                return true;
            }
        }
        false
    }

    /// Scan file candidates: walk the scan paths and collect all regular
    /// files above the minimum size threshold. (op 81)
    fn scan_candidates(&self, include_hidden: bool, follow_symlinks: bool) -> Vec<FileEntry> {
        let mut candidates = Vec::new();

        for scan_path in &self.scan_paths {
            if !scan_path.exists() || self.is_backup(scan_path) {
                continue;
            }

            let mut walker = WalkDir::new(scan_path)
                .min_depth(1)
                .follow_links(follow_symlinks)
                .into_iter();

            while let Some(entry) = walker.next() {
                let entry = match entry {
                    Ok(e) => e,
                    Err(_) => continue,
                };

                // Prune directories that should be excluded
                if entry.file_type().is_dir() {
                    let dir_path = entry.path();
                    if self.should_exclude(dir_path, include_hidden) {
                        walker.skip_current_dir();
                        continue;
                    }
                    continue;
                }

                if !entry.file_type().is_file() {
                    continue;
                }

                let path = entry.path();
                if self.should_exclude(path, include_hidden) {
                    continue;
                }

                let metadata = match std::fs::metadata(path) {
                    Ok(m) => m,
                    Err(_) => continue,
                };

                let size = crate::util::disk::physical_size(metadata.clone());
                if size < self.min_size {
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

    /// Group files by size as a fast pre-filter. Only files that share the
    /// same size can possibly be duplicates. (op 83)
    fn group_by_size(files: Vec<FileEntry>) -> Vec<Vec<FileEntry>> {
        let mut size_groups: HashMap<u64, Vec<FileEntry>> = HashMap::new();
        for file in files {
            size_groups.entry(file.size).or_default().push(file);
        }
        size_groups
            .into_iter()
            .filter(|(_, group)| group.len() > 1)
            .map(|(_, group)| group)
            .collect()
    }

    /// Generate partial hashes for all files in a size group. Only files
    /// with matching partial hashes proceed to full hashing. (op 85)
    async fn hash_partial_group(&self, group: Vec<FileEntry>) -> Vec<FileEntry> {
        let sample = self.partial_sample_size;
        let sem = Arc::new(tokio::sync::Semaphore::new(self.concurrency));
        let mut handles = Vec::with_capacity(group.len());

        for file in group {
            let sem = Arc::clone(&sem);
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                let hash = hasher::partial_hash(&file.path, sample).await.ok()?;
                Some(FileEntry {
                    partial_hash: Some(hash),
                    ..file
                })
            }));
        }

        let mut result = Vec::with_capacity(handles.len());
        for handle in handles {
            if let Ok(Some(file)) = handle.await {
                result.push(file);
            }
        }
        result
    }

    /// Generate full BLAKE3 hashes for files that share a partial hash.
    /// (op 86)
    async fn hash_full_group(&self, group: Vec<FileEntry>) -> Vec<FileEntry> {
        let sem = Arc::new(tokio::sync::Semaphore::new(self.concurrency));
        let mut handles = Vec::with_capacity(group.len());

        for file in group {
            let sem = Arc::clone(&sem);
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                let hash = hasher::full_hash(&file.path).await.ok()?;
                Some(FileEntry {
                    blake3_hash: Some(hash),
                    ..file
                })
            }));
        }

        let mut result = Vec::with_capacity(handles.len());
        for handle in handles {
            if let Ok(Some(file)) = handle.await {
                result.push(file);
            }
        }
        result
    }

    /// Group files by their partial hash within a size group. Returns
    /// sub-groups where partial hashes match.
    fn group_by_partial_hash(files: Vec<FileEntry>) -> Vec<Vec<FileEntry>> {
        let mut hash_groups: HashMap<String, Vec<FileEntry>> = HashMap::new();
        for file in files {
            if let Some(ref hash) = file.partial_hash {
                hash_groups.entry(hash.clone()).or_default().push(file);
            }
        }
        hash_groups
            .into_iter()
            .filter(|(_, group)| group.len() > 1)
            .map(|(_, group)| group)
            .collect()
    }

    /// Group files by their full BLAKE3 hash. Returns clusters of
    /// identical files. (op 89, 92)
    fn group_by_full_hash(files: Vec<FileEntry>) -> Vec<Vec<FileEntry>> {
        let mut hash_groups: HashMap<String, Vec<FileEntry>> = HashMap::new();
        for file in files {
            if let Some(ref hash) = file.blake3_hash {
                hash_groups.entry(hash.clone()).or_default().push(file);
            }
        }
        hash_groups
            .into_iter()
            .filter(|(_, group)| group.len() > 1)
            .map(|(_, group)| group)
            .collect()
    }

    /// Generate image fingerprints for image files in a cluster. (op 95)
    async fn fingerprint_images(&self, files: &mut [FileEntry]) {
        let sem = Arc::new(tokio::sync::Semaphore::new(self.concurrency));
        let mut handles = Vec::new();

        for file in files.iter() {
            if !file.is_image {
                continue;
            }
            let path = file.path.clone();
            let sem = Arc::clone(&sem);
            handles.push(tokio::spawn(async move {
                let _permit = sem.acquire().await.ok()?;
                ImageFingerprint::compute(&path).ok()
            }));
        }

        // Collect results in order, then assign back to the corresponding
        // image file entries.
        let mut fingerprints = Vec::with_capacity(handles.len());
        for handle in handles {
            let fp = handle.await.ok().flatten();
            fingerprints.push(fp);
        }

        let mut fp_idx = 0;
        for file in files.iter_mut() {
            if file.is_image {
                if let Some(Some(fp)) = fingerprints.get(fp_idx) {
                    file.image_fingerprint = Some(fp.clone());
                }
                fp_idx += 1;
            }
        }
    }

    /// Find similar images across all image files using Hamming distance
    /// on their fingerprints. Returns clusters of similar (but not
    /// necessarily identical) images. (op 94, 96)
    fn find_similar_images(files: &[FileEntry], max_distance: u32) -> Vec<Vec<FileEntry>> {
        let images: Vec<&FileEntry> = files
            .iter()
            .filter(|f| f.image_fingerprint.is_some())
            .collect();
        if images.len() < 2 {
            return Vec::new();
        }

        let mut clusters: Vec<Vec<FileEntry>> = Vec::new();
        let mut assigned: Vec<bool> = vec![false; images.len()];

        for i in 0..images.len() {
            if assigned[i] {
                continue;
            }
            let mut cluster = vec![images[i].clone()];
            assigned[i] = true;

            for j in (i + 1)..images.len() {
                if assigned[j] {
                    continue;
                }
                if let (Some(fp_i), Some(fp_j)) =
                    (&images[i].image_fingerprint, &images[j].image_fingerprint)
                {
                    let dist = fp_i.hamming_distance(fp_j);
                    if dist <= max_distance {
                        cluster.push(images[j].clone());
                        assigned[j] = true;
                    }
                }
            }

            if cluster.len() > 1 {
                clusters.push(cluster);
            }
        }

        clusters
    }

    /// Build duplicate clusters from hash-matched groups. (op 92, 99)
    fn build_clusters(hash_groups: Vec<Vec<FileEntry>>) -> Vec<DuplicateCluster> {
        hash_groups
            .into_iter()
            .map(DuplicateCluster::from_files)
            .collect()
    }

    /// Convert clusters to findings and emit them. (op 100-104, 108)
    fn clusters_to_findings(clusters: &[DuplicateCluster]) -> Vec<Finding> {
        let mut findings = Vec::with_capacity(clusters.len());

        for cluster in clusters {
            if cluster.files.len() < 2 {
                continue;
            }

            let hash_short = cluster
                .hash
                .as_deref()
                .map(|h| h.chars().take(12).collect::<String>())
                .unwrap_or_else(|| "unknown".to_string());

            let reclaimable = cluster.reclaimable_bytes();

            // Determine if this is an image cluster
            let is_image_cluster = cluster.files.iter().any(|f| f.is_image);
            let has_screenshots = cluster.files.iter().any(|f| f.is_screenshot);

            let title = if is_image_cluster && !cluster.similar {
                "Duplicate images detected"
            } else if cluster.similar {
                "Similar images detected"
            } else if has_screenshots {
                "Duplicate screenshots detected"
            } else {
                "Duplicate files detected"
            };

            let severity = if cluster.confidence >= 0.95 {
                Severity::Medium
            } else if cluster.confidence >= 0.80 {
                Severity::Low
            } else {
                Severity::Info
            };

            let keep_path = cluster
                .keep_file
                .as_ref()
                .map(|f| f.path.to_string_lossy().to_string())
                .unwrap_or_default();

            let delete_paths: Vec<String> = cluster
                .delete_files
                .iter()
                .map(|f| f.path.to_string_lossy().to_string())
                .collect();

            let description =
                format!(
                "Found {} {} with BLAKE3 hash {}. Reclaimable: {}. Keep: {}. Confidence: {:.0}%",
                cluster.files.len(),
                if cluster.similar { "similar files" } else { "identical files" },
                hash_short,
                crate::util::disk::format_bytes(reclaimable),
                keep_path,
                cluster.confidence * 100.0
            );

            let finding = Finding::new(
                EngineId::Duplicate,
                severity,
                Category::DuplicateFile,
                Target::Path(
                    cluster
                        .keep_file
                        .as_ref()
                        .map(|f| f.path.clone())
                        .unwrap_or_else(|| cluster.files[0].path.clone()),
                ),
                title,
                description,
            )
            .with_size(reclaimable)
            .with_metadata(
                "hash".to_string(),
                serde_json::json!(cluster.hash.as_deref().unwrap_or("similar")),
            )
            .with_metadata(
                "file_count".to_string(),
                serde_json::json!(cluster.files.len()),
            )
            .with_metadata("keep_path".to_string(), serde_json::json!(keep_path))
            .with_metadata("delete_paths".to_string(), serde_json::json!(delete_paths))
            .with_metadata(
                "confidence".to_string(),
                serde_json::json!(cluster.confidence),
            )
            .with_metadata(
                "is_image_cluster".to_string(),
                serde_json::json!(is_image_cluster),
            )
            .with_metadata(
                "has_screenshots".to_string(),
                serde_json::json!(has_screenshots),
            )
            .with_metadata(
                "all_paths".to_string(),
                serde_json::json!(cluster
                    .files
                    .iter()
                    .map(|f| f.path.to_string_lossy().to_string())
                    .collect::<Vec<_>>()),
            )
            .with_hint(
                "Review duplicate files. The recommended file to keep is in 'keep_path'. \
                 Files in 'delete_paths' are safe candidates for deletion."
                    .to_string(),
            );

            findings.push(finding);
        }

        findings
    }

    /// Main duplicate detection pipeline.
    ///
    /// 1. Scan file candidates (op 81)
    /// 2. Group by size (op 83)
    /// 3. Partial hash each size group (op 85)
    /// 4. Group by partial hash
    /// 5. Full hash partial-hash matches (op 86)
    /// 6. Group by full hash → identical clusters (op 89, 92)
    /// 7. Fingerprint images (op 95)
    /// 8. Find similar images (op 94, 96)
    /// 9. Select safest deletion candidate (op 100-103)
    /// 10. Build findings (op 104, 108)
    pub async fn detect_duplicates(
        &self,
        include_hidden: bool,
        follow_symlinks: bool,
    ) -> (Vec<DuplicateCluster>, u64) {
        let start = Instant::now();
        let _ = start; // used for timing if needed later

        // Step 1: Scan candidates
        let candidates = self.scan_candidates(include_hidden, follow_symlinks);
        let items_scanned = candidates.len() as u64;

        if candidates.is_empty() {
            return (Vec::new(), items_scanned);
        }

        // Step 2: Group by size
        let size_groups = Self::group_by_size(candidates);

        // Step 3-6: Hash and cluster
        let mut all_clusters = Vec::new();

        for size_group in size_groups {
            // Step 3: Partial hash
            let partial_hashed = self.hash_partial_group(size_group).await;
            if partial_hashed.is_empty() {
                continue;
            }

            // Step 4: Group by partial hash
            let partial_groups = Self::group_by_partial_hash(partial_hashed);

            for partial_group in partial_groups {
                // Step 5: Full hash
                let full_hashed = self.hash_full_group(partial_group).await;
                if full_hashed.is_empty() {
                    continue;
                }

                // Step 6: Group by full hash → identical clusters
                let hash_groups = Self::group_by_full_hash(full_hashed);
                let mut clusters = Self::build_clusters(hash_groups);
                all_clusters.append(&mut clusters);
            }
        }

        // Step 7-8: Image fingerprinting and similar image detection
        let mut all_files: Vec<FileEntry> = Vec::new();
        for cluster in &all_clusters {
            all_files.extend(cluster.files.clone());
        }

        if !all_files.is_empty() {
            self.fingerprint_images(&mut all_files).await;
        }

        // Find similar images (not identical, but visually close)
        if self.similar_images {
            let similar_clusters = Self::find_similar_images(&all_files, 8);
            let mut similar_dup_clusters: Vec<DuplicateCluster> = similar_clusters
                .into_iter()
                .map(|files| {
                    let mut c = DuplicateCluster::from_files(files);
                    c.similar = true;
                    c.confidence = 0.75; // lower confidence for similar (not identical)
                    c.select_deletion_candidates();
                    c
                })
                .collect();
            all_clusters.append(&mut similar_dup_clusters);
        }

        (all_clusters, items_scanned)
    }
}

#[async_trait]
impl Engine for DuplicateEngine {
    fn id(&self) -> EngineId {
        EngineId::Duplicate
    }

    fn name(&self) -> &'static str {
        "Duplicate Detection"
    }

    fn description(&self) -> &'static str {
        "Finds duplicate files using BLAKE3 hashing and perceptual image fingerprinting"
    }

    async fn validate(&self, _ctx: &ScanContext) -> Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> Result<EngineStats, EngineError> {
        let start = Instant::now();

        let include_hidden = ctx.config.include_hidden;
        let follow_symlinks = ctx.config.follow_symlinks;

        let (clusters, items_scanned) = self
            .detect_duplicates(include_hidden, follow_symlinks)
            .await;

        let findings = Self::clusters_to_findings(&clusters);
        let findings_count = findings.len() as u64;

        for finding in findings {
            ctx.emit(finding).await;
        }

        Ok(EngineStats {
            engine: self.id(),
            duration: start.elapsed(),
            items_scanned,
            findings_count,
            errors_count: 0,
        })
    }
}

// ---------------------------------------------------------------------------
// Free-function operations (ops 82, 84, 88, 90, 91, 98, 99, 107, 108, 109,
// 110). These are standalone helpers that operate on the duplicate-detection
// data structures without requiring a `DuplicateEngine` instance.
// ---------------------------------------------------------------------------

/// Compute the Levenshtein edit distance between two strings.
fn levenshtein(a: &str, b: &str) -> usize {
    let a_chars: Vec<char> = a.chars().collect();
    let b_chars: Vec<char> = b.chars().collect();
    let m = a_chars.len();
    let n = b_chars.len();

    if m == 0 {
        return n;
    }
    if n == 0 {
        return m;
    }

    let mut prev = (0..=n).collect::<Vec<usize>>();
    let mut curr = vec![0usize; n + 1];

    for i in 1..=m {
        curr[0] = i;
        for j in 1..=n {
            let cost = if a_chars[i - 1] == b_chars[j - 1] {
                0
            } else {
                1
            };
            curr[j] = (prev[j] + 1).min(curr[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut curr);
    }

    prev[n]
}

/// op 82: Compare filenames — returns a normalized similarity score (0.0–1.0)
/// based on the Levenshtein distance between the file stems. A score of 1.0
/// means identical filenames; 0.0 means completely different.
#[allow(dead_code)]
pub fn compare_filenames(a: &Path, b: &Path) -> f64 {
    let name_a = a.file_stem().and_then(|s| s.to_str()).unwrap_or("");
    let name_b = b.file_stem().and_then(|s| s.to_str()).unwrap_or("");

    if name_a.is_empty() && name_b.is_empty() {
        return 1.0;
    }
    let max_len = name_a.chars().count().max(name_b.chars().count());
    if max_len == 0 {
        return 1.0;
    }

    let dist = levenshtein(name_a, name_b);
    1.0 - (dist as f64 / max_len as f64)
}

/// op 84: Generate hashes — computes a full BLAKE3 hash of a file and returns
/// the hex-encoded hash string. Returns `None` if the file cannot be read.
#[allow(dead_code)]
pub fn generate_hash(path: &Path) -> Option<String> {
    let data = std::fs::read(path).ok()?;
    Some(blake3::hash(&data).to_hex().to_string())
}

/// Result of comparing two files' metadata (op 88).
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub struct MetadataComparison {
    /// Whether the two files have identical sizes.
    pub size_match: bool,
    /// Absolute difference in size (bytes).
    pub size_diff: u64,
    /// Whether the two files have identical modification times.
    pub modified_match: bool,
    /// Absolute difference in modification time (seconds).
    pub modified_diff_secs: u64,
    /// Whether the two files have identical permissions.
    pub permissions_match: bool,
}

/// op 88: Compare metadata — compares file sizes, modification times, and
/// permissions between two `Metadata` values.
#[allow(dead_code)]
pub fn compare_metadata(a: &Metadata, b: &Metadata) -> MetadataComparison {
    let size_a = a.len();
    let size_b = b.len();

    let mod_a = a
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mod_b = b
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    MetadataComparison {
        size_match: size_a == size_b,
        size_diff: size_a.abs_diff(size_b),
        modified_match: mod_a == mod_b,
        modified_diff_secs: mod_a.abs_diff(mod_b),
        permissions_match: a.permissions() == b.permissions(),
    }
}

/// A pair of files that are likely renamed copies of each other (op 90).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RenamedDuplicate {
    /// The path considered the "original".
    pub original: PathBuf,
    /// The path considered the renamed copy.
    pub renamed: PathBuf,
    /// Filename similarity score (0.0–1.0).
    pub similarity: f64,
}

/// op 90: Detect renamed duplicates — identifies files in a cluster that are
/// likely renamed copies. All files in a `DuplicateCluster` share the same
/// BLAKE3 hash, so pairs with similar (but not identical) filenames are
/// considered renamed duplicates.
#[allow(dead_code)]
pub fn detect_renamed_duplicates(cluster: &DuplicateCluster) -> Vec<RenamedDuplicate> {
    let files = &cluster.files;
    let mut result = Vec::new();

    for i in 0..files.len() {
        for j in (i + 1)..files.len() {
            if files[i].file_name == files[j].file_name {
                continue;
            }
            let sim = compare_filenames(&files[i].path, &files[j].path);
            if sim > 0.5 {
                result.push(RenamedDuplicate {
                    original: files[i].path.clone(),
                    renamed: files[j].path.clone(),
                    similarity: sim,
                });
            }
        }
    }

    result
}

/// A folder that appears to be a copy of another folder (op 91).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CopiedFolder {
    /// The folder considered the source.
    pub source: PathBuf,
    /// Folders that appear to be copies of the source.
    pub copies: Vec<PathBuf>,
    /// Structural similarity score (0.0–1.0).
    pub similarity: f64,
}

/// Compute a structural signature for a directory by hashing the sorted list
/// of file hashes it contains (recursively). Two folders with the same
/// signature contain identical file content.
fn folder_signature(dir: &Path) -> String {
    let mut hashes: Vec<String> = Vec::new();
    for entry in WalkDir::new(dir).min_depth(1) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if entry.file_type().is_file() {
            if let Some(h) = generate_hash(entry.path()) {
                hashes.push(h);
            }
        }
    }
    hashes.sort();
    let combined = hashes.join(",");
    blake3::hash(combined.as_bytes()).to_hex().to_string()
}

/// op 91: Detect copied folders — identifies immediate subdirectories of
/// `path` that appear to be copies of each other by comparing their file
/// structure and content hashes.
#[allow(dead_code)]
pub fn detect_copied_folders(path: &Path) -> Vec<CopiedFolder> {
    let mut groups: HashMap<String, Vec<PathBuf>> = HashMap::new();

    let entries = match std::fs::read_dir(path) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    for entry in entries.flatten() {
        let p = entry.path();
        if !p.is_dir() {
            continue;
        }
        let sig = folder_signature(&p);
        groups.entry(sig).or_default().push(p);
    }

    groups
        .into_iter()
        .filter(|(_, dirs)| dirs.len() > 1)
        .map(|(_, dirs)| CopiedFolder {
            source: dirs[0].clone(),
            copies: dirs[1..].to_vec(),
            similarity: 1.0,
        })
        .collect()
}

/// op 98: Detect blurry images — uses the image file's byte-level variance
/// as a proxy for sharpness. High variance indicates a sharp image; low
/// variance indicates a blurry image. Returns `Some(true)` if the image is
/// likely blurry, `Some(false)` if sharp, or `None` if it cannot be
/// determined.
#[allow(dead_code)]
pub fn detect_blurry_images(path: &Path) -> Option<bool> {
    let data = std::fs::read(path).ok()?;
    if data.is_empty() {
        return None;
    }

    // Skip metadata headers (first 256 bytes) and sample pixel-region bytes.
    let start = 256.min(data.len());
    let region = &data[start..];
    if region.is_empty() {
        return None;
    }

    let target_samples = 1024usize;
    let step = (region.len() / target_samples).max(1);
    let mut samples: Vec<f64> = Vec::with_capacity(target_samples);
    let mut i = 0;
    while i < region.len() && samples.len() < target_samples {
        samples.push(region[i] as f64);
        i += step;
    }

    if samples.is_empty() {
        return None;
    }

    let mean = samples.iter().sum::<f64>() / samples.len() as f64;
    let variance = samples.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / samples.len() as f64;

    // Low variance => few details => likely blurry.
    Some(variance < 100.0)
}

/// A confidence ranking for a duplicate cluster (op 99).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct ConfidenceRank {
    /// Confidence score (0.0–1.0).
    pub score: f64,
    /// Categorical label derived from the score.
    pub label: ConfidenceLabel,
}

/// Categorical confidence labels (op 99).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ConfidenceLabel {
    /// High confidence (score >= 0.8).
    High,
    /// Medium confidence (0.5 <= score < 0.8).
    Medium,
    /// Low confidence (score < 0.5).
    Low,
}

/// op 99: Rank duplicate confidence — ranks how confident we are that the
/// files in a cluster are true duplicates, based on hash match, filename
/// similarity, and metadata similarity.
#[allow(dead_code)]
pub fn rank_confidence(cluster: &DuplicateCluster) -> ConfidenceRank {
    // Hash match contributes the most weight.
    let hash_score = if cluster.hash.is_some() && !cluster.similar {
        0.6
    } else {
        0.3
    };

    // Average pairwise filename similarity.
    let files = &cluster.files;
    let mut name_sim = 0.0f64;
    let mut name_count = 0usize;
    for i in 0..files.len() {
        for j in (i + 1)..files.len() {
            name_sim += compare_filenames(&files[i].path, &files[j].path);
            name_count += 1;
        }
    }
    let name_score = if name_count > 0 {
        (name_sim / name_count as f64) * 0.2
    } else {
        0.0
    };

    // Average pairwise metadata similarity.
    let mut meta_score = 0.0f64;
    let mut meta_count = 0usize;
    for i in 0..files.len() {
        for j in (i + 1)..files.len() {
            if let (Some(ma), Some(mb)) = (
                std::fs::metadata(&files[i].path).ok(),
                std::fs::metadata(&files[j].path).ok(),
            ) {
                let cmp = compare_metadata(&ma, &mb);
                let mut s = 0.0f64;
                if cmp.size_match {
                    s += 0.5;
                }
                if cmp.modified_match {
                    s += 0.3;
                }
                if cmp.permissions_match {
                    s += 0.2;
                }
                meta_score += s;
                meta_count += 1;
            }
        }
    }
    let meta_score = if meta_count > 0 {
        (meta_score / meta_count as f64) * 0.2
    } else {
        0.0
    };

    let score = (hash_score + name_score + meta_score).min(1.0);
    let label = if score >= 0.8 {
        ConfidenceLabel::High
    } else if score >= 0.5 {
        ConfidenceLabel::Medium
    } else {
        ConfidenceLabel::Low
    };

    ConfidenceRank { score, label }
}

/// A duplicate found within a nested directory structure (op 107).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct RecursiveDuplicate {
    /// The shared BLAKE3 hash.
    pub hash: String,
    /// Paths of all files sharing this hash.
    pub paths: Vec<PathBuf>,
    /// Maximum nesting depth among the duplicate paths.
    pub depth: usize,
}

/// op 107: Detect recursive duplication — finds duplicate content within
/// nested directory structures by walking `path` recursively and grouping
/// files by their BLAKE3 hash.
#[allow(dead_code)]
pub fn detect_recursive_duplication(path: &Path) -> Vec<RecursiveDuplicate> {
    let mut hash_groups: HashMap<String, Vec<(PathBuf, usize)>> = HashMap::new();

    for entry in WalkDir::new(path).min_depth(1) {
        let entry = match entry {
            Ok(e) => e,
            Err(_) => continue,
        };
        if !entry.file_type().is_file() {
            continue;
        }
        let depth = entry.depth();
        if let Some(h) = generate_hash(entry.path()) {
            hash_groups
                .entry(h)
                .or_default()
                .push((entry.path().to_path_buf(), depth));
        }
    }

    hash_groups
        .into_iter()
        .filter(|(_, entries)| entries.len() > 1)
        .map(|(hash, entries)| {
            let depth = entries.iter().map(|(_, d)| *d).max().unwrap_or(0);
            let paths = entries.into_iter().map(|(p, _)| p).collect();
            RecursiveDuplicate { hash, paths, depth }
        })
        .collect()
}

/// A summary report of duplicate detection results (op 108).
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DuplicateReport {
    /// Number of duplicate clusters.
    pub cluster_count: usize,
    /// Total reclaimable space in bytes.
    pub total_reclaimable_bytes: u64,
    /// Total number of files across all clusters.
    pub total_files: usize,
    /// Human-readable recommendations.
    pub recommendations: Vec<String>,
}

/// op 108: Generate duplicate reports — creates a summary report with total
/// reclaimable space, cluster count, and recommendations.
#[allow(dead_code)]
pub fn generate_report(clusters: &[DuplicateCluster]) -> DuplicateReport {
    let cluster_count = clusters.len();
    let total_reclaimable_bytes = clusters.iter().map(|c| c.reclaimable_bytes()).sum();
    let total_files: usize = clusters.iter().map(|c| c.files.len()).sum();

    let mut recommendations = Vec::new();
    if total_reclaimable_bytes > 0 {
        recommendations.push(format!(
            "Reclaim {} by deleting duplicate files.",
            crate::util::disk::format_bytes(total_reclaimable_bytes)
        ));
    }
    if clusters.iter().any(|c| c.files.iter().any(|f| f.is_image)) {
        recommendations
            .push("Review duplicate images — keep the highest-resolution copy.".to_string());
    }
    if clusters.iter().any(|c| c.similar) {
        recommendations
            .push("Similar (not identical) images detected — review before deleting.".to_string());
    }
    if recommendations.is_empty() {
        recommendations.push("No duplicates found — nothing to clean up.".to_string());
    }

    DuplicateReport {
        cluster_count,
        total_reclaimable_bytes,
        total_files,
        recommendations,
    }
}

/// op 109: Ignore duplicate whitelist — returns `true` if `path` matches any
/// entry in the user-configured `whitelist`. A match occurs when the path
/// starts with a whitelist entry or contains it as a substring.
#[allow(dead_code)]
pub fn is_whitelisted(path: &Path, whitelist: &[PathBuf]) -> bool {
    let path_str = path.to_string_lossy();
    whitelist
        .iter()
        .any(|w| path.starts_with(w) || path_str.contains(w.to_string_lossy().as_ref()))
}

/// A single record in the scan database (op 110).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ScanRecord {
    /// BLAKE3 hash of the file.
    pub hash: String,
    /// File size in bytes.
    pub size: u64,
    /// Modification time as Unix timestamp (seconds).
    pub modified: u64,
}

/// op 110: Maintain scan database — stores previous scan results so that
/// unchanged files (same size and mtime) can skip re-hashing on subsequent
/// scans. The database can be saved to and loaded from a JSON file.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ScanDatabase {
    /// Map of file path to its scan record.
    pub entries: HashMap<PathBuf, ScanRecord>,
}

impl ScanDatabase {
    /// Create a new empty scan database.
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self {
            entries: HashMap::new(),
        }
    }

    /// Record (or update) a file's scan result.
    #[allow(dead_code)]
    pub fn record(&mut self, path: PathBuf, hash: String, size: u64, modified: u64) {
        self.entries.insert(
            path,
            ScanRecord {
                hash,
                size,
                modified,
            },
        );
    }

    /// Returns `true` if the file at `path` is unchanged since the last scan
    /// (matching size and modification time).
    #[allow(dead_code)]
    pub fn is_unchanged(&self, path: &Path, size: u64, modified: u64) -> bool {
        self.entries
            .get(path)
            .map(|r| r.size == size && r.modified == modified)
            .unwrap_or(false)
    }

    /// Returns the cached hash for a file, if present.
    #[allow(dead_code)]
    pub fn hash_for(&self, path: &Path) -> Option<&str> {
        self.entries.get(path).map(|r| r.hash.as_str())
    }

    /// Save the scan database to a JSON file.
    #[allow(dead_code)]
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        let json = serde_json::to_string_pretty(self).map_err(std::io::Error::other)?;
        std::fs::write(path, json)
    }

    /// Load a scan database from a JSON file.
    #[allow(dead_code)]
    pub fn load(path: &Path) -> std::io::Result<Self> {
        let data = std::fs::read_to_string(path)?;
        serde_json::from_str(&data)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_duplicate_engine_default() {
        let engine = DuplicateEngine::new();
        assert_eq!(engine.id(), EngineId::Duplicate);
        assert_eq!(engine.name(), "Duplicate Detection");
        assert!(engine.min_size >= 1024);
    }

    #[test]
    fn test_group_by_size_filters_unique_sizes() {
        let files = vec![
            FileEntry {
                path: PathBuf::from("/a.txt"),
                size: 100,
                modified: 0,
                file_name: "a.txt".to_string(),
                blake3_hash: None,
                partial_hash: None,
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
            FileEntry {
                path: PathBuf::from("/b.txt"),
                size: 100,
                modified: 0,
                file_name: "b.txt".to_string(),
                blake3_hash: None,
                partial_hash: None,
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
            FileEntry {
                path: PathBuf::from("/c.txt"),
                size: 200,
                modified: 0,
                file_name: "c.txt".to_string(),
                blake3_hash: None,
                partial_hash: None,
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
        ];

        let groups = DuplicateEngine::group_by_size(files);
        // Only the two files with size 100 should be grouped
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].len(), 2);
    }

    #[test]
    fn test_group_by_size_returns_empty_for_all_unique() {
        let files = vec![
            FileEntry {
                path: PathBuf::from("/a.txt"),
                size: 100,
                modified: 0,
                file_name: "a.txt".to_string(),
                blake3_hash: None,
                partial_hash: None,
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
            FileEntry {
                path: PathBuf::from("/b.txt"),
                size: 200,
                modified: 0,
                file_name: "b.txt".to_string(),
                blake3_hash: None,
                partial_hash: None,
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
        ];

        let groups = DuplicateEngine::group_by_size(files);
        assert!(groups.is_empty());
    }

    #[test]
    fn test_whitelist_excludes_paths() {
        let engine = DuplicateEngine::new();
        assert!(engine.is_whitelisted(&PathBuf::from("/Users/test/.git/config")));
        assert!(engine.is_whitelisted(&PathBuf::from("/Users/test/Library/Caches/foo")));
        assert!(!engine.is_whitelisted(&PathBuf::from("/Users/test/Downloads/file.txt")));
    }

    #[test]
    fn test_should_exclude_hidden_files() {
        let engine = DuplicateEngine::new();
        // Hidden file (name starts with '.') is excluded when include_hidden is false
        assert!(engine.should_exclude(&PathBuf::from("/Users/test/.hidden_file.txt"), false));
        // Non-hidden file is not excluded
        assert!(!engine.should_exclude(&PathBuf::from("/Users/test/Downloads/file.txt"), false));
        // When include_hidden is true, hidden files are not excluded
        assert!(!engine.should_exclude(&PathBuf::from("/Users/test/.hidden_file.txt"), true));
        // Files inside hidden directories are not excluded by the hidden check
        // (directory pruning is handled during the walk)
        assert!(!engine.should_exclude(&PathBuf::from("/Users/test/.hidden/file.txt"), false));
    }

    #[test]
    fn test_build_clusters_from_hash_groups() {
        let files = vec![
            FileEntry {
                path: PathBuf::from("/a.txt"),
                size: 100,
                modified: 100,
                file_name: "a.txt".to_string(),
                blake3_hash: Some("abc123".to_string()),
                partial_hash: Some("partial".to_string()),
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
            FileEntry {
                path: PathBuf::from("/b.txt"),
                size: 100,
                modified: 200,
                file_name: "b.txt".to_string(),
                blake3_hash: Some("abc123".to_string()),
                partial_hash: Some("partial".to_string()),
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
        ];

        let clusters = DuplicateEngine::build_clusters(vec![files]);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].files.len(), 2);
        // Should keep the newest file (higher mtime)
        assert!(clusters[0].keep_file.is_some());
        assert_eq!(clusters[0].keep_file.as_ref().unwrap().modified, 200);
    }

    #[test]
    fn test_clusters_to_findings_emits_correct_metadata() {
        let files = vec![
            FileEntry {
                path: PathBuf::from("/keep.txt"),
                size: 100,
                modified: 200,
                file_name: "keep.txt".to_string(),
                blake3_hash: Some("abc123".to_string()),
                partial_hash: Some("partial".to_string()),
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
            FileEntry {
                path: PathBuf::from("/delete.txt"),
                size: 100,
                modified: 100,
                file_name: "delete.txt".to_string(),
                blake3_hash: Some("abc123".to_string()),
                partial_hash: Some("partial".to_string()),
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
        ];

        let clusters = DuplicateEngine::build_clusters(vec![files]);
        let findings = DuplicateEngine::clusters_to_findings(&clusters);

        assert_eq!(findings.len(), 1);
        let f = &findings[0];
        assert_eq!(f.engine, EngineId::Duplicate);
        assert_eq!(f.category, Category::DuplicateFile);
        assert!(f.metadata.contains_key("keep_path"));
        assert!(f.metadata.contains_key("delete_paths"));
        assert!(f.metadata.contains_key("confidence"));
        assert!(f.metadata.contains_key("hash"));
    }

    #[test]
    fn test_reclaimable_bytes_counts_only_deletable() {
        let files = vec![
            FileEntry {
                path: PathBuf::from("/keep.txt"),
                size: 500,
                modified: 200,
                file_name: "keep.txt".to_string(),
                blake3_hash: Some("abc".to_string()),
                partial_hash: Some("p".to_string()),
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
            FileEntry {
                path: PathBuf::from("/del1.txt"),
                size: 500,
                modified: 100,
                file_name: "del1.txt".to_string(),
                blake3_hash: Some("abc".to_string()),
                partial_hash: Some("p".to_string()),
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
            FileEntry {
                path: PathBuf::from("/del2.txt"),
                size: 500,
                modified: 50,
                file_name: "del2.txt".to_string(),
                blake3_hash: Some("abc".to_string()),
                partial_hash: Some("p".to_string()),
                image_fingerprint: None,
                is_screenshot: false,
                is_image: false,
            },
        ];

        let clusters = DuplicateEngine::build_clusters(vec![files]);
        assert_eq!(clusters.len(), 1);
        // 2 files to delete × 500 bytes = 1000 reclaimable
        assert_eq!(clusters[0].reclaimable_bytes(), 1000);
        assert_eq!(clusters[0].delete_files.len(), 2);
    }

    #[tokio::test]
    async fn test_detect_duplicates_finds_identical_files() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();

        // Create two identical files
        let content = b"Hello, this is duplicate content for testing!";
        std::fs::write(dir.join("original.txt"), content).unwrap();
        std::fs::write(dir.join("copy.txt"), content).unwrap();
        // Create a unique file
        std::fs::write(dir.join("unique.txt"), b"This is unique content").unwrap();

        let engine = DuplicateEngine::new()
            .with_scan_paths(vec![dir.clone()])
            .with_min_size(1)
            .with_concurrency(2);

        let (clusters, items_scanned) = engine.detect_duplicates(false, false).await;

        assert_eq!(items_scanned, 3);
        assert_eq!(clusters.len(), 1);
        assert_eq!(clusters[0].files.len(), 2);
        // Reclaimable = size of one file (the one to delete)
        assert!(clusters[0].reclaimable_bytes() > 0);
    }

    #[tokio::test]
    async fn test_detect_duplicates_no_duplicates() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();

        std::fs::write(dir.join("a.txt"), b"content a").unwrap();
        std::fs::write(dir.join("b.txt"), b"content b").unwrap();
        std::fs::write(dir.join("c.txt"), b"content c").unwrap();

        let engine = DuplicateEngine::new()
            .with_scan_paths(vec![dir])
            .with_min_size(1)
            .with_concurrency(2);

        let (clusters, items_scanned) = engine.detect_duplicates(false, false).await;

        assert_eq!(items_scanned, 3);
        assert!(clusters.is_empty());
    }

    #[tokio::test]
    async fn test_detect_duplicates_preserves_newest() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();

        let content = b"same content for mtime test";
        let old_path = dir.join("old.txt");
        let new_path = dir.join("new.txt");
        std::fs::write(&old_path, content).unwrap();
        std::fs::write(&new_path, content).unwrap();

        // Set different mtimes: old.txt is older
        let old_time = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        let new_time = std::time::SystemTime::now();
        use std::fs::FileTimes;
        let _ = std::fs::File::options()
            .write(true)
            .open(&old_path)
            .and_then(|f| f.set_times(FileTimes::new().set_modified(old_time)));
        let _ = std::fs::File::options()
            .write(true)
            .open(&new_path)
            .and_then(|f| f.set_times(FileTimes::new().set_modified(new_time)));

        let engine = DuplicateEngine::new()
            .with_scan_paths(vec![dir])
            .with_min_size(1)
            .with_concurrency(2);

        let (clusters, _) = engine.detect_duplicates(false, false).await;

        assert_eq!(clusters.len(), 1);
        let cluster = &clusters[0];
        assert!(cluster.keep_file.is_some());
        // The newest file (new.txt) should be kept
        assert_eq!(cluster.keep_file.as_ref().unwrap().path, new_path);
        // The oldest file (old.txt) should be marked for deletion
        assert_eq!(cluster.delete_files.len(), 1);
        assert_eq!(cluster.delete_files[0].path, old_path);
    }

    #[tokio::test]
    async fn test_detect_duplicates_handles_empty_dir() {
        let tmp = TempDir::new().unwrap();
        let engine = DuplicateEngine::new()
            .with_scan_paths(vec![tmp.path().to_path_buf()])
            .with_min_size(1);

        let (clusters, items_scanned) = engine.detect_duplicates(false, false).await;
        assert_eq!(items_scanned, 0);
        assert!(clusters.is_empty());
    }

    #[tokio::test]
    async fn test_detect_duplicates_skips_whitelisted() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().to_path_buf();

        let content = b"whitelist test content";
        std::fs::write(dir.join("a.txt"), content).unwrap();
        std::fs::write(dir.join("b.txt"), content).unwrap();

        // Add the temp dir to the whitelist so it gets excluded
        let engine = DuplicateEngine::new()
            .with_scan_paths(vec![dir.clone()])
            .with_min_size(1)
            .with_whitelist_entry(dir.to_string_lossy().to_string());

        let (clusters, items_scanned) = engine.detect_duplicates(false, false).await;
        assert_eq!(items_scanned, 0);
        assert!(clusters.is_empty());
    }

    #[test]
    fn test_find_similar_images_empty_for_no_fingerprints() {
        let files = vec![FileEntry {
            path: PathBuf::from("/img.png"),
            size: 100,
            modified: 0,
            file_name: "img.png".to_string(),
            blake3_hash: None,
            partial_hash: None,
            image_fingerprint: None,
            is_screenshot: false,
            is_image: true,
        }];

        let clusters = DuplicateEngine::find_similar_images(&files, 8);
        assert!(clusters.is_empty());
    }

    // --- Tests for the new free-function operations ---

    fn make_file_entry(path: &str, size: u64, modified: u64, hash: Option<&str>) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            size,
            modified,
            file_name: PathBuf::from(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            blake3_hash: hash.map(|h| h.to_string()),
            partial_hash: None,
            image_fingerprint: None,
            is_screenshot: false,
            is_image: false,
        }
    }

    #[test]
    fn test_compare_filenames_identical() {
        let a = PathBuf::from("/docs/report.txt");
        let b = PathBuf::from("/other/report.pdf");
        // Same stem "report" => similarity 1.0
        assert!((compare_filenames(&a, &b) - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_compare_filenames_similar() {
        let a = PathBuf::from("/docs/report.txt");
        let b = PathBuf::from("/docs/reports.txt");
        let sim = compare_filenames(&a, &b);
        // "report" vs "reports" — 1 edit out of 7 chars => ~0.857
        assert!(sim > 0.8 && sim < 1.0);
    }

    #[test]
    fn test_compare_filenames_different() {
        let a = PathBuf::from("/docs/report.txt");
        let b = PathBuf::from("/photos/vacation.jpg");
        let sim = compare_filenames(&a, &b);
        assert!(sim < 0.5);
    }

    #[test]
    fn test_generate_hash_identical_files() {
        let tmp = TempDir::new().unwrap();
        let p1 = tmp.path().join("a.txt");
        let p2 = tmp.path().join("b.txt");
        std::fs::write(&p1, b"hello hash").unwrap();
        std::fs::write(&p2, b"hello hash").unwrap();

        let h1 = generate_hash(&p1).unwrap();
        let h2 = generate_hash(&p2).unwrap();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_generate_hash_missing_file() {
        assert!(generate_hash(&PathBuf::from("/nonexistent/path/file.txt")).is_none());
    }

    #[test]
    fn test_compare_metadata_identical_files() {
        let tmp = TempDir::new().unwrap();
        let p1 = tmp.path().join("a.txt");
        let p2 = tmp.path().join("b.txt");
        std::fs::write(&p1, b"same content").unwrap();
        std::fs::write(&p2, b"same content").unwrap();

        let m1 = std::fs::metadata(&p1).unwrap();
        let m2 = std::fs::metadata(&p2).unwrap();

        let cmp = compare_metadata(&m1, &m2);
        assert!(cmp.size_match);
        assert_eq!(cmp.size_diff, 0);
        assert!(cmp.permissions_match);
    }

    #[test]
    fn test_compare_metadata_different_sizes() {
        let tmp = TempDir::new().unwrap();
        let p1 = tmp.path().join("a.txt");
        let p2 = tmp.path().join("b.txt");
        std::fs::write(&p1, b"short").unwrap();
        std::fs::write(&p2, b"much longer content").unwrap();

        let m1 = std::fs::metadata(&p1).unwrap();
        let m2 = std::fs::metadata(&p2).unwrap();

        let cmp = compare_metadata(&m1, &m2);
        assert!(!cmp.size_match);
        assert!(cmp.size_diff > 0);
    }

    #[test]
    fn test_detect_renamed_duplicates_finds_similar_names() {
        let files = vec![
            make_file_entry("/docs/report.txt", 100, 100, Some("abc")),
            make_file_entry("/docs/reports.txt", 100, 100, Some("abc")),
        ];
        let cluster = DuplicateCluster::from_files(files);

        let renamed = detect_renamed_duplicates(&cluster);
        assert_eq!(renamed.len(), 1);
        assert!(renamed[0].similarity > 0.5);
    }

    #[test]
    fn test_detect_renamed_duplicates_ignores_identical_names() {
        let files = vec![
            make_file_entry("/a/report.txt", 100, 100, Some("abc")),
            make_file_entry("/b/report.txt", 100, 100, Some("abc")),
        ];
        let cluster = DuplicateCluster::from_files(files);

        let renamed = detect_renamed_duplicates(&cluster);
        // Same filename => not a renamed duplicate
        assert!(renamed.is_empty());
    }

    #[test]
    fn test_detect_copied_folders_finds_identical_structure() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let folder_a = root.join("project_a");
        let folder_b = root.join("project_b");
        std::fs::create_dir_all(&folder_a).unwrap();
        std::fs::create_dir_all(&folder_b).unwrap();

        // Identical content in both folders
        std::fs::write(folder_a.join("readme.txt"), b"readme").unwrap();
        std::fs::write(folder_b.join("readme.txt"), b"readme").unwrap();
        std::fs::write(folder_a.join("code.rs"), b"fn main() {}").unwrap();
        std::fs::write(folder_b.join("code.rs"), b"fn main() {}").unwrap();

        let copies = detect_copied_folders(&root);
        assert_eq!(copies.len(), 1);
        assert_eq!(copies[0].copies.len(), 1);
    }

    #[test]
    fn test_detect_copied_folders_ignores_unique_folders() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let folder_a = root.join("unique_a");
        let folder_b = root.join("unique_b");
        std::fs::create_dir_all(&folder_a).unwrap();
        std::fs::create_dir_all(&folder_b).unwrap();
        std::fs::write(folder_a.join("a.txt"), b"content a").unwrap();
        std::fs::write(folder_b.join("b.txt"), b"different content b").unwrap();

        let copies = detect_copied_folders(&root);
        assert!(copies.is_empty());
    }

    #[test]
    fn test_detect_blurry_images_low_variance_is_blurry() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("blurry.png");
        // Uniform bytes after the 256-byte header => low variance => blurry
        let data = vec![128u8; 10_000];
        std::fs::write(&path, &data).unwrap();

        let blurry = detect_blurry_images(&path);
        assert_eq!(blurry, Some(true));
    }

    #[test]
    fn test_detect_blurry_images_high_variance_is_sharp() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("sharp.png");
        // High-variance byte pattern => sharp
        let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&path, &data).unwrap();

        let blurry = detect_blurry_images(&path);
        assert_eq!(blurry, Some(false));
    }

    #[test]
    fn test_detect_blurry_images_missing_file() {
        assert!(detect_blurry_images(&PathBuf::from("/nonexistent.png")).is_none());
    }

    #[test]
    fn test_rank_confidence_high_for_identical() {
        let tmp = TempDir::new().unwrap();
        let p1 = tmp.path().join("report.txt");
        let p2 = tmp.path().join("report.txt");
        std::fs::write(&p1, b"content").unwrap();
        std::fs::write(&p2, b"content").unwrap();

        let files = vec![
            make_file_entry(p1.to_str().unwrap(), 7, 100, Some("abc")),
            make_file_entry(p2.to_str().unwrap(), 7, 100, Some("abc")),
        ];
        let cluster = DuplicateCluster::from_files(files);

        let rank = rank_confidence(&cluster);
        assert!(rank.score >= 0.8);
        assert_eq!(rank.label, ConfidenceLabel::High);
    }

    #[test]
    fn test_rank_confidence_label_ranges() {
        // Similar (non-identical) cluster with no hash => lower confidence
        let files = vec![
            make_file_entry("/a/vacation.jpg", 100, 100, None),
            make_file_entry("/b/trip.jpg", 100, 100, None),
        ];
        let mut cluster = DuplicateCluster::from_files(files);
        cluster.similar = true;
        cluster.select_deletion_candidates();

        let rank = rank_confidence(&cluster);
        assert!(rank.score < 1.0);
    }

    #[test]
    fn test_detect_recursive_duplication_finds_nested_duplicates() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        let sub = root.join("nested/deep");
        std::fs::create_dir_all(&sub).unwrap();

        let content = b"duplicate content in nested structure";
        std::fs::write(root.join("top.txt"), content).unwrap();
        std::fs::write(sub.join("bottom.txt"), content).unwrap();

        let dups = detect_recursive_duplication(&root);
        assert_eq!(dups.len(), 1);
        assert_eq!(dups[0].paths.len(), 2);
        assert!(dups[0].depth >= 2);
    }

    #[test]
    fn test_detect_recursive_duplication_no_duplicates() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path().to_path_buf();

        std::fs::write(root.join("a.txt"), b"unique a").unwrap();
        std::fs::write(root.join("b.txt"), b"unique b").unwrap();

        let dups = detect_recursive_duplication(&root);
        assert!(dups.is_empty());
    }

    #[test]
    fn test_generate_report_with_clusters() {
        let files = vec![
            make_file_entry("/keep.txt", 500, 200, Some("abc")),
            make_file_entry("/del.txt", 500, 100, Some("abc")),
        ];
        let clusters = DuplicateEngine::build_clusters(vec![files]);

        let report = generate_report(&clusters);
        assert_eq!(report.cluster_count, 1);
        assert_eq!(report.total_files, 2);
        assert_eq!(report.total_reclaimable_bytes, 500);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_generate_report_empty() {
        let report = generate_report(&[]);
        assert_eq!(report.cluster_count, 0);
        assert_eq!(report.total_reclaimable_bytes, 0);
        assert!(!report.recommendations.is_empty());
    }

    #[test]
    fn test_is_whitelisted_free_function_matches_prefix() {
        let whitelist = vec![PathBuf::from("/Users/test/.git")];
        assert!(is_whitelisted(
            &PathBuf::from("/Users/test/.git/config"),
            &whitelist
        ));
    }

    #[test]
    fn test_is_whitelisted_free_function_no_match() {
        let whitelist = vec![PathBuf::from("/Users/test/.git")];
        assert!(!is_whitelisted(
            &PathBuf::from("/Users/test/Downloads/file.txt"),
            &whitelist
        ));
    }

    #[test]
    fn test_scan_database_record_and_lookup() {
        let mut db = ScanDatabase::new();
        db.record(PathBuf::from("/a.txt"), "abc123".to_string(), 100, 1000);

        assert!(db.is_unchanged(&PathBuf::from("/a.txt"), 100, 1000));
        assert!(!db.is_unchanged(&PathBuf::from("/a.txt"), 200, 1000));
        assert_eq!(db.hash_for(&PathBuf::from("/a.txt")), Some("abc123"));
        assert_eq!(db.hash_for(&PathBuf::from("/missing.txt")), None);
    }

    #[test]
    fn test_scan_database_save_and_load() {
        let tmp = TempDir::new().unwrap();
        let db_path = tmp.path().join("scan_db.json");

        let mut db = ScanDatabase::new();
        db.record(PathBuf::from("/a.txt"), "hash_a".to_string(), 100, 1000);
        db.record(PathBuf::from("/b.txt"), "hash_b".to_string(), 200, 2000);
        db.save(&db_path).unwrap();

        let loaded = ScanDatabase::load(&db_path).unwrap();
        assert_eq!(loaded.entries.len(), 2);
        assert!(loaded.is_unchanged(&PathBuf::from("/a.txt"), 100, 1000));
        assert_eq!(loaded.hash_for(&PathBuf::from("/b.txt")), Some("hash_b"));
    }

    #[test]
    fn test_scan_database_default_is_empty() {
        let db = ScanDatabase::default();
        assert!(db.entries.is_empty());
    }

    #[test]
    fn test_with_config_applies_profile_overrides() {
        use crate::config::profiles::OptimizationProfile;
        use crate::config::Config;

        let mut config = Config::default();
        config.profile = OptimizationProfile::Aggressive;
        let engine = DuplicateEngine::new().with_config(&config);
        assert!(engine.similar_images);
        assert!(engine.enabled);
        assert_eq!(engine.min_size, 1024);
    }

    #[test]
    fn test_with_config_conservative_raises_min_size() {
        use crate::config::profiles::OptimizationProfile;
        use crate::config::Config;

        let mut config = Config::default();
        config.profile = OptimizationProfile::Conservative;
        let engine = DuplicateEngine::new().with_config(&config);
        assert_eq!(engine.min_size, 10 * 1024 * 1024);
        assert!(!engine.similar_images);
    }
}
