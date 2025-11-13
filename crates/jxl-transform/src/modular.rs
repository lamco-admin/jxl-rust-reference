//! Modular mode for lossless and near-lossless encoding
//!
//! JPEG XL's modular mode uses:
//! - Predictive coding with multiple predictor modes
//! - Meta-Adaptive (MA) tree for context modeling
//! - Reversible color transforms
//! - Palette encoding for images with few colors

use jxl_core::{JxlError, JxlResult};

/// Predictor modes for modular encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Predictor {
    /// No prediction (use actual value)
    Zero,
    /// Left pixel prediction
    Left,
    /// Top pixel prediction
    Top,
    /// Average of left and top
    Average,
    /// Paeth predictor (from PNG)
    Paeth,
    /// Select between left, top, and average
    Select,
    /// Gradient predictor
    Gradient,
    /// Weighted predictor
    Weighted,
}

impl Predictor {
    /// Predict pixel value based on context
    pub fn predict(&self, left: i32, top: i32, top_left: i32) -> i32 {
        match self {
            Predictor::Zero => 0,
            Predictor::Left => left,
            Predictor::Top => top,
            Predictor::Average => (left + top) / 2,
            Predictor::Paeth => paeth_predictor(left, top, top_left),
            Predictor::Select => {
                // Select mode: choose best of left, top, or average
                let avg = (left + top) / 2;
                let grad_left = (left - top_left).abs();
                let grad_top = (top - top_left).abs();

                if grad_left < grad_top {
                    left
                } else if grad_top < grad_left {
                    top
                } else {
                    avg
                }
            }
            Predictor::Gradient => {
                // Gradient predictor: left + top - top_left
                left + top - top_left
            }
            Predictor::Weighted => {
                // Weighted predictor (simplified)
                let w_left = if (top - top_left).abs() < (left - top_left).abs() {
                    3
                } else {
                    1
                };
                let w_top = 4 - w_left;
                (left * w_left + top * w_top) / 4
            }
        }
    }
}

/// Paeth predictor from PNG specification
fn paeth_predictor(a: i32, b: i32, c: i32) -> i32 {
    let p = a + b - c;
    let pa = (p - a).abs();
    let pb = (p - b).abs();
    let pc = (p - c).abs();

    if pa <= pb && pa <= pc {
        a
    } else if pb <= pc {
        b
    } else {
        c
    }
}

/// Meta-Adaptive tree node for context modeling
#[derive(Debug, Clone)]
pub struct MATreeNode {
    /// Property index for decision
    pub property: usize,
    /// Split value
    pub split_value: i32,
    /// Left child (if split_value < threshold)
    pub left: Option<Box<MATreeNode>>,
    /// Right child (if split_value >= threshold)
    pub right: Option<Box<MATreeNode>>,
    /// Leaf context (if this is a leaf node)
    pub context: Option<u32>,
}

impl MATreeNode {
    /// Create a leaf node with context
    pub fn leaf(context: u32) -> Self {
        Self {
            property: 0,
            split_value: 0,
            left: None,
            right: None,
            context: Some(context),
        }
    }

    /// Create a split node
    pub fn split(property: usize, split_value: i32, left: MATreeNode, right: MATreeNode) -> Self {
        Self {
            property,
            split_value,
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
            context: None,
        }
    }

    /// Get context for given properties
    pub fn get_context(&self, properties: &[i32]) -> u32 {
        if let Some(ctx) = self.context {
            return ctx;
        }

        let property_value = properties.get(self.property).copied().unwrap_or(0);

        if property_value < self.split_value {
            if let Some(ref left) = self.left {
                left.get_context(properties)
            } else {
                0
            }
        } else if let Some(ref right) = self.right {
            right.get_context(properties)
        } else {
            0
        }
    }
}

/// Modular image representation
#[derive(Debug, Clone)]
pub struct ModularImage {
    /// Width of the image
    pub width: usize,
    /// Height of the image
    pub height: usize,
    /// Number of channels
    pub num_channels: usize,
    /// Bit depth per channel
    pub bit_depth: u8,
    /// Image data (channel-planar format)
    pub data: Vec<Vec<i32>>,
}

impl ModularImage {
    /// Create a new modular image
    pub fn new(width: usize, height: usize, num_channels: usize, bit_depth: u8) -> Self {
        let size = width * height;
        let data = vec![vec![0i32; size]; num_channels];

        Self {
            width,
            height,
            num_channels,
            bit_depth,
            data,
        }
    }

    /// Apply predictor to channel
    pub fn apply_predictor(
        &self,
        channel: usize,
        predictor: Predictor,
        output: &mut Vec<i32>,
    ) -> JxlResult<()> {
        if channel >= self.num_channels {
            return Err(JxlError::InvalidParameter(format!(
                "Channel {} out of range",
                channel
            )));
        }

        let chan_data = &self.data[channel];
        output.clear();
        output.reserve(chan_data.len());

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let pixel = chan_data[idx];

                // Get context pixels
                let left = if x > 0 {
                    chan_data[idx - 1]
                } else {
                    0
                };

                let top = if y > 0 {
                    chan_data[idx - self.width]
                } else {
                    0
                };

                let top_left = if x > 0 && y > 0 {
                    chan_data[idx - self.width - 1]
                } else {
                    0
                };

                // Predict and compute residual
                let prediction = predictor.predict(left, top, top_left);
                let residual = pixel - prediction;
                output.push(residual);
            }
        }

        Ok(())
    }

    /// Inverse predictor to reconstruct channel
    pub fn inverse_predictor(
        &mut self,
        channel: usize,
        predictor: Predictor,
        residuals: &[i32],
    ) -> JxlResult<()> {
        if channel >= self.num_channels {
            return Err(JxlError::InvalidParameter(format!(
                "Channel {} out of range",
                channel
            )));
        }

        if residuals.len() != self.width * self.height {
            return Err(JxlError::InvalidParameter(
                "Residuals size mismatch".to_string(),
            ));
        }

        let chan_data = &mut self.data[channel];

        for y in 0..self.height {
            for x in 0..self.width {
                let idx = y * self.width + x;
                let residual = residuals[idx];

                // Get context pixels (already reconstructed)
                let left = if x > 0 { chan_data[idx - 1] } else { 0 };

                let top = if y > 0 {
                    chan_data[idx - self.width]
                } else {
                    0
                };

                let top_left = if x > 0 && y > 0 {
                    chan_data[idx - self.width - 1]
                } else {
                    0
                };

                // Predict and add residual
                let prediction = predictor.predict(left, top, top_left);
                chan_data[idx] = prediction + residual;
            }
        }

        Ok(())
    }
}

/// Reversible Color Transform (RCT) for lossless compression
/// Uses a modified YCoCg-R transform that is perfectly reversible
pub fn apply_rct(r: &[i32], g: &[i32], b: &[i32], output: &mut [Vec<i32>]) {
    assert_eq!(r.len(), g.len());
    assert_eq!(r.len(), b.len());

    output[0].clear();
    output[1].clear();
    output[2].clear();

    for i in 0..r.len() {
        // YCoCg-R transform (perfectly reversible)
        // Co = R - B
        // t = B + (Co >> 1)
        // Cg = G - t
        // Y = t + (Cg >> 1)

        let co = r[i] - b[i];
        let t = b[i] + (co >> 1);
        let cg = g[i] - t;
        let y = t + (cg >> 1);

        output[0].push(y);
        output[1].push(co);
        output[2].push(cg);
    }
}

/// Inverse Reversible Color Transform
pub fn inverse_rct(y: &[i32], co: &[i32], cg: &[i32], output: &mut [Vec<i32>]) {
    assert_eq!(y.len(), co.len());
    assert_eq!(y.len(), cg.len());

    output[0].clear(); // R
    output[1].clear(); // G
    output[2].clear(); // B

    for i in 0..y.len() {
        // Inverse YCoCg-R (perfectly reversible)
        let t = y[i] - (cg[i] >> 1);
        let g = cg[i] + t;
        let b = t - (co[i] >> 1);
        let r = b + co[i];

        output[0].push(r);
        output[1].push(g);
        output[2].push(b);
    }
}

/// Palette encoding for images with few unique colors
#[derive(Debug, Clone)]
pub struct Palette {
    /// Palette colors (up to 256)
    pub colors: Vec<Vec<i32>>,
    /// Number of colors
    pub size: usize,
}

impl Palette {
    /// Create a new palette
    pub fn new() -> Self {
        Self {
            colors: Vec::new(),
            size: 0,
        }
    }

    /// Build palette from image
    pub fn build_from_image(&mut self, image: &ModularImage, max_colors: usize) -> bool {
        use std::collections::HashMap;

        let mut color_map: HashMap<Vec<i32>, usize> = HashMap::new();

        // Collect unique colors
        for i in 0..image.width * image.height {
            let mut color = Vec::new();
            for ch in 0..image.num_channels {
                color.push(image.data[ch][i]);
            }

            if !color_map.contains_key(&color) {
                if color_map.len() >= max_colors {
                    // Too many colors, palette not beneficial
                    return false;
                }
                color_map.insert(color.clone(), color_map.len());
            }
        }

        // Build palette
        self.colors = color_map.keys().cloned().collect();
        self.size = self.colors.len();
        true
    }

    /// Encode image using palette
    pub fn encode(&self, image: &ModularImage) -> Vec<u8> {
        use std::collections::HashMap;

        let mut color_to_idx: HashMap<Vec<i32>, u8> = HashMap::new();
        for (idx, color) in self.colors.iter().enumerate() {
            color_to_idx.insert(color.clone(), idx as u8);
        }

        let mut indices = Vec::new();
        for i in 0..image.width * image.height {
            let mut color = Vec::new();
            for ch in 0..image.num_channels {
                color.push(image.data[ch][i]);
            }
            let idx = *color_to_idx.get(&color).unwrap_or(&0);
            indices.push(idx);
        }

        indices
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_predictors() {
        assert_eq!(Predictor::Left.predict(10, 5, 3), 10);
        assert_eq!(Predictor::Top.predict(10, 5, 3), 5);
        assert_eq!(Predictor::Average.predict(10, 6, 3), 8);
        assert_eq!(Predictor::Gradient.predict(10, 6, 3), 13); // 10 + 6 - 3
    }

    #[test]
    fn test_paeth_predictor() {
        // Paeth should select the closest predictor
        // p = a + b - c = 10 + 5 - 8 = 7
        // |p - a| = |7 - 10| = 3
        // |p - b| = |7 - 5| = 2
        // |p - c| = |7 - 8| = 1
        // c is closest, so return 8
        assert_eq!(paeth_predictor(10, 5, 8), 8);

        // p = a + b - c = 5 + 10 - 8 = 7
        // |p - a| = |7 - 5| = 2
        // |p - b| = |7 - 10| = 3
        // |p - c| = |7 - 8| = 1
        // c is closest, so return 8
        assert_eq!(paeth_predictor(5, 10, 8), 8);
    }

    #[test]
    fn test_modular_predictor_roundtrip() {
        let mut img = ModularImage::new(4, 4, 1, 8);

        // Fill with test pattern
        for y in 0..4 {
            for x in 0..4 {
                img.data[0][y * 4 + x] = (x + y * 4) as i32;
            }
        }

        // Apply predictor
        let mut residuals = Vec::new();
        img.apply_predictor(0, Predictor::Gradient, &mut residuals)
            .unwrap();

        // Inverse predictor
        let mut reconstructed = ModularImage::new(4, 4, 1, 8);
        reconstructed
            .inverse_predictor(0, Predictor::Gradient, &residuals)
            .unwrap();

        // Verify perfect reconstruction
        assert_eq!(img.data[0], reconstructed.data[0]);
    }

    #[test]
    fn test_rct_roundtrip() {
        let r = vec![100, 150, 200];
        let g = vec![50, 100, 150];
        let b = vec![25, 75, 125];

        let mut ycocg = vec![Vec::new(); 3];
        apply_rct(&r, &g, &b, &mut ycocg);

        let mut rgb = vec![Vec::new(); 3];
        inverse_rct(&ycocg[0], &ycocg[1], &ycocg[2], &mut rgb);

        assert_eq!(r, rgb[0]);
        assert_eq!(g, rgb[1]);
        assert_eq!(b, rgb[2]);
    }

    #[test]
    fn test_ma_tree() {
        // Create simple tree: if property 0 < 10 -> context 0, else -> context 1
        let tree = MATreeNode::split(0, 10, MATreeNode::leaf(0), MATreeNode::leaf(1));

        assert_eq!(tree.get_context(&[5]), 0);
        assert_eq!(tree.get_context(&[15]), 1);
    }

    #[test]
    fn test_palette() {
        let mut img = ModularImage::new(2, 2, 3, 8);

        // Create image with 2 colors: red and blue
        img.data[0][0] = 255; // Red
        img.data[1][0] = 0;
        img.data[2][0] = 0;

        img.data[0][1] = 0; // Blue
        img.data[1][1] = 0;
        img.data[2][1] = 255;

        img.data[0][2] = 255; // Red
        img.data[1][2] = 0;
        img.data[2][2] = 0;

        img.data[0][3] = 0; // Blue
        img.data[1][3] = 0;
        img.data[2][3] = 255;

        let mut palette = Palette::new();
        assert!(palette.build_from_image(&img, 256));
        assert_eq!(palette.size, 2);

        let indices = palette.encode(&img);
        assert_eq!(indices.len(), 4);
    }
}
