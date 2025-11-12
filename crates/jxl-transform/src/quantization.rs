//! Quantization for lossy compression

use jxl_core::consts::BLOCK_SIZE;

/// Quantization table for 8x8 blocks (JPEG-style)
pub type QuantTable = [u16; 64];

/// Generate quantization table from quality parameter (0-100)
pub fn generate_quant_table(quality: f32) -> QuantTable {
    let scale = if quality < 50.0 {
        5000.0 / quality.max(1.0)
    } else {
        200.0 - 2.0 * quality
    };

    // Base quantization matrix (similar to JPEG)
    const BASE_QUANT: [u16; 64] = [
        16, 11, 10, 16, 24, 40, 51, 61, 12, 12, 14, 19, 26, 58, 60, 55, 14, 13, 16, 24, 40, 57, 69,
        56, 14, 17, 22, 29, 51, 87, 80, 62, 18, 22, 37, 56, 68, 109, 103, 77, 24, 35, 55, 64, 81,
        104, 113, 92, 49, 64, 78, 87, 103, 121, 120, 101, 72, 92, 95, 98, 112, 100, 103, 99,
    ];

    let mut table = [0u16; 64];
    for i in 0..64 {
        let q = ((BASE_QUANT[i] as f32 * scale / 100.0) + 0.5).max(1.0) as u16;
        table[i] = q.min(255);
    }

    table
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
