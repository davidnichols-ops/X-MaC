//! Performance benchmarks for scan-critical paths.
//!
//! These benchmarks measure throughput on synthetic directory trees to
//! establish a baseline for real-world performance. Run with:
//!
//! ```bash
//! cargo test --lib -- benchmarks --nocapture --test-threads=1
//! ```
//!
//! Or for a quick check:
//! ```bash
//! cargo test --lib -- benchmarks --nocapture
//! ```

use std::path::PathBuf;
use std::time::Instant;
use tempfile::TempDir;
use walkdir::WalkDir;

#[cfg(unix)]
use std::os::unix::fs::MetadataExt;

/// Generate a synthetic directory tree with `n_files` files spread across
/// `n_dirs` subdirectories. Each file is `file_size` bytes of random-ish data.
/// Returns the temp directory (keep it alive for the benchmark duration).
fn generate_tree(n_files: usize, n_dirs: usize, file_size: usize) -> TempDir {
    let tmp = TempDir::new().expect("create temp dir");
    let files_per_dir = n_files / n_dirs.max(1);

    for d in 0..n_dirs {
        let dir = tmp.path().join(format!("d{}", d));
        std::fs::create_dir_all(&dir).expect("create dir");
        for f in 0..files_per_dir {
            let path = dir.join(format!("file_{}.bin", f));
            // Write deterministic but varied content
            let content: Vec<u8> = (0..file_size)
                .map(|i| ((i + d * 137 + f * 251) % 256) as u8)
                .collect();
            std::fs::write(&path, &content).expect("write file");
        }
    }
    tmp
}

/// Count files in a directory tree (for reporting throughput).
fn count_files(path: &std::path::Path) -> usize {
    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

/// Format throughput as "N files in T.Ms (X files/s, Y MB/s)".
fn format_throughput(n_files: usize, elapsed_secs: f64, bytes: u64) -> String {
    let files_per_sec = n_files as f64 / elapsed_secs;
    let mb_per_sec = bytes as f64 / 1_048_576.0 / elapsed_secs;
    format!(
        "{} files in {:.2}s ({:.0} files/s, {:.1} MB/s)",
        n_files, elapsed_secs, files_per_sec, mb_per_sec
    )
}

// ---------------------------------------------------------------------------
// Disk walk benchmarks
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    /// Benchmark: directory walk + size summation on 1,000 files.
    /// This is the hot path for the disk engine's `dir_size_fast`.
    #[test]
    fn bench_dir_walk_1k_files() {
        let tmp = generate_tree(1_000, 10, 4_096);
        let n = count_files(tmp.path());

        let start = Instant::now();
        let total_size: u64 = WalkDir::new(tmp.path())
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();
        let elapsed = start.elapsed();

        println!(
            "bench_dir_walk_1k_files: {}",
            format_throughput(n, elapsed.as_secs_f64(), total_size)
        );
        assert!(total_size > 0);
        assert!(n == 1_000);
    }

    /// Benchmark: directory walk + size summation on 10,000 files.
    #[test]
    fn bench_dir_walk_10k_files() {
        let tmp = generate_tree(10_000, 50, 4_096);
        let n = count_files(tmp.path());

        let start = Instant::now();
        let total_size: u64 = WalkDir::new(tmp.path())
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.len())
            .sum();
        let elapsed = start.elapsed();

        println!(
            "bench_dir_walk_10k_files: {}",
            format_throughput(n, elapsed.as_secs_f64(), total_size)
        );
        assert!(total_size > 0);
    }

    /// Benchmark: directory walk with physical size (blocks * 512).
    /// This is what `dir_size_physical` does — measures the overhead of
    /// accessing block counts vs logical size.
    #[test]
    fn bench_dir_walk_physical_10k() {
        let tmp = generate_tree(10_000, 50, 4_096);
        let n = count_files(tmp.path());

        let start = Instant::now();
        let total_size: u64 = WalkDir::new(tmp.path())
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(|m| m.blocks() * 512)
            .sum();
        let elapsed = start.elapsed();

        println!(
            "bench_dir_walk_physical_10k: {}",
            format_throughput(n, elapsed.as_secs_f64(), total_size)
        );
        assert!(total_size > 0);
    }

    // ---------------------------------------------------------------------------
    // BLAKE3 hashing benchmarks
    // ---------------------------------------------------------------------------

    /// Benchmark: full BLAKE3 hash of 100 small files (4KB each).
    /// Measures the hashing overhead for typical cache files.
    #[tokio::test]
    async fn bench_full_hash_100_small_files() {
        let tmp = generate_tree(100, 1, 4_096);
        let files: Vec<PathBuf> = WalkDir::new(tmp.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        let start = Instant::now();
        for path in &files {
            let _ = crate::engines::duplicate::hasher::full_hash(path)
                .await
                .unwrap();
        }
        let elapsed = start.elapsed();
        let total_bytes = files.len() * 4_096;

        println!(
            "bench_full_hash_100_small_files: {}",
            format_throughput(files.len(), elapsed.as_secs_f64(), total_bytes as u64)
        );
    }

    /// Benchmark: full BLAKE3 hash of 10 medium files (1MB each).
    #[tokio::test]
    async fn bench_full_hash_10_medium_files() {
        let tmp = generate_tree(10, 1, 1_048_576);
        let files: Vec<PathBuf> = WalkDir::new(tmp.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        let start = Instant::now();
        for path in &files {
            let _ = crate::engines::duplicate::hasher::full_hash(path)
                .await
                .unwrap();
        }
        let elapsed = start.elapsed();
        let total_bytes = files.len() * 1_048_576;

        println!(
            "bench_full_hash_10_medium_files: {}",
            format_throughput(files.len(), elapsed.as_secs_f64(), total_bytes as u64)
        );
    }

    /// Benchmark: partial BLAKE3 hash (first/last 4KB) of 100 files.
    /// This is the fast pre-filter path — should be much faster than full hash.
    #[tokio::test]
    async fn bench_partial_hash_100_files() {
        let tmp = generate_tree(100, 1, 100_000);
        let files: Vec<PathBuf> = WalkDir::new(tmp.path())
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| e.path().to_path_buf())
            .collect();

        let start = Instant::now();
        for path in &files {
            let _ = crate::engines::duplicate::hasher::partial_hash(path, 4_096)
                .await
                .unwrap();
        }
        let elapsed = start.elapsed();

        println!(
            "bench_partial_hash_100_files: {} (partial only, 8KB read per file)",
            format_throughput(files.len(), elapsed.as_secs_f64(), 0)
        );
    }

    // ---------------------------------------------------------------------------
    // Duplicate detection end-to-end benchmark
    // ---------------------------------------------------------------------------

    /// Benchmark: duplicate detection on 500 files with 10 duplicate pairs.
    /// Measures the full pipeline: walk → size group → partial hash → full hash.
    #[tokio::test]
    async fn bench_duplicate_detection_500_files() {
        let tmp = generate_tree(500, 10, 8_192);
        let n = count_files(tmp.path());

        // Create 10 duplicate pairs by copying files
        let source_dir = tmp.path().join("d0");
        for i in 0..10 {
            let src = source_dir.join(format!("file_{}.bin", i));
            let dst = tmp.path().join("d1").join(format!("copy_{}.bin", i));
            if src.exists() {
                std::fs::copy(&src, &dst).expect("copy file");
            }
        }

        let engine = crate::engines::duplicate::engine::DuplicateEngine::new()
            .with_scan_paths(vec![tmp.path().to_path_buf()])
            .with_min_size(1);

        let start = Instant::now();
        let (clusters, items_scanned) = engine.detect_duplicates(false, false).await;
        let elapsed = start.elapsed();

        let dup_count: usize = clusters.iter().map(|c| c.files.len()).sum();
        println!(
            "bench_duplicate_detection_500_files: {} files scanned, {} dup clusters ({} dup files), {:.2}s",
            items_scanned, clusters.len(), dup_count, elapsed.as_secs_f64()
        );
        assert!(items_scanned >= n as u64);
    }

    /// Benchmark: duplicate detection on 2,000 files (larger scale).
    #[tokio::test]
    async fn bench_duplicate_detection_2k_files() {
        let tmp = generate_tree(2_000, 40, 4_096);

        // Create 20 duplicate pairs
        let source_dir = tmp.path().join("d0");
        for i in 0..20 {
            let src = source_dir.join(format!("file_{}.bin", i));
            let dst = tmp.path().join("d1").join(format!("copy_{}.bin", i));
            if src.exists() {
                std::fs::copy(&src, &dst).expect("copy file");
            }
        }

        let engine = crate::engines::duplicate::engine::DuplicateEngine::new()
            .with_scan_paths(vec![tmp.path().to_path_buf()])
            .with_min_size(1);

        let start = Instant::now();
        let (clusters, items_scanned) = engine.detect_duplicates(false, false).await;
        let elapsed = start.elapsed();

        println!(
            "bench_duplicate_detection_2k_files: {} files scanned, {} clusters, {:.2}s ({:.0} files/s)",
            items_scanned, clusters.len(), elapsed.as_secs_f64(),
            items_scanned as f64 / elapsed.as_secs_f64()
        );
    }

    // ---------------------------------------------------------------------------
    // Scan cache benchmark (incremental rescan)
    // ---------------------------------------------------------------------------

    /// Benchmark: build scan cache + incremental rescan (no changes).
    /// Measures the overhead of the cache-based incremental scan path.
    #[test]
    fn bench_scan_cache_build_and_rescan() {
        let tmp = generate_tree(1_000, 10, 2_048);
        let n = count_files(tmp.path());

        // Build cache
        let start = Instant::now();
        let cache = crate::engines::disk::engine::DiskEngine::build_scan_cache(tmp.path());
        let build_secs = start.elapsed().as_secs_f64();

        // Save cache outside the scanned tree so rescan sees no new files
        let cache_dir = TempDir::new().expect("cache dir");
        let cache_path = cache_dir.path().join(".xmac_scan_cache.json");
        let start = Instant::now();
        crate::engines::disk::engine::save_scan_cache(&cache, &cache_path).expect("save cache");
        let save_secs = start.elapsed().as_secs_f64();

        // Load cache
        let start = Instant::now();
        let loaded =
            crate::engines::disk::engine::load_scan_cache(&cache_path).expect("load cache");
        let load_secs = start.elapsed().as_secs_f64();

        // Incremental rescan (no changes)
        let start = Instant::now();
        let diff = crate::engines::disk::engine::incremental_rescan(tmp.path(), &loaded);
        let rescan_secs = start.elapsed().as_secs_f64();

        println!(
            "bench_scan_cache: {} files — build {:.2}s, save {:.3}s, load {:.3}s, rescan {:.2}s (0 changes: {} added, {} modified, {} removed)",
            n, build_secs, save_secs, load_secs, rescan_secs,
            diff.added.len(), diff.modified.len(), diff.removed.len()
        );
        assert!(diff.added.is_empty());
        assert!(diff.removed.is_empty());
    }

    /// Benchmark: incremental rescan with 100 added files.
    #[test]
    fn bench_incremental_rescan_with_additions() {
        let tmp = generate_tree(1_000, 10, 2_048);
        let cache = crate::engines::disk::engine::DiskEngine::build_scan_cache(tmp.path());

        // Add 100 new files
        let dir = tmp.path().join("new_dir");
        std::fs::create_dir_all(&dir).expect("mkdir");
        for i in 0..100 {
            std::fs::write(dir.join(format!("new_{}.bin", i)), vec![0u8; 1024]).expect("write");
        }

        let start = Instant::now();
        let diff = crate::engines::disk::engine::incremental_rescan(tmp.path(), &cache);
        let rescan_secs = start.elapsed().as_secs_f64();

        println!(
            "bench_incremental_rescan_additions: 1100 files (100 added) — rescan {:.2}s, found {} added",
            rescan_secs, diff.added.len()
        );
        assert_eq!(diff.added.len(), 100);
    }
}
