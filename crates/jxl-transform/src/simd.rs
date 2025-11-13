//! SIMD-optimized implementations for DCT and color transforms
//!
//! Provides infrastructure for 2-4x performance improvements using platform-specific SIMD:
//! - x86/x86_64: SSE2, AVX2
//! - ARM: NEON
//! - Currently uses fallback to scalar implementation (SIMD implementations are TODO)

/// SIMD capability detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum SimdLevel {
    /// No SIMD support
    Scalar,
    /// SSE2 (x86/x86_64)
    Sse2,
    /// AVX2 (x86/x86_64)
    Avx2,
    /// NEON (ARM)
    Neon,
}

impl SimdLevel {
    /// Detect best available SIMD level for current CPU
    pub fn detect() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            if is_x86_feature_detected!("avx2") {
                return SimdLevel::Avx2;
            }
            if is_x86_feature_detected!("sse2") {
                return SimdLevel::Sse2;
            }
        }

        #[cfg(target_arch = "x86")]
        {
            if is_x86_feature_detected!("sse2") {
                return SimdLevel::Sse2;
            }
        }

        #[cfg(target_arch = "aarch64")]
        {
            // NEON is always available on aarch64
            return SimdLevel::Neon;
        }

        SimdLevel::Scalar
    }

    /// Get human-readable name
    pub fn name(&self) -> &'static str {
        match self {
            SimdLevel::Scalar => "Scalar (no SIMD)",
            SimdLevel::Sse2 => "SSE2",
            SimdLevel::Avx2 => "AVX2",
            SimdLevel::Neon => "NEON",
        }
    }

    /// Check if hardware supports this SIMD level
    pub fn is_supported(&self) -> bool {
        matches!(Self::detect(), level if level >= *self)
    }
}

/// Dispatch DCT to best available SIMD implementation
///
/// Currently falls back to scalar implementation.
/// TODO: Implement SSE2/AVX2/NEON optimized versions
pub fn dct_8x8_simd(input: &[f32; 64], output: &mut [f32; 64]) {
    // TODO: Dispatch to SIMD implementation based on detected level
    // For now, use scalar implementation
    crate::dct8x8_forward(input, output);
}

/// Dispatch IDCT to best available SIMD implementation
///
/// Currently falls back to scalar implementation.
/// TODO: Implement SSE2/AVX2/NEON optimized versions
pub fn idct_8x8_simd(input: &[f32; 64], output: &mut [f32; 64]) {
    // TODO: Dispatch to SIMD implementation based on detected level
    // For now, use scalar implementation
    crate::dct8x8_inverse(input, output);
}

/// RGB to XYB color conversion with SIMD dispatch
///
/// Currently falls back to scalar implementation.
/// TODO: Implement SIMD version
pub fn rgb_to_xyb_simd(rgb: &[f32], xyb: &mut [f32], count: usize) {
    // Scalar fallback for now
    for i in 0..count {
        let r = rgb[i * 3];
        let g = rgb[i * 3 + 1];
        let b = rgb[i * 3 + 2];

        // XYB conversion (libjxl values)
        let l = 0.3 * r + 0.3 * g + 0.3 * b;
        let m = 0.622 * r + 0.622 * g + 0.622 * b;
        let s = 0.078 * r + 0.078 * g + 0.078 * b;

        xyb[i * 3] = l - m;       // X
        xyb[i * 3 + 1] = l + m;   // Y
        xyb[i * 3 + 2] = s - m;   // B-Y
    }
}

/// Benchmark SIMD vs scalar performance
pub fn benchmark_simd() -> (f64, f64, SimdLevel) {
    use std::time::Instant;

    let input = [1.0f32; 64];
    let mut output = [0.0f32; 64];
    let iterations = 10000;

    // Benchmark scalar
    let start = Instant::now();
    for _ in 0..iterations {
        crate::dct8x8_forward(&input, &mut output);
    }
    let scalar_time = start.elapsed().as_secs_f64();

    // Benchmark SIMD (currently same as scalar)
    let start = Instant::now();
    for _ in 0..iterations {
        dct_8x8_simd(&input, &mut output);
    }
    let simd_time = start.elapsed().as_secs_f64();

    let level = SimdLevel::detect();

    (scalar_time, simd_time, level)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simd_detection() {
        let level = SimdLevel::detect();
        println!("Detected SIMD level: {}", level.name());
        // Just verify it doesn't crash
        assert!(matches!(
            level,
            SimdLevel::Scalar | SimdLevel::Sse2 | SimdLevel::Avx2 | SimdLevel::Neon
        ));
    }

    #[test]
    fn test_simd_level_comparison() {
        assert!(SimdLevel::Scalar <= SimdLevel::Sse2);
        assert!(SimdLevel::Sse2 <= SimdLevel::Avx2);
        assert!(SimdLevel::Avx2 >= SimdLevel::Sse2);
    }

    #[test]
    fn test_dct_simd_correctness() {
        let input = [
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
            8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
            8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
            8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
            1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0,
            8.0, 7.0, 6.0, 5.0, 4.0, 3.0, 2.0, 1.0,
        ];

        let mut scalar_output = [0.0f32; 64];
        let mut simd_output = [0.0f32; 64];

        crate::dct8x8_forward(&input, &mut scalar_output);
        dct_8x8_simd(&input, &mut simd_output);

        // Should produce identical results (currently both use scalar)
        for i in 0..64 {
            assert_eq!(scalar_output[i], simd_output[i]);
        }
    }

    #[test]
    fn test_idct_simd_correctness() {
        let input = [
            10.0, 2.0, 1.0, 0.5, 0.2, 0.1, 0.05, 0.02,
            2.0, 1.0, 0.5, 0.2, 0.1, 0.05, 0.02, 0.01,
            1.0, 0.5, 0.2, 0.1, 0.05, 0.02, 0.01, 0.0,
            0.5, 0.2, 0.1, 0.05, 0.02, 0.01, 0.0, 0.0,
            0.2, 0.1, 0.05, 0.02, 0.01, 0.0, 0.0, 0.0,
            0.1, 0.05, 0.02, 0.01, 0.0, 0.0, 0.0, 0.0,
            0.05, 0.02, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0,
            0.02, 0.01, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
        ];

        let mut scalar_output = [0.0f32; 64];
        let mut simd_output = [0.0f32; 64];

        crate::dct8x8_inverse(&input, &mut scalar_output);
        idct_8x8_simd(&input, &mut simd_output);

        // Should produce identical results (currently both use scalar)
        for i in 0..64 {
            assert_eq!(scalar_output[i], simd_output[i]);
        }
    }

    #[test]
    fn test_rgb_to_xyb_simd() {
        let rgb = vec![
            1.0, 0.5, 0.2,
            0.8, 0.6, 0.4,
            0.3, 0.7, 0.9,
            0.1, 0.2, 0.3,
        ];
        let mut xyb = vec![0.0; 12];

        rgb_to_xyb_simd(&rgb, &mut xyb, 4);

        // Verify XYB conversion was applied
        for i in 0..4 {
            let r = rgb[i * 3];
            let g = rgb[i * 3 + 1];
            let b = rgb[i * 3 + 2];

            let l = 0.3 * r + 0.3 * g + 0.3 * b;
            let m = 0.622 * r + 0.622 * g + 0.622 * b;
            let s = 0.078 * r + 0.078 * g + 0.078 * b;

            let expected_x = l - m;
            let expected_y = l + m;
            let expected_b = s - m;

            assert!((xyb[i * 3] - expected_x).abs() < 0.001);
            assert!((xyb[i * 3 + 1] - expected_y).abs() < 0.001);
            assert!((xyb[i * 3 + 2] - expected_b).abs() < 0.001);
        }
    }

    #[test]
    fn test_benchmark_simd() {
        let (scalar_time, simd_time, level) = benchmark_simd();
        println!("SIMD level: {}", level.name());
        println!("Scalar time: {:.6}s", scalar_time);
        println!("SIMD time: {:.6}s", simd_time);

        // Both should be positive
        assert!(scalar_time > 0.0);
        assert!(simd_time > 0.0);

        // Currently they should be approximately equal (both use scalar)
        let ratio = scalar_time / simd_time;
        println!("Performance ratio: {:.2}x", ratio);
        assert!((ratio - 1.0).abs() < 0.5, "Ratio should be close to 1.0");
    }
}
