//! BLAKE3 hashing utilities for duplicate detection.
//!
//! Implements operations 84 (generate hashes), 85 (generate partial hashes),
//! 86 (generate full hashes), 87 (compare hashes).
//!
//! The hashing strategy uses a two-phase approach for efficiency:
//! 1. **Partial hash** — hash the first and last N bytes of each file. This
//!    is a fast pre-filter that eliminates most non-duplicates without
//!    reading the entire file.
//! 2. **Full hash** — hash the entire file. Only performed on files that
//!    share the same size AND partial hash, dramatically reducing I/O.

use std::path::Path;

/// Compute a full BLAKE3 hash of a file. Returns the hex-encoded hash string.
pub async fn full_hash(path: &Path) -> Result<String, std::io::Error> {
    use tokio::io::AsyncReadExt;

    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; 65536];

    loop {
        let bytes_read = file.read(&mut buffer).await?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Compute a partial BLAKE3 hash of a file by hashing the first and last
/// `sample_size` bytes. For files smaller than `2 * sample_size`, the entire
/// file is hashed.
///
/// Returns the hex-encoded hash string.
pub async fn partial_hash(path: &Path, sample_size: u64) -> Result<String, std::io::Error> {
    use tokio::io::{AsyncReadExt, AsyncSeekExt};

    let metadata = tokio::fs::metadata(path).await?;
    let file_size = metadata.len();

    let mut file = tokio::fs::File::open(path).await?;
    let mut hasher = blake3::Hasher::new();
    let mut buffer = vec![0u8; sample_size as usize];

    // If the file is small enough, just hash the whole thing
    if file_size <= sample_size * 2 {
        let mut buf = vec![0u8; file_size as usize];
        file.read_exact(&mut buf).await?;
        hasher.update(&buf);
        return Ok(hasher.finalize().to_hex().to_string());
    }

    // Hash the first N bytes
    let first_read = file.read(&mut buffer).await?;
    if first_read == 0 {
        return Ok(hasher.finalize().to_hex().to_string());
    }
    hasher.update(&buffer[..first_read]);

    // Seek to the last N bytes
    let seek_pos = file_size.saturating_sub(sample_size);
    file.seek(std::io::SeekFrom::Start(seek_pos)).await?;

    // Hash the last N bytes
    let mut last_buffer = vec![0u8; sample_size as usize];
    let last_read = file.read(&mut last_buffer).await?;
    if last_read > 0 {
        hasher.update(&last_buffer[..last_read]);
    }

    Ok(hasher.finalize().to_hex().to_string())
}

/// Compare two hash strings for equality. (op 87)
#[allow(dead_code)]
pub fn hashes_match(a: &str, b: &str) -> bool {
    a == b
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_full_hash_identical_files() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("a.txt");
        let path2 = tmp.path().join("b.txt");

        std::fs::write(&path1, b"hello world").unwrap();
        std::fs::write(&path2, b"hello world").unwrap();

        let hash1 = full_hash(&path1).await.unwrap();
        let hash2 = full_hash(&path2).await.unwrap();

        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_full_hash_different_files() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("a.txt");
        let path2 = tmp.path().join("b.txt");

        std::fs::write(&path1, b"hello world").unwrap();
        std::fs::write(&path2, b"hello earth").unwrap();

        let hash1 = full_hash(&path1).await.unwrap();
        let hash2 = full_hash(&path2).await.unwrap();

        assert_ne!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_partial_hash_small_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("small.txt");
        std::fs::write(&path, b"tiny").unwrap();

        let partial = partial_hash(&path, 4096).await.unwrap();
        let full = full_hash(&path).await.unwrap();

        // For small files, partial hash should equal full hash
        assert_eq!(partial, full);
    }

    #[tokio::test]
    async fn test_partial_hash_large_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("large.bin");

        // Create a file larger than 2 * sample_size
        let content: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&path, &content).unwrap();

        let partial = partial_hash(&path, 4096).await.unwrap();
        let full = full_hash(&path).await.unwrap();

        // Partial and full should differ for large files
        assert_ne!(partial, full);
    }

    #[tokio::test]
    async fn test_partial_hash_identical_large_files() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("a.bin");
        let path2 = tmp.path().join("b.bin");

        let content: Vec<u8> = (0..100_000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&path1, &content).unwrap();
        std::fs::write(&path2, &content).unwrap();

        let partial1 = partial_hash(&path1, 4096).await.unwrap();
        let partial2 = partial_hash(&path2, 4096).await.unwrap();

        assert_eq!(partial1, partial2);
    }

    #[tokio::test]
    async fn test_partial_hash_different_large_files_same_ends() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("a.bin");
        let path2 = tmp.path().join("b.bin");

        // Same first and last 4KB, different middle
        let mut content1 = vec![0u8; 100_000];
        let mut content2 = vec![0u8; 100_000];
        // Fill first and last 4096 bytes identically
        for i in 0..4096 {
            content1[i] = (i % 256) as u8;
            content2[i] = (i % 256) as u8;
            content1[99_999 - i] = (i % 256) as u8;
            content2[99_999 - i] = (i % 256) as u8;
        }
        // Different middle
        content1[50_000] = 1;
        content2[50_000] = 2;

        std::fs::write(&path1, &content1).unwrap();
        std::fs::write(&path2, &content2).unwrap();

        let partial1 = partial_hash(&path1, 4096).await.unwrap();
        let partial2 = partial_hash(&path2, 4096).await.unwrap();

        // Partial hashes should match (same first/last 4KB)
        assert_eq!(partial1, partial2);

        // But full hashes should differ
        let full1 = full_hash(&path1).await.unwrap();
        let full2 = full_hash(&path2).await.unwrap();
        assert_ne!(full1, full2);
    }

    #[test]
    fn test_hashes_match() {
        assert!(hashes_match("abc123", "abc123"));
        assert!(!hashes_match("abc123", "abc124"));
    }
}
