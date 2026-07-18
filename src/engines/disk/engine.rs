use async_trait::async_trait;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Instant, SystemTime};
use walkdir::WalkDir;

use crate::cli::args::DiskArgs;
use crate::core::context::ScanContext;
use crate::core::engine::Engine;
use crate::core::error::EngineError;
use crate::core::types::{Category, EngineId, EngineStats, Finding, Severity, Target};
use crate::util::disk::physical_size;

/// Disk volume breakdown: total container capacity, free space,
/// system-volume used, and data-volume used — queried via `diskutil info`
/// for accuracy on APFS (statvfs on the snapshot root is unreliable).
struct ApfsStats {
    /// Total APFS container capacity in bytes.
    total: u64,
    /// APFS container free space in bytes.
    free: u64,
    /// macOS system volume used bytes (Macintosh HD, read-only sealed).
    system_used: u64,
    /// User data volume used bytes (the writable "Data" volume).
    data_used: u64,
    /// Size of /Applications directory in bytes (best-effort).
    applications_used: u64,
}

/// A removable/external disk detected via `diskutil list`.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct RemovableDisk {
    /// Device identifier (e.g. `/dev/disk4`).
    pub identifier: String,
    /// Human-readable description (e.g. "external, physical").
    pub description: String,
}

/// A single entry in the filesystem scan cache.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanCacheEntry {
    /// Absolute file path.
    pub path: String,
    /// Physical size in bytes.
    pub size: u64,
    /// Last modification time as Unix timestamp seconds.
    pub modified_secs: Option<i64>,
}

/// Cached filesystem scan results — a simple JSON-serializable snapshot
/// of a directory tree's file sizes and modification times.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScanCache {
    /// Cache format version.
    pub version: u32,
    /// Root directory that was scanned.
    pub root: String,
    /// Cached file entries.
    pub entries: Vec<ScanCacheEntry>,
}

/// Result of comparing a cached scan with the current filesystem state.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct IncrementalScanResult {
    /// Files present now but not in the cache.
    pub added: Vec<PathBuf>,
    /// Files present in both but with changed size or mtime.
    pub modified: Vec<PathBuf>,
    /// Files in the cache but no longer present.
    pub removed: Vec<PathBuf>,
}

/// Async version of `apfs_stats` — offloads the blocking `dir_size_fast`
/// calls to spawn_blocking so the disk engine doesn't stall the async
/// runtime when running concurrently with other engines.
async fn apfs_stats_async() -> Option<ApfsStats> {
    // Run the diskutil commands and plist parsing on a blocking thread.
    let base = tokio::task::spawn_blocking(|| {
        // Reuse the synchronous apfs_stats but with zeroed applications_used;
        // we'll compute that part async below.
        apfs_stats_base()
    })
    .await
    .ok()?;

    let mut stats = base?;

    // Compute /Applications sizes concurrently via spawn_blocking.
    let apps_path = PathBuf::from("/Applications");
    let sys_apps_path = PathBuf::from("/System/Applications");
    let (apps_size, sys_apps_size) = tokio::join!(
        dir_size_fast_async(apps_path),
        dir_size_fast_async(sys_apps_path),
    );
    stats.applications_used = apps_size + sys_apps_size;
    Some(stats)
}

/// Synchronous base for apfs_stats — returns stats with applications_used=0.
/// The applications size is computed separately via async spawn_blocking.
fn apfs_stats_base() -> Option<ApfsStats> {
    // Ask diskutil for the APFS container that hosts the boot volume.
    use std::process::Command;

    fn parse_bytes(plist_output: &str, key: &str) -> Option<u64> {
        let needle = format!("<key>{}</key>", key);
        let pos = plist_output.find(&needle)?;
        let after = &plist_output[pos + needle.len()..];
        let int_start = after.find("<integer>")? + "<integer>".len();
        let int_end = after.find("</integer>")?;
        let int_str = &after[int_start..int_end];
        int_str.parse::<u64>().ok()
    }

    let root_str = Command::new("diskutil")
        .args(["info", "-plist", "/"])
        .output()
        .ok()?
        .stdout;
    let root_str = String::from_utf8_lossy(&root_str);

    let data_str = Command::new("diskutil")
        .args(["info", "-plist", "/System/Volumes/Data"])
        .output()
        .ok()?
        .stdout;
    let data_str = String::from_utf8_lossy(&data_str);

    let container_free = parse_bytes(&root_str, "APFSContainerFree")
        .or_else(|| parse_bytes(&data_str, "APFSContainerFree"))?;
    let container_total = parse_bytes(&root_str, "APFSContainerSize")
        .or_else(|| parse_bytes(&root_str, "TotalSize"))
        .or_else(|| parse_bytes(&data_str, "APFSContainerSize"))?;

    let system_used = parse_bytes(&root_str, "CapacityInUse").unwrap_or(0);
    let data_used = parse_bytes(&data_str, "CapacityInUse").unwrap_or(0);

    Some(ApfsStats {
        total: container_total,
        free: container_free,
        system_used,
        data_used,
        applications_used: 0, // computed async in apfs_stats_async
    })
}

/// Synchronous apfs_stats — kept for tests. Use `apfs_stats_async` in
/// production code to avoid stalling the async runtime.
#[allow(dead_code)]
fn apfs_stats() -> Option<ApfsStats> {
    // Ask diskutil for the APFS container that hosts the boot volume.
    // We parse the output of `diskutil list -plist disk3` (or whichever
    // identifier hosts the boot volume) but the simplest cross-machine
    // approach is to call `diskutil info -plist /` and `diskutil info -plist /System/Volumes/Data`.
    use std::process::Command;

    fn parse_bytes(plist_output: &str, key: &str) -> Option<u64> {
        // Extremely lightweight plist key extraction — avoids pulling in
        // a full plist parser for two integer values.
        let needle = format!("<key>{}</key>", key);
        let pos = plist_output.find(&needle)?;
        let after = &plist_output[pos + needle.len()..];
        let int_start = after.find("<integer>")? + "<integer>".len();
        let int_end = after.find("</integer>")?;
        after[int_start..int_end].trim().parse::<u64>().ok()
    }

    // / is the sealed system snapshot. /System/Volumes/Data is the writable data volume.
    let root_plist = Command::new("diskutil")
        .args(["info", "-plist", "/"])
        .output()
        .ok()?;
    let data_plist = Command::new("diskutil")
        .args(["info", "-plist", "/System/Volumes/Data"])
        .output()
        .ok()?;

    let root_str = String::from_utf8_lossy(&root_plist.stdout);
    let data_str = String::from_utf8_lossy(&data_plist.stdout);

    // Container free is authoritative from either volume.
    let container_free = parse_bytes(&root_str, "APFSContainerFree")
        .or_else(|| parse_bytes(&data_str, "APFSContainerFree"))?;
    let container_total = parse_bytes(&root_str, "APFSContainerSize")
        .or_else(|| parse_bytes(&root_str, "TotalSize"))
        .or_else(|| parse_bytes(&data_str, "APFSContainerSize"))?;

    // "CapacityInUse" is the actual bytes used by this APFS volume.
    let system_used = parse_bytes(&root_str, "CapacityInUse").unwrap_or(0);
    let data_used = parse_bytes(&data_str, "CapacityInUse").unwrap_or(0);

    // Best-effort: sum physical sizes of /Applications and /System/Applications.
    let applications_used = dir_size_fast(std::path::Path::new("/Applications"))
        + dir_size_fast(std::path::Path::new("/System/Applications"));

    Some(ApfsStats {
        total: container_total,
        free: container_free,
        system_used,
        data_used,
        applications_used,
    })
}

/// Linux volume stats via statvfs on the root filesystem.
/// Returns total, free, used, and /usr size as a proxy for "system".
fn linux_stats() -> Option<ApfsStats> {
    use std::ffi::CString;
    use std::mem::MaybeUninit;

    let path_c = CString::new("/").ok()?;
    unsafe {
        let mut buf: MaybeUninit<libc::statvfs> = MaybeUninit::uninit();
        if libc::statvfs(path_c.as_ptr(), buf.as_mut_ptr()) != 0 {
            return None;
        }
        let buf = buf.assume_init();

        #[allow(clippy::unnecessary_cast)]
        let block_size = buf.f_frsize as u64;
        #[allow(clippy::unnecessary_cast)]
        let total = buf.f_blocks as u64 * block_size;
        #[allow(clippy::unnecessary_cast)]
        let free = buf.f_bavail as u64 * block_size;
        #[allow(clippy::unnecessary_cast)]
        let used = (buf.f_blocks as u64 - buf.f_bfree as u64) * block_size;

        // Best-effort: /usr as "system", home as "data"
        let system_used = dir_size_fast(std::path::Path::new("/usr"));
        let data_used = used.saturating_sub(system_used);
        let applications_used = dir_size_fast(std::path::Path::new("/opt"));

        Some(ApfsStats {
            total,
            free,
            system_used,
            data_used,
            applications_used,
        })
    }
}

/// Cross-platform volume stats — dispatches to apfs_stats on macOS,
/// linux_stats on Linux. Synchronous version (kept for tests).
#[allow(dead_code)]
fn volume_stats() -> Option<ApfsStats> {
    if cfg!(target_os = "macos") {
        apfs_stats()
    } else if cfg!(target_os = "linux") {
        linux_stats()
    } else {
        None
    }
}

/// Async version of volume_stats — uses spawn_blocking for the blocking
/// diskutil commands and dir_size_fast calls.
async fn volume_stats_async() -> Option<ApfsStats> {
    if cfg!(target_os = "macos") {
        apfs_stats_async().await
    } else if cfg!(target_os = "linux") {
        tokio::task::spawn_blocking(linux_stats)
            .await
            .ok()
            .flatten()
    } else {
        None
    }
}

/// Quick physical size sum (no recursion limit) — used only for /Applications.
fn dir_size_fast(path: &std::path::Path) -> u64 {
    if !path.exists() {
        return 0;
    }
    WalkDir::new(path)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .filter_map(|e| e.metadata().ok())
        .map(physical_size)
        .sum()
}

/// Async wrapper for `dir_size_fast` — offloads the blocking WalkDir to
/// the tokio blocking thread pool so it doesn't stall the async runtime.
async fn dir_size_fast_async(path: PathBuf) -> u64 {
    tokio::task::spawn_blocking(move || dir_size_fast(&path))
        .await
        .unwrap_or(0)
}

/// Simple MIME type lookup from a lowercase file extension (no external
/// crate dependency). Falls back to `application/octet-stream` for
/// unknown extensions.
pub fn mime_type_for_extension(ext: &str) -> &'static str {
    match ext {
        // Text
        "txt" | "md" | "log" => "text/plain",
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "csv" => "text/csv",
        // Code
        "rs" => "text/rust",
        "py" => "text/x-python",
        "js" => "application/javascript",
        "ts" => "application/typescript",
        "json" => "application/json",
        "xml" => "application/xml",
        "yaml" | "yml" => "application/yaml",
        "toml" => "application/toml",
        "sh" => "application/x-sh",
        // Images
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "tiff" | "tif" => "image/tiff",
        "bmp" => "image/bmp",
        "heic" => "image/heic",
        // Audio
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "aac" => "audio/aac",
        "flac" => "audio/flac",
        "m4a" => "audio/mp4",
        // Video
        "mp4" => "video/mp4",
        "mov" => "video/quicktime",
        "avi" => "video/x-msvideo",
        "mkv" => "video/x-matroska",
        "webm" => "video/webm",
        // Archives
        "zip" => "application/zip",
        "tar" => "application/x-tar",
        "gz" => "application/gzip",
        "bz2" => "application/x-bzip2",
        "7z" => "application/x-7z-compressed",
        "rar" => "application/x-rar-compressed",
        // Documents
        "pdf" => "application/pdf",
        "doc" => "application/msword",
        "docx" => "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
        "xls" => "application/vnd.ms-excel",
        "xlsx" => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
        "ppt" => "application/vnd.ms-powerpoint",
        "pptx" => "application/vnd.openxmlformats-officedocument.presentationml.presentation",
        // Executables / disk images
        "exe" => "application/x-executable",
        "bin" => "application/octet-stream",
        "dmg" => "application/x-apple-diskimage",
        "iso" => "application/x-iso9660-image",
        "deb" => "application/vnd.debian.binary-package",
        "rpm" => "application/x-rpm",
        "plist" => "application/x-plist",
        // Fonts
        "ttf" => "font/ttf",
        "otf" => "font/otf",
        "woff" => "font/woff",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

/// The disk engine analyzes disk usage and emits findings for the top
/// largest directories and files under the given paths. Unlike the clean
/// engine, these findings are informational (Severity::Info) — they help
/// the user understand where disk space is being used.
///
/// Uses **physical block size** (`blocks * 512`) rather than logical file
/// size to correctly handle sparse files (e.g. Docker.raw, sparse VM
/// images) that report a large logical size but use far less disk space.
/// Directory-level findings are emitted as `SystemInfo` (not counted in
/// reclaimable totals) to avoid double-counting with file-level findings.
pub struct DiskEngine {
    args: DiskArgs,
}

impl DiskEngine {
    pub fn new(args: DiskArgs) -> Self {
        Self { args }
    }

    /// Sum physical disk usage (blocks * 512) for all files under a path.
    /// This correctly handles sparse files — `metadata().len()` reports
    /// logical size, but `metadata().blocks()` reports actual blocks used.
    fn dir_size_physical(path: &std::path::Path) -> u64 {
        WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| e.metadata().ok())
            .map(physical_size)
            .sum()
    }

    /// Async wrapper for `dir_size_physical` — offloads the blocking WalkDir
    /// to the tokio blocking thread pool so it doesn't stall the async runtime
    /// when the disk engine runs concurrently with other engines.
    async fn dir_size_physical_async(path: PathBuf) -> u64 {
        tokio::task::spawn_blocking(move || Self::dir_size_physical(&path))
            .await
            .unwrap_or(0)
    }

    /// Walk a directory tree for large files, returning (path, size) pairs.
    /// Offloaded to spawn_blocking to avoid stalling the async runtime.
    fn walk_large_files(path: &std::path::Path, min_size: u64) -> Vec<(PathBuf, u64)> {
        let mut results = Vec::new();
        for entry in WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
        {
            let size = entry.metadata().map(physical_size).unwrap_or(0);
            if size >= min_size {
                results.push((entry.path().to_path_buf(), size));
            }
        }
        results
    }

    /// Async wrapper for `walk_large_files`.
    async fn walk_large_files_async(path: PathBuf, min_size: u64) -> Vec<(PathBuf, u64)> {
        tokio::task::spawn_blocking(move || Self::walk_large_files(&path, min_size))
            .await
            .unwrap_or_default()
    }

    // ===== Phase 1 operations (ops 7-30) =====

    /// **Op 7:** Collect file creation timestamps for all files under a path.
    /// Uses `std::fs::Metadata::created()` which returns the birth time
    /// (`st_birthtime`) on macOS and Linux where supported.
    #[allow(dead_code)]
    fn file_creation_dates(path: &std::path::Path) -> Vec<(PathBuf, Option<SystemTime>)> {
        WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let created = e.metadata().ok().and_then(|m| m.created().ok());
                (e.path().to_path_buf(), created)
            })
            .collect()
    }

    /// **Op 9:** Collect file access timestamps for all files under a path.
    /// Uses `std::fs::Metadata::accessed()` which returns `st_atime`.
    #[allow(dead_code)]
    fn file_access_dates(path: &std::path::Path) -> Vec<(PathBuf, Option<SystemTime>)> {
        WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .map(|e| {
                let accessed = e.metadata().ok().and_then(|m| m.accessed().ok());
                (e.path().to_path_buf(), accessed)
            })
            .collect()
    }

    /// **Op 14:** Classify a file's MIME type from its extension using a
    /// simple lookup table (no external crate dependency).
    #[allow(dead_code)]
    pub fn classify_mime(path: &std::path::Path) -> &'static str {
        match path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
        {
            Some(ext) => mime_type_for_extension(&ext),
            None => "application/octet-stream",
        }
    }

    /// **Op 19:** Rank files by age (oldest first) using modification or
    /// creation date. Returns the top N oldest files with their timestamps.
    #[allow(dead_code)]
    fn rank_oldest_files(path: &std::path::Path, top_n: usize) -> Vec<(PathBuf, SystemTime)> {
        let mut files: Vec<(PathBuf, SystemTime)> = WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| {
                let m = e.metadata().ok()?;
                let ts = m.modified().ok().or_else(|| m.created().ok())?;
                Some((e.path().to_path_buf(), ts))
            })
            .collect();
        files.sort_by_key(|(_, ts)| *ts);
        files.truncate(top_n);
        files
    }

    /// **Op 20:** Rank files by access date (least recently accessed first).
    /// Returns the top N least-recently-used files with their access times.
    #[allow(dead_code)]
    fn rank_unused_files(path: &std::path::Path, top_n: usize) -> Vec<(PathBuf, SystemTime)> {
        let mut files: Vec<(PathBuf, SystemTime)> = WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| {
                let m = e.metadata().ok()?;
                let ts = m.accessed().ok().or_else(|| m.modified().ok())?;
                Some((e.path().to_path_buf(), ts))
            })
            .collect();
        files.sort_by_key(|(_, ts)| *ts);
        files.truncate(top_n);
        files
    }

    /// **Op 29 (helper):** Build a scan cache from walking a directory tree.
    /// Collects file paths, physical sizes, and modification timestamps.
    #[allow(dead_code)]
    fn build_scan_cache(path: &std::path::Path) -> ScanCache {
        let entries: Vec<ScanCacheEntry> = WalkDir::new(path)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .filter_map(|e| {
                let m = e.metadata().ok()?;
                let modified_secs = m
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64);
                Some(ScanCacheEntry {
                    path: e.path().to_string_lossy().to_string(),
                    size: physical_size(m),
                    modified_secs,
                })
            })
            .collect();
        ScanCache {
            version: 1,
            root: path.to_string_lossy().to_string(),
            entries,
        }
    }
}

#[async_trait]
impl Engine for DiskEngine {
    fn id(&self) -> EngineId {
        EngineId::All
    }

    fn name(&self) -> &'static str {
        "Disk Engine"
    }

    fn description(&self) -> &'static str {
        "Analyzes disk usage — top directories and files by physical size"
    }

    async fn validate(&self, _ctx: &ScanContext) -> std::result::Result<(), EngineError> {
        Ok(())
    }

    async fn scan(&self, ctx: Arc<ScanContext>) -> std::result::Result<EngineStats, EngineError> {
        let start = Instant::now();
        let mut items_scanned = 0u64;
        let mut findings_count = 0u64;

        let min_size = byte_unit::Byte::from_str(&self.args.min_size)
            .map(|b| b.get_bytes() as u64)
            .unwrap_or(100 * 1024 * 1024);

        let search_paths = if self.args.paths.is_empty() {
            vec![crate::util::macos::MacosUtils::home_dir()]
        } else {
            self.args.paths.clone()
        };

        // Emit a volume-level summary finding so the GUI can render the
        // full disk donut chart (total / system / data / free / apps)
        // without needing a separate API call.
        if let Some(stats) = volume_stats_async().await {
            let total_known_used = stats.system_used + stats.data_used;
            let vol_label = if cfg!(target_os = "macos") {
                "Macintosh HD"
            } else {
                "Root filesystem (/)"
            };
            let vol_finding = Finding::new(
                EngineId::All,
                Severity::Info,
                Category::SystemInfo,
                Target::Path(std::path::PathBuf::from("/")),
                format!(
                    "Volume: {} total, {} used, {} free",
                    crate::util::disk::format_bytes(stats.total),
                    crate::util::disk::format_bytes(total_known_used),
                    crate::util::disk::format_bytes(stats.free)
                ),
                format!(
                    "{} — {} total capacity, {} data + {} system used, {} available",
                    vol_label,
                    crate::util::disk::format_bytes(stats.total),
                    crate::util::disk::format_bytes(stats.data_used),
                    crate::util::disk::format_bytes(stats.system_used),
                    crate::util::disk::format_bytes(stats.free)
                ),
            )
            .with_size(total_known_used)
            // Rich JSON so Swift can build accurate donut segments:
            // volume_total  — total capacity bytes
            // volume_used   — system + data combined (for the "used%" centre label)
            // volume_free   — free bytes
            // volume_system — system volume bytes
            // volume_data   — writable data volume bytes (home + apps + etc)
            // volume_apps   — /Applications (macOS) or /opt (Linux) physical size
            .with_hint(format!(
                "{{\"volume_total\":{},\"volume_used\":{},\"volume_free\":{},\
                 \"volume_system\":{},\"volume_data\":{},\"volume_apps\":{}}}",
                stats.total,
                total_known_used,
                stats.free,
                stats.system_used,
                stats.data_used,
                stats.applications_used,
            ));
            ctx.emit(vol_finding).await;
            findings_count += 1;
        }

        for search_path in &search_paths {
            if !search_path.exists() {
                continue;
            }
            // Never scan Time Machine / backup volumes — they're not cleanable
            // and walking them is extremely slow (millions of hardlinks).
            if crate::util::backup::is_backup_path(search_path) {
                continue;
            }

            // ---- Directory-level breakdown (immediate children) ----
            // Emitted as SystemInfo so they don't contribute to
            // total_reclaimable_bytes (which would double-count with the
            // file-level findings below).
            //
            // Directory sizes are computed via spawn_blocking to avoid
            // stalling the async runtime when the disk engine runs
            // concurrently with other engines.
            let mut entries: Vec<(PathBuf, u64, bool)> = Vec::new();

            // Read immediate children synchronously (cheap — one readdir).
            // Then compute directory sizes concurrently via spawn_blocking.
            if let Ok(dir_entries) = std::fs::read_dir(search_path) {
                let children: Vec<(PathBuf, bool)> = dir_entries
                    .flatten()
                    .map(|entry| {
                        let path = entry.path();
                        let is_dir = path.is_dir();
                        (path, is_dir)
                    })
                    .collect();

                // Compute directory sizes with bounded concurrency (max 3 at
                // a time) via buffer_unordered. This overlaps I/O waits
                // without spawning dozens of blocking threads.
                // For files, use the metadata directly (cheap).
                use futures::stream::StreamExt;

                let dir_indices: Vec<usize> = children
                    .iter()
                    .enumerate()
                    .filter(|(_, (_, is_dir))| *is_dir)
                    .map(|(i, _)| i)
                    .collect();

                let dir_size_futures = dir_indices.into_iter().map(|i| {
                    let p = children[i].0.clone();
                    async move {
                        let size = Self::dir_size_physical_async(p).await;
                        (i, size)
                    }
                });

                let dir_size_stream = futures::stream::iter(dir_size_futures).buffer_unordered(3);

                let dir_sizes: std::collections::HashMap<usize, u64> =
                    dir_size_stream.collect().await;

                for (i, (path, is_dir)) in children.iter().enumerate() {
                    let size = if *is_dir {
                        *dir_sizes.get(&i).unwrap_or(&0)
                    } else {
                        std::fs::metadata(path).map(physical_size).unwrap_or(0)
                    };
                    items_scanned += 1;
                    if size >= min_size {
                        entries.push((path.clone(), size, *is_dir));
                    }
                }
            }

            entries.sort_by_key(|e| std::cmp::Reverse(e.1));
            entries.truncate(self.args.top);

            for (path, size, is_dir) in entries {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());

                let title = if is_dir {
                    format!(
                        "Disk usage: {} ({} dir)",
                        name,
                        crate::util::disk::format_bytes(size)
                    )
                } else {
                    format!(
                        "Disk usage: {} ({} file)",
                        name,
                        crate::util::disk::format_bytes(size)
                    )
                };

                // SystemInfo category → not counted in reclaimable totals.
                let finding = Finding::new(
                    EngineId::All,
                    Severity::Info,
                    Category::SystemInfo,
                    Target::Path(path.clone()),
                    title,
                    format!(
                        "{} '{}' in {} — {} (physical disk usage)",
                        if is_dir { "Directory" } else { "File" },
                        name,
                        search_path.display(),
                        crate::util::disk::format_bytes(size)
                    ),
                )
                .with_size(size)
                .with_hint(if is_dir {
                    "Large directory — drill down with: xmac disk <path>".to_string()
                } else {
                    "Large file — review and delete if no longer needed".to_string()
                });

                ctx.emit(finding).await;
                findings_count += 1;
            }

            // ---- File-level breakdown (recursive, top N largest files) ----
            // These use LargeFile category and DO contribute to reclaimable
            // totals. Physical block size avoids counting sparse file holes.
            // The recursive WalkDir is offloaded to spawn_blocking so it
            // doesn't stall the async runtime when running concurrently
            // with other engines.
            let large_files = Self::walk_large_files_async(search_path.clone(), min_size).await;
            items_scanned += large_files.len() as u64;

            let mut large_files = large_files;
            large_files.sort_by_key(|e| std::cmp::Reverse(e.1));
            large_files.truncate(self.args.top);

            for (path, size) in large_files {
                let name = path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.to_string_lossy().to_string());

                let finding = Finding::new(
                    EngineId::All,
                    Severity::Info,
                    Category::LargeFile,
                    Target::Path(path.clone()),
                    format!(
                        "Large file: {} ({})",
                        name,
                        crate::util::disk::format_bytes(size)
                    ),
                    format!(
                        "File '{}' — {} (physical disk usage)",
                        path.display(),
                        crate::util::disk::format_bytes(size)
                    ),
                )
                .with_size(size)
                .with_hint("Review and delete if no longer needed".to_string());

                ctx.emit(finding).await;
                findings_count += 1;
            }
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

impl Default for DiskEngine {
    fn default() -> Self {
        Self::new(DiskArgs {
            top: 20,
            min_size: "100M".to_string(),
            paths: Vec::new(),
        })
    }
}

// ===== Phase 1 standalone operations (ops 21-30) =====

/// **Op 21:** Detect removable/external media by parsing `diskutil list`
/// output (macOS). Returns a list of external disk identifiers and their
/// descriptions. On non-macOS platforms, returns an empty vector.
#[allow(dead_code)]
pub fn detect_removable_media() -> Vec<RemovableDisk> {
    if !cfg!(target_os = "macos") {
        return Vec::new();
    }
    use std::process::Command;
    let stdout = match Command::new("diskutil").args(["list"]).output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };
    parse_diskutil_list(&stdout)
}

/// Parse `diskutil list` text output for external/removable disks.
/// Lines like `/dev/disk4 (external, physical):` indicate removable media.
pub fn parse_diskutil_list(output: &str) -> Vec<RemovableDisk> {
    let mut disks = Vec::new();
    for line in output.lines() {
        // Look for device header lines: /dev/diskN (external, ...)
        if line.starts_with("/dev/disk") && line.contains("(external") {
            let identifier = line.split_whitespace().next().unwrap_or("").to_string();
            let description = line
                .split('(')
                .nth(1)
                .and_then(|s| s.split(')').next())
                .unwrap_or("external")
                .to_string();
            disks.push(RemovableDisk {
                identifier,
                description,
            });
        }
    }
    disks
}

/// **Op 22:** Scan mounted external volumes (under `/Volumes/` on macOS).
/// Returns `(mount_point, total_size_bytes)` for each non-system volume found.
/// The boot volume (`Macintosh HD`, `Data`) is excluded.
#[allow(dead_code)]
pub fn scan_external_drives() -> Vec<(PathBuf, u64)> {
    let volumes_dir = std::path::Path::new("/Volumes");
    if !volumes_dir.exists() {
        return Vec::new();
    }
    let mut results = Vec::new();
    if let Ok(entries) = std::fs::read_dir(volumes_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            // Skip the boot/system volumes.
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            if name == "Macintosh HD" || name == "Data" || name.starts_with("Macintosh HD") {
                continue;
            }
            let size = dir_size_fast(&path);
            results.push((path, size));
        }
    }
    results
}

/// **Op 23:** Detect network-mounted filesystems by parsing `mount` command
/// output. Returns mount points for NFS, SMB, AFP, CIFS, WebDAV, and SSHFS
/// filesystems.
#[allow(dead_code)]
pub fn detect_network_mounts() -> Vec<PathBuf> {
    use std::process::Command;
    let stdout = match Command::new("mount").output() {
        Ok(o) => String::from_utf8_lossy(&o.stdout).to_string(),
        Err(_) => return Vec::new(),
    };
    parse_mount_output(&stdout)
}

/// Parse `mount` text output for network filesystem mount points.
/// `mount` output format: `<device> on <mountpoint> (<fstype>, <options>)`
pub fn parse_mount_output(output: &str) -> Vec<PathBuf> {
    const NETWORK_FS_TYPES: &[&str] = &["nfs", "smbfs", "afpfs", "cifs", "webdav", "fuse.sshfs"];
    let mut mounts = Vec::new();
    for line in output.lines() {
        // Extract the mount point and filesystem type.
        if let Some(on_pos) = line.find(" on ") {
            let rest = &line[on_pos + 4..];
            if let Some(paren_pos) = rest.rfind(" (") {
                let mountpoint = rest[..paren_pos].trim();
                let fs_info = &rest[paren_pos + 2..];
                if NETWORK_FS_TYPES.iter().any(|fs| fs_info.contains(fs)) {
                    mounts.push(PathBuf::from(mountpoint));
                }
            }
        }
    }
    mounts
}

/// **Op 24:** Detect cloud-synced folders (iCloud, Dropbox, Google Drive,
/// OneDrive). Returns `(service_name, path)` tuples for each detected sync
/// folder.
#[allow(dead_code)]
pub fn detect_cloud_synced_folders() -> Vec<(String, PathBuf)> {
    let mut results = Vec::new();
    if let Some(p) = detect_icloud_drive() {
        results.push(("iCloud Drive".to_string(), p));
    }
    if let Some(p) = detect_dropbox() {
        results.push(("Dropbox".to_string(), p));
    }
    if let Some(p) = detect_google_drive() {
        results.push(("Google Drive".to_string(), p));
    }
    if let Some(p) = detect_onedrive() {
        results.push(("OneDrive".to_string(), p));
    }
    results
}

/// **Op 25:** Detect iCloud Drive data — checks `~/Library/Mobile Documents/`.
#[allow(dead_code)]
pub fn detect_icloud_drive() -> Option<PathBuf> {
    let home = crate::util::macos::MacosUtils::home_dir();
    let path = home.join("Library/Mobile Documents");
    if path.exists() && path.is_dir() {
        Some(path)
    } else {
        None
    }
}

/// **Op 26:** Detect Dropbox data — checks `~/Dropbox` and
/// `~/Library/CloudStorage/Dropbox*`.
#[allow(dead_code)]
pub fn detect_dropbox() -> Option<PathBuf> {
    let home = crate::util::macos::MacosUtils::home_dir();
    // Check ~/Dropbox first.
    let dropbox_home = home.join("Dropbox");
    if dropbox_home.exists() && dropbox_home.is_dir() {
        return Some(dropbox_home);
    }
    // Check ~/Library/CloudStorage/Dropbox*
    find_cloud_storage_dir(&home, "Dropbox")
}

/// **Op 27:** Detect Google Drive data — checks `~/Google Drive File Stream`
/// and `~/Library/CloudStorage/GoogleDrive*`.
#[allow(dead_code)]
pub fn detect_google_drive() -> Option<PathBuf> {
    let home = crate::util::macos::MacosUtils::home_dir();
    // Check ~/Google Drive File Stream (older path).
    let gdfs = home.join("Google Drive File Stream");
    if gdfs.exists() && gdfs.is_dir() {
        return Some(gdfs);
    }
    // Check ~/Library/CloudStorage/GoogleDrive*
    find_cloud_storage_dir(&home, "GoogleDrive")
}

/// **Op 28:** Detect OneDrive data — checks `~/OneDrive` and
/// `~/Library/CloudStorage/OneDrive*`.
#[allow(dead_code)]
pub fn detect_onedrive() -> Option<PathBuf> {
    let home = crate::util::macos::MacosUtils::home_dir();
    // Check ~/OneDrive first.
    let onedrive_home = home.join("OneDrive");
    if onedrive_home.exists() && onedrive_home.is_dir() {
        return Some(onedrive_home);
    }
    // Check ~/Library/CloudStorage/OneDrive*
    find_cloud_storage_dir(&home, "OneDrive")
}

/// Helper: search `~/Library/CloudStorage/` for a directory whose name starts
/// with the given prefix (case-insensitive). Returns the first match.
fn find_cloud_storage_dir(home: &std::path::Path, prefix: &str) -> Option<PathBuf> {
    let cloud_storage = home.join("Library/CloudStorage");
    let entries = std::fs::read_dir(&cloud_storage).ok()?;
    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str.to_lowercase().starts_with(&prefix.to_lowercase()) && entry.path().is_dir() {
            return Some(entry.path());
        }
    }
    None
}

/// **Op 29:** Save scan results to a JSON cache file.
#[allow(dead_code)]
pub fn save_scan_cache(cache: &ScanCache, cache_path: &std::path::Path) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(cache)
        .map_err(std::io::Error::other)?;
    std::fs::write(cache_path, json)
}

/// **Op 29:** Load scan results from a JSON cache file.
#[allow(dead_code)]
pub fn load_scan_cache(cache_path: &std::path::Path) -> Option<ScanCache> {
    let json = std::fs::read_to_string(cache_path).ok()?;
    serde_json::from_str(&json).ok()
}

/// **Op 30:** Compare a cached scan with the current filesystem state.
/// Returns the list of paths that have been added, modified, or removed
/// since the cache was written. Only changed directories need to be rescanned.
#[allow(dead_code)]
pub fn incremental_rescan(
    path: &std::path::Path,
    cached: &ScanCache,
) -> IncrementalScanResult {
    let current = DiskEngine::build_scan_cache(path);

    let cached_map: HashMap<&str, &ScanCacheEntry> =
        cached.entries.iter().map(|e| (e.path.as_str(), e)).collect();
    let current_map: HashMap<&str, &ScanCacheEntry> =
        current.entries.iter().map(|e| (e.path.as_str(), e)).collect();

    let mut added = Vec::new();
    let mut modified = Vec::new();
    let mut removed = Vec::new();

    for entry in &current.entries {
        match cached_map.get(entry.path.as_str()) {
            None => added.push(PathBuf::from(&entry.path)),
            Some(cached_entry) => {
                if entry.size != cached_entry.size
                    || entry.modified_secs != cached_entry.modified_secs
                {
                    modified.push(PathBuf::from(&entry.path));
                }
            }
        }
    }
    for entry in &cached.entries {
        if !current_map.contains_key(entry.path.as_str()) {
            removed.push(PathBuf::from(&entry.path));
        }
    }

    IncrementalScanResult {
        added,
        modified,
        removed,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::args::DiskArgs;
    use crate::util::disk::format_bytes;
    use std::fs;
    use std::path::PathBuf;
    use std::time::SystemTime;
    use tempfile::TempDir;

    #[test]
    fn test_disk_engine_new_preserves_args() {
        let args = DiskArgs {
            top: 5,
            min_size: "500M".to_string(),
            paths: vec![PathBuf::from("/tmp")],
        };
        let engine = DiskEngine::new(args);
        assert_eq!(engine.id(), EngineId::All);
        assert_eq!(engine.name(), "Disk Engine");
        assert!(!engine.description().is_empty());
    }

    #[test]
    fn test_disk_engine_default_values() {
        let engine = DiskEngine::default();
        assert_eq!(engine.id(), EngineId::All);
        assert_eq!(engine.name(), "Disk Engine");
        assert!(engine.description().contains("disk usage"));
    }

    #[test]
    fn test_dir_size_physical_nonexistent_returns_zero() {
        let missing = std::path::Path::new("/this/path/does/not/exist/xyz");
        assert_eq!(DiskEngine::dir_size_physical(missing), 0);
    }

    #[test]
    fn test_dir_size_physical_sums_files() {
        let dir = TempDir::new().unwrap();
        // Write a few files with known logical sizes.
        fs::write(dir.path().join("a.bin"), vec![0u8; 4096]).unwrap();
        fs::write(dir.path().join("b.bin"), vec![0u8; 8192]).unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("c.bin"), vec![0u8; 2048]).unwrap();

        let size = DiskEngine::dir_size_physical(dir.path());
        // Physical size is blocks*512, which is >= logical size sum.
        assert!(size >= 4096 + 8192 + 2048);
        assert!(size > 0);
    }

    #[test]
    fn test_dir_size_fast_nonexistent_returns_zero() {
        let missing = std::path::Path::new("/this/path/does/not/exist/abc");
        assert_eq!(dir_size_fast(missing), 0);
    }

    #[test]
    fn test_dir_size_fast_sums_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("x.bin"), vec![0u8; 4096]).unwrap();
        let nested = dir.path().join("nested/deep");
        fs::create_dir_all(&nested).unwrap();
        fs::write(nested.join("y.bin"), vec![0u8; 8192]).unwrap();

        let size = dir_size_fast(dir.path());
        assert!(size >= 4096 + 8192);
    }

    #[test]
    fn test_top_n_sorting_truncates_largest() {
        // Replicates the sort-by-descending-size + truncate(top) logic
        // used in DiskEngine::scan for directory entries.
        let mut entries: Vec<(PathBuf, u64, bool)> = vec![
            (PathBuf::from("/a"), 100, true),
            (PathBuf::from("/b"), 500, true),
            (PathBuf::from("/c"), 300, true),
            (PathBuf::from("/d"), 50, false),
            (PathBuf::from("/e"), 1000, true),
        ];
        entries.sort_by_key(|e| std::cmp::Reverse(e.1));
        entries.truncate(3);

        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].1, 1000);
        assert_eq!(entries[1].1, 500);
        assert_eq!(entries[2].1, 300);
    }

    #[test]
    fn test_format_bytes_units_scale_correctly() {
        assert!(format_bytes(0).contains("B"));
        assert!(format_bytes(1024).contains("KB"));
        assert!(format_bytes(1024 * 1024).contains("MB"));
        assert!(format_bytes(1024 * 1024 * 1024).contains("GB"));
    }

    #[test]
    fn test_apfs_stats_struct_fields_are_consistent() {
        // Sanity-check the ApfsStats struct can be constructed and that
        // total = system + data + free is a plausible invariant (we only
        // verify field arithmetic, not real disk values).
        let stats = ApfsStats {
            total: 1_000_000_000,
            free: 400_000_000,
            system_used: 200_000_000,
            data_used: 400_000_000,
            applications_used: 50_000_000,
        };
        assert_eq!(
            stats.system_used + stats.data_used + stats.free,
            1_000_000_000
        );
        assert!(stats.applications_used <= stats.data_used);
    }

    // ===== Phase 1 operation tests (ops 7-30) =====

    #[test]
    fn test_file_creation_dates_returns_timestamps() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "hello").unwrap();
        fs::write(dir.path().join("b.txt"), "world").unwrap();
        let dates = DiskEngine::file_creation_dates(dir.path());
        assert_eq!(dates.len(), 2);
        for (_, ts) in &dates {
            if let Some(t) = ts {
                assert!(*t <= SystemTime::now());
            }
        }
    }

    #[test]
    fn test_file_creation_dates_nonexistent_returns_empty() {
        let missing = std::path::Path::new("/this/path/does/not/exist/zzz");
        assert!(DiskEngine::file_creation_dates(missing).is_empty());
    }

    #[test]
    fn test_file_access_dates_returns_timestamps() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "hello").unwrap();
        fs::write(dir.path().join("b.txt"), "world").unwrap();
        let dates = DiskEngine::file_access_dates(dir.path());
        assert_eq!(dates.len(), 2);
        for (_, ts) in &dates {
            if let Some(t) = ts {
                assert!(*t <= SystemTime::now());
            }
        }
    }

    #[test]
    fn test_file_access_dates_nonexistent_returns_empty() {
        let missing = std::path::Path::new("/this/path/does/not/exist/yyy");
        assert!(DiskEngine::file_access_dates(missing).is_empty());
    }

    #[test]
    fn test_classify_mime_known_extensions() {
        assert_eq!(
            DiskEngine::classify_mime(std::path::Path::new("photo.jpg")),
            "image/jpeg"
        );
        assert_eq!(
            DiskEngine::classify_mime(std::path::Path::new("doc.pdf")),
            "application/pdf"
        );
        assert_eq!(
            DiskEngine::classify_mime(std::path::Path::new("video.mp4")),
            "video/mp4"
        );
        assert_eq!(
            DiskEngine::classify_mime(std::path::Path::new("code.rs")),
            "text/rust"
        );
        assert_eq!(
            DiskEngine::classify_mime(std::path::Path::new("data.json")),
            "application/json"
        );
    }

    #[test]
    fn test_classify_mime_case_insensitive() {
        assert_eq!(
            DiskEngine::classify_mime(std::path::Path::new("IMAGE.PNG")),
            "image/png"
        );
        assert_eq!(
            DiskEngine::classify_mime(std::path::Path::new("Doc.PDF")),
            "application/pdf"
        );
    }

    #[test]
    fn test_classify_mime_unknown_extension() {
        assert_eq!(
            DiskEngine::classify_mime(std::path::Path::new("file.xyzunknown")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_classify_mime_no_extension() {
        assert_eq!(
            DiskEngine::classify_mime(std::path::Path::new("Makefile")),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_mime_type_for_extension_table() {
        assert_eq!(mime_type_for_extension("png"), "image/png");
        assert_eq!(mime_type_for_extension("zip"), "application/zip");
        assert_eq!(mime_type_for_extension("mp3"), "audio/mpeg");
        assert_eq!(mime_type_for_extension("html"), "text/html");
        assert_eq!(
            mime_type_for_extension("unknown"),
            "application/octet-stream"
        );
    }

    #[test]
    fn test_rank_oldest_files_orders_by_mtime() {
        let dir = TempDir::new().unwrap();
        let path_a = dir.path().join("old.txt");
        let path_b = dir.path().join("new.txt");
        fs::write(&path_a, "old").unwrap();
        fs::write(&path_b, "new").unwrap();
        let old_time = SystemTime::now() - std::time::Duration::from_secs(3600);
        std::fs::File::open(&path_a)
            .unwrap()
            .set_modified(old_time)
            .unwrap();
        let ranked = DiskEngine::rank_oldest_files(dir.path(), 10);
        assert_eq!(ranked.len(), 2);
        assert_eq!(ranked[0].0, path_a);
        assert_eq!(ranked[1].0, path_b);
        assert!(ranked[0].1 < ranked[1].1);
    }

    #[test]
    fn test_rank_oldest_files_truncates_to_top_n() {
        let dir = TempDir::new().unwrap();
        for i in 0..5 {
            let path = dir.path().join(format!("file_{i}.txt"));
            fs::write(&path, format!("content {i}")).unwrap();
            let t = SystemTime::now() - std::time::Duration::from_secs(i * 60);
            std::fs::File::open(&path)
                .unwrap()
                .set_modified(t)
                .unwrap();
        }
        let ranked = DiskEngine::rank_oldest_files(dir.path(), 3);
        assert_eq!(ranked.len(), 3);
        assert!(ranked[0].1 <= ranked[1].1);
        assert!(ranked[1].1 <= ranked[2].1);
    }

    #[test]
    fn test_rank_oldest_files_nonexistent_returns_empty() {
        let missing = std::path::Path::new("/this/path/does/not/exist/www");
        assert!(DiskEngine::rank_oldest_files(missing, 10).is_empty());
    }

    #[test]
    fn test_rank_unused_files_orders_by_atime() {
        let dir = TempDir::new().unwrap();
        let path_a = dir.path().join("unused.txt");
        let path_b = dir.path().join("recent.txt");
        fs::write(&path_a, "unused").unwrap();
        fs::write(&path_b, "recent").unwrap();
        let old_time = SystemTime::now() - std::time::Duration::from_secs(7200);
        std::fs::File::open(&path_a)
            .unwrap()
            .set_modified(old_time)
            .unwrap();
        let ranked = DiskEngine::rank_unused_files(dir.path(), 10);
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].1 <= ranked[1].1);
    }

    #[test]
    fn test_rank_unused_files_truncates_to_top_n() {
        let dir = TempDir::new().unwrap();
        for i in 0..4 {
            let path = dir.path().join(format!("f_{i}.txt"));
            fs::write(&path, format!("data {i}")).unwrap();
            let t = SystemTime::now() - std::time::Duration::from_secs(i * 120);
            std::fs::File::open(&path)
                .unwrap()
                .set_modified(t)
                .unwrap();
        }
        let ranked = DiskEngine::rank_unused_files(dir.path(), 2);
        assert_eq!(ranked.len(), 2);
        assert!(ranked[0].1 <= ranked[1].1);
    }

    #[test]
    fn test_rank_unused_files_nonexistent_returns_empty() {
        let missing = std::path::Path::new("/this/path/does/not/exist/vvv");
        assert!(DiskEngine::rank_unused_files(missing, 10).is_empty());
    }

    #[test]
    fn test_detect_removable_media_returns_vec() {
        let disks = detect_removable_media();
        if cfg!(target_os = "macos") {
            let _ = disks.len();
        } else {
            assert!(disks.is_empty());
        }
    }

    #[test]
    fn test_parse_diskutil_list_finds_external() {
        let output = "\
/dev/disk0 (internal):
   #:        TYPE NAME
/dev/disk2 (internal, physical):
   #:        TYPE NAME
/dev/disk4 (external, physical):
   #:        TYPE NAME
/dev/disk5 (external, virtual):
   #:        TYPE NAME
";
        let disks = parse_diskutil_list(output);
        assert_eq!(disks.len(), 2);
        assert_eq!(disks[0].identifier, "/dev/disk4");
        assert!(disks[0].description.contains("external"));
        assert_eq!(disks[1].identifier, "/dev/disk5");
    }

    #[test]
    fn test_parse_diskutil_list_no_external_returns_empty() {
        let output = "\
/dev/disk0 (internal):
   #:        TYPE NAME
/dev/disk2 (internal, physical):
";
        let disks = parse_diskutil_list(output);
        assert!(disks.is_empty());
    }

    #[test]
    fn test_scan_external_drives_returns_vec() {
        let drives = scan_external_drives();
        let _ = drives.len();
    }

    #[test]
    fn test_detect_network_mounts_returns_vec() {
        let mounts = detect_network_mounts();
        let _ = mounts.len();
    }

    #[test]
    fn test_parse_mount_output_finds_network_fs() {
        let output = "\
/dev/disk1s1 on / (apfs, local, journaled)
map -hosts on /net (autofs, nosuid, automounted, nobrowse)
//server/share on /Volumes/share (smbfs, nodev, nosuid, mounted by user)
nfs://server/data on /mnt/data (nfs, nodev, nosuid, read-only)
";
        let mounts = parse_mount_output(output);
        assert_eq!(mounts.len(), 2);
        assert_eq!(mounts[0], PathBuf::from("/Volumes/share"));
        assert_eq!(mounts[1], PathBuf::from("/mnt/data"));
    }

    #[test]
    fn test_parse_mount_output_no_network_returns_empty() {
        let output = "\
/dev/disk1s1 on / (apfs, local, journaled)
/dev/disk2s1 on /System/Volumes/Data (apfs, local, journaled)
";
        let mounts = parse_mount_output(output);
        assert!(mounts.is_empty());
    }

    #[test]
    fn test_detect_cloud_synced_folders_returns_vec() {
        let folders = detect_cloud_synced_folders();
        let _ = folders.len();
    }

    #[test]
    fn test_detect_icloud_drive_returns_option() {
        let result = detect_icloud_drive();
        let _ = result.is_some();
    }

    #[test]
    fn test_detect_dropbox_returns_option() {
        let result = detect_dropbox();
        let _ = result.is_some();
    }

    #[test]
    fn test_detect_google_drive_returns_option() {
        let result = detect_google_drive();
        let _ = result.is_some();
    }

    #[test]
    fn test_detect_onedrive_returns_option() {
        let result = detect_onedrive();
        let _ = result.is_some();
    }

    #[test]
    fn test_find_cloud_storage_dir_finds_prefix_match() {
        let dir = TempDir::new().unwrap();
        let cloud = dir.path().join("Library/CloudStorage");
        fs::create_dir_all(&cloud).unwrap();
        fs::create_dir(cloud.join("Dropbox-Personal")).unwrap();
        fs::create_dir(cloud.join("OneDrive-Work")).unwrap();

        let dropbox = find_cloud_storage_dir(dir.path(), "Dropbox");
        assert!(dropbox.is_some());
        assert!(dropbox
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("Dropbox"));

        let onedrive = find_cloud_storage_dir(dir.path(), "OneDrive");
        assert!(onedrive.is_some());
        assert!(onedrive
            .unwrap()
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("OneDrive"));

        let gdrive = find_cloud_storage_dir(dir.path(), "GoogleDrive");
        assert!(gdrive.is_none());
    }

    #[test]
    fn test_save_and_load_scan_cache_roundtrip() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("a.txt"), "hello").unwrap();
        fs::write(dir.path().join("b.txt"), "world").unwrap();

        let cache = DiskEngine::build_scan_cache(dir.path());
        assert_eq!(cache.version, 1);
        assert_eq!(cache.entries.len(), 2);

        let cache_file = dir.path().join("cache.json");
        save_scan_cache(&cache, &cache_file).unwrap();
        assert!(cache_file.exists());

        let loaded = load_scan_cache(&cache_file).unwrap();
        assert_eq!(loaded.version, cache.version);
        assert_eq!(loaded.root, cache.root);
        assert_eq!(loaded.entries.len(), cache.entries.len());

        let loaded_paths: std::collections::HashSet<&str> =
            loaded.entries.iter().map(|e| e.path.as_str()).collect();
        for entry in &cache.entries {
            assert!(loaded_paths.contains(entry.path.as_str()));
        }
    }

    #[test]
    fn test_load_scan_cache_nonexistent_returns_none() {
        let result = load_scan_cache(std::path::Path::new("/nonexistent/cache.json"));
        assert!(result.is_none());
    }

    #[test]
    fn test_build_scan_cache_collects_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("x.bin"), vec![0u8; 4096]).unwrap();
        let sub = dir.path().join("sub");
        fs::create_dir(&sub).unwrap();
        fs::write(sub.join("y.bin"), vec![0u8; 8192]).unwrap();

        let cache = DiskEngine::build_scan_cache(dir.path());
        assert_eq!(cache.entries.len(), 2);
        for entry in &cache.entries {
            assert!(entry.modified_secs.is_some());
            assert!(entry.size > 0);
        }
    }

    #[test]
    fn test_incremental_rescan_detects_added_files() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("existing.txt"), "data").unwrap();
        let cache = DiskEngine::build_scan_cache(dir.path());
        fs::write(dir.path().join("new.txt"), "new data").unwrap();

        let result = incremental_rescan(dir.path(), &cache);
        assert_eq!(result.added.len(), 1);
        assert!(result.added[0].to_string_lossy().ends_with("new.txt"));
        assert!(result.modified.is_empty());
        assert!(result.removed.is_empty());
    }

    #[test]
    fn test_incremental_rescan_detects_modified_files() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("file.txt");
        fs::write(&file_path, "original").unwrap();
        let cache = DiskEngine::build_scan_cache(dir.path());

        let old_time = SystemTime::now() - std::time::Duration::from_secs(1800);
        fs::write(&file_path, "modified content").unwrap();
        std::fs::File::open(&file_path)
            .unwrap()
            .set_modified(old_time)
            .unwrap();

        let result = incremental_rescan(dir.path(), &cache);
        assert!(result.added.is_empty());
        assert_eq!(result.modified.len(), 1);
        assert!(result.modified[0].to_string_lossy().ends_with("file.txt"));
        assert!(result.removed.is_empty());
    }

    #[test]
    fn test_incremental_rescan_detects_removed_files() {
        let dir = TempDir::new().unwrap();
        let file_path = dir.path().join("doomed.txt");
        fs::write(&file_path, "soon to be deleted").unwrap();
        let cache = DiskEngine::build_scan_cache(dir.path());

        fs::remove_file(&file_path).unwrap();

        let result = incremental_rescan(dir.path(), &cache);
        assert!(result.added.is_empty());
        assert!(result.modified.is_empty());
        assert_eq!(result.removed.len(), 1);
        assert!(result.removed[0].to_string_lossy().ends_with("doomed.txt"));
    }

    #[test]
    fn test_incremental_rescan_no_changes_returns_empty() {
        let dir = TempDir::new().unwrap();
        fs::write(dir.path().join("stable.txt"), "unchanged").unwrap();
        let cache = DiskEngine::build_scan_cache(dir.path());

        let result = incremental_rescan(dir.path(), &cache);
        assert!(result.added.is_empty());
        assert!(result.modified.is_empty());
        assert!(result.removed.is_empty());
    }

    #[test]
    fn test_incremental_scan_result_struct_fields() {
        let result = IncrementalScanResult {
            added: vec![PathBuf::from("/new")],
            modified: vec![PathBuf::from("/changed")],
            removed: vec![PathBuf::from("/gone")],
        };
        assert_eq!(result.added.len(), 1);
        assert_eq!(result.modified.len(), 1);
        assert_eq!(result.removed.len(), 1);
    }

    #[test]
    fn test_removable_disk_struct() {
        let disk = RemovableDisk {
            identifier: "/dev/disk4".to_string(),
            description: "external, physical".to_string(),
        };
        assert_eq!(disk.identifier, "/dev/disk4");
        assert_eq!(disk.description, "external, physical");
    }

    #[test]
    fn test_scan_cache_serde_roundtrip() {
        let cache = ScanCache {
            version: 1,
            root: "/test".to_string(),
            entries: vec![ScanCacheEntry {
                path: "/test/file.txt".to_string(),
                size: 1024,
                modified_secs: Some(1700000000),
            }],
        };
        let json = serde_json::to_string(&cache).unwrap();
        let decoded: ScanCache = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.version, 1);
        assert_eq!(decoded.root, "/test");
        assert_eq!(decoded.entries.len(), 1);
        assert_eq!(decoded.entries[0].size, 1024);
        assert_eq!(decoded.entries[0].modified_secs, Some(1700000000));
    }
}
