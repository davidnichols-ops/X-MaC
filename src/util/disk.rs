use std::path::Path;
use walkdir::WalkDir;

/// Sum the **physical** disk usage (blocks × 512) of all files under a path.
///
/// Uses `st_blocks()` rather than `len()` to correctly handle sparse files
/// (e.g. Docker.raw, sparse VM disk images) that report a large logical
/// size but occupy far less physical space on disk.
pub fn dir_size(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }

    WalkDir::new(path)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(physical_size)
        .sum()
}

/// Physical disk usage of a single file (actual blocks used, not logical
/// size). Falls back to logical size if blocks is 0 (some filesystems).
pub fn file_size_physical(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(physical_size)
        .unwrap_or(0)
}

/// Get physical size from metadata: `st_blocks * 512`, falling back to
/// logical `len()` if blocks is 0.
pub fn physical_size(m: std::fs::Metadata) -> u64 {
    use std::os::unix::fs::MetadataExt;
    let blocks = <std::fs::Metadata as MetadataExt>::blocks(&m) * 512;
    if blocks > 0 { blocks } else { m.len() }
}

pub fn format_bytes(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    format!("{:.1} {}", size, UNITS[unit_index])
}

