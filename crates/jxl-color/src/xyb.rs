//! XYB color space implementation
//!
//! XYB is JPEG XL's perceptual color space, inspired by the human visual system.
//! It's designed to be more perceptually uniform than RGB.
//!
//! This implementation uses the actual JPEG XL specification values for production use.

/// Opsin absorbance matrix from JPEG XL specification (libjxl production values)
/// These values model human cone cell sensitivity for perceptually uniform color space
const OPSIN_ABSORBANCE_MATRIX: [[f32; 3]; 3] = [
    [0.30, 0.622, 0.078],
    [0.23, 0.692, 0.078],
    [0.24342268924547819, 0.20476744424496821, 0.55180986650951361],
];

/// Inverse opsin absorbance matrix (from libjxl)
const OPSIN_ABSORBANCE_INV_MATRIX: [[f32; 3]; 3] = [
    [11.031566901960783, -9.866943921568629, -0.16462299647058826],
    [-3.254147380392157, 4.418770392156863, -0.16462299647058826],
    [-3.6588512862745097, 2.7129230470588235, 1.9459282392156863],
];

/// Opsin absorbance bias (from libjxl)
const OPSIN_ABSORBANCE_BIAS: f32 = 0.0037930732552754493;

/// Convert linear RGB to XYB color space (JPEG XL production algorithm)
pub fn rgb_to_xyb(r: f32, g: f32, b: f32) -> (f32, f32, f32) {
    // Step 1: Apply opsin absorbance matrix with bias
    let mixed0 = OPSIN_ABSORBANCE_MATRIX[0][0] * r
        + OPSIN_ABSORBANCE_MATRIX[0][1] * g
        + OPSIN_ABSORBANCE_MATRIX[0][2] * b
        + OPSIN_ABSORBANCE_BIAS;

    let mixed1 = OPSIN_ABSORBANCE_MATRIX[1][0] * r
        + OPSIN_ABSORBANCE_MATRIX[1][1] * g
        + OPSIN_ABSORBANCE_MATRIX[1][2] * b
        + OPSIN_ABSORBANCE_BIAS;

    let mixed2 = OPSIN_ABSORBANCE_MATRIX[2][0] * r
        + OPSIN_ABSORBANCE_MATRIX[2][1] * g
        + OPSIN_ABSORBANCE_MATRIX[2][2] * b
        + OPSIN_ABSORBANCE_BIAS;

    // Step 2: Clamp negative values to zero (can happen with bias)
    let mixed0 = mixed0.max(0.0);
    let mixed1 = mixed1.max(0.0);
    let mixed2 = mixed2.max(0.0);

    // Step 3: Apply cube root and remove bias
    let mixed0 = mixed0.cbrt() - OPSIN_ABSORBANCE_BIAS.cbrt();
    let mixed1 = mixed1.cbrt() - OPSIN_ABSORBANCE_BIAS.cbrt();
    let mixed2 = mixed2.cbrt() - OPSIN_ABSORBANCE_BIAS.cbrt();

    // Step 4: Transform to XYB
    let x = (mixed0 - mixed1) * 0.5;
    let y = (mixed0 + mixed1) * 0.5;
    let b_minus_y = mixed2;

    (x, y, b_minus_y)
}

/// Convert XYB to linear RGB color space (JPEG XL production algorithm)
pub fn xyb_to_rgb(x: f32, y: f32, b_minus_y: f32) -> (f32, f32, f32) {
    // Step 1: Reverse XYB to mixed (LMS-like) space
    let mixed0 = x + y;
    let mixed1 = y - x;
    let mixed2 = b_minus_y;

    // Step 2: Add back bias and cube
    let bias_cbrt = OPSIN_ABSORBANCE_BIAS.cbrt();
    let mixed0 = (mixed0 + bias_cbrt).powi(3) - OPSIN_ABSORBANCE_BIAS;
    let mixed1 = (mixed1 + bias_cbrt).powi(3) - OPSIN_ABSORBANCE_BIAS;
    let mixed2 = (mixed2 + bias_cbrt).powi(3) - OPSIN_ABSORBANCE_BIAS;

    // Step 3: Apply inverse opsin absorbance matrix
    let r = OPSIN_ABSORBANCE_INV_MATRIX[0][0] * mixed0
        + OPSIN_ABSORBANCE_INV_MATRIX[0][1] * mixed1
        + OPSIN_ABSORBANCE_INV_MATRIX[0][2] * mixed2;

    let g = OPSIN_ABSORBANCE_INV_MATRIX[1][0] * mixed0
        + OPSIN_ABSORBANCE_INV_MATRIX[1][1] * mixed1
        + OPSIN_ABSORBANCE_INV_MATRIX[1][2] * mixed2;

    let b = OPSIN_ABSORBANCE_INV_MATRIX[2][0] * mixed0
        + OPSIN_ABSORBANCE_INV_MATRIX[2][1] * mixed1
        + OPSIN_ABSORBANCE_INV_MATRIX[2][2] * mixed2;

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

        // Allow tolerance for cube root/power approximations and matrix precision
        let tolerance = 0.001;
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
