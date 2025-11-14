//! Adaptive quantization for perceptually better compression
//!
//! Adaptive quantization varies the quantization strength based on local image
//! characteristics. Smooth regions can be quantized more aggressively while
//! preserving details and edges. This provides better visual quality at the
//! same file size, or smaller files at the same visual quality.

use jxl_core::JxlResult;

/// Complexity metric for an 8x8 block
#[derive(Debug, Clone, Copy)]
pub struct BlockComplexity {
    /// Variance-based complexity measure
    pub variance: f32,
    /// Edge strength measure (high frequency energy)
    pub edge_strength: f32,
    /// Combined complexity score
    pub complexity_score: f32,
}

impl BlockComplexity {
    /// Compute complexity metrics for an 8x8 block
    pub fn compute(block: &[f32; 64]) -> Self {
        let variance = Self::compute_variance(block);
        let edge_strength = Self::compute_edge_strength(block);

        // Combine metrics: high variance or high edge strength = complex
        let complexity_score = (variance.sqrt() + edge_strength) / 2.0;

        Self {
            variance,
            edge_strength,
            complexity_score,
        }
    }

    /// Compute variance of pixel values in block
    fn compute_variance(block: &[f32; 64]) -> f32 {
        let mean: f32 = block.iter().sum::<f32>() / 64.0;
        let variance: f32 = block.iter()
            .map(|&x| {
                let diff = x - mean;
                diff * diff
            })
            .sum::<f32>() / 64.0;
        variance
    }

    /// Compute edge strength using simple gradient approximation
    fn compute_edge_strength(block: &[f32; 64]) -> f32 {
        let mut total_gradient = 0.0;

        // Compute horizontal and vertical gradients
        for y in 0..8 {
            for x in 0..7 {
                // Horizontal gradient
                let idx = y * 8 + x;
                let dx = (block[idx + 1] - block[idx]).abs();
                total_gradient += dx;
            }
        }

        for y in 0..7 {
            for x in 0..8 {
                // Vertical gradient
                let idx = y * 8 + x;
                let dy = (block[idx + 8] - block[idx]).abs();
                total_gradient += dy;
            }
        }

        // Normalize by number of gradients computed
        total_gradient / (7.0 * 8.0 + 8.0 * 7.0)
    }
}

/// Adaptive quantization map
///
/// Stores per-block quantization scaling factors based on local complexity.
pub struct AdaptiveQuantMap {
    /// Quantization scale for each block
    scales: Vec<f32>,
    /// Number of blocks horizontally
    blocks_x: usize,
    /// Number of blocks vertically
    blocks_y: usize,
    /// Base quality level (0-100)
    base_quality: f32,
}

impl AdaptiveQuantMap {
    /// Create adaptive quantization map for an image
    ///
    /// # Arguments
    /// * `width` - Image width in pixels
    /// * `height` - Image height in pixels
    /// * `blocks` - Pre-computed 8x8 blocks in raster order
    /// * `base_quality` - Base quality level (0-100)
    pub fn new(
        width: usize,
        height: usize,
        blocks: &[[f32; 64]],
        base_quality: f32,
    ) -> JxlResult<Self> {
        let blocks_x = (width + 7) / 8;
        let blocks_y = (height + 7) / 8;

        assert_eq!(blocks.len(), blocks_x * blocks_y, "Block count mismatch");

        // Compute complexity for each block
        let mut complexities = Vec::with_capacity(blocks.len());
        for block in blocks {
            complexities.push(BlockComplexity::compute(block));
        }

        // Compute quantization scales based on complexity
        let scales = Self::compute_scales(&complexities, base_quality);

        Ok(Self {
            scales,
            blocks_x,
            blocks_y,
            base_quality,
        })
    }

    /// Compute quantization scale factors from complexity metrics
    fn compute_scales(complexities: &[BlockComplexity], base_quality: f32) -> Vec<f32> {
        let mut scales = Vec::with_capacity(complexities.len());

        // Quality factor affects how much we adapt
        // High quality = less adaptation (preserve everything)
        // Low quality = more adaptation (quantize smooth areas aggressively)
        let adaptation_strength = 1.0 - (base_quality / 100.0).powf(0.5);

        for complexity in complexities {
            let scale = Self::complexity_to_scale(complexity, adaptation_strength);
            scales.push(scale);
        }

        scales
    }

    /// Convert complexity metrics to quantization scale
    ///
    /// Returns a scale factor where:
    /// - 1.0 = use base quantization
    /// - > 1.0 = quantize more (smooth areas)
    /// - < 1.0 = quantize less (preserve details)
    fn complexity_to_scale(complexity: &BlockComplexity, adaptation_strength: f32) -> f32 {
        // Classify the block
        let is_smooth = complexity.variance < 100.0;
        let has_edges = complexity.edge_strength > 10.0;
        let is_textured = complexity.variance > 500.0;

        // Base scale is 1.0 (use base quantization)
        let mut scale = 1.0;

        if has_edges {
            // Preserve edges - reduce quantization
            scale *= 0.7;
        } else if is_smooth {
            // Smooth area - can quantize more aggressively
            scale *= 1.0 + (0.5 * adaptation_strength);
        }

        if is_textured {
            // Texture - preserve some detail
            scale *= 0.85;
        }

        // Clamp to reasonable range
        scale.clamp(0.5, 2.0)
    }

    /// Get quantization scale for a specific block
    pub fn get_scale(&self, block_x: usize, block_y: usize) -> f32 {
        if block_x >= self.blocks_x || block_y >= self.blocks_y {
            return 1.0;
        }
        self.scales[block_y * self.blocks_x + block_x]
    }

    /// Get number of blocks horizontally
    pub fn blocks_x(&self) -> usize {
        self.blocks_x
    }

    /// Get number of blocks vertically
    pub fn blocks_y(&self) -> usize {
        self.blocks_y
    }

    /// Serialize the AQ map to a compact format
    /// Returns quantized scales as u8 values (scale * 51.0, clamped to 0-255)
    /// This allows scales [0.5, 2.0] to be represented with ~1.5% precision
    pub fn serialize(&self) -> Vec<u8> {
        self.scales
            .iter()
            .map(|&scale| {
                // Map [0.5, 2.0] to [0, 255]
                // 0.5 → 0, 1.0 → 85, 2.0 → 255
                let quantized = ((scale - 0.5) * 170.0).round();
                quantized.clamp(0.0, 255.0) as u8
            })
            .collect()
    }

    /// Deserialize AQ map from compact format
    pub fn deserialize(
        serialized: &[u8],
        width: usize,
        height: usize,
        base_quality: f32,
    ) -> JxlResult<Self> {
        let blocks_x = (width + 7) / 8;
        let blocks_y = (height + 7) / 8;

        if serialized.len() != blocks_x * blocks_y {
            return Err(jxl_core::JxlError::InvalidParameter(format!(
                "AQ map size mismatch: expected {}, got {}",
                blocks_x * blocks_y,
                serialized.len()
            )));
        }

        let scales = serialized
            .iter()
            .map(|&quantized| {
                // Map [0, 255] back to [0.5, 2.0]
                let scale = (quantized as f32 / 170.0) + 0.5;
                scale.clamp(0.5, 2.0)
            })
            .collect();

        Ok(Self {
            scales,
            blocks_x,
            blocks_y,
            base_quality,
        })
    }
}

/// Apply adaptive quantization to a set of DCT coefficients
///
/// # Arguments
/// * `coefficients` - DCT coefficients (8x8 blocks)
/// * `base_quant_table` - Base quantization table (64 values)
/// * `aq_map` - Adaptive quantization map
///
/// # Returns
/// Quantized coefficients as i16 values
pub fn adaptive_quantize(
    coefficients: &[[f32; 64]],
    base_quant_table: &[u32; 64],
    aq_map: &AdaptiveQuantMap,
) -> Vec<i16> {
    let mut quantized = Vec::with_capacity(coefficients.len() * 64);

    for (block_idx, block) in coefficients.iter().enumerate() {
        let block_y = block_idx / aq_map.blocks_x();
        let block_x = block_idx % aq_map.blocks_x();
        let scale = aq_map.get_scale(block_x, block_y);

        // Apply scaled quantization to this block
        for (i, &coeff) in block.iter().enumerate() {
            let quant_step = (base_quant_table[i] as f32 * scale).max(1.0);
            let quantized_value = (coeff / quant_step).round() as i16;
            quantized.push(quantized_value);
        }
    }

    quantized
}

/// Apply adaptive dequantization
pub fn adaptive_dequantize(
    quantized: &[i16],
    base_quant_table: &[u32; 64],
    aq_map: &AdaptiveQuantMap,
) -> Vec<[f32; 64]> {
    let num_blocks = quantized.len() / 64;
    let mut dequantized = Vec::with_capacity(num_blocks);

    for block_idx in 0..num_blocks {
        let block_y = block_idx / aq_map.blocks_x();
        let block_x = block_idx % aq_map.blocks_x();
        let scale = aq_map.get_scale(block_x, block_y);

        let mut block = [0.0f32; 64];
        for i in 0..64 {
            let idx = block_idx * 64 + i;
            let quant_step = (base_quant_table[i] as f32 * scale).max(1.0);
            block[i] = quantized[idx] as f32 * quant_step;
        }
        dequantized.push(block);
    }

    dequantized
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_complexity_smooth() {
        // Smooth block (all same value)
        let smooth_block = [100.0f32; 64];
        let complexity = BlockComplexity::compute(&smooth_block);

        assert!(complexity.variance < 0.1, "Smooth block should have low variance");
        assert!(complexity.edge_strength < 0.1, "Smooth block should have low edge strength");
        assert!(complexity.complexity_score < 1.0, "Smooth block should have low complexity");
    }

    #[test]
    fn test_block_complexity_gradient() {
        // Gradient block
        let mut gradient_block = [0.0f32; 64];
        for i in 0..64 {
            gradient_block[i] = (i * 4) as f32;
        }
        let complexity = BlockComplexity::compute(&gradient_block);

        assert!(complexity.variance > 100.0, "Gradient should have variance");
        assert!(complexity.edge_strength > 1.0, "Gradient should have some edge strength");
    }

    #[test]
    fn test_block_complexity_edges() {
        // Block with sharp vertical edge in middle
        let mut edge_block = [0.0f32; 64];
        for i in 0..64 {
            // Left half is 0, right half is 255
            edge_block[i] = if i % 8 < 4 { 0.0 } else { 255.0 };
        }
        let complexity = BlockComplexity::compute(&edge_block);

        assert!(complexity.variance > 1000.0, "Edge should have high variance");
        // Edge strength threshold adjusted based on actual gradient computation
        // Vertical edge gives horizontal gradients but no vertical gradients
        // Expected: 8 transitions of 255 / 112 total gradients ≈ 18.2
        assert!(complexity.edge_strength > 18.0,
                "Edge should have high edge strength, got {}", complexity.edge_strength);
    }

    #[test]
    fn test_adaptive_quant_map_creation() {
        let blocks = vec![[100.0f32; 64]; 4]; // 2x2 blocks
        let aq_map = AdaptiveQuantMap::new(16, 16, &blocks, 90.0).unwrap();

        assert_eq!(aq_map.blocks_x(), 2);
        assert_eq!(aq_map.blocks_y(), 2);

        // All blocks are smooth, should allow more quantization
        for y in 0..2 {
            for x in 0..2 {
                let scale = aq_map.get_scale(x, y);
                assert!(scale >= 0.5 && scale <= 2.0, "Scale should be in valid range");
            }
        }
    }

    #[test]
    fn test_adaptive_quantize_dequantize() {
        // Create simple test data
        let blocks = vec![
            [100.0f32; 64], // Smooth block
        ];
        let aq_map = AdaptiveQuantMap::new(8, 8, &blocks, 90.0).unwrap();

        let base_quant = [10u32; 64];
        let coeffs = vec![[50.0f32; 64]];

        // Quantize and dequantize
        let quantized = adaptive_quantize(&coeffs, &base_quant, &aq_map);
        assert_eq!(quantized.len(), 64);

        let dequantized = adaptive_dequantize(&quantized, &base_quant, &aq_map);
        assert_eq!(dequantized.len(), 1);

        // Values should be approximately preserved
        for (orig, deq) in coeffs[0].iter().zip(dequantized[0].iter()) {
            let error = (orig - deq).abs();
            assert!(error < 20.0, "Quantization error too large: {}", error);
        }
    }

    #[test]
    fn test_complexity_to_scale() {
        // Smooth block should get higher scale (more quantization)
        let smooth = BlockComplexity {
            variance: 50.0,
            edge_strength: 1.0,
            complexity_score: 5.0,
        };
        let smooth_scale = AdaptiveQuantMap::complexity_to_scale(&smooth, 0.5);
        assert!(smooth_scale >= 1.0, "Smooth blocks should allow more quantization");

        // Edge block should get lower scale (less quantization)
        let edge = BlockComplexity {
            variance: 200.0,
            edge_strength: 50.0,
            complexity_score: 40.0,
        };
        let edge_scale = AdaptiveQuantMap::complexity_to_scale(&edge, 0.5);
        assert!(edge_scale < 1.0, "Edge blocks should use less quantization");
    }
}
