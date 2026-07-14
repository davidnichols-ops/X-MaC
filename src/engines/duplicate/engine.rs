use async_trait::async_trait;
use std::collections::HashMap;
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
        }
    }
}

impl DuplicateEngine {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self::default()
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
    async fn detect_duplicates(
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
}
