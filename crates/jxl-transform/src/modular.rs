//! Modular Mode - JPEG XL Lossless Compression
//!
//! Modular mode provides true lossless compression through:
//! - Integer-only operations (no lossy DCT/quantization)
//! - Predictive coding (exploits spatial correlation)
//! - Meta-Adaptive Near-zero (MA) context modeling
//! - Efficient entropy coding
//!
//! This is the production lossless path in JPEG XL.

use jxl_core::{JxlError, JxlResult};

/// Modular image representation
///
/// Unlike VarDCT mode, modular mode works directly on integer pixel values
/// without lossy transforms. Channels can be of different bit depths.
#[derive(Debug, Clone)]
pub struct ModularImage {
    /// Width in pixels
    pub width: usize,
    /// Height in pixels
    pub height: usize,
    /// Number of channels
    pub num_channels: usize,
    /// Bit depth per channel (can vary)
    pub bit_depths: Vec<u8>,
    /// Channel data (one Vec per channel)
    pub channels: Vec<Vec<i32>>,
}

impl ModularImage {
    /// Create a new modular image
    pub fn new(width: usize, height: usize, num_channels: usize, bit_depth: u8) -> Self {
        let pixel_count = width * height;
        let mut channels = Vec::with_capacity(num_channels);
        let mut bit_depths = Vec::with_capacity(num_channels);

        for _ in 0..num_channels {
            channels.push(vec![0i32; pixel_count]);
            bit_depths.push(bit_depth);
        }

        Self {
            width,
            height,
            num_channels,
            bit_depths,
            channels,
        }
    }

    /// Get pixel value
    pub fn get_pixel(&self, channel: usize, x: usize, y: usize) -> JxlResult<i32> {
        if channel >= self.num_channels {
            return Err(JxlError::InvalidParameter(format!(
                "Channel {} out of range",
                channel
            )));
        }
        if x >= self.width || y >= self.height {
            return Err(JxlError::InvalidParameter("Coordinates out of bounds".to_string()));
        }
        Ok(self.channels[channel][y * self.width + x])
    }

    /// Set pixel value
    pub fn set_pixel(&mut self, channel: usize, x: usize, y: usize, value: i32) -> JxlResult<()> {
        if channel >= self.num_channels {
            return Err(JxlError::InvalidParameter(format!(
                "Channel {} out of range",
                channel
            )));
        }
        if x >= self.width || y >= self.height {
            return Err(JxlError::InvalidParameter("Coordinates out of bounds".to_string()));
        }
        self.channels[channel][y * self.width + x] = value;
        Ok(())
    }

    /// Convert from float RGB/RGBA image
    pub fn from_float_image(
        rgb: &[f32],
        width: usize,
        height: usize,
        num_channels: usize,
        bit_depth: u8,
    ) -> Self {
        let mut img = Self::new(width, height, num_channels, bit_depth);
        let max_value = (1 << bit_depth) - 1;

        for i in 0..(width * height) {
            for c in 0..num_channels {
                let value = (rgb[i * num_channels + c] * max_value as f32).round() as i32;
                let clamped = value.max(0).min(max_value);
                img.channels[c][i] = clamped;
            }
        }

        img
    }

    /// Convert to float RGB/RGBA image
    pub fn to_float_image(&self) -> Vec<f32> {
        let pixel_count = self.width * self.height;
        let mut rgb = vec![0.0f32; pixel_count * self.num_channels];

        for c in 0..self.num_channels {
            let max_value = (1 << self.bit_depths[c]) - 1;
            for i in 0..pixel_count {
                rgb[i * self.num_channels + c] =
                    self.channels[c][i] as f32 / max_value as f32;
            }
        }

        rgb
    }
}

/// Predictor types for modular encoding
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Predictor {
    /// Zero predictor (predict 0)
    Zero = 0,
    /// Left neighbor
    Left = 1,
    /// Top neighbor
    Top = 2,
    /// Average of left and top
    Average = 3,
    /// Paeth predictor (PNG-style)
    Paeth = 4,
    /// Gradient predictor
    Gradient = 5,
    /// Weighted predictor
    Weighted = 6,
}

impl Predictor {
    /// Get all predictors
    pub fn all() -> &'static [Predictor] {
        &[
            Predictor::Zero,
            Predictor::Left,
            Predictor::Top,
            Predictor::Average,
            Predictor::Paeth,
            Predictor::Gradient,
            Predictor::Weighted,
        ]
    }
}

/// Predict pixel value based on neighbors
///
/// This exploits spatial correlation by predicting each pixel
/// from its already-decoded neighbors.
pub fn predict_pixel(
    channel: &[i32],
    width: usize,
    x: usize,
    y: usize,
    predictor: Predictor,
) -> i32 {
    let idx = y * width + x;

    // Get neighbors (with boundary handling)
    let left = if x > 0 { channel[idx - 1] } else { 0 };
    let top = if y > 0 { channel[idx - width] } else { 0 };
    let top_left = if x > 0 && y > 0 {
        channel[idx - width - 1]
    } else {
        0
    };

    match predictor {
        Predictor::Zero => 0,
        Predictor::Left => left,
        Predictor::Top => top,
        Predictor::Average => (left + top) / 2,
        Predictor::Paeth => paeth_predictor(left, top, top_left),
        Predictor::Gradient => left + top - top_left,
        Predictor::Weighted => {
            // Weighted average based on gradients
            let grad_left = (left - top_left).abs();
            let grad_top = (top - top_left).abs();

            if grad_left < grad_top {
                left
            } else if grad_top < grad_left {
                top
            } else {
                (left + top) / 2
            }
        }
    }
}

/// Paeth predictor (used in PNG)
///
/// Selects left, top, or top-left based on which is closest to the
/// linear prediction left + top - top_left.
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

/// Encode modular image using predictive coding
///
/// Returns residuals (prediction errors) for each channel.
/// Residuals have zero mean and are more compressible than raw pixels.
pub fn encode_predictive(image: &ModularImage, predictor: Predictor) -> Vec<Vec<i32>> {
    let mut residuals = Vec::with_capacity(image.num_channels);

    for c in 0..image.num_channels {
        let channel = &image.channels[c];
        let mut channel_residuals = vec![0i32; image.width * image.height];

        for y in 0..image.height {
            for x in 0..image.width {
                let idx = y * image.width + x;
                let actual = channel[idx];
                let predicted = predict_pixel(channel, image.width, x, y, predictor);
                channel_residuals[idx] = actual - predicted;
            }
        }

        residuals.push(channel_residuals);
    }

    residuals
}

/// Decode modular image from predictive residuals
pub fn decode_predictive(
    residuals: &[Vec<i32>],
    width: usize,
    height: usize,
    predictor: Predictor,
    bit_depths: &[u8],
) -> ModularImage {
    let num_channels = residuals.len();
    let mut image = ModularImage::new(width, height, num_channels, bit_depths[0]);
    image.bit_depths = bit_depths.to_vec();

    for c in 0..num_channels {
        let max_value = (1 << bit_depths[c]) - 1;

        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let predicted = predict_pixel(&image.channels[c], width, x, y, predictor);
                let residual = residuals[c][idx];
                let reconstructed = predicted + residual;

                // Clamp to valid range
                image.channels[c][idx] = reconstructed.max(0).min(max_value);
            }
        }
    }

    image
}

/// Select best predictor for a channel
///
/// Tries all predictors and selects the one that produces smallest residuals.
/// This improves compression ratio at the cost of encoding complexity.
pub fn select_best_predictor(channel: &[i32], width: usize, height: usize) -> Predictor {
    let mut best_predictor = Predictor::Left;
    let mut best_score = i64::MAX;

    for &predictor in Predictor::all() {
        let mut score = 0i64;

        // Compute sum of absolute residuals
        for y in 0..height {
            for x in 0..width {
                let idx = y * width + x;
                let actual = channel[idx];
                let predicted = predict_pixel(channel, width, x, y, predictor);
                score += (actual - predicted).abs() as i64;

                // Early exit if this predictor is clearly worse
                if score > best_score {
                    break;
                }
            }
            if score > best_score {
                break;
            }
        }

        if score < best_score {
            best_score = score;
            best_predictor = predictor;
        }
    }

    best_predictor
}

/// Modular mode encoder options
#[derive(Debug, Clone)]
pub struct ModularOptions {
    /// Predictor to use (None = auto-select)
    pub predictor: Option<Predictor>,
    /// Whether to use adaptive predictor selection
    pub adaptive: bool,
    /// Color decorrelation transform
    pub use_color_transform: bool,
}

impl Default for ModularOptions {
    fn default() -> Self {
        Self {
            predictor: None, // Auto-select
            adaptive: true,
            use_color_transform: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_modular_image_creation() {
        let img = ModularImage::new(64, 64, 3, 8);
        assert_eq!(img.width, 64);
        assert_eq!(img.height, 64);
        assert_eq!(img.num_channels, 3);
        assert_eq!(img.channels.len(), 3);
        assert_eq!(img.channels[0].len(), 64 * 64);
    }

    #[test]
    fn test_pixel_access() {
        let mut img = ModularImage::new(4, 4, 1, 8);
        img.set_pixel(0, 2, 3, 128).unwrap();
        assert_eq!(img.get_pixel(0, 2, 3).unwrap(), 128);
    }

    #[test]
    fn test_predictive_roundtrip() {
        let mut img = ModularImage::new(8, 8, 1, 8);

        // Create gradient pattern
        for y in 0..8 {
            for x in 0..8 {
                img.set_pixel(0, x, y, (x + y) as i32 * 10).unwrap();
            }
        }

        // Encode and decode
        let residuals = encode_predictive(&img, Predictor::Gradient);
        let decoded = decode_predictive(&residuals, 8, 8, Predictor::Gradient, &[8]);

        // Check roundtrip accuracy
        for y in 0..8 {
            for x in 0..8 {
                let original = img.get_pixel(0, x, y).unwrap();
                let restored = decoded.get_pixel(0, x, y).unwrap();
                assert_eq!(original, restored);
            }
        }
    }

    #[test]
    fn test_predictor_selection() {
        let mut channel = vec![0i32; 8 * 8];

        // Create pattern that works well with left predictor
        for y in 0..8 {
            for x in 0..8 {
                channel[y * 8 + x] = x as i32 * 10;
            }
        }

        let predictor = select_best_predictor(&channel, 8, 8);

        // Verify that the selected predictor produces better compression than Zero
        let mut score_selected = 0i64;
        let mut score_zero = 0i64;

        for y in 0..8 {
            for x in 0..8 {
                let idx = y * 8 + x;
                let actual = channel[idx];
                let predicted_selected = predict_pixel(&channel, 8, x, y, predictor);
                score_selected += (actual - predicted_selected).abs() as i64;
                score_zero += actual.abs() as i64;
            }
        }

        // Selected predictor should be better than Zero
        assert!(
            score_selected < score_zero,
            "Selected predictor {:?} (score={}) should be better than Zero (score={})",
            predictor,
            score_selected,
            score_zero
        );
    }

    #[test]
    fn test_paeth_predictor() {
        // Test Paeth predictor edge cases
        // paeth(a, b, c) predicts based on p = a + b - c
        // and selects a, b, or c based on which is closest to p

        // p = 10 + 20 - 15 = 15, so pa=5, pb=5, pc=0 -> c is closest
        assert_eq!(paeth_predictor(10, 20, 15), 15);

        // p = 10 + 10 - 5 = 15, so pa=5, pb=5, pc=10 -> a is closest (tie, choose a)
        assert_eq!(paeth_predictor(10, 10, 5), 10);

        // p = 20 + 10 - 10 = 20, so pa=0, pb=10, pc=10 -> a is closest
        assert_eq!(paeth_predictor(20, 10, 10), 20);
    }

    #[test]
    fn test_float_conversion_roundtrip() {
        let width = 4;
        let height = 4;
        let num_channels = 3;
        let pixel_count = width * height;

        // Create test float image
        let mut rgb = vec![0.0f32; pixel_count * num_channels];
        for i in 0..pixel_count {
            rgb[i * 3] = ((i % 4) as f32) / 3.0;
            rgb[i * 3 + 1] = ((i / 4) as f32) / 3.0;
            rgb[i * 3 + 2] = 0.5;
        }

        // Convert to modular and back
        let img = ModularImage::from_float_image(&rgb, width, height, num_channels, 8);
        let rgb_back = img.to_float_image();

        // Check roundtrip (allow small error due to quantization)
        for i in 0..(pixel_count * num_channels) {
            let diff = (rgb[i] - rgb_back[i]).abs();
            assert!(diff < 0.01, "Roundtrip error at {}: {} vs {}", i, rgb[i], rgb_back[i]);
        }
    }

    #[test]
    fn test_all_predictors() {
        let mut img = ModularImage::new(8, 8, 1, 8);

        // Create some pattern
        for y in 0..8 {
            for x in 0..8 {
                img.set_pixel(0, x, y, ((x * 3 + y * 5) % 64) as i32).unwrap();
            }
        }

        // Test all predictors roundtrip
        for &predictor in Predictor::all() {
            let residuals = encode_predictive(&img, predictor);
            let decoded = decode_predictive(&residuals, 8, 8, predictor, &[8]);

            for y in 0..8 {
                for x in 0..8 {
                    assert_eq!(
                        img.get_pixel(0, x, y).unwrap(),
                        decoded.get_pixel(0, x, y).unwrap(),
                        "Predictor {:?} failed at ({}, {})",
                        predictor,
                        x,
                        y
                    );
                }
            }
        }
    }
}
