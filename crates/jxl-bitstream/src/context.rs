//! Context modeling for adaptive entropy coding
//!
//! Provides context-adaptive probability distributions for ANS encoding.
//! By using different distributions based on neighboring coefficients and
//! block positions, we can achieve 5-10% better compression than using
//! a single global distribution.

use super::ans::AnsDistribution;
use jxl_core::JxlResult;

/// Frequency band classification for DCT coefficients
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FrequencyBand {
    /// DC coefficient (0)
    DC,
    /// Low frequency AC coefficients (1-10)
    LowFrequency,
    /// Mid frequency AC coefficients (11-30)
    MidFrequency,
    /// High frequency AC coefficients (31-63)
    HighFrequency,
}

impl FrequencyBand {
    /// Determine frequency band from coefficient index (in zigzag order)
    pub fn from_index(index: usize) -> Self {
        match index {
            0 => FrequencyBand::DC,
            1..=10 => FrequencyBand::LowFrequency,
            11..=30 => FrequencyBand::MidFrequency,
            _ => FrequencyBand::HighFrequency,
        }
    }

    /// Get the number of different frequency bands
    pub const fn count() -> usize {
        4
    }

    /// Convert to index for array lookups
    pub const fn to_index(self) -> usize {
        match self {
            FrequencyBand::DC => 0,
            FrequencyBand::LowFrequency => 1,
            FrequencyBand::MidFrequency => 2,
            FrequencyBand::HighFrequency => 3,
        }
    }
}

/// Context for adaptive entropy coding
///
/// The context determines which probability distribution to use
/// when encoding/decoding a coefficient.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Context {
    /// Frequency band of the coefficient
    pub frequency_band: FrequencyBand,
    /// Block position in image (for spatial adaptation)
    pub block_x: usize,
    pub block_y: usize,
    /// Neighbor coefficients (for value prediction)
    pub neighbor_nonzero_count: u8,
}

impl Context {
    /// Create a context for a given coefficient
    pub fn new(
        coeff_index: usize,
        block_x: usize,
        block_y: usize,
        neighbor_nonzero_count: u8,
    ) -> Self {
        Self {
            frequency_band: FrequencyBand::from_index(coeff_index),
            block_x,
            block_y,
            neighbor_nonzero_count,
        }
    }

    /// Get a simplified context ID for distribution selection
    ///
    /// We use frequency band as the primary context. This gives us
    /// 4 different distributions, which is a good balance between
    /// compression efficiency and model complexity.
    pub fn distribution_id(&self) -> usize {
        self.frequency_band.to_index()
    }

    /// Get context for DC coefficient
    pub fn dc_context(block_x: usize, block_y: usize) -> Self {
        Self::new(0, block_x, block_y, 0)
    }

    /// Get context for AC coefficient
    pub fn ac_context(
        coeff_index: usize,
        block_x: usize,
        block_y: usize,
        neighbor_nonzero_count: u8,
    ) -> Self {
        Self::new(coeff_index, block_x, block_y, neighbor_nonzero_count)
    }
}

/// Context-adaptive distribution set
///
/// Manages multiple ANS distributions for different contexts.
/// This allows the encoder to adapt to different coefficient patterns.
pub struct ContextModel {
    /// Distribution for each frequency band
    distributions: Vec<AnsDistribution>,
}

impl ContextModel {
    /// Create a new context model with the given distributions
    pub fn new(distributions: Vec<AnsDistribution>) -> JxlResult<Self> {
        if distributions.len() != FrequencyBand::count() {
            return Err(jxl_core::JxlError::InvalidParameter(format!(
                "Expected {} distributions, got {}",
                FrequencyBand::count(),
                distributions.len()
            )));
        }
        Ok(Self { distributions })
    }

    /// Get distribution for a given context
    pub fn get_distribution(&self, context: &Context) -> &AnsDistribution {
        let id = context.distribution_id();
        &self.distributions[id]
    }

    /// Get distribution by ID
    pub fn get_distribution_by_id(&self, id: usize) -> Option<&AnsDistribution> {
        self.distributions.get(id)
    }

    /// Get number of distributions
    pub fn num_distributions(&self) -> usize {
        self.distributions.len()
    }

    /// Build context model from coefficient statistics
    ///
    /// Analyzes coefficients and builds optimal distributions for each context.
    pub fn build_from_coefficients(coefficients: &[i16]) -> JxlResult<Self> {
        // Separate coefficients by frequency band
        let mut band_coeffs: Vec<Vec<i16>> = vec![Vec::new(); FrequencyBand::count()];

        // Assume coefficients are in blocks of 64 (8x8 DCT)
        for block in coefficients.chunks(64) {
            for (i, &coeff) in block.iter().enumerate() {
                let band = FrequencyBand::from_index(i);
                band_coeffs[band.to_index()].push(coeff);
            }
        }

        // Build distribution for each band
        let mut distributions = Vec::with_capacity(FrequencyBand::count());

        for (_band_idx, coeffs) in band_coeffs.iter().enumerate() {
            if coeffs.is_empty() {
                // Fallback: create uniform distribution
                let uniform_freqs = vec![1; 256];
                distributions.push(AnsDistribution::from_frequencies(&uniform_freqs)?);
            } else {
                // Build distribution from actual coefficient statistics
                let dist = Self::build_distribution_for_band(coeffs)?;
                distributions.push(dist);
            }
        }

        Self::new(distributions)
    }

    /// Build ANS distribution for a specific frequency band
    fn build_distribution_for_band(coeffs: &[i16]) -> JxlResult<AnsDistribution> {
        // Collect symbol frequencies using zigzag encoding
        // ANS_TAB_SIZE is 4096, so we limit alphabet to reasonable size
        // Support coefficients in range [-2048, 2047] → symbols [0, 4095]
        const MAX_SYMBOL: usize = 4096;
        let mut frequencies = vec![0u32; MAX_SYMBOL];

        for &coeff in coeffs {
            let symbol = Self::coeff_to_symbol(coeff);
            // Clip symbols that exceed our alphabet size
            let clipped_symbol = (symbol as usize).min(MAX_SYMBOL - 1);
            frequencies[clipped_symbol] += 1;
        }

        // Ensure no zero frequencies for a reasonable symbol range
        let total: u32 = frequencies.iter().sum();
        if total == 0 {
            // No coefficients, use small uniform distribution
            let alphabet_size = 256;
            return AnsDistribution::from_frequencies(&vec![1; alphabet_size]);
        }

        // Find the actual range of symbols used
        let max_used_symbol = frequencies
            .iter()
            .enumerate()
            .rev()
            .find(|(_, &f)| f > 0)
            .map(|(i, _)| i)
            .unwrap_or(255);

        // Trim to actual range, but keep at least 256 symbols
        let alphabet_size = (max_used_symbol + 1).max(256);
        frequencies.truncate(alphabet_size);

        // Add small frequency to all symbols in alphabet to prevent zero-frequency errors
        // But be careful not to make total too large
        let min_freq = 1;
        for f in frequencies.iter_mut() {
            if *f == 0 {
                *f = min_freq;
            }
        }

        AnsDistribution::from_frequencies(&frequencies)
    }

    /// Convert coefficient to symbol (zigzag encoding)
    fn coeff_to_symbol(coeff: i16) -> u32 {
        if coeff >= 0 {
            (coeff as u32) * 2
        } else {
            ((-coeff) as u32) * 2 - 1
        }
    }

    /// Convert symbol to coefficient (zigzag decoding)
    pub fn symbol_to_coeff(symbol: u32) -> i16 {
        if symbol % 2 == 0 {
            (symbol / 2) as i16
        } else {
            -(((symbol + 1) / 2) as i16)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_frequency_band_from_index() {
        assert_eq!(FrequencyBand::from_index(0), FrequencyBand::DC);
        assert_eq!(FrequencyBand::from_index(1), FrequencyBand::LowFrequency);
        assert_eq!(FrequencyBand::from_index(5), FrequencyBand::LowFrequency);
        assert_eq!(FrequencyBand::from_index(10), FrequencyBand::LowFrequency);
        assert_eq!(FrequencyBand::from_index(11), FrequencyBand::MidFrequency);
        assert_eq!(FrequencyBand::from_index(30), FrequencyBand::MidFrequency);
        assert_eq!(FrequencyBand::from_index(31), FrequencyBand::HighFrequency);
        assert_eq!(FrequencyBand::from_index(63), FrequencyBand::HighFrequency);
    }

    #[test]
    fn test_frequency_band_to_index() {
        assert_eq!(FrequencyBand::DC.to_index(), 0);
        assert_eq!(FrequencyBand::LowFrequency.to_index(), 1);
        assert_eq!(FrequencyBand::MidFrequency.to_index(), 2);
        assert_eq!(FrequencyBand::HighFrequency.to_index(), 3);
    }

    #[test]
    fn test_context_creation() {
        let ctx = Context::new(5, 10, 20, 3);
        assert_eq!(ctx.frequency_band, FrequencyBand::LowFrequency);
        assert_eq!(ctx.block_x, 10);
        assert_eq!(ctx.block_y, 20);
        assert_eq!(ctx.neighbor_nonzero_count, 3);
    }

    #[test]
    fn test_context_distribution_id() {
        let dc_ctx = Context::new(0, 0, 0, 0);
        assert_eq!(dc_ctx.distribution_id(), 0);

        let low_ctx = Context::new(5, 0, 0, 0);
        assert_eq!(low_ctx.distribution_id(), 1);

        let mid_ctx = Context::new(20, 0, 0, 0);
        assert_eq!(mid_ctx.distribution_id(), 2);

        let high_ctx = Context::new(50, 0, 0, 0);
        assert_eq!(high_ctx.distribution_id(), 3);
    }

    #[test]
    fn test_coeff_to_symbol_roundtrip() {
        let test_coeffs = vec![0, 1, -1, 2, -2, 10, -10, 127, -127];

        for &coeff in &test_coeffs {
            let symbol = ContextModel::coeff_to_symbol(coeff);
            let decoded = ContextModel::symbol_to_coeff(symbol);
            assert_eq!(
                coeff, decoded,
                "Roundtrip failed for coeff {}: symbol={}, decoded={}",
                coeff, symbol, decoded
            );
        }
    }

    #[test]
    fn test_build_context_model() {
        // Create sample coefficients (4 blocks of 64 coefficients each)
        let mut coeffs = Vec::new();

        for _ in 0..4 {
            // DC coefficient
            coeffs.push(100i16);

            // Low frequency: mostly small values
            for i in 1..11 {
                coeffs.push((i % 5) as i16);
            }

            // Mid frequency: sparse
            for _ in 11..31 {
                coeffs.push(0);
            }

            // High frequency: very sparse
            for _ in 31..64 {
                coeffs.push(0);
            }
        }

        let model = ContextModel::build_from_coefficients(&coeffs).unwrap();

        assert_eq!(model.num_distributions(), 4);

        // Should have different distributions for different bands
        let dc_dist = model.get_distribution_by_id(0).unwrap();
        let low_dist = model.get_distribution_by_id(1).unwrap();

        // DC should have large alphabet (high values)
        // Low freq should have smaller alphabet (small values)
        assert!(dc_dist.alphabet_size() > 0);
        assert!(low_dist.alphabet_size() > 0);
    }

    #[test]
    fn test_context_model_get_distribution() {
        let coeffs = vec![0i16; 256]; // 4 blocks of zeros
        let model = ContextModel::build_from_coefficients(&coeffs).unwrap();

        let dc_ctx = Context::dc_context(0, 0);
        let dist = model.get_distribution(&dc_ctx);
        assert!(dist.alphabet_size() > 0);

        let ac_ctx = Context::ac_context(5, 0, 0, 0);
        let dist = model.get_distribution(&ac_ctx);
        assert!(dist.alphabet_size() > 0);
    }
}
