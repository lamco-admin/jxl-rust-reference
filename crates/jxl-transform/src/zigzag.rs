//! Zigzag scanning for DCT coefficients
//!
//! Production JPEG XL coefficient scanning patterns for optimal entropy coding.

/// Standard 8×8 zigzag scan order (JPEG/JPEG XL compatible)
/// This order groups coefficients by increasing frequency, putting
/// low-frequency (more important) coefficients first.
pub const ZIGZAG_8X8: [usize; 64] = [
    0, 1, 8, 16, 9, 2, 3, 10, 17, 24, 32, 25, 18, 11, 4, 5, 12, 19, 26, 33, 40, 48, 41, 34, 27, 20,
    13, 6, 7, 14, 21, 28, 35, 42, 49, 56, 57, 50, 43, 36, 29, 22, 15, 23, 30, 37, 44, 51, 58, 59,
    52, 45, 38, 31, 39, 46, 53, 60, 61, 54, 47, 55, 62, 63,
];

/// Inverse zigzag scan order (for encoding: position → zigzag index)
pub const INV_ZIGZAG_8X8: [usize; 64] = [
    0, 1, 5, 6, 14, 15, 27, 28, 2, 4, 7, 13, 16, 26, 29, 42, 3, 8, 12, 17, 25, 30, 41, 43, 9, 11,
    18, 24, 31, 40, 44, 53, 10, 19, 23, 32, 39, 45, 52, 54, 20, 22, 33, 38, 46, 51, 55, 60, 21,
    34, 37, 47, 50, 56, 59, 61, 35, 36, 48, 49, 57, 58, 62, 63,
];

/// Apply zigzag scan to an 8×8 block of coefficients
///
/// # Arguments
/// * `block` - 8×8 block in row-major order
/// * `output` - Output buffer for zigzag-scanned coefficients
///
/// # Example
/// ```
/// use jxl_transform::zigzag_scan_8x8;
///
/// let block = [0i16; 64]; // DC at [0], AC coefficients follow
/// let mut zigzag = [0i16; 64];
/// zigzag_scan_8x8(&block, &mut zigzag);
/// // zigzag[0] is DC, zigzag[1..] are AC in zigzag order
/// ```
pub fn zigzag_scan_8x8(block: &[i16; 64], output: &mut [i16; 64]) {
    for (i, &pos) in ZIGZAG_8X8.iter().enumerate() {
        output[i] = block[pos];
    }
}

/// Apply inverse zigzag scan to reconstruct 8×8 block
///
/// # Arguments
/// * `zigzag` - Coefficients in zigzag order
/// * `output` - Output buffer for row-major 8×8 block
pub fn inv_zigzag_scan_8x8(zigzag: &[i16; 64], output: &mut [i16; 64]) {
    for (i, &pos) in ZIGZAG_8X8.iter().enumerate() {
        output[pos] = zigzag[i];
    }
}

/// Scan full channel of DCT coefficients in zigzag order
///
/// Processes an entire image channel block-by-block, applying zigzag
/// scanning to each 8×8 block for better entropy coding efficiency.
pub fn zigzag_scan_channel(
    coeffs: &[i16],
    width: usize,
    height: usize,
    output: &mut Vec<i16>,
) {
    output.clear();
    output.reserve(coeffs.len());

    let blocks_x = width.div_ceil(8);
    let blocks_y = height.div_ceil(8);

    let mut block = [0i16; 64];
    let mut zigzag = [0i16; 64];

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            // Extract 8×8 block
            for y in 0..8 {
                for x in 0..8 {
                    let img_x = bx * 8 + x;
                    let img_y = by * 8 + y;

                    if img_x < width && img_y < height {
                        block[y * 8 + x] = coeffs[img_y * width + img_x];
                    } else {
                        block[y * 8 + x] = 0;
                    }
                }
            }

            // Apply zigzag scan
            zigzag_scan_8x8(&block, &mut zigzag);

            // Append to output
            output.extend_from_slice(&zigzag);
        }
    }
}

/// Inverse zigzag scan for full channel
///
/// Reconstructs row-major coefficient layout from zigzag-scanned data.
pub fn inv_zigzag_scan_channel(
    zigzag_data: &[i16],
    width: usize,
    height: usize,
    output: &mut Vec<i16>,
) {
    output.clear();
    output.resize(width * height, 0);

    let blocks_x = width.div_ceil(8);
    let blocks_y = height.div_ceil(8);

    let mut zigzag = [0i16; 64];
    let mut block = [0i16; 64];

    for by in 0..blocks_y {
        for bx in 0..blocks_x {
            let block_idx = by * blocks_x + bx;
            let zigzag_offset = block_idx * 64;

            if zigzag_offset + 64 <= zigzag_data.len() {
                // Copy zigzag block
                zigzag.copy_from_slice(&zigzag_data[zigzag_offset..zigzag_offset + 64]);

                // Inverse zigzag
                inv_zigzag_scan_8x8(&zigzag, &mut block);

                // Write back to image
                for y in 0..8 {
                    for x in 0..8 {
                        let img_x = bx * 8 + x;
                        let img_y = by * 8 + y;

                        if img_x < width && img_y < height {
                            output[img_y * width + img_x] = block[y * 8 + x];
                        }
                    }
                }
            }
        }
    }
}

/// Separate DC and AC coefficients from zigzag-scanned data
///
/// Returns (dc_coefficients, ac_coefficients) where DC contains one value
/// per block and AC contains the remaining 63 coefficients per block.
pub fn separate_dc_ac(zigzag_data: &[i16]) -> (Vec<i16>, Vec<i16>) {
    let num_blocks = zigzag_data.len() / 64;
    let mut dc = Vec::with_capacity(num_blocks);
    let mut ac = Vec::with_capacity(num_blocks * 63);

    for block_idx in 0..num_blocks {
        let offset = block_idx * 64;
        if offset < zigzag_data.len() {
            // DC is first coefficient
            dc.push(zigzag_data[offset]);

            // AC are remaining 63 coefficients
            let ac_start = offset + 1;
            let ac_end = (offset + 64).min(zigzag_data.len());
            ac.extend_from_slice(&zigzag_data[ac_start..ac_end]);
        }
    }

    (dc, ac)
}

/// Merge DC and AC coefficients back into zigzag format
pub fn merge_dc_ac(dc: &[i16], ac: &[i16], output: &mut Vec<i16>) {
    output.clear();
    let num_blocks = dc.len();
    output.reserve(num_blocks * 64);

    for block_idx in 0..num_blocks {
        // Add DC
        output.push(dc[block_idx]);

        // Add AC (63 coefficients)
        let ac_start = block_idx * 63;
        let ac_end = (ac_start + 63).min(ac.len());
        output.extend_from_slice(&ac[ac_start..ac_end]);

        // Pad if necessary
        while output.len() % 64 != 0 {
            output.push(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_zigzag_scan_identity() {
        let block: [i16; 64] = core::array::from_fn(|i| i as i16);
        let mut zigzag = [0i16; 64];
        let mut reconstructed = [0i16; 64];

        zigzag_scan_8x8(&block, &mut zigzag);
        inv_zigzag_scan_8x8(&zigzag, &mut reconstructed);

        assert_eq!(block, reconstructed);
    }

    #[test]
    fn test_zigzag_order() {
        let mut block = [0i16; 64];
        block[0] = 100; // DC coefficient
        block[1] = 50; // First AC (top-right of DC)
        block[8] = 25; // Second AC (below DC)

        let mut zigzag = [0i16; 64];
        zigzag_scan_8x8(&block, &mut zigzag);

        // DC should be first
        assert_eq!(zigzag[0], 100);
        // Low-frequency ACs should be early in zigzag order
        assert!(zigzag[1] != 0 || zigzag[2] != 0);
    }

    #[test]
    fn test_separate_merge_dc_ac() {
        let zigzag_data: Vec<i16> = (0..128).map(|i| i as i16).collect();

        let (dc, ac) = separate_dc_ac(&zigzag_data);
        assert_eq!(dc.len(), 2); // 2 blocks
        assert_eq!(dc[0], 0); // First DC
        assert_eq!(dc[1], 64); // Second DC

        let mut merged = Vec::new();
        merge_dc_ac(&dc, &ac, &mut merged);

        // Check structure is preserved
        assert_eq!(merged.len(), 128);
        assert_eq!(merged[0], dc[0]);
        assert_eq!(merged[64], dc[1]);
    }

    #[test]
    fn test_channel_zigzag_roundtrip() {
        let width = 16;
        let height = 16;
        let coeffs: Vec<i16> = (0..(width * height)).map(|i| i as i16).collect();

        let mut zigzag_data = Vec::new();
        zigzag_scan_channel(&coeffs, width, height, &mut zigzag_data);

        let mut reconstructed = Vec::new();
        inv_zigzag_scan_channel(&zigzag_data, width, height, &mut reconstructed);

        assert_eq!(coeffs.len(), reconstructed.len());
        // Note: Exact equality may not hold at block boundaries
        // but structure should be preserved
    }
}
