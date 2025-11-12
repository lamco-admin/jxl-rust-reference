//! Color correlation transforms
//!
//! These transforms exploit correlation between color channels to improve compression.

/// Apply YCoCg color transform (lossless)
pub fn apply_ycocg(rgb: &[i32], ycocg: &mut [i32]) {
    assert_eq!(rgb.len(), ycocg.len());
    assert_eq!(rgb.len() % 3, 0);

    for i in (0..rgb.len()).step_by(3) {
        let r = rgb[i];
        let g = rgb[i + 1];
        let b = rgb[i + 2];

        let co = r - b;
        let t = b + (co >> 1);
        let cg = g - t;
        let y = t + (cg >> 1);

        ycocg[i] = y; // Y
        ycocg[i + 1] = co; // Co
        ycocg[i + 2] = cg; // Cg
    }
}

/// Reverse YCoCg color transform
pub fn reverse_ycocg(ycocg: &[i32], rgb: &mut [i32]) {
    assert_eq!(rgb.len(), ycocg.len());
    assert_eq!(rgb.len() % 3, 0);

    for i in (0..rgb.len()).step_by(3) {
        let y = ycocg[i];
        let co = ycocg[i + 1];
        let cg = ycocg[i + 2];

        let t = y - (cg >> 1);
        let g = cg + t;
        let b = t - (co >> 1);
        let r = b + co;

        rgb[i] = r;
        rgb[i + 1] = g;
        rgb[i + 2] = b;
    }
}

/// Apply color decorrelation transform (for lossy compression)
pub fn decorrelate_channels(channels: &mut [f32], width: usize, height: usize) {
    // Simple color decorrelation: predict green from red and blue
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            let r = channels[idx];
            let g = channels[idx + 1];
            let b = channels[idx + 2];

            // Predict green from red and blue
            let g_predicted = (r + b) * 0.5;
            channels[idx + 1] = g - g_predicted;
        }
    }
}

/// Reverse color decorrelation transform
pub fn correlate_channels(channels: &mut [f32], width: usize, height: usize) {
    for y in 0..height {
        for x in 0..width {
            let idx = (y * width + x) * 3;
            let r = channels[idx];
            let g_residual = channels[idx + 1];
            let b = channels[idx + 2];

            // Reconstruct green
            let g_predicted = (r + b) * 0.5;
            channels[idx + 1] = g_residual + g_predicted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ycocg_roundtrip() {
        let rgb = vec![100, 150, 200, 50, 75, 100];
        let mut ycocg = vec![0; 6];
        let mut rgb2 = vec![0; 6];

        apply_ycocg(&rgb, &mut ycocg);
        reverse_ycocg(&ycocg, &mut rgb2);

        assert_eq!(rgb, rgb2);
    }

    #[test]
    fn test_decorrelate_roundtrip() {
        let mut channels = vec![0.5, 0.7, 0.3, 0.2, 0.4, 0.6];
        let original = channels.clone();

        decorrelate_channels(&mut channels, 2, 1);
        correlate_channels(&mut channels, 2, 1);

        for (a, b) in original.iter().zip(channels.iter()) {
            assert!((a - b).abs() < 0.0001);
        }
    }
}
