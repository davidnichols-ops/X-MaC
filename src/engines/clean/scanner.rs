use std::path::Path;
use std::time::{Duration, SystemTime};
use walkdir::WalkDir;

use crate::core::types::ScanConfig;
use crate::util::disk::dir_size;

pub struct CleanScanner;

impl CleanScanner {
    pub fn scan_directory<'a>(
        path: &'a Path,
        config: &'a ScanConfig,
    ) -> impl Iterator<Item = walkdir::DirEntry> + 'a {
        WalkDir::new(path)
            .follow_links(config.follow_symlinks)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(move |e| {
                if !config.include_hidden {
                    let file_name = e.file_name().to_string_lossy();
                    if file_name.starts_with('.') {
                        return false;
                    }
                }

                if !config.exclude_patterns.is_empty() {
                    let path_str = e.path().to_string_lossy();
                    for pattern in &config.exclude_patterns {
                        if glob_match(pattern, &path_str) {
                            return false;
                        }
                    }
                }

                true
            })
    }

    pub fn dir_size(path: &Path) -> u64 {
        dir_size(path)
    }

    pub fn is_older_than(path: &Path, duration: Duration) -> bool {
        match std::fs::metadata(path) {
            Ok(metadata) => {
                if let Ok(modified) = metadata.modified() {
                    if let Ok(elapsed) = SystemTime::now().duration_since(modified) {
                        return elapsed > duration;
                    }
                }
                false
            }
            Err(_) => false,
        }
    }

    pub fn file_size(path: &Path) -> u64 {
        crate::util::disk::file_size_physical(path)
    }
}

fn glob_match(pattern: &str, path: &str) -> bool {
    let pattern = pattern.to_lowercase();
    let path_lower = path.to_lowercase();

    if pattern.contains('*') {
        let parts: Vec<&str> = pattern.split('*').collect();
        let mut pos = 0;
        for part in parts {
            if part.is_empty() {
                continue;
            }
            if let Some(found) = path_lower[pos..].find(part) {
                pos += found + part.len();
            } else {
                return false;
            }
        }
        true
    } else {
        path_lower.contains(&pattern)
    }
}
