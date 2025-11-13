//! SIMD-optimized XYB color space conversions
//!
//! Platform-specific SIMD optimizations for RGB⟷XYB conversion using:
//! - AVX2 for x86_64
//! - NEON for ARM/AArch64
//! - Fallback to scalar implementation on other platforms

use super::xyb::{rgb_to_xyb, xyb_to_rgb};

/// Check if AVX2 is available at runtime
#[cfg(target_arch = "x86_64")]
pub fn has_avx2() -> bool {
    #[cfg(target_feature = "avx2")]
    {
        true
    }
    #[cfg(not(target_feature = "avx2"))]
    {
        is_x86_feature_detected!("avx2")
    }
}

/// Check if NEON is available at runtime
#[cfg(target_arch = "aarch64")]
pub fn has_neon() -> bool {
    true // Always available on AArch64
}

#[cfg(target_arch = "arm")]
pub fn has_neon() -> bool {
    std::arch::is_arm_feature_detected!("neon")
}

/// Auto-selecting RGB to XYB batch conversion
///
/// Converts multiple RGB triplets to XYB using the fastest available SIMD implementation.
/// Input and output are flat arrays where every 3 elements form an RGB/XYB triplet.
///
/// # Arguments
/// * `rgb` - Flat array of RGB values [R0, G0, B0, R1, G1, B1, ...]
/// * `xyb` - Output flat array for XYB values [X0, Y0, B0, X1, Y1, B1, ...]
/// * `count` - Number of pixels (rgb.len() and xyb.len() must both be count * 3)
pub fn rgb_to_xyb_batch(rgb: &[f32], xyb: &mut [f32], count: usize) {
    assert_eq!(rgb.len(), count * 3);
    assert_eq!(xyb.len(), count * 3);

    #[cfg(target_arch = "x86_64")]
    {
        if has_avx2() {
            unsafe { rgb_to_xyb_batch_avx2(rgb, xyb, count) };
            return;
        }
    }

    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    {
        if has_neon() {
            unsafe { rgb_to_xyb_batch_neon(rgb, xyb, count) };
            return;
        }
    }

    // Scalar fallback
    rgb_to_xyb_batch_scalar(rgb, xyb, count);
}

/// Auto-selecting XYB to RGB batch conversion
pub fn xyb_to_rgb_batch(xyb: &[f32], rgb: &mut [f32], count: usize) {
    assert_eq!(xyb.len(), count * 3);
    assert_eq!(rgb.len(), count * 3);

    #[cfg(target_arch = "x86_64")]
    {
        if has_avx2() {
            unsafe { xyb_to_rgb_batch_avx2(xyb, rgb, count) };
            return;
        }
    }

    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    {
        if has_neon() {
            unsafe { xyb_to_rgb_batch_neon(xyb, rgb, count) };
            return;
        }
    }

    // Scalar fallback
    xyb_to_rgb_batch_scalar(xyb, rgb, count);
}

// ============================================================================
// Scalar Implementation (optimized for auto-vectorization)
// ============================================================================

/// Optimized scalar batch conversion RGB to XYB
///
/// This implementation is structured to allow compiler auto-vectorization.
/// Processing pixels in sequence allows SIMD instructions to be generated automatically.
#[inline]
fn rgb_to_xyb_batch_scalar(rgb: &[f32], xyb: &mut [f32], count: usize) {
    for i in 0..count {
        let r = rgb[i * 3];
        let g = rgb[i * 3 + 1];
        let b = rgb[i * 3 + 2];

        let (x, y, b_minus_y) = rgb_to_xyb(r, g, b);

        xyb[i * 3] = x;
        xyb[i * 3 + 1] = y;
        xyb[i * 3 + 2] = b_minus_y;
    }
}

/// Optimized scalar batch conversion XYB to RGB
#[inline]
fn xyb_to_rgb_batch_scalar(xyb: &[f32], rgb: &mut [f32], count: usize) {
    for i in 0..count {
        let x = xyb[i * 3];
        let y = xyb[i * 3 + 1];
        let b_minus_y = xyb[i * 3 + 2];

        let (r, g, b) = xyb_to_rgb(x, y, b_minus_y);

        rgb[i * 3] = r;
        rgb[i * 3 + 1] = g;
        rgb[i * 3 + 2] = b;
    }
}

// ============================================================================
// AVX2 Implementation (x86_64)
// ============================================================================

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn rgb_to_xyb_batch_avx2(rgb: &[f32], xyb: &mut [f32], count: usize) {
    // For now, use the scalar implementation with AVX2 enabled
    // This allows the compiler to auto-vectorize with AVX2 instructions
    rgb_to_xyb_batch_scalar(rgb, xyb, count);
}

#[cfg(target_arch = "x86_64")]
#[target_feature(enable = "avx2")]
unsafe fn xyb_to_rgb_batch_avx2(xyb: &[f32], rgb: &mut [f32], count: usize) {
    xyb_to_rgb_batch_scalar(xyb, rgb, count);
}

// ============================================================================
// NEON Implementation (ARM/AArch64)
// ============================================================================

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
#[target_feature(enable = "neon")]
unsafe fn rgb_to_xyb_batch_neon(rgb: &[f32], xyb: &mut [f32], count: usize) {
    // Use the scalar implementation with NEON enabled for auto-vectorization
    rgb_to_xyb_batch_scalar(rgb, xyb, count);
}

#[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
#[target_feature(enable = "neon")]
unsafe fn xyb_to_rgb_batch_neon(xyb: &[f32], rgb: &mut [f32], count: usize) {
    xyb_to_rgb_batch_scalar(xyb, rgb, count);
}

// ============================================================================
// Convenience functions for image-sized buffers
// ============================================================================

/// Convert a full RGB image buffer to XYB with SIMD optimization
///
/// # Arguments
/// * `rgb_image` - Flat RGB buffer [R0,G0,B0,R1,G1,B1,...]
/// * `xyb_image` - Output XYB buffer [X0,Y0,B0,X1,Y1,B1,...]
/// * `width` - Image width
/// * `height` - Image height
pub fn rgb_to_xyb_image_simd(
    rgb_image: &[f32],
    xyb_image: &mut [f32],
    width: usize,
    height: usize,
) {
    let pixel_count = width * height;
    assert_eq!(rgb_image.len(), pixel_count * 3);
    assert_eq!(xyb_image.len(), pixel_count * 3);

    rgb_to_xyb_batch(rgb_image, xyb_image, pixel_count);
}

/// Convert a full XYB image buffer to RGB with SIMD optimization
pub fn xyb_to_rgb_image_simd(
    xyb_image: &[f32],
    rgb_image: &mut [f32],
    width: usize,
    height: usize,
) {
    let pixel_count = width * height;
    assert_eq!(xyb_image.len(), pixel_count * 3);
    assert_eq!(rgb_image.len(), pixel_count * 3);

    xyb_to_rgb_batch(xyb_image, rgb_image, pixel_count);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_to_xyb_batch_correctness() {
        // Test with a few pixels
        let rgb = vec![
            1.0, 0.0, 0.0, // Red
            0.0, 1.0, 0.0, // Green
            0.0, 0.0, 1.0, // Blue
            0.5, 0.5, 0.5, // Gray
        ];

        let mut xyb_scalar = vec![0.0f32; 12];
        let mut xyb_batch = vec![0.0f32; 12];

        // Compute using scalar per-pixel
        for i in 0..4 {
            let (x, y, b) = rgb_to_xyb(rgb[i * 3], rgb[i * 3 + 1], rgb[i * 3 + 2]);
            xyb_scalar[i * 3] = x;
            xyb_scalar[i * 3 + 1] = y;
            xyb_scalar[i * 3 + 2] = b;
        }

        // Compute using batch
        rgb_to_xyb_batch(&rgb, &mut xyb_batch, 4);

        // Check that results match
        for i in 0..12 {
            let diff = (xyb_scalar[i] - xyb_batch[i]).abs();
            assert!(
                diff < 1e-6,
                "Mismatch at index {}: scalar={}, batch={}",
                i,
                xyb_scalar[i],
                xyb_batch[i]
            );
        }
    }

    #[test]
    fn test_xyb_to_rgb_batch_correctness() {
        let xyb = vec![
            0.1, 0.5, 0.2,
            -0.1, 0.3, 0.4,
            0.0, 0.0, 0.0,
            0.2, 0.8, -0.1,
        ];

        let mut rgb_scalar = vec![0.0f32; 12];
        let mut rgb_batch = vec![0.0f32; 12];

        // Compute using scalar per-pixel
        for i in 0..4 {
            let (r, g, b) = xyb_to_rgb(xyb[i * 3], xyb[i * 3 + 1], xyb[i * 3 + 2]);
            rgb_scalar[i * 3] = r;
            rgb_scalar[i * 3 + 1] = g;
            rgb_scalar[i * 3 + 2] = b;
        }

        // Compute using batch
        xyb_to_rgb_batch(&xyb, &mut rgb_batch, 4);

        // Check that results match
        for i in 0..12 {
            let diff = (rgb_scalar[i] - rgb_batch[i]).abs();
            assert!(
                diff < 1e-5,
                "Mismatch at index {}: scalar={}, batch={}",
                i,
                rgb_scalar[i],
                rgb_batch[i]
            );
        }
    }

    #[test]
    fn test_xyb_roundtrip_batch() {
        let rgb_orig = vec![
            0.8, 0.2, 0.3,
            0.1, 0.9, 0.4,
            0.5, 0.5, 0.5,
            0.0, 0.0, 0.0,
        ];

        let mut xyb = vec![0.0f32; 12];
        let mut rgb_back = vec![0.0f32; 12];

        rgb_to_xyb_batch(&rgb_orig, &mut xyb, 4);
        xyb_to_rgb_batch(&xyb, &mut rgb_back, 4);

        // Check roundtrip accuracy
        for i in 0..12 {
            let diff = (rgb_orig[i] - rgb_back[i]).abs();
            assert!(
                diff < 0.01,
                "Roundtrip error at {}: orig={}, back={}",
                i,
                rgb_orig[i],
                rgb_back[i]
            );
        }
    }

    #[test]
    fn test_image_simd_functions() {
        let width = 4;
        let height = 4;
        let pixel_count = width * height;

        let mut rgb = vec![0.0f32; pixel_count * 3];
        for i in 0..pixel_count {
            rgb[i * 3] = (i % 3) as f32 / 3.0;
            rgb[i * 3 + 1] = ((i + 1) % 3) as f32 / 3.0;
            rgb[i * 3 + 2] = ((i + 2) % 3) as f32 / 3.0;
        }

        let mut xyb = vec![0.0f32; pixel_count * 3];
        let mut rgb_back = vec![0.0f32; pixel_count * 3];

        rgb_to_xyb_image_simd(&rgb, &mut xyb, width, height);
        xyb_to_rgb_image_simd(&xyb, &mut rgb_back, width, height);

        // Check roundtrip
        for i in 0..(pixel_count * 3) {
            let diff = (rgb[i] - rgb_back[i]).abs();
            assert!(diff < 0.01, "Image roundtrip error at index {}", i);
        }
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_avx2_detection() {
        let _ = has_avx2();
    }

    #[test]
    #[cfg(any(target_arch = "aarch64", target_arch = "arm"))]
    fn test_neon_detection() {
        let _ = has_neon();
    }
}
