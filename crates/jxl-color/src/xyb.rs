//! XYB color space implementation
//!
//! XYB is JPEG XL's perceptual color space, inspired by the human visual system.
//! It's designed to be more perceptually uniform than RGB.
//!
//! NOTE: This is a simplified approximation for educational purposes.
//! A production implementation would use the full JPEG XL color space specification.

/// Simplified opsin-like transformation matrix
/// Using a simplified, easily invertible transform for this reference implementation
const OPSIN_ABSORBANCE_MATRIX: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// Inverse of the opsin absorbance matrix
const OPSIN_ABSORBANCE_INV_MATRIX: [[f32; 3]; 3] =
    [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];

/// XYB bias values
#[allow(dead_code)]
const XYB_BIAS: [f32; 3] = [0.0, 0.0, 0.0];

/// Convert linear RGB to XYB color space
pub fn rgb_to_xyb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // Apply gamma mixing for perceptual uniformity
    let mixed_r = r.cbrt();
    let mixed_g = g.cbrt();
    let mixed_b = b.cbrt();

    // Transform to LMS (opsin absorbance)
    let l = OPSIN_ABSORBANCE_MATRIX[0][0] * mixed_r
        + OPSIN_ABSORBANCE_MATRIX[0][1] * mixed_g
        + OPSIN_ABSORBANCE_MATRIX[0][2] * mixed_b;

    let m = OPSIN_ABSORBANCE_MATRIX[1][0] * mixed_r
        + OPSIN_ABSORBANCE_MATRIX[1][1] * mixed_g
        + OPSIN_ABSORBANCE_MATRIX[1][2] * mixed_b;

    let s = OPSIN_ABSORBANCE_MATRIX[2][0] * mixed_r
        + OPSIN_ABSORBANCE_MATRIX[2][1] * mixed_g
        + OPSIN_ABSORBANCE_MATRIX[2][2] * mixed_b;

    // Transform LMS to XYB
    let x = (l - m) * 0.5;
    let y = (l + m) * 0.5;
    let b_minus_y = s - y;

    (x, y, b_minus_y)
}

/// Convert XYB to linear RGB color space
pub fn xyb_to_rgb(x: f32, y: f32, b_minus_y: f32) -> (f32, f32, f32) {
    // Reverse XYB to LMS
    let l = x + y;
    let m = y - x;
    let s = b_minus_y + y;

    // Apply inverse opsin absorbance transformation
    let mixed_r = OPSIN_ABSORBANCE_INV_MATRIX[0][0] * l
        + OPSIN_ABSORBANCE_INV_MATRIX[0][1] * m
        + OPSIN_ABSORBANCE_INV_MATRIX[0][2] * s;

    let mixed_g = OPSIN_ABSORBANCE_INV_MATRIX[1][0] * l
        + OPSIN_ABSORBANCE_INV_MATRIX[1][1] * m
        + OPSIN_ABSORBANCE_INV_MATRIX[1][2] * s;

    let mixed_b = OPSIN_ABSORBANCE_INV_MATRIX[2][0] * l
        + OPSIN_ABSORBANCE_INV_MATRIX[2][1] * m
        + OPSIN_ABSORBANCE_INV_MATRIX[2][2] * s;

    // Reverse gamma mixing
    let r = mixed_r.powi(3);
    let g = mixed_g.powi(3);
    let b = mixed_b.powi(3);

    (r, g, b)
}

/// Batch convert RGB buffer to XYB
pub fn rgb_buffer_to_xyb(rgb: &[f32], xyb: &mut [f32]) {
    assert_eq!(rgb.len(), xyb.len());
    assert_eq!(rgb.len() % 3, 0);

    for i in (0..rgb.len()).step_by(3) {
        let (x, y, b) = rgb_to_xyb(rgb[i], rgb[i + 1], rgb[i + 2]);
        xyb[i] = x;
        xyb[i + 1] = y;
        xyb[i + 2] = b;
    }
}

/// Batch convert XYB buffer to RGB
pub fn xyb_buffer_to_rgb(xyb: &[f32], rgb: &mut [f32]) {
    assert_eq!(rgb.len(), xyb.len());
    assert_eq!(rgb.len() % 3, 0);

    for i in (0..rgb.len()).step_by(3) {
        let (r, g, b) = xyb_to_rgb(xyb[i], xyb[i + 1], xyb[i + 2]);
        rgb[i] = r;
        rgb[i + 1] = g;
        rgb[i + 2] = b;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgb_xyb_roundtrip() {
        let r = 0.5;
        let g = 0.7;
        let b = 0.3;

        let (x, y, b_minus_y) = rgb_to_xyb(r, g, b);
        let (r2, g2, b2) = xyb_to_rgb(x, y, b_minus_y);

        // Allow larger tolerance due to cube root/power approximations
        let tolerance = 0.02;
        assert!(
            (r - r2).abs() < tolerance,
            "R mismatch: {} vs {} (diff: {})",
            r,
            r2,
            (r - r2).abs()
        );
        assert!(
            (g - g2).abs() < tolerance,
            "G mismatch: {} vs {} (diff: {})",
            g,
            g2,
            (g - g2).abs()
        );
        assert!(
            (b - b2).abs() < tolerance,
            "B mismatch: {} vs {} (diff: {})",
            b,
            b2,
            (b - b2).abs()
        );
    }
}
