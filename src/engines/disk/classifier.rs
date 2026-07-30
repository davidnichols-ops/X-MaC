//! Category classifier for capability #2 (Explain why my disk is full).
//!
//! Buckets the existing `xmac disk` scan findings into 8 high-level
//! categories so the user sees a meaningful breakdown instead of a flat
//! list of paths.
//!
//! Design notes:
//! - Path-based rules are checked first (more reliable than extensions).
//! - Extension-based rules are a fallback for files without strong path
//!   signals (e.g. media files living anywhere).
//! - Conservative: anything we can't confidently classify goes to
//!   `Unknown`, never to a category that implies "safe to reclaim".
//!
//! See `docs/CAPABILITY_2_DESIGN.md` for the full spec.

use std::path::Path;

/// The 8 user-visible buckets for capability #2.
///
/// The names are exactly what appears in the `--explain` output.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Bucket {
    Caches,
    DevArtifacts,
    Media,
    Archives,
    Applications,
    Backups,
    Unknown,
}

impl Bucket {
    /// Human-readable label for output.
    pub fn label(&self) -> &'static str {
        match self {
            Bucket::Caches => "caches",
            Bucket::DevArtifacts => "dev artifacts",
            Bucket::Media => "media",
            Bucket::Archives => "archives",
            Bucket::Applications => "applications",
            Bucket::Backups => "backups",
            Bucket::Unknown => "unknown",
        }
    }
}

/// Classify a path into one of the 8 buckets.
///
/// Rules (in order):
/// 1. Path substring matches one of the well-known paths → that bucket.
/// 2. File extension matches media/archive/app → that bucket.
/// 3. Fallback: `Unknown`.
pub fn classify(path: &Path) -> Bucket {
    let path_str = path.to_string_lossy();

    // ---- Path-based rules (most reliable) ----

    // Common directory names that always indicate caches/dev/etc.
    // This catches the top-level directory case (e.g. when the disk
    // engine emits a finding for the dir itself, before walking into it).
    if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
        let n = name.to_ascii_lowercase();
        // Cache-like directory names.
        if matches!(n.as_str(), "cache" | "caches" | "logs" | "log") {
            return Bucket::Caches;
        }
        // Trash counts as backups (user-emptiable storage).
        if matches!(n.as_str(), ".trash" | "trash") {
            return Bucket::Backups;
        }
        // Dev-artifact directory names.
        if matches!(
            n.as_str(),
            "node_modules"
                | "target"
                | "dist"
                | "build"
                | "out"
                | ".next"
                | ".nuxt"
                | "__pycache__"
                | ".pytest_cache"
                | ".mypy_cache"
                | ".ruff_cache"
                | ".tox"
                | ".venv"
                | "venv"
                | ".gradle"
                | "coverage"
                | ".parcel-cache"
                | ".turbo"
                | "deriveddata"
        ) {
            return Bucket::DevArtifacts;
        }
    }

    // Caches — OS / app caches (substrings inside known cache dirs)
    if contains_any(&path_str, &[
        "/Library/Caches/",
        "/.cache/",
        "/Library/Logs/",
        "/.npm/",
        "/.cargo/",
        "/.rustup/",
        "/.bundle/",
        "/.gem/",
        "/.pyenv/cache/",
    ]) {
        return Bucket::Caches;
    }

    // Dev artifacts — build outputs, dependency dirs
    if contains_any(&path_str, &[
        "/node_modules/",
        "/target/",          // Rust
        "/.gradle/",
        "/build/",           // Gradle, CMake, Ant
        "/dist/",            // JS bundlers
        "/.next/",           // Next.js
        "/.nuxt/",           // Nuxt
        "/__pycache__/",
        "/.pytest_cache/",
        "/DerivedData/",     // Xcode
        "/.xcuserstate/",
        "/.git/objects/",
        "/.venv/",
        "/venv/",
        "/.tox/",
        "/.mypy_cache/",
        "/.ruff_cache/",
        "/.parcel-cache/",
        "/.turbo/",
        "/coverage/",
        "/.cargo/registry/",
        "/.cargo/git/",
    ]) {
        return Bucket::DevArtifacts;
    }

    // Applications — .app bundles, /Applications
    if contains_any(&path_str, &[
        "/Applications/",
    ]) || path_str.ends_with(".app/") || path_str.contains(".app/Contents/") {
        return Bucket::Applications;
    }

    // Backups — Trash, iOS backups, Time Machine
    if contains_any(&path_str, &[
        "/.Trash/",
        "/MobileSync/",
        "/Backups.backupdb/",
        "/.MobileBackups/",
    ]) || path_str.ends_with(".iosbackup")
        || path_str.ends_with(".backup")
        || path_str.ends_with(".tmbackup")
    {
        return Bucket::Backups;
    }

    // Archives — installer / archive formats
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        let ext_lower = ext.to_ascii_lowercase();
        if matches!(
            ext_lower.as_str(),
            "zip" | "tar"
            | "gz"
            | "bz2"
            | "xz"
            | "7z"
            | "rar"
            | "dmg"
            | "pkg"
            | "iso"
            | "zst"
        ) {
            return Bucket::Archives;
        }

        // Media — images / video / audio
        if is_media_extension(&ext_lower) {
            return Bucket::Media;
        }
    }

    // Fallback
    Bucket::Unknown
}

/// Helper: does `haystack` contain any of the `needles`?
fn contains_any(haystack: &str, needles: &[&str]) -> bool {
    needles.iter().any(|n| haystack.contains(n))
}

/// Is this extension a media file (image, video, audio)?
fn is_media_extension(ext: &str) -> bool {
    matches!(ext,
        // Images
        "jpg" | "jpeg" | "png" | "gif" | "bmp" | "webp" | "tiff" | "tif"
        | "heic" | "heif" | "raw" | "cr2" | "nef" | "arw" | "dng"
        | "svg" | "ico" | "psd" | "ai" | "eps"
        // Video
        | "mp4" | "mov" | "m4v" | "mkv" | "avi" | "wmv" | "flv" | "webm"
        | "mpg" | "mpeg" | "m2v" | "3gp"
        // Audio
        | "mp3" | "m4a" | "wav" | "flac" | "ogg" | "aac" | "wma" | "aiff"
        | "alac"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_classify_caches_os_cache() {
        let p = PathBuf::from("/Users/david/Library/Caches/com.apple.Safari/cache.db");
        assert_eq!(classify(&p), Bucket::Caches);
    }

    #[test]
    fn test_classify_caches_dotcache() {
        let p = PathBuf::from("/home/user/.cache/pip/selfcheck.json");
        assert_eq!(classify(&p), Bucket::Caches);
    }

    #[test]
    fn test_classify_caches_npm() {
        let p = PathBuf::from("/Users/david/Projects/foo/node_modules/.bin/foo");
        // Note: node_modules matches DevArtifacts first, not Caches.
        // The Caches rule for .npm/ applies to top-level cache dirs.
        let p2 = PathBuf::from("/Users/david/.npm/_cacache/index.json");
        assert_eq!(classify(&p2), Bucket::Caches);
        assert_eq!(classify(&p), Bucket::DevArtifacts);
    }

    #[test]
    fn test_classify_dev_artifacts_node_modules() {
        let p = PathBuf::from("/Users/david/Projects/foo/node_modules/lodash/index.js");
        assert_eq!(classify(&p), Bucket::DevArtifacts);
    }

    #[test]
    fn test_classify_dev_artifacts_rust_target() {
        let p = PathBuf::from("/Users/david/Projects/foo/target/release/binary");
        assert_eq!(classify(&p), Bucket::DevArtifacts);
    }

    #[test]
    fn test_classify_dev_artifacts_xcode_derived_data() {
        let p = PathBuf::from("/Users/david/Library/Developer/Xcode/DerivedData/SomeApp/Build/Products/Release/App");
        assert_eq!(classify(&p), Bucket::DevArtifacts);
    }

    #[test]
    fn test_classify_dev_artifacts_python_venv() {
        let p = PathBuf::from("/Users/david/projects/app/.venv/lib/python3.13/site-packages/foo.py");
        assert_eq!(classify(&p), Bucket::DevArtifacts);
    }

    #[test]
    fn test_classify_media_jpg() {
        let p = PathBuf::from("/Users/david/Pictures/Vacation/IMG_0001.jpg");
        assert_eq!(classify(&p), Bucket::Media);
    }

    #[test]
    fn test_classify_media_mp4() {
        let p = PathBuf::from("/Users/david/Movies/clip.mp4");
        assert_eq!(classify(&p), Bucket::Media);
    }

    #[test]
    fn test_classify_media_raw() {
        let p = PathBuf::from("/Users/david/Pictures/RAW/DSC_0001.cr2");
        assert_eq!(classify(&p), Bucket::Media);
    }

    #[test]
    fn test_classify_archives_zip() {
        let p = PathBuf::from("/Users/david/Downloads/backup.zip");
        assert_eq!(classify(&p), Bucket::Archives);
    }

    #[test]
    fn test_classify_archives_dmg() {
        let p = PathBuf::from("/Users/david/Downloads/installer.dmg");
        assert_eq!(classify(&p), Bucket::Archives);
    }

    #[test]
    fn test_classify_archives_pkg() {
        let p = PathBuf::from("/Users/david/Downloads/tool.pkg");
        assert_eq!(classify(&p), Bucket::Archives);
    }

    #[test]
    fn test_classify_applications_app_bundle() {
        let p = PathBuf::from("/Applications/Safari.app/Contents/Info.plist");
        assert_eq!(classify(&p), Bucket::Applications);
    }

    #[test]
    fn test_classify_applications_under_applications() {
        let p = PathBuf::from("/Applications/Some App.app/Contents/MacOS/binary");
        assert_eq!(classify(&p), Bucket::Applications);
    }

    #[test]
    fn test_classify_backups_trash() {
        let p = PathBuf::from("/Users/david/.Trash/old_file.txt");
        assert_eq!(classify(&p), Bucket::Backups);
    }

    #[test]
    fn test_classify_backups_ios() {
        let p = PathBuf::from("/Users/david/Library/Application Support/MobileSync/Backup/abcdef/Info.plist");
        assert_eq!(classify(&p), Bucket::Backups);
    }

    #[test]
    fn test_classify_backups_time_machine() {
        let p = PathBuf::from("/Volumes/Time Machine Backups/Backups.backupdb/Mac/Latest/Mac.sparsebundle");
        assert_eq!(classify(&p), Bucket::Backups);
    }

    #[test]
    fn test_classify_unknown() {
        let p = PathBuf::from("/Users/david/Documents/random.xyz");
        assert_eq!(classify(&p), Bucket::Unknown);
    }

    #[test]
    fn test_classify_unknown_text_file() {
        let p = PathBuf::from("/Users/david/Documents/notes.txt");
        // .txt is not in any media/archive list — goes to Unknown.
        assert_eq!(classify(&p), Bucket::Unknown);
    }

    #[test]
    fn test_label_strings_are_stable() {
        // The labels are part of the user-facing output contract.
        // Changing them is a breaking change.
        assert_eq!(Bucket::Caches.label(), "caches");
        assert_eq!(Bucket::DevArtifacts.label(), "dev artifacts");
        assert_eq!(Bucket::Media.label(), "media");
        assert_eq!(Bucket::Archives.label(), "archives");
        assert_eq!(Bucket::Applications.label(), "applications");
        assert_eq!(Bucket::Backups.label(), "backups");
        assert_eq!(Bucket::Unknown.label(), "unknown");
    }

    #[test]
    fn test_no_path_panics() {
        // Regression test: the classifier must handle any path without
        // panicking — empty paths, paths with no extension, paths with
        // unicode, very long paths.
        for p in &[
            "",
            "/",
            ".",
            "..",
            "/usr/bin/python3",
            "/var/folders/xx/T/TemporaryItems/random",
            "/Users/david/Café/notes.txt",
            "/Users/david/file without extension",
        ] {
            let path = PathBuf::from(p);
            let _ = classify(&path); // must not panic
        }
    }

    #[test]
    fn test_classify_dir_with_cache_name() {
        // The disk engine emits a finding for the dir itself (not just
        // its contents). The classifier should recognize a top-level
        // dir called "cache" as Caches.
        assert_eq!(
            classify(&PathBuf::from("/Users/david/cache")),
            Bucket::Caches
        );
        assert_eq!(
            classify(&PathBuf::from("/Users/david/Library/Caches")),
            Bucket::Caches
        );
        assert_eq!(
            classify(&PathBuf::from("/Users/david/logs")),
            Bucket::Caches
        );
    }

    #[test]
    fn test_classify_dir_with_target_name() {
        // The disk engine emits a finding for the `target/` dir.
        assert_eq!(
            classify(&PathBuf::from("/Users/david/projects/foo/target")),
            Bucket::DevArtifacts
        );
        assert_eq!(
            classify(&PathBuf::from("/Users/david/projects/foo/node_modules")),
            Bucket::DevArtifacts
        );
        assert_eq!(
            classify(&PathBuf::from("/Users/david/projects/foo/dist")),
            Bucket::DevArtifacts
        );
        assert_eq!(
            classify(&PathBuf::from("/Users/david/projects/app/build")),
            Bucket::DevArtifacts
        );
    }
}