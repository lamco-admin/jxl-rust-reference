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

/// Dequantize DCT coefficients
pub fn dequantize(coeffs: &[i16; 64], quant_table: &QuantTable, output: &mut [f32; 64]) {
    for i in 0..64 {
        let q = quant_table[i] as f32;
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
