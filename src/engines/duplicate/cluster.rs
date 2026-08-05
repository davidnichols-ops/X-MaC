//! Duplicate cluster data structures and deletion candidate selection.
//!
//! Implements operations 92 (group duplicate clusters), 99 (rank confidence),
//! 100 (select safest deletion candidate), 101 (preserve newest), 102
//! (preserve highest-resolution image), 103 (preserve original path).

use super::scanner::ImageFingerprint;

/// A single file entry with metadata and hash information.
#[derive(Debug, Clone)]
pub struct FileEntry {
    /// Full filesystem path to the file.
    pub path: std::path::PathBuf,
    /// Physical size on disk in bytes.
    pub size: u64,
    /// Modification time as Unix timestamp (seconds).
    pub modified: u64,
    /// File name (last path component).
    #[allow(dead_code)]
    pub file_name: String,
    /// Full BLAKE3 hash (hex string), computed during scan.
    pub blake3_hash: Option<String>,
    /// Partial BLAKE3 hash (hex string), computed from first/last N bytes.
    pub partial_hash: Option<String>,
    /// Perceptual fingerprint for image files.
    pub image_fingerprint: Option<ImageFingerprint>,
    /// Whether this file matches the screenshot filename pattern.
    pub is_screenshot: bool,
    /// Whether this file is an image (by extension).
    pub is_image: bool,
}

/// A cluster of duplicate (or similar) files.
#[derive(Debug, Clone)]
pub struct DuplicateCluster {
    /// All files in the cluster (including the one to keep).
    pub files: Vec<FileEntry>,
    /// The BLAKE3 hash shared by all files in the cluster (None for similar
    /// image clusters that are not byte-identical).
    pub hash: Option<String>,
    /// The file recommended to keep (safest candidate).
    pub keep_file: Option<FileEntry>,
    /// Files recommended for deletion (all except the keep file).
    pub delete_files: Vec<FileEntry>,
    /// Confidence score (0.0–1.0) that these are true duplicates.
    pub confidence: f64,
    /// Whether this is a "similar" cluster (perceptual match) rather than
    /// an "identical" cluster (byte-exact match).
    pub similar: bool,
}

impl DuplicateCluster {
    /// Build a cluster from a group of files that share the same hash.
    /// Automatically selects the safest deletion candidate.
    pub fn from_files(files: Vec<FileEntry>) -> Self {
        let hash = files.first().and_then(|f| f.blake3_hash.clone());
        let mut cluster = Self {
            files,
            hash,
            keep_file: None,
            delete_files: Vec::new(),
            confidence: 1.0,
            similar: false,
        };
        cluster.select_deletion_candidates();
        cluster
    }

    /// Select the safest deletion candidate using a multi-factor scoring
    /// algorithm. The file to **keep** is chosen by:
    ///
    /// 1. **Newest mtime** — preserve the most recently modified file (op 101)
    /// 2. **Largest size** — if mtimes are equal, keep the larger file (op 102)
    /// 3. **Original path** — if sizes are equal, prefer the path that looks
    ///    more "original" (shorter path, not in a "copy"/"backup" dir) (op 103)
    ///
    /// All other files in the cluster are marked for deletion.
    pub fn select_deletion_candidates(&mut self) {
        if self.files.is_empty() {
            return;
        }

        // Sort files by priority: newest first, then largest, then "most original"
        let mut sorted = self.files.clone();
        sorted.sort_by(|a, b| {
            // Primary: newest mtime (descending)
            b.modified
                .cmp(&a.modified)
                // Secondary: largest size (descending)
                .then(b.size.cmp(&a.size))
                // Tertiary: "originality" score (ascending — lower = more original)
                .then_with(|| path_originality_score(&a.path).cmp(&path_originality_score(&b.path)))
                // Quaternary: alphabetical path for deterministic ordering
                .then(a.path.cmp(&b.path))
        });

        let keep = sorted.remove(0);
        self.keep_file = Some(keep);
        self.delete_files = sorted;

        // Confidence: 1.0 for identical hashes, lower for similar images
        if self.similar {
            self.confidence = 0.75;
        } else if self.hash.is_some() {
            self.confidence = 1.0;
        } else {
            self.confidence = 0.5;
        }
    }

    /// Calculate the total reclaimable bytes (sum of all delete_files sizes).
    pub fn reclaimable_bytes(&self) -> u64 {
        self.delete_files.iter().map(|f| f.size).sum()
    }
}

/// Score a path's "originality" — lower scores indicate more original paths.
/// Paths containing "copy", "backup", " (1)", " (2)" etc. are less original.
fn path_originality_score(path: &std::path::Path) -> u32 {
    let path_str = path.to_string_lossy().to_lowercase();
    let mut score = 0u32;

    // Penalize paths that look like copies
    if path_str.contains("copy") {
        score += 10;
    }
    if path_str.contains("backup") {
        score += 10;
    }
    if path_str.contains(" (1)") || path_str.contains(" (2)") || path_str.contains(" (3)") {
        score += 5;
    }
    if path_str.contains("-copy") || path_str.contains("_copy") {
        score += 5;
    }
    // Shorter paths are slightly more "original"
    score += path_str.len().min(100) as u32 / 10;

    score
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn make_file(path: &str, size: u64, modified: u64) -> FileEntry {
        FileEntry {
            path: PathBuf::from(path),
            size,
            modified,
            file_name: PathBuf::from(path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default(),
            blake3_hash: Some("abc123".to_string()),
            partial_hash: Some("partial".to_string()),
            image_fingerprint: None,
            is_screenshot: false,
            is_image: false,
        }
    }

    #[test]
    fn test_select_deletion_keeps_newest() {
        let files = vec![
            make_file("/old.txt", 100, 100),
            make_file("/new.txt", 100, 200),
        ];
        let cluster = DuplicateCluster::from_files(files);
        assert_eq!(cluster.keep_file.as_ref().unwrap().modified, 200);
        assert_eq!(cluster.delete_files.len(), 1);
        assert_eq!(cluster.delete_files[0].modified, 100);
    }

    #[test]
    fn test_select_deletion_keeps_largest_on_equal_mtime() {
        let files = vec![
            make_file("/small.txt", 100, 200),
            make_file("/big.txt", 500, 200),
        ];
        let cluster = DuplicateCluster::from_files(files);
        assert_eq!(cluster.keep_file.as_ref().unwrap().size, 500);
    }

    #[test]
    fn test_select_deletion_prefers_original_path() {
        let files = vec![
            make_file("/Documents/original.txt", 100, 200),
            make_file("/Downloads/copy of original.txt", 100, 200),
        ];
        let cluster = DuplicateCluster::from_files(files);
        // The original path should be kept (lower originality score)
        assert_eq!(
            cluster.keep_file.as_ref().unwrap().path,
            PathBuf::from("/Documents/original.txt")
        );
    }

    #[test]
    fn test_reclaimable_bytes() {
        let files = vec![
            make_file("/keep.txt", 100, 200),
            make_file("/del1.txt", 100, 100),
            make_file("/del2.txt", 100, 50),
        ];
        let cluster = DuplicateCluster::from_files(files);
        // 2 files to delete × 100 bytes = 200
        assert_eq!(cluster.reclaimable_bytes(), 200);
    }

    #[test]
    fn test_confidence_identical() {
        let files = vec![make_file("/a.txt", 100, 200), make_file("/b.txt", 100, 100)];
        let cluster = DuplicateCluster::from_files(files);
        assert_eq!(cluster.confidence, 1.0);
    }

    #[test]
    fn test_confidence_similar() {
        let files = vec![make_file("/a.png", 100, 200), make_file("/b.png", 100, 100)];
        let mut cluster = DuplicateCluster::from_files(files);
        cluster.similar = true;
        cluster.select_deletion_candidates();
        assert_eq!(cluster.confidence, 0.75);
    }

    #[test]
    fn test_empty_cluster() {
        let cluster = DuplicateCluster::from_files(Vec::new());
        assert!(cluster.keep_file.is_none());
        assert!(cluster.delete_files.is_empty());
    }

    #[test]
    fn test_path_originality_score_penalizes_copies() {
        assert!(
            path_originality_score(&PathBuf::from("/original.txt"))
                < path_originality_score(&PathBuf::from("/copy of original.txt"))
        );
        assert!(
            path_originality_score(&PathBuf::from("/file.txt"))
                < path_originality_score(&PathBuf::from("/file (1).txt"))
        );
    }
}
