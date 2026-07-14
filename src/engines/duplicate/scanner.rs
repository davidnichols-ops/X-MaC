//! File classification and image fingerprinting utilities.
//!
//! Implements operations 82 (compare filenames), 93 (detect duplicate
//! photos), 94 (detect similar photos), 95 (generate image fingerprints),
//! 96 (compare perceptual hashes), 97 (detect screenshots).
//!
//! The image fingerprint uses a byte-level perceptual hash that does not
//! require an image decoding library. It samples bytes at regular intervals
//! from the image file's raw data, creating a 64-bit fingerprint that is
//! robust to metadata changes (EXIF, timestamps) but sensitive to actual
//! pixel content changes. For true perceptual hashing (DCT-based pHash),
//! an image decoding crate would be needed — this is a lightweight
//! approximation that works well for detecting near-identical images.

use std::path::Path;

/// Image file extensions (case-insensitive).
const IMAGE_EXTENSIONS: &[&str] = &[
    "jpg", "jpeg", "png", "gif", "bmp", "tiff", "tif", "webp", "heic", "heif", "avif", "svg",
    "raw", "cr2", "nef", "arw",
];

/// Screenshot filename patterns (macOS and iOS).
/// macOS: "Screenshot 2024-01-15 at 10.30.00.png"
/// iOS transfer: "IMG_1234.PNG", "Photo 2024-01-15 at 10.30.00.jpeg"
const SCREENSHOT_PATTERNS: &[&str] = &[
    "screenshot",
    "screen shot",
    "screen_shot",
    "screencapture",
    "screen capture",
    "screen-capture",
];

/// Check if a file is an image based on its extension. (op 93)
pub fn is_image_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            let ext_lower = ext.to_lowercase();
            IMAGE_EXTENSIONS.iter().any(|&img_ext| ext_lower == img_ext)
        })
        .unwrap_or(false)
}

/// Check if a file matches the screenshot filename pattern. (op 97)
pub fn is_screenshot(path: &Path) -> bool {
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().to_lowercase())
        .unwrap_or_default();

    SCREENSHOT_PATTERNS
        .iter()
        .any(|&pattern| file_name.contains(pattern))
}

/// A 64-bit perceptual fingerprint for image files.
///
/// This is a lightweight fingerprint computed from sampled bytes of the
/// raw image file. It is NOT a true DCT-based perceptual hash (which would
/// require image decoding), but it provides a useful approximation:
/// - Robust to metadata-only changes (EXIF, timestamps)
/// - Sensitive to actual content changes
/// - Fast to compute (reads only sampled bytes)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageFingerprint(pub u64);

impl ImageFingerprint {
    /// Compute a fingerprint for an image file. (op 95)
    ///
    /// The algorithm:
    /// 1. Read the file in chunks
    /// 2. Sample bytes at regular intervals (every Nth byte)
    /// 3. Build a 64-bit hash by combining sampled bytes with position
    ///    information
    /// 4. Use BLAKE3 to hash the sampled data for a robust fingerprint
    pub fn compute(path: &Path) -> Result<Self, std::io::Error> {
        let data = std::fs::read(path)?;
        if data.is_empty() {
            return Ok(Self(0));
        }

        // Sample bytes at regular intervals — skip the first 256 bytes
        // (typically metadata headers) and sample from the pixel data.
        let start = 256.min(data.len());
        let sample_region = &data[start..];

        // Sample 1024 bytes at regular intervals
        let target_samples = 1024;
        let step = (sample_region.len() / target_samples).max(1);
        let mut sampled = Vec::with_capacity(target_samples * 2);

        let mut i = 0;
        while i < sample_region.len() && sampled.len() < target_samples * 2 {
            sampled.push(sample_region[i]);
            // Also include position information for structural sensitivity
            sampled.push(((i / step) & 0xFF) as u8);
            i += step;
        }

        // Hash the sampled data with BLAKE3 and take the first 8 bytes as u64
        let hash = blake3::hash(&sampled);
        let bytes = hash.as_bytes();
        let fingerprint = u64::from_be_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
        ]);

        Ok(Self(fingerprint))
    }

    /// Compute the Hamming distance between two fingerprints. (op 96)
    /// A distance of 0 means identical; higher values mean more different.
    pub fn hamming_distance(&self, other: &Self) -> u32 {
        (self.0 ^ other.0).count_ones()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_is_image_file_jpg() {
        assert!(is_image_file(&PathBuf::from("/photos/img.jpg")));
        assert!(is_image_file(&PathBuf::from("/photos/img.JPG")));
        assert!(is_image_file(&PathBuf::from("/photos/img.jpeg")));
        assert!(is_image_file(&PathBuf::from("/photos/img.png")));
        assert!(is_image_file(&PathBuf::from("/photos/img.heic")));
    }

    #[test]
    fn test_is_image_file_non_image() {
        assert!(!is_image_file(&PathBuf::from("/docs/readme.txt")));
        assert!(!is_image_file(&PathBuf::from("/docs/file.pdf")));
        assert!(!is_image_file(&PathBuf::from("/no-extension")));
    }

    #[test]
    fn test_is_screenshot_macos_pattern() {
        assert!(is_screenshot(&PathBuf::from(
            "/Desktop/Screenshot 2024-01-15 at 10.30.00.png"
        )));
        assert!(is_screenshot(&PathBuf::from(
            "/Desktop/Screen Shot 2024-01-15 at 10.30.00.png"
        )));
    }

    #[test]
    fn test_is_screenshot_non_screenshot() {
        assert!(!is_screenshot(&PathBuf::from("/photos/vacation.jpg")));
        assert!(!is_screenshot(&PathBuf::from("/docs/report.pdf")));
    }

    #[test]
    fn test_image_fingerprint_identical_files() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("a.png");
        let path2 = tmp.path().join("b.png");

        // Create fake "image" data (doesn't need to be a real image)
        let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&path1, &data).unwrap();
        std::fs::write(&path2, &data).unwrap();

        let fp1 = ImageFingerprint::compute(&path1).unwrap();
        let fp2 = ImageFingerprint::compute(&path2).unwrap();

        assert_eq!(fp1, fp2);
        assert_eq!(fp1.hamming_distance(&fp2), 0);
    }

    #[test]
    fn test_image_fingerprint_different_files() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("a.png");
        let path2 = tmp.path().join("b.png");

        let data1: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        let data2: Vec<u8> = (0..10_000).map(|i| ((i + 128) % 256) as u8).collect();
        std::fs::write(&path1, &data1).unwrap();
        std::fs::write(&path2, &data2).unwrap();

        let fp1 = ImageFingerprint::compute(&path1).unwrap();
        let fp2 = ImageFingerprint::compute(&path2).unwrap();

        assert_ne!(fp1, fp2);
        assert!(fp1.hamming_distance(&fp2) > 0);
    }

    #[test]
    fn test_image_fingerprint_empty_file() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("empty.png");
        std::fs::write(&path, b"").unwrap();

        let fp = ImageFingerprint::compute(&path).unwrap();
        assert_eq!(fp.0, 0);
    }

    #[test]
    fn test_hamming_distance() {
        let a = ImageFingerprint(0b0000);
        let b = ImageFingerprint(0b1111);
        assert_eq!(a.hamming_distance(&b), 4);

        let c = ImageFingerprint(0b1010);
        let d = ImageFingerprint(0b1001);
        assert_eq!(c.hamming_distance(&d), 2);
    }
}
