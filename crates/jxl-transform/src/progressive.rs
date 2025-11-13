//! Progressive Decoding Support
//!
//! Progressive decoding allows JPEG XL images to be decoded in multiple passes,
//! providing increasingly higher quality previews. This is essential for:
//! - Fast preview generation (DC-first rendering)
//! - Responsive user experience (progressive loading)
//! - Bandwidth-constrained scenarios (incremental refinement)
//!
//! JPEG XL progressive decoding uses:
//! - DC pass: 8×8 downsampled image (1/64 coefficients)
//! - AC passes: Progressive refinement with additional coefficients

use jxl_core::{JxlError, JxlResult};

/// Block size for DCT transforms
const BLOCK_SIZE: usize = 8;

/// Progressive pass configuration
#[derive(Debug, Clone)]
pub struct ProgressivePass {
    /// Pass index (0 = DC only)
    pub pass_index: usize,
    /// Downsampling factor (1 = full resolution)
    pub downsample: usize,
    /// Number of DCT coefficients to use (1-64)
    pub num_coefficients: usize,
    /// Description of this pass
    pub description: String,
}

impl ProgressivePass {
    /// DC-only pass (fastest preview, 1/64 data)
    pub fn dc_only() -> Self {
        Self {
            pass_index: 0,
            downsample: 8,
            num_coefficients: 1,
            description: "DC only (8×8 downsampled)".to_string(),
        }
    }

    /// Low frequency pass (8 coefficients, basic structure)
    pub fn low_frequency() -> Self {
        Self {
            pass_index: 1,
            downsample: 1,
            num_coefficients: 8,
            description: "Low frequency (8 coefficients)".to_string(),
        }
    }

    /// Medium frequency pass (21 coefficients, more detail)
    pub fn medium_frequency() -> Self {
        Self {
            pass_index: 2,
            downsample: 1,
            num_coefficients: 21,
            description: "Medium frequency (21 coefficients)".to_string(),
        }
    }

    /// Full quality pass (all 64 coefficients)
    pub fn full_quality() -> Self {
        Self {
            pass_index: 3,
            downsample: 1,
            num_coefficients: 64,
            description: "Full quality (64 coefficients)".to_string(),
        }
    }

    /// Get standard progressive pass sequence
    pub fn standard_sequence() -> Vec<ProgressivePass> {
        vec![
            Self::dc_only(),
            Self::low_frequency(),
            Self::medium_frequency(),
            Self::full_quality(),
        ]
    }
}

/// Progressive decoder state
#[derive(Debug, Clone)]
pub struct ProgressiveDecoder {
    /// Image dimensions
    pub width: usize,
    pub height: usize,
    /// Number of channels
    pub num_channels: usize,
    /// Current pass index
    pub current_pass: usize,
    /// Accumulated DCT coefficients (progressive refinement)
    pub coefficients: Vec<Vec<f32>>,
}

impl ProgressiveDecoder {
    /// Create a new progressive decoder
    pub fn new(width: usize, height: usize, num_channels: usize) -> Self {
        let pixel_count = width * height;
        let mut coefficients = Vec::with_capacity(num_channels);

        for _ in 0..num_channels {
            coefficients.push(vec![0.0f32; pixel_count]);
        }

        Self {
            width,
            height,
            num_channels,
            current_pass: 0,
            coefficients,
        }
    }

    /// Add DC coefficients (pass 0)
    ///
    /// DC coefficients provide an 8×8 downsampled preview.
    /// This is the fastest way to get an initial image representation.
    pub fn add_dc_pass(&mut self, dc_coeffs: &[Vec<f32>]) -> JxlResult<()> {
        if dc_coeffs.len() != self.num_channels {
            return Err(JxlError::InvalidParameter(
                "DC coefficients channel count mismatch".to_string(),
            ));
        }

        let blocks_x = (self.width + BLOCK_SIZE - 1) / BLOCK_SIZE;
        let blocks_y = (self.height + BLOCK_SIZE - 1) / BLOCK_SIZE;

        for c in 0..self.num_channels {
            if dc_coeffs[c].len() != blocks_x * blocks_y {
                return Err(JxlError::InvalidParameter(
                    "DC coefficients size mismatch".to_string(),
                ));
            }

            // Spread DC values across each 8×8 block
            for block_y in 0..blocks_y {
                for block_x in 0..blocks_x {
                    let dc_value = dc_coeffs[c][block_y * blocks_x + block_x];

                    // Fill the entire block with DC value (coefficient 0)
                    for y in 0..BLOCK_SIZE {
                        for x in 0..BLOCK_SIZE {
                            let pixel_y = block_y * BLOCK_SIZE + y;
                            let pixel_x = block_x * BLOCK_SIZE + x;

                            if pixel_y < self.height && pixel_x < self.width {
                                let idx = pixel_y * self.width + pixel_x;
                                // DC is at position 0 in zigzag order
                                self.coefficients[c][idx] = if x == 0 && y == 0 {
                                    dc_value
                                } else {
                                    0.0
                                };
                            }
                        }
                    }
                }
            }
        }

        self.current_pass = 1;
        Ok(())
    }

    /// Add AC coefficients for progressive refinement
    ///
    /// AC coefficients progressively add detail to the image.
    /// Multiple AC passes can be applied for gradual quality improvement.
    pub fn add_ac_pass(
        &mut self,
        ac_coeffs: &[Vec<f32>],
        num_coefficients: usize,
    ) -> JxlResult<()> {
        if ac_coeffs.len() != self.num_channels {
            return Err(JxlError::InvalidParameter(
                "AC coefficients channel count mismatch".to_string(),
            ));
        }

        if num_coefficients > 64 {
            return Err(JxlError::InvalidParameter(
                "num_coefficients must be <= 64".to_string(),
            ));
        }

        // Merge AC coefficients with existing ones
        for c in 0..self.num_channels {
            if ac_coeffs[c].len() != self.width * self.height {
                return Err(JxlError::InvalidParameter(
                    "AC coefficients size mismatch".to_string(),
                ));
            }

            // Add AC coefficients (progressive refinement)
            for i in 0..(self.width * self.height) {
                self.coefficients[c][i] = ac_coeffs[c][i];
            }
        }

        self.current_pass += 1;
        Ok(())
    }

    /// Get current image quality (0.0-1.0)
    pub fn get_quality(&self) -> f32 {
        match self.current_pass {
            0 => 0.0,    // No data yet
            1 => 0.25,   // DC only
            2 => 0.5,    // Low frequency
            3 => 0.75,   // Medium frequency
            _ => 1.0,    // Full quality
        }
    }

    /// Check if decoding is complete
    pub fn is_complete(&self) -> bool {
        self.current_pass >= 4 // All passes received
    }
}

/// Extract DC coefficients from full DCT coefficients
///
/// DC coefficients are the (0,0) coefficient of each 8×8 block,
/// representing the average value of the block.
pub fn extract_dc_coefficients(
    dct_coeffs: &[f32],
    width: usize,
    height: usize,
) -> Vec<f32> {
    let blocks_x = (width + BLOCK_SIZE - 1) / BLOCK_SIZE;
    let blocks_y = (height + BLOCK_SIZE - 1) / BLOCK_SIZE;
    let mut dc_coeffs = vec![0.0f32; blocks_x * blocks_y];

    for block_y in 0..blocks_y {
        for block_x in 0..blocks_x {
            let block_start_y = block_y * BLOCK_SIZE;
            let block_start_x = block_x * BLOCK_SIZE;

            // DC coefficient is at (0, 0) of each block
            if block_start_y < height && block_start_x < width {
                let idx = block_start_y * width + block_start_x;
                dc_coeffs[block_y * blocks_x + block_x] = dct_coeffs[idx];
            }
        }
    }

    dc_coeffs
}

/// Generate DC-only preview image
///
/// Creates an 8×8 downsampled image from DC coefficients only.
/// This is extremely fast and provides an initial preview.
pub fn generate_dc_preview(dc_coeffs: &[Vec<f32>], width: usize, height: usize) -> Vec<Vec<f32>> {
    let blocks_x = (width + BLOCK_SIZE - 1) / BLOCK_SIZE;
    let blocks_y = (height + BLOCK_SIZE - 1) / BLOCK_SIZE;
    let preview_width = blocks_x;
    let preview_height = blocks_y;

    let mut preview = Vec::with_capacity(dc_coeffs.len());

    for c in 0..dc_coeffs.len() {
        let mut channel = vec![0.0f32; preview_width * preview_height];

        for block_y in 0..blocks_y {
            for block_x in 0..blocks_x {
                let dc_value = dc_coeffs[c][block_y * blocks_x + block_x];
                channel[block_y * preview_width + block_x] = dc_value;
            }
        }

        preview.push(channel);
    }

    preview
}

/// Upsample DC preview to full resolution
///
/// Simple nearest-neighbor upsampling of DC preview.
/// Each DC value is replicated 8×8 times.
pub fn upsample_dc_preview(
    dc_preview: &[Vec<f32>],
    target_width: usize,
    target_height: usize,
) -> Vec<Vec<f32>> {
    let preview_width = dc_preview[0].len() / ((target_height + BLOCK_SIZE - 1) / BLOCK_SIZE);
    let preview_height = (target_height + BLOCK_SIZE - 1) / BLOCK_SIZE;

    let mut upsampled = Vec::with_capacity(dc_preview.len());

    for c in 0..dc_preview.len() {
        let mut channel = vec![0.0f32; target_width * target_height];

        for y in 0..target_height {
            for x in 0..target_width {
                let block_x = x / BLOCK_SIZE;
                let block_y = y / BLOCK_SIZE;

                if block_y < preview_height && block_x < preview_width {
                    let dc_value = dc_preview[c][block_y * preview_width + block_x];
                    channel[y * target_width + x] = dc_value;
                }
            }
        }

        upsampled.push(channel);
    }

    upsampled
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progressive_pass_creation() {
        let pass = ProgressivePass::dc_only();
        assert_eq!(pass.pass_index, 0);
        assert_eq!(pass.downsample, 8);
        assert_eq!(pass.num_coefficients, 1);
    }

    #[test]
    fn test_standard_sequence() {
        let sequence = ProgressivePass::standard_sequence();
        assert_eq!(sequence.len(), 4);
        assert_eq!(sequence[0].num_coefficients, 1);  // DC only
        assert_eq!(sequence[1].num_coefficients, 8);  // Low freq
        assert_eq!(sequence[2].num_coefficients, 21); // Medium freq
        assert_eq!(sequence[3].num_coefficients, 64); // Full
    }

    #[test]
    fn test_progressive_decoder_creation() {
        let decoder = ProgressiveDecoder::new(64, 64, 3);
        assert_eq!(decoder.width, 64);
        assert_eq!(decoder.height, 64);
        assert_eq!(decoder.num_channels, 3);
        assert_eq!(decoder.current_pass, 0);
    }

    #[test]
    fn test_dc_extraction() {
        let width = 16;
        let height = 16;
        let mut dct_coeffs = vec![0.0f32; width * height];

        // Set DC values (top-left corner of each block)
        for block_y in 0..2 {
            for block_x in 0..2 {
                let y = block_y * 8;
                let x = block_x * 8;
                dct_coeffs[y * width + x] = ((block_y * 2 + block_x) * 10) as f32;
            }
        }

        let dc_coeffs = extract_dc_coefficients(&dct_coeffs, width, height);
        assert_eq!(dc_coeffs.len(), 4); // 2×2 blocks
        assert_eq!(dc_coeffs[0], 0.0);
        assert_eq!(dc_coeffs[1], 10.0);
        assert_eq!(dc_coeffs[2], 20.0);
        assert_eq!(dc_coeffs[3], 30.0);
    }

    #[test]
    fn test_dc_preview_generation() {
        let width = 16;
        let height = 16;
        let dc_coeffs = vec![
            vec![1.0, 2.0, 3.0, 4.0], // Channel 0
            vec![5.0, 6.0, 7.0, 8.0], // Channel 1
        ];

        let preview = generate_dc_preview(&dc_coeffs, width, height);
        assert_eq!(preview.len(), 2);
        assert_eq!(preview[0].len(), 4); // 2×2 blocks
        assert_eq!(preview[0][0], 1.0);
        assert_eq!(preview[1][3], 8.0);
    }

    #[test]
    fn test_dc_upsampling() {
        let dc_preview = vec![vec![10.0, 20.0, 30.0, 40.0]]; // 2×2 blocks
        let upsampled = upsample_dc_preview(&dc_preview, 16, 16);

        assert_eq!(upsampled[0].len(), 16 * 16);

        // Check that DC values are replicated 8×8
        assert_eq!(upsampled[0][0], 10.0);  // Block (0,0)
        assert_eq!(upsampled[0][8], 20.0);  // Block (1,0)
        assert_eq!(upsampled[0][128], 30.0); // Block (0,1)
    }

    #[test]
    fn test_progressive_quality_tracking() {
        let mut decoder = ProgressiveDecoder::new(64, 64, 3);
        assert_eq!(decoder.get_quality(), 0.0);

        let dc_coeffs = vec![vec![0.0; 64]; 3];
        decoder.add_dc_pass(&dc_coeffs).unwrap();
        assert_eq!(decoder.get_quality(), 0.25);

        assert!(!decoder.is_complete());
    }
}
