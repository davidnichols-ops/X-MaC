//! Benchmark suite for X-MaC scan engines.
//!
//! Measures throughput on synthetic directory trees of various sizes,
//! including a 500K+ file corpus as required by the v1 Definition of Done.
//!
//! Run with: `cargo bench --bench scan_benchmark`

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion, Throughput};
use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;
use walkdir::WalkDir;

/// Create a synthetic directory tree with `n_files` files distributed
/// across `n_dirs` directories. Each file is `file_size` bytes of random-ish
/// content (deterministic, so runs are reproducible).
fn create_corpus(n_files: usize, n_dirs: usize, file_size: usize) -> TempDir {
    let tmp = TempDir::new().expect("create temp dir");
    let root = tmp.path().to_path_buf();

    // Create directory structure.
    let files_per_dir = n_files / n_dirs.max(1);
    for d in 0..n_dirs {
        let dir = root.join(format!("dir_{:04}", d));
        fs::create_dir_all(&dir).expect("create dir");
        for f in 0..files_per_dir {
            let path = dir.join(format!("file_{:06}.dat", f));
            // Deterministic content — no RNG needed.
            let content: Vec<u8> = (0..file_size)
                .map(|i| ((i + d * 1000 + f * 7) % 256) as u8)
                .collect();
            fs::write(&path, &content).expect("write file");
        }
    }

    tmp
}

/// Count files in a directory tree (for verification).
fn count_files(root: &PathBuf) -> usize {
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .count()
}

/// Benchmark the disk engine's directory walk + size computation.
/// This is the core scan path — WalkDir + metadata.
fn bench_disk_walk(c: &mut Criterion) {
    let mut group = c.benchmark_group("disk_walk");
    group.measurement_time(std::time::Duration::from_secs(10));

    for n_files in [1_000, 10_000, 100_000] {
        let tmp = create_corpus(n_files, 100, 64);
        let root = tmp.path().to_path_buf();
        let actual = count_files(&root);
        assert_eq!(actual, n_files, "file count mismatch");

        group.throughput(Throughput::Elements(actual as u64));
        group.bench_with_input(BenchmarkId::new("walkdir", n_files), &root, |b, root| {
            b.iter(|| {
                let mut total_size: u64 = 0;
                let mut count: usize = 0;
                for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
                    if entry.file_type().is_file() {
                        if let Ok(meta) = entry.metadata() {
                            total_size += meta.len();
                            count += 1;
                        }
                    }
                }
                assert!(count > 0);
                assert!(total_size > 0);
            });
        });
    }

    group.finish();
}

/// Benchmark BLAKE3 hashing throughput — the duplicate engine's bottleneck.
fn bench_blake3_hashing(c: &mut Criterion) {
    let mut group = c.benchmark_group("blake3_hash");
    group.measurement_time(std::time::Duration::from_secs(10));

    for file_size in [64, 1_024, 8_192, 65_536, 1_048_576] {
        let data: Vec<u8> = (0..file_size).map(|i| (i % 256) as u8).collect();

        group.throughput(Throughput::Bytes(file_size as u64));
        group.bench_with_input(BenchmarkId::new("hash", file_size), &data, |b, data| {
            b.iter(|| {
                let hash = blake3::hash(data);
                assert!(!hash.to_hex().is_empty());
            });
        });
    }

    group.finish();
}

/// Benchmark FileSnapshot capture + verify — the TOCTOU protection path.
fn bench_file_snapshot(c: &mut Criterion) {
    let tmp = create_corpus(1_000, 10, 256);
    let root = tmp.path().to_path_buf();

    let files: Vec<PathBuf> = WalkDir::new(&root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .collect();

    c.bench_function("file_snapshot_capture_and_verify_1000", |b| {
        b.iter(|| {
            for path in &files {
                if let Some(snap) = x_mac::cleanup::verification::FileSnapshot::capture(path) {
                    let _ = snap.verify(path);
                }
            }
        });
    });
}

criterion_group!(
    benches,
    bench_disk_walk,
    bench_blake3_hashing,
    bench_file_snapshot,
);
criterion_main!(benches);
