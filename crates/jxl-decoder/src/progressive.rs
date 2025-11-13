//! Progressive decoding support for JPEG XL
//!
//! Allows decoding images in multiple passes:
//! 1. DC-only pass: Low-resolution preview (1/8 resolution)
//! 2. Progressive AC passes: Gradually refine image quality
//! 3. Full quality: Complete image reconstruction

use jxl_core::{Dimensions, JxlError, JxlResult};
use jxl_transform::BLOCK_SIZE;

/// Progressive decoding pass level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ProgressivePass {
    /// DC coefficients only (1/8 resolution preview)
    DcOnly,
    /// First AC pass (low frequencies)
    AcPass1,
    /// Second AC pass (medium frequencies)
    AcPass2,
    /// Third AC pass (high frequencies)
    AcPass3,
    /// Full quality (all coefficients)
    Full,
}

impl ProgressivePass {
    /// Get the next pass level
    pub fn next(&self) -> Option<Self> {
        match self {
            ProgressivePass::DcOnly => Some(ProgressivePass::AcPass1),
            ProgressivePass::AcPass1 => Some(ProgressivePass::AcPass2),
            ProgressivePass::AcPass2 => Some(ProgressivePass::AcPass3),
            ProgressivePass::AcPass3 => Some(ProgressivePass::Full),
            ProgressivePass::Full => None,
        }
    }

    /// Get the number of AC coefficients available at this pass
    pub fn ac_coefficient_count(&self) -> usize {
        match self {
            ProgressivePass::DcOnly => 0,
            ProgressivePass::AcPass1 => 15, // First 15 AC coefficients
            ProgressivePass::AcPass2 => 31, // First 31 AC coefficients
            ProgressivePass::AcPass3 => 47, // First 47 AC coefficients
            ProgressivePass::Full => 63,    // All 63 AC coefficients
        }
    }

    /// Get approximate quality percentage
    pub fn quality_percentage(&self) -> u8 {
        match self {
            ProgressivePass::DcOnly => 20,
            ProgressivePass::AcPass1 => 40,
            ProgressivePass::AcPass2 => 60,
            ProgressivePass::AcPass3 => 80,
            ProgressivePass::Full => 100,
        }
    }
}

/// Progressive decoder state
#[derive(Debug, Clone)]
pub struct ProgressiveDecoder {
    /// Current pass
    pub current_pass: ProgressivePass,
    /// Image dimensions
    pub dimensions: Dimensions,
    /// DC coefficients (stored separately for progressive decode)
    pub dc_coefficients: Vec<Vec<f32>>,
    /// AC coefficients (accumulated across passes)
    pub ac_coefficients: Vec<Vec<f32>>,
    /// Number of channels
    pub num_channels: usize,
}

impl ProgressiveDecoder {
    /// Create a new progressive decoder
    pub fn new(dimensions: Dimensions, num_channels: usize) -> Self {
        let width = dimensions.width as usize;
        let height = dimensions.height as usize;

        // Calculate DC image size (downsampled by 8x8)
        let dc_width = width.div_ceil(BLOCK_SIZE);
        let dc_height = height.div_ceil(BLOCK_SIZE);
        let dc_size = dc_width * dc_height;

        // Full AC size
        let ac_size = width * height;

        Self {
            current_pass: ProgressivePass::DcOnly,
            dimensions,
            dc_coefficients: vec![vec![0.0; dc_size]; num_channels],
            ac_coefficients: vec![vec![0.0; ac_size]; num_channels],
            num_channels,
        }
    }

    /// Decode DC coefficients for preview
    pub fn decode_dc_pass(&mut self, dc_data: &[Vec<f32>]) -> JxlResult<()> {
        if dc_data.len() != self.num_channels {
            return Err(JxlError::InvalidParameter(format!(
                "Expected {} channels, got {}",
                self.num_channels,
                dc_data.len()
            )));
        }

        for (i, channel_dc) in dc_data.iter().enumerate() {
            if channel_dc.len() != self.dc_coefficients[i].len() {
                return Err(JxlError::InvalidParameter(
                    "DC coefficient count mismatch".to_string(),
                ));
            }
            self.dc_coefficients[i].copy_from_slice(channel_dc);
        }

        self.current_pass = ProgressivePass::DcOnly;
        Ok(())
    }

    /// Decode AC coefficients for a progressive pass
    pub fn decode_ac_pass(
        &mut self,
        ac_data: &[Vec<f32>],
        pass: ProgressivePass,
    ) -> JxlResult<()> {
        if pass == ProgressivePass::DcOnly {
            return Err(JxlError::InvalidParameter(
                "Use decode_dc_pass for DC-only pass".to_string(),
            ));
        }

        if ac_data.len() != self.num_channels {
            return Err(JxlError::InvalidParameter(format!(
                "Expected {} channels, got {}",
                self.num_channels,
                ac_data.len()
            )));
        }

        // Accumulate AC coefficients
        for (i, channel_ac) in ac_data.iter().enumerate() {
            if channel_ac.len() != self.ac_coefficients[i].len() {
                return Err(JxlError::InvalidParameter(
                    "AC coefficient count mismatch".to_string(),
                ));
            }

            // Add new AC coefficients to existing ones
            for (j, &coeff) in channel_ac.iter().enumerate() {
                self.ac_coefficients[i][j] += coeff;
            }
        }

        self.current_pass = pass;
        Ok(())
    }

    /// Get DC-only preview image (1/8 resolution)
    pub fn get_dc_preview(&self) -> Vec<Vec<f32>> {
        self.dc_coefficients.clone()
    }

    /// Reconstruct image at current quality level
    pub fn reconstruct_image(&self) -> Vec<Vec<f32>> {
        let width = self.dimensions.width as usize;
        let height = self.dimensions.height as usize;
        let blocks_x = width.div_ceil(BLOCK_SIZE);
        let blocks_y = height.div_ceil(BLOCK_SIZE);

        let mut reconstructed = vec![vec![0.0; width * height]; self.num_channels];

        for channel in 0..self.num_channels {
            for block_y in 0..blocks_y {
                for block_x in 0..blocks_x {
                    let dc_idx = block_y * blocks_x + block_x;
                    let dc = self.dc_coefficients[channel][dc_idx];

                    // Reconstruct block
                    for y in 0..BLOCK_SIZE.min(height - block_y * BLOCK_SIZE) {
                        for x in 0..BLOCK_SIZE.min(width - block_x * BLOCK_SIZE) {
                            let pixel_idx = (block_y * BLOCK_SIZE + y) * width
                                + (block_x * BLOCK_SIZE + x);

                            // Start with DC value
                            let mut value = dc;

                            // Add AC contribution if available
                            if self.current_pass != ProgressivePass::DcOnly {
                                value += self.ac_coefficients[channel][pixel_idx];
                            }

                            reconstructed[channel][pixel_idx] = value;
                        }
                    }
                }
            }
        }

        reconstructed
    }

    /// Get current pass
    pub fn current_pass(&self) -> ProgressivePass {
        self.current_pass
    }

    /// Check if decoding is complete
    pub fn is_complete(&self) -> bool {
        self.current_pass == ProgressivePass::Full
    }

    /// Get progress percentage (0-100)
    pub fn progress_percentage(&self) -> u8 {
        self.current_pass.quality_percentage()
    }
}

/// Progressive scan configuration
#[derive(Debug, Clone)]
pub struct ScanConfiguration {
    /// Number of scans (passes)
    pub num_scans: usize,
    /// AC coefficients per scan
    pub coefficients_per_scan: Vec<usize>,
}

impl ScanConfiguration {
    /// Create default progressive scan configuration
    pub fn default_progressive() -> Self {
        Self {
            num_scans: 4,
            coefficients_per_scan: vec![
                15, // Scan 1: Low frequencies (AC 0-14)
                16, // Scan 2: Medium frequencies (AC 15-30)
                16, // Scan 3: Medium-high frequencies (AC 31-46)
                16, // Scan 4: High frequencies (AC 47-62)
            ],
        }
    }

    /// Create fast progressive scan (fewer passes)
    pub fn fast_progressive() -> Self {
        Self {
            num_scans: 2,
            coefficients_per_scan: vec![
                31, // Scan 1: Low-medium frequencies
                32, // Scan 2: High frequencies
            ],
        }
    }

    /// Create fine progressive scan (more passes for smoother progression)
    pub fn fine_progressive() -> Self {
        Self {
            num_scans: 6,
            coefficients_per_scan: vec![
                10, // Very low frequencies
                11, // Low frequencies
                11, // Low-medium frequencies
                11, // Medium frequencies
                10, // Medium-high frequencies
                10, // High frequencies
            ],
        }
    }

    /// Validate configuration
    pub fn validate(&self) -> JxlResult<()> {
        let total: usize = self.coefficients_per_scan.iter().sum();
        if total != 63 {
            return Err(JxlError::InvalidParameter(format!(
                "Scan configuration must cover all 63 AC coefficients, got {}",
                total
            )));
        }
        Ok(())
    }
}

/// Progressive encoding/decoding configuration
#[derive(Debug, Clone)]
pub struct ProgressiveConfig {
    /// Enable progressive mode
    pub enabled: bool,
    /// Scan configuration
    pub scan_config: ScanConfiguration,
    /// Send DC-only pass first
    pub dc_first: bool,
}

impl Default for ProgressiveConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            scan_config: ScanConfiguration::default_progressive(),
            dc_first: true,
        }
    }
}

impl ProgressiveConfig {
    /// Create progressive configuration
    pub fn progressive() -> Self {
        Self {
            enabled: true,
            scan_config: ScanConfiguration::default_progressive(),
            dc_first: true,
        }
    }

    /// Create fast progressive configuration
    pub fn fast_progressive() -> Self {
        Self {
            enabled: true,
            scan_config: ScanConfiguration::fast_progressive(),
            dc_first: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_progressive_pass_ordering() {
        assert!(ProgressivePass::DcOnly < ProgressivePass::AcPass1);
        assert!(ProgressivePass::AcPass1 < ProgressivePass::AcPass2);
        assert!(ProgressivePass::AcPass2 < ProgressivePass::AcPass3);
        assert!(ProgressivePass::AcPass3 < ProgressivePass::Full);
    }

    #[test]
    fn test_progressive_pass_next() {
        assert_eq!(
            ProgressivePass::DcOnly.next(),
            Some(ProgressivePass::AcPass1)
        );
        assert_eq!(
            ProgressivePass::AcPass1.next(),
            Some(ProgressivePass::AcPass2)
        );
        assert_eq!(ProgressivePass::Full.next(), None);
    }

    #[test]
    fn test_progressive_pass_coefficient_count() {
        assert_eq!(ProgressivePass::DcOnly.ac_coefficient_count(), 0);
        assert_eq!(ProgressivePass::AcPass1.ac_coefficient_count(), 15);
        assert_eq!(ProgressivePass::Full.ac_coefficient_count(), 63);
    }

    #[test]
    fn test_progressive_decoder_creation() {
        let dims = Dimensions::new(64, 64);
        let decoder = ProgressiveDecoder::new(dims, 3);

        assert_eq!(decoder.num_channels, 3);
        assert_eq!(decoder.current_pass, ProgressivePass::DcOnly);
        assert!(!decoder.is_complete());
    }

    #[test]
    fn test_dc_pass_decode() {
        let dims = Dimensions::new(16, 16);
        let mut decoder = ProgressiveDecoder::new(dims, 1);

        // 16x16 image = 2x2 DC blocks
        let dc_data = vec![vec![1.0, 2.0, 3.0, 4.0]];
        decoder.decode_dc_pass(&dc_data).unwrap();

        assert_eq!(decoder.current_pass, ProgressivePass::DcOnly);
        let preview = decoder.get_dc_preview();
        assert_eq!(preview[0], dc_data[0]);
    }

    #[test]
    fn test_ac_pass_accumulation() {
        let dims = Dimensions::new(16, 16);
        let mut decoder = ProgressiveDecoder::new(dims, 1);

        // Decode DC first
        let dc_data = vec![vec![1.0, 2.0, 3.0, 4.0]];
        decoder.decode_dc_pass(&dc_data).unwrap();

        // Add AC pass 1
        let ac_data1 = vec![vec![0.5; 16 * 16]];
        decoder
            .decode_ac_pass(&ac_data1, ProgressivePass::AcPass1)
            .unwrap();
        assert_eq!(decoder.current_pass, ProgressivePass::AcPass1);

        // Add AC pass 2
        let ac_data2 = vec![vec![0.3; 16 * 16]];
        decoder
            .decode_ac_pass(&ac_data2, ProgressivePass::AcPass2)
            .unwrap();
        assert_eq!(decoder.current_pass, ProgressivePass::AcPass2);

        // Check accumulation
        assert_eq!(decoder.ac_coefficients[0][0], 0.5 + 0.3);
    }

    #[test]
    fn test_scan_configuration_validation() {
        let config = ScanConfiguration::default_progressive();
        assert!(config.validate().is_ok());

        let total: usize = config.coefficients_per_scan.iter().sum();
        assert_eq!(total, 63);
    }

    #[test]
    fn test_scan_configuration_variants() {
        let default = ScanConfiguration::default_progressive();
        assert_eq!(default.num_scans, 4);

        let fast = ScanConfiguration::fast_progressive();
        assert_eq!(fast.num_scans, 2);
        assert!(fast.validate().is_ok());

        let fine = ScanConfiguration::fine_progressive();
        assert_eq!(fine.num_scans, 6);
        assert!(fine.validate().is_ok());
    }

    #[test]
    fn test_progressive_config() {
        let config = ProgressiveConfig::default();
        assert!(!config.enabled);

        let prog = ProgressiveConfig::progressive();
        assert!(prog.enabled);
        assert!(prog.dc_first);
    }

    #[test]
    fn test_progress_percentage() {
        assert_eq!(ProgressivePass::DcOnly.quality_percentage(), 20);
        assert_eq!(ProgressivePass::AcPass1.quality_percentage(), 40);
        assert_eq!(ProgressivePass::Full.quality_percentage(), 100);
    }
}
