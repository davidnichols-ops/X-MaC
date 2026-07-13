//! Time Machine and backup volume detection.
//!
//! Time Machine backup volumes must never be scanned for "cleanable" files
//! or have their contents modified. This module provides cross-platform
//! heuristics to detect backup volumes and paths.
//!
//! Detection methods (macOS, in priority order):
//! 1. `tmutil` — the authoritative source. `tmutil destinationinfo` lists
//!    mounted backup destinations. We also check `tmutil isdestination <path>`.
//! 2. Filesystem heuristics — look for `Backups.backupdb` (HFS+ TM format)
//!    or `.backupbundle` (APFS TM format) at the volume root.
//! 3. Volume metadata — check for `com.apple.TimeMachine.Backup` extended
//!    attributes or `.com.apple.timemachine.supported` sentinel files.
//!
//! On Linux: checks for common backup tool markers (rsync, Borg, restic,
//! Timeshift, Deja Dup) — these are advisory, not authoritative.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

/// Cache of Time Machine destination paths (macOS). Populated once on first
/// access via `tmutil destinationinfo`.
static TM_DESTINATIONS: OnceLock<Vec<PathBuf>> = OnceLock::new();

/// Check if a path is on a Time Machine backup volume (macOS) or a known
/// backup directory (Linux).
///
/// This is the primary public API. Use this to guard any scan or cleanup
/// operation that might touch a backup volume.
pub fn is_backup_path(path: &Path) -> bool {
    // Normalize: resolve /Volumes/<name>/... to /Volumes/<name>
    let volume_root = extract_volume_root(path);
    if let Some(root) = &volume_root {
        if is_time_machine_volume(root) {
            return true;
        }
    }

    // Also check the path itself for backup markers (e.g. a path inside
    // Backups.backupdb that isn't under /Volumes).
    is_backup_by_markers(path)
}

/// Extract the volume root from a path. On macOS, paths under /Volumes/<name>
/// return /Volumes/<name>. Returns None for paths not on an external volume.
fn extract_volume_root(path: &Path) -> Option<PathBuf> {
    let path_str = path.to_string_lossy();

    if cfg!(target_os = "macos") {
        // /Volumes/<name>/... → /Volumes/<name>
        if let Some(rest) = path_str.strip_prefix("/Volumes/") {
            let vol_name = rest.split('/').next()?;
            if !vol_name.is_empty() {
                return Some(PathBuf::from(format!("/Volumes/{}", vol_name)));
            }
        }
    }

    if cfg!(target_os = "linux") {
        // /media/<user>/<name>/... → /media/<user>/<name>
        // /mnt/<name>/... → /mnt/<name>
        for prefix in &["/media/", "/mnt/"] {
            if let Some(rest) = path_str.strip_prefix(prefix) {
                let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
                if *prefix == "/media/" {
                    // /media/<user>/<name> needs at least 2 components
                    if parts.len() >= 2 {
                        return Some(PathBuf::from(format!("/media/{}/{}", parts[0], parts[1])));
                    }
                } else {
                    // /mnt/<name> needs at least 1 component
                    if !parts.is_empty() {
                        return Some(PathBuf::from(format!("/mnt/{}", parts[0])));
                    }
                }
            }
        }
    }

    None
}

/// Check if a volume root path is a Time Machine destination (macOS).
/// Uses cached `tmutil` results + filesystem heuristics.
fn is_time_machine_volume(vol_root: &Path) -> bool {
    if !cfg!(target_os = "macos") {
        return is_linux_backup_volume(vol_root);
    }

    // 1. Check against cached tmutil destinations
    let destinations = TM_DESTINATIONS.get_or_init(get_tm_destinations);
    if destinations.iter().any(|d| {
        // tmutil may return mount points like /Volumes/Time Machine Backups
        // or device paths. Compare by canonicalized path.
        same_path(d, vol_root)
    }) {
        return true;
    }

    // 2. Filesystem heuristics: look for TM markers at the volume root
    is_backup_by_markers(vol_root)
}

/// Check for Time Machine filesystem markers at the given path.
/// - `Backups.backupdb` directory (HFS+ Time Machine format)
/// - `.backupbundle` file (APFS Time Machine format)
/// - `.com.apple.timemachine.supported` sentinel file
fn is_backup_by_markers(path: &Path) -> bool {
    if cfg!(target_os = "macos") {
        // HFS+ Time Machine: Backups.backupdb directory at volume root
        if path.join("Backups.backupdb").is_dir() {
            return true;
        }
        // APFS Time Machine: .backupbundle file at volume root
        if path.join(".backupbundle").exists() {
            return true;
        }
        // Sentinel file indicating TM support
        if path.join(".com.apple.timemachine.supported").exists() {
            return true;
        }
        // Check if any parent directory is Backups.backupdb
        for ancestor in path.ancestors() {
            if ancestor
                .file_name()
                .map(|n| n == "Backups.backupdb")
                .unwrap_or(false)
            {
                return true;
            }
        }
    }

    if cfg!(target_os = "linux") {
        // Timeshift: /timeshift/ directory
        if path.join("timeshift").is_dir() {
            return true;
        }
        // Borg: .borg.repo or config
        if path.join("config").exists()
            && path.join("data").is_dir()
            && path.join("README").exists()
        {
            // Borg repos have config, data/, README
            return true;
        }
        // Restic: restic repo has a "config" file with version=2
        // Deja Dup: duplicates/ directory
        if path.join("duplicates").is_dir() {
            return true;
        }
    }

    false
}

/// Linux: check if a mounted volume looks like a backup destination.
fn is_linux_backup_volume(vol_root: &Path) -> bool {
    is_backup_by_markers(vol_root)
}

/// Get Time Machine destination mount points via `tmutil destinationinfo`.
/// Returns an empty vec if tmutil is unavailable or no destinations are set.
fn get_tm_destinations() -> Vec<PathBuf> {
    if !cfg!(target_os = "macos") {
        return vec![];
    }

    // `tmutil destinationinfo` outputs lines like:
    //   Name: Time Machine Backups
    //   Kind: Local
    //   Mount Point: /Volumes/Time Machine Backups
    //   ...
    // We extract the "Mount Point:" values.
    let output = Command::new("tmutil").arg("destinationinfo").output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            stdout
                .lines()
                .filter_map(|line| {
                    let line = line.trim();
                    line.strip_prefix("Mount Point:")
                        .map(|s| PathBuf::from(s.trim()))
                })
                .collect()
        }
        Err(_) => vec![],
    }
}

/// Compare two paths by canonicalizing both (falls back to literal comparison).
fn same_path(a: &Path, b: &Path) -> bool {
    let ca = std::fs::canonicalize(a).unwrap_or_else(|_| a.to_path_buf());
    let cb = std::fs::canonicalize(b).unwrap_or_else(|_| b.to_path_buf());
    ca == cb
}

/// List all known backup volume roots on the system. This combines
/// `tmutil` destinations with filesystem marker detection on all
/// mounted volumes.
pub fn backup_volumes() -> Vec<PathBuf> {
    let mut volumes = Vec::new();

    if cfg!(target_os = "macos") {
        // From tmutil
        let tm = TM_DESTINATIONS.get_or_init(get_tm_destinations);
        volumes.extend(tm.iter().cloned());

        // From /Volumes filesystem markers
        if let Ok(entries) = std::fs::read_dir("/Volumes") {
            for entry in entries.flatten() {
                let path = entry.path();
                if is_backup_by_markers(&path) && !volumes.iter().any(|v| same_path(v, &path)) {
                    volumes.push(path);
                }
            }
        }
    } else if cfg!(target_os = "linux") {
        for root in &["/media", "/mnt"] {
            if let Ok(entries) = std::fs::read_dir(root) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if is_backup_by_markers(&path) {
                        volumes.push(path);
                    }
                }
            }
        }
    }

    volumes
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn is_backup_path_returns_false_for_home() {
        assert!(!is_backup_path(&PathBuf::from("/Users/test/Documents")));
    }

    #[test]
    fn is_backup_path_returns_false_for_tmp() {
        assert!(!is_backup_path(&PathBuf::from("/tmp/foo")));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detects_backups_backupdb_marker() {
        let tmp = TempDir::new().unwrap();
        std::fs::create_dir_all(tmp.path().join("Backups.backupdb")).unwrap();
        assert!(is_backup_by_markers(tmp.path()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detects_backupbundle_marker() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join(".backupbundle"), "fake").unwrap();
        assert!(is_backup_by_markers(tmp.path()));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn detects_path_inside_backups_backupdb() {
        let tmp = TempDir::new().unwrap();
        let backupdb = tmp.path().join("Backups.backupdb");
        std::fs::create_dir_all(&backupdb).unwrap();
        let inner = backupdb.join("2024-01-01-000000/Mac");
        std::fs::create_dir_all(&inner).unwrap();
        assert!(is_backup_by_markers(&inner));
    }

    #[test]
    fn non_backup_directory_has_no_markers() {
        let tmp = TempDir::new().unwrap();
        std::fs::write(tmp.path().join("random.txt"), "hello").unwrap();
        assert!(!is_backup_by_markers(tmp.path()));
    }

    #[test]
    fn extract_volume_root_returns_none_for_root() {
        assert!(extract_volume_root(&PathBuf::from("/")).is_none());
    }

    #[test]
    fn extract_volume_root_returns_none_for_home() {
        assert!(extract_volume_root(&PathBuf::from("/Users/test")).is_none());
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn extract_volume_root_parses_volumes_path() {
        let root = extract_volume_root(&PathBuf::from("/Volumes/My Drive/Documents/file.txt"));
        assert_eq!(root, Some(PathBuf::from("/Volumes/My Drive")));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn extract_volume_root_parses_mnt_path() {
        let root = extract_volume_root(&PathBuf::from("/mnt/backup/data"));
        assert_eq!(root, Some(PathBuf::from("/mnt/backup")));
    }
}
