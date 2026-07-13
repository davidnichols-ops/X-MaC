use async_trait::async_trait;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
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

        let block_size = buf.f_frsize;
        let total = buf.f_blocks as u64 * block_size;
        let free = buf.f_bavail as u64 * block_size;
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
/// linux_stats on Linux.
fn volume_stats() -> Option<ApfsStats> {
    if cfg!(target_os = "macos") {
        apfs_stats()
    } else if cfg!(target_os = "linux") {
        linux_stats()
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
        if let Some(stats) = volume_stats() {
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
            let mut entries: Vec<(PathBuf, u64, bool)> = Vec::new();

            if let Ok(dir_entries) = std::fs::read_dir(search_path) {
                for entry in dir_entries.flatten() {
                    let path = entry.path();
                    let is_dir = path.is_dir();
                    let size = if is_dir {
                        Self::dir_size_physical(&path)
                    } else {
                        entry.metadata().map(physical_size).unwrap_or(0)
                    };
                    items_scanned += 1;
                    if size >= min_size {
                        entries.push((path, size, is_dir));
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
            let mut large_files: Vec<(PathBuf, u64)> = Vec::new();

            let walker = WalkDir::new(search_path)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
                .filter(|e| e.file_type().is_file());

            for entry in walker {
                items_scanned += 1;
                let size = entry.metadata().map(physical_size).unwrap_or(0);
                if size >= min_size {
                    large_files.push((entry.path().to_path_buf(), size));
                }
            }

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
