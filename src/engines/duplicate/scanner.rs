//! File classification and image fingerprinting utilities.
//!
//! Implements operations 82 (compare filenames), 93 (detect duplicate
//! photos), 94 (detect similar photos), 95 (generate image fingerprints),
//! 96 (compare perceptual hashes), 97 (detect screenshots).
//!
//! The image fingerprint uses a true DCT-based perceptual hash (pHash).
//! The image is decoded to grayscale, resized to 32x32, transformed with
//! a 2D Discrete Cosine Transform (DCT-II), and the top-left 8x8
//! low-frequency coefficients are compared against their median to produce
//! a 64-bit hash. This is robust to resize, compression, and metadata
//! changes while remaining sensitive to actual content.

use image::imageops::FilterType;
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

/// A 64-bit perceptual fingerprint for image files. (op 95)
///
/// The fingerprint is a true DCT-based perceptual hash (pHash):
/// - Decode the image to grayscale
/// - Resize to 32x32
/// - Apply a 2D DCT-II
/// - Take the top-left 8x8 low-frequency coefficients
/// - Compare each coefficient to the median of the block (excluding DC)
/// - Each bit = 1 if coefficient > median, else 0 → 64-bit hash
///
/// This is robust to resize, compression, format, and metadata changes
/// while remaining sensitive to actual content. If the image cannot be
/// decoded, a lightweight byte-level fallback fingerprint is used.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ImageFingerprint(pub u64);

impl ImageFingerprint {
    /// Compute a perceptual fingerprint for an image file. (op 95)
    ///
    /// Tries the DCT-based pHash first. If the image cannot be decoded
    /// (e.g. unsupported format or corrupt data), falls back to the
    /// byte-level fingerprint.
    pub fn compute(path: &Path) -> Result<Self, std::io::Error> {
        if let Some(hash) = compute_phash(path) {
            return Ok(Self(hash));
        }
        Self::compute_byte_fingerprint(path)
    }

    /// Compute a lightweight byte-level fingerprint fallback. (op 95)
    ///
    /// Samples bytes at regular intervals from the raw image file data
    /// and hashes them with BLAKE3. Robust to metadata-only changes but
    /// less perceptually meaningful than the DCT-based pHash.
    pub fn compute_byte_fingerprint(path: &Path) -> Result<Self, std::io::Error> {
        let data = std::fs::read(path)?;
        if data.is_empty() {
            return Ok(Self(0));
        }

        let start = 256.min(data.len());
        let sample_region = &data[start..];

        let target_samples = 1024;
        let step = (sample_region.len() / target_samples).max(1);
        let mut sampled = Vec::with_capacity(target_samples * 2);

        let mut i = 0;
        while i < sample_region.len() && sampled.len() < target_samples * 2 {
            sampled.push(sample_region[i]);
            sampled.push(((i / step) & 0xFF) as u8);
            i += step;
        }

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
        hamming_distance(self.0, other.0)
    }
}

/// Compute the Hamming distance between two 64-bit hashes. (op 96)
/// A distance of 0 means identical; higher values mean more different.
pub fn hamming_distance(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

/// Compute a DCT-based perceptual hash (pHash) for an image file.
///
/// Returns `None` if the image cannot be opened or decoded.
pub fn compute_phash(path: &Path) -> Option<u64> {
    let img = image::open(path).ok()?;
    let gray = img.grayscale();
    let resized = image::imageops::resize(&gray, 32, 32, FilterType::Lanczos3);

    let mut matrix = vec![vec![0.0f64; 32]; 32];
    for y in 0..32u32 {
        for x in 0..32u32 {
            matrix[y as usize][x as usize] = f64::from(resized.get_pixel(x, y).0[0]);
        }
    }

    let dct = dct_2d(&matrix, 32);

    let mut block = [0.0f64; 64];
    for row in 0..8 {
        for col in 0..8 {
            block[row * 8 + col] = dct[row][col];
        }
    }

    let mut for_median: Vec<f64> = block.to_vec();
    for_median.remove(0);
    for_median.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = for_median[for_median.len() / 2];

    let mut hash: u64 = 0;
    for (i, &coeff) in block.iter().enumerate() {
        if coeff > median {
            hash |= 1u64 << i;
        }
    }

    Some(hash)
}

/// Apply a 1D DCT-II to a slice of length `size`.
///
/// X[k] = sum_{n=0}^{N-1} x[n] * cos(pi * k * (2n + 1) / (2N))
fn dct_1d(row: &[f64], size: usize) -> Vec<f64> {
    let mut out = vec![0.0; size];
    let n_f = size as f64;
    for (k, out_k) in out.iter_mut().enumerate() {
        let mut sum = 0.0;
        for (n, &row_n) in row.iter().enumerate().take(size) {
            let angle = std::f64::consts::PI * (k as f64) * (2.0 * n as f64 + 1.0) / (2.0 * n_f);
            sum += row_n * angle.cos();
        }
        *out_k = sum;
    }
    out
}

/// Apply a 2D DCT-II to a `size` x `size` matrix by applying the 1D
/// DCT to each row, then to each column of the result.
fn dct_2d(matrix: &[Vec<f64>], size: usize) -> Vec<Vec<f64>> {
    let mut rows_dct: Vec<Vec<f64>> = Vec::with_capacity(size);
    for row in matrix.iter().take(size) {
        rows_dct.push(dct_1d(row, size));
    }

    let mut result = vec![vec![0.0; size]; size];
    for col in 0..size {
        let column: Vec<f64> = rows_dct.iter().take(size).map(|r| r[col]).collect();
        let col_dct = dct_1d(&column, size);
        for row in 0..size {
            result[row][col] = col_dct[row];
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{ImageBuffer, Luma};
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn make_gradient_image(width: u32, height: u32) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let mut img = ImageBuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                let val = (((x + y) % 256) as u8).max(1);
                img.put_pixel(x, y, Luma([val]));
            }
        }
        img
    }

    fn make_solid_image(width: u32, height: u32, val: u8) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let mut img = ImageBuffer::new(width, height);
        for y in 0..height {
            for x in 0..width {
                img.put_pixel(x, y, Luma([val]));
            }
        }
        img
    }

    fn make_radial_gradient_image(width: u32, height: u32) -> ImageBuffer<Luma<u8>, Vec<u8>> {
        let mut img = ImageBuffer::new(width, height);
        let cx = width as f64 / 2.0;
        let cy = height as f64 / 2.0;
        let max_dist = (cx * cx + cy * cy).sqrt();
        for y in 0..height {
            for x in 0..width {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                let dist = (dx * dx + dy * dy).sqrt();
                let val = (255.0 * (1.0 - dist / max_dist)).round().clamp(0.0, 255.0) as u8;
                img.put_pixel(x, y, Luma([val]));
            }
        }
        img
    }

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
    fn test_phash_identical_images() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("a.png");
        let path2 = tmp.path().join("b.png");

        let img = make_gradient_image(64, 64);
        img.save(&path1).unwrap();
        img.save(&path2).unwrap();

        let h1 = compute_phash(&path1).expect("phash for image 1");
        let h2 = compute_phash(&path2).expect("phash for image 2");

        assert_eq!(h1, h2);
        assert_eq!(hamming_distance(h1, h2), 0);
    }

    #[test]
    fn test_phash_different_images() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("grad.png");
        let path2 = tmp.path().join("solid.png");

        let img1 = make_gradient_image(64, 64);
        let img2 = make_solid_image(64, 64, 200);
        img1.save(&path1).unwrap();
        img2.save(&path2).unwrap();

        let h1 = compute_phash(&path1).expect("phash for gradient");
        let h2 = compute_phash(&path2).expect("phash for solid");

        assert_ne!(h1, h2);
        assert!(hamming_distance(h1, h2) > 0);
    }

    #[test]
    fn test_phash_resilient_to_resize() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("small.png");
        let path2 = tmp.path().join("large.png");

        let img1 = make_radial_gradient_image(64, 64);
        let img2 = make_radial_gradient_image(128, 128);
        img1.save(&path1).unwrap();
        img2.save(&path2).unwrap();

        let h1 = compute_phash(&path1).expect("phash for small");
        let h2 = compute_phash(&path2).expect("phash for large");

        assert!(
            hamming_distance(h1, h2) <= 15,
            "resize-invariant: distance = {}",
            hamming_distance(h1, h2)
        );
    }

    #[test]
    fn test_phash_resilient_to_format() {
        let tmp = TempDir::new().unwrap();
        let png_path = tmp.path().join("img.png");
        let bmp_path = tmp.path().join("img.bmp");

        let img = make_gradient_image(64, 64);
        img.save(&png_path).unwrap();
        img.save(&bmp_path).unwrap();

        let h1 = compute_phash(&png_path).expect("phash for png");
        let h2 = compute_phash(&bmp_path).expect("phash for bmp");

        assert_eq!(hamming_distance(h1, h2), 0);
    }

    #[test]
    fn test_image_fingerprint_identical_files() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("a.png");
        let path2 = tmp.path().join("b.png");

        let img = make_gradient_image(64, 64);
        img.save(&path1).unwrap();
        img.save(&path2).unwrap();

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

        let img1 = make_gradient_image(64, 64);
        let img2 = make_solid_image(64, 64, 200);
        img1.save(&path1).unwrap();
        img2.save(&path2).unwrap();

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
    fn test_image_fingerprint_undecodable_falls_back() {
        let tmp = TempDir::new().unwrap();
        let path1 = tmp.path().join("a.png");
        let path2 = tmp.path().join("b.png");

        let data: Vec<u8> = (0..10_000).map(|i| (i % 256) as u8).collect();
        std::fs::write(&path1, &data).unwrap();
        std::fs::write(&path2, &data).unwrap();

        let fp1 = ImageFingerprint::compute(&path1).unwrap();
        let fp2 = ImageFingerprint::compute(&path2).unwrap();

        assert_eq!(fp1, fp2);
        assert_eq!(fp1.hamming_distance(&fp2), 0);
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

    #[test]
    fn test_hamming_distance_free_function() {
        assert_eq!(hamming_distance(0, 0), 0);
        assert_eq!(hamming_distance(0, 0xFFFF_FFFF_FFFF_FFFF), 64);
        assert_eq!(hamming_distance(0b1010, 0b1001), 2);
    }

    #[test]
    fn test_dct_1d_constant_input() {
        let input = vec![10.0; 4];
        let out = dct_1d(&input, 4);
        assert!((out[0] - 40.0).abs() < 1e-9);
        for k in 1..4 {
            assert!(out[k].abs() < 1e-9, "k={} out={}", k, out[k]);
        }
    }

    #[test]
    fn test_dct_2d_constant_input() {
        let matrix = vec![vec![5.0; 4]; 4];
        let out = dct_2d(&matrix, 4);
        assert!((out[0][0] - 80.0).abs() < 1e-9);
        for r in 0..4 {
            for c in 0..4 {
                if r == 0 && c == 0 {
                    continue;
                }
                assert!(out[r][c].abs() < 1e-9, "[{}][{}] = {}", r, c, out[r][c]);
            }
        }
    }

    #[test]
    fn test_dct_2d_separable() {
        let matrix = vec![
            vec![1.0, 2.0, 3.0, 4.0],
            vec![5.0, 6.0, 7.0, 8.0],
            vec![9.0, 10.0, 11.0, 12.0],
            vec![13.0, 14.0, 15.0, 16.0],
        ];
        let out = dct_2d(&matrix, 4);
        assert!((out[0][0] - 136.0).abs() < 1e-9);
        assert!(out[0][0] > out[0][1].abs());
        assert!(out[0][0] > out[1][0].abs());
    }
}
