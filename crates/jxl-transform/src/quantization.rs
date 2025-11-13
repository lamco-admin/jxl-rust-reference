//! Quantization for lossy compression
//!
//! Production JPEG XL quantization with XYB-tuned matrices for optimal perceptual quality.

use jxl_core::consts::BLOCK_SIZE;

/// Quantization table for 8x8 blocks
pub type QuantTable = [u16; 64];

/// Channel-specific quantization tables for XYB color space
#[derive(Debug, Clone)]
pub struct XybQuantTables {
    pub x_table: QuantTable,
    pub y_table: QuantTable,
    pub b_table: QuantTable,
}

/// Generate XYB-tuned quantization tables from quality parameter (0-100)
///
/// JPEG XL uses different quantization for each XYB channel because:
/// - Y channel (luma): Most perceptually important, lower quantization
/// - X channel (red-green): Chroma, higher quantization acceptable
/// - B-Y channel (blue-yellow): Chroma, higher quantization acceptable
pub fn generate_xyb_quant_tables(quality: f32) -> XybQuantTables {
    let scale = quality_to_scale(quality);

    // Y channel (luma) - tuned for XYB perceptual encoding
    // Balanced for both quality and compression
    const Y_BASE: [u16; 64] = [
        12, 8, 7, 12, 18, 30, 38, 46,
        8, 8, 10, 14, 20, 44, 45, 42,
        10, 10, 12, 18, 30, 44, 52, 42,
        10, 13, 17, 22, 38, 66, 60, 47,
        14, 17, 28, 42, 51, 82, 78, 58,
        18, 26, 42, 48, 61, 78, 86, 69,
        38, 48, 58, 66, 78, 91, 90, 76,
        54, 69, 72, 74, 84, 75, 78, 75,
    ];

    // X channel (red-green chroma) - can use more aggressive quantization
    const X_BASE: [u16; 64] = [
        16, 12, 10, 16, 24, 40, 51, 61,
        12, 12, 14, 19, 26, 58, 60, 55,
        14, 13, 16, 24, 40, 57, 69, 56,
        14, 17, 22, 29, 51, 87, 80, 62,
        18, 22, 37, 56, 68, 109, 103, 77,
        24, 35, 55, 64, 81, 104, 113, 92,
        49, 64, 78, 87, 103, 121, 120, 101,
        72, 92, 95, 98, 112, 100, 103, 99,
    ];

    // B-Y channel (blue-yellow chroma) - similar to X channel
    const B_BASE: [u16; 64] = [
        16, 12, 10, 16, 24, 40, 51, 61,
        12, 12, 14, 19, 26, 58, 60, 55,
        14, 13, 16, 24, 40, 57, 69, 56,
        14, 17, 22, 29, 51, 87, 80, 62,
        18, 22, 37, 56, 68, 109, 103, 77,
        24, 35, 55, 64, 81, 104, 113, 92,
        49, 64, 78, 87, 103, 121, 120, 101,
        72, 92, 95, 98, 112, 100, 103, 99,
    ];

    let y_table = scale_quant_table(&Y_BASE, scale);
    let x_table = scale_quant_table(&X_BASE, scale);
    let b_table = scale_quant_table(&B_BASE, scale);

    XybQuantTables {
        x_table,
        y_table,
        b_table,
    }
}

/// Convert quality (0-100) to quantization scale factor
fn quality_to_scale(quality: f32) -> f32 {
    let quality = quality.clamp(0.0, 100.0);
    if quality < 50.0 {
        // Low quality: aggressive scaling
        5000.0 / quality.max(1.0)
    } else {
        // High quality: gentler scaling
        200.0 - 2.0 * quality
    }
}

/// Scale a base quantization table by quality factor
fn scale_quant_table(base: &[u16; 64], scale: f32) -> QuantTable {
    let mut table = [0u16; 64];
    for i in 0..64 {
        let q = ((base[i] as f32 * scale / 100.0) + 0.5).max(1.0) as u16;
        table[i] = q.min(255);
    }
    table
}

/// Generate legacy quantization table from quality parameter (0-100)
///
/// This uses a JPEG-style quantization matrix and is kept for backward compatibility.
/// For production XYB encoding, use `generate_xyb_quant_tables` instead.
pub fn generate_quant_table(quality: f32) -> QuantTable {
    let scale = quality_to_scale(quality);

    // Base quantization matrix (similar to JPEG)
    const BASE_QUANT: [u16; 64] = [
        16, 11, 10, 16, 24, 40, 51, 61,
        12, 12, 14, 19, 26, 58, 60, 55,
        14, 13, 16, 24, 40, 57, 69, 56,
        14, 17, 22, 29, 51, 87, 80, 62,
        18, 22, 37, 56, 68, 109, 103, 77,
        24, 35, 55, 64, 81, 104, 113, 92,
        49, 64, 78, 87, 103, 121, 120, 101,
        72, 92, 95, 98, 112, 100, 103, 99,
    ];

    scale_quant_table(&BASE_QUANT, scale)
}

/// Quantize DCT coefficients
pub fn quantize(coeffs: &[f32; 64], quant_table: &QuantTable, output: &mut [i16; 64]) {
    for i in 0..64 {
        let q = quant_table[i] as f32;
        output[i] = (coeffs[i] / q).round() as i16;
    }
}

/// Quantize with adaptive scaling factor
pub fn quantize_adaptive(
    coeffs: &[f32; 64],
    quant_table: &QuantTable,
    scale_factor: f32,
    output: &mut [i16; 64],
) {
    for i in 0..64 {
        let q = (quant_table[i] as f32) * scale_factor;
        output[i] = (coeffs[i] / q).round() as i16;
    }
}

/// Dequantize DCT coefficients
pub fn dequantize(coeffs: &[i16; 64], quant_table: &QuantTable, output: &mut [f32; 64]) {
    for i in 0..64 {
        let q = quant_table[i] as f32;
        output[i] = coeffs[i] as f32 * q;
    }
}

/// Dequantize with adaptive scaling factor
pub fn dequantize_adaptive(
    coeffs: &[i16; 64],
    quant_table: &QuantTable,
    scale_factor: f32,
    output: &mut [f32; 64],
) {
    for i in 0..64 {
        let q = (quant_table[i] as f32) * scale_factor;
        output[i] = coeffs[i] as f32 * q;
    }
}

/// Quantize a channel of DCT coefficients
pub fn quantize_channel(
    dct_coeffs: &[f32],
    width: usize,
    height: usize,
    quant_table: &QuantTable,
    output: &mut Vec<i16>,
) {
    output.clear();
    output.resize(width * height, 0);

    let mut block = [0.0f32; 64];
    let mut quant_block = [0i16; 64];

    for block_y in (0..height).step_by(BLOCK_SIZE) {
        for block_x in (0..width).step_by(BLOCK_SIZE) {
            // Extract block
            for y in 0..BLOCK_SIZE.min(height - block_y) {
                for x in 0..BLOCK_SIZE.min(width - block_x) {
                    block[y * BLOCK_SIZE + x] = dct_coeffs[(block_y + y) * width + (block_x + x)];
                }
            }

            // Quantize
            quantize(&block, quant_table, &mut quant_block);

            // Store
            for y in 0..BLOCK_SIZE.min(height - block_y) {
                for x in 0..BLOCK_SIZE.min(width - block_x) {
                    output[(block_y + y) * width + (block_x + x)] = quant_block[y * BLOCK_SIZE + x];
                }
            }
        }
    }
}

/// Quantize a channel with adaptive per-block scaling
pub fn quantize_channel_adaptive(
    dct_coeffs: &[f32],
    width: usize,
    height: usize,
    quant_table: &QuantTable,
    scale_map: &[f32],
    output: &mut Vec<i16>,
) {
    output.clear();
    output.resize(width * height, 0);

    let mut block = [0.0f32; 64];
    let mut quant_block = [0i16; 64];

    let blocks_x = (width + BLOCK_SIZE - 1) / BLOCK_SIZE;

    for block_idx_y in 0..(height + BLOCK_SIZE - 1) / BLOCK_SIZE {
        for block_idx_x in 0..blocks_x {
            let block_y = block_idx_y * BLOCK_SIZE;
            let block_x = block_idx_x * BLOCK_SIZE;

            // Get adaptive scale for this block
            let block_idx = block_idx_y * blocks_x + block_idx_x;
            let scale = scale_map[block_idx];

            // Extract block
            for y in 0..BLOCK_SIZE.min(height - block_y) {
                for x in 0..BLOCK_SIZE.min(width - block_x) {
                    block[y * BLOCK_SIZE + x] = dct_coeffs[(block_y + y) * width + (block_x + x)];
                }
            }

            // Quantize with adaptive scaling
            quantize_adaptive(&block, quant_table, scale, &mut quant_block);

            // Store
            for y in 0..BLOCK_SIZE.min(height - block_y) {
                for x in 0..BLOCK_SIZE.min(width - block_x) {
                    output[(block_y + y) * width + (block_x + x)] = quant_block[y * BLOCK_SIZE + x];
                }
            }
        }
    }
}

/// Compute block complexity (AC energy) for adaptive quantization
///
/// Returns the RMS of AC coefficients, which indicates block detail/complexity.
/// Higher values = more detail = needs finer quantization.
pub fn compute_block_complexity(dct_block: &[f32; 64]) -> f32 {
    // Sum of squared AC coefficients (skip DC at index 0)
    let mut sum_sq = 0.0;
    for i in 1..64 {
        sum_sq += dct_block[i] * dct_block[i];
    }

    // RMS of AC coefficients
    (sum_sq / 63.0).sqrt()
}

/// Generate adaptive quantization scale map for all blocks
///
/// This implements perceptual adaptive quantization:
/// - Complex blocks (high AC energy) get finer quantization (scale < 1.0)
/// - Flat blocks (low AC energy) get coarser quantization (scale > 1.0)
/// - Average scaling is normalized to maintain target compression
///
/// # Arguments
/// * `dct_coeffs` - DCT coefficients for the entire channel
/// * `width` - Image width in pixels (must be multiple of BLOCK_SIZE)
/// * `height` - Image height in pixels (must be multiple of BLOCK_SIZE)
/// * `strength` - Adaptation strength (0.0 = no adaptation, 1.0 = full adaptation)
///
/// # Returns
/// Vector of scale factors, one per block, in raster order
pub fn generate_adaptive_quant_map(
    dct_coeffs: &[f32],
    width: usize,
    height: usize,
    strength: f32,
) -> Vec<f32> {
    let blocks_x = (width + BLOCK_SIZE - 1) / BLOCK_SIZE;
    let blocks_y = (height + BLOCK_SIZE - 1) / BLOCK_SIZE;
    let num_blocks = blocks_x * blocks_y;

    let mut complexities = Vec::with_capacity(num_blocks);
    let mut block = [0.0f32; 64];

    // Compute complexity for each block
    for block_idx_y in 0..blocks_y {
        for block_idx_x in 0..blocks_x {
            let block_y = block_idx_y * BLOCK_SIZE;
            let block_x = block_idx_x * BLOCK_SIZE;

            // Extract block
            for y in 0..BLOCK_SIZE {
                for x in 0..BLOCK_SIZE {
                    if block_y + y < height && block_x + x < width {
                        block[y * BLOCK_SIZE + x] = dct_coeffs[(block_y + y) * width + (block_x + x)];
                    } else {
                        block[y * BLOCK_SIZE + x] = 0.0;
                    }
                }
            }

            let complexity = compute_block_complexity(&block);
            complexities.push(complexity);
        }
    }

    // Compute statistics for normalization
    let mean_complexity: f32 = complexities.iter().sum::<f32>() / num_blocks as f32;
    let mean_complexity = mean_complexity.max(1.0); // Avoid division by zero

    // Generate scale factors with perceptual weighting
    let mut scales = Vec::with_capacity(num_blocks);
    for &complexity in &complexities {
        // Relative complexity (1.0 = average complexity)
        let rel_complexity = complexity / mean_complexity;

        // Adaptive scaling:
        // - High complexity (rel > 1.0): scale < 1.0 (finer quantization)
        // - Low complexity (rel < 1.0): scale > 1.0 (coarser quantization)
        // Using power function for smooth perceptual adaptation
        let base_scale = if rel_complexity > 0.0 {
            rel_complexity.powf(-0.5) // Square root inverse
        } else {
            1.0
        };

        // Blend with strength parameter (0.0 = no adaptation, 1.0 = full adaptation)
        let scale = 1.0 + strength * (base_scale - 1.0);

        // Clamp to reasonable range [0.5, 2.0]
        let scale = scale.clamp(0.5, 2.0);

        scales.push(scale);
    }

    // Normalize scales to maintain average = 1.0 (preserves target bitrate)
    let mean_scale: f32 = scales.iter().sum::<f32>() / num_blocks as f32;
    if mean_scale > 0.0 {
        for scale in &mut scales {
            *scale /= mean_scale;
        }
    }

    scales
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_complexity_flat() {
        // Flat block (only DC)
        let mut block = [0.0f32; 64];
        block[0] = 100.0; // DC coefficient

        let complexity = compute_block_complexity(&block);
        assert!(complexity < 0.01); // Should be near zero
    }

    #[test]
    fn test_block_complexity_detailed() {
        // Detailed block (strong AC coefficients)
        let mut block = [0.0f32; 64];
        block[0] = 100.0; // DC
        for i in 1..64 {
            block[i] = 10.0; // Strong AC
        }

        let complexity = compute_block_complexity(&block);
        assert!(complexity > 5.0); // Should be significant
    }

    #[test]
    fn test_adaptive_quant_map_normalization() {
        // Create dummy DCT coefficients (4x4 blocks = 32x32 pixels)
        let width = 32;
        let height = 32;
        let dct_coeffs = vec![1.0f32; width * height];

        let scales = generate_adaptive_quant_map(&dct_coeffs, width, height, 1.0);

        // Should have one scale per block
        let blocks_x = width / BLOCK_SIZE;
        let blocks_y = height / BLOCK_SIZE;
        assert_eq!(scales.len(), blocks_x * blocks_y);

        // Mean scale should be approximately 1.0 (normalized)
        let mean_scale: f32 = scales.iter().sum::<f32>() / scales.len() as f32;
        assert!((mean_scale - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_adaptive_quant_strength_zero() {
        // With strength = 0, all scales should be 1.0 (no adaptation)
        let width = 16;
        let height = 16;
        let dct_coeffs = vec![1.0f32; width * height];

        let scales = generate_adaptive_quant_map(&dct_coeffs, width, height, 0.0);

        for scale in scales {
            assert!((scale - 1.0).abs() < 0.01);
        }
    }
}
