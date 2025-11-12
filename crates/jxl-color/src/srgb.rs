//! sRGB color space transformations

use num_traits::Float;

/// Convert sRGB to linear RGB (gamma expansion)
pub fn srgb_to_linear(srgb: f32) -> f32 {
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

/// Convert linear RGB to sRGB (gamma compression)
pub fn linear_to_srgb(linear: f32) -> f32 {
    if linear <= 0.0031308 {
        linear * 12.92
    } else {
        1.055 * linear.powf(1.0 / 2.4) - 0.055
    }
}

/// Convert sRGB buffer to linear RGB
pub fn srgb_buffer_to_linear(srgb: &[f32], linear: &mut [f32]) {
    assert_eq!(srgb.len(), linear.len());
    for (s, l) in srgb.iter().zip(linear.iter_mut()) {
        *l = srgb_to_linear(*s);
    }
}

/// Convert linear RGB buffer to sRGB
pub fn linear_buffer_to_srgb(linear: &[f32], srgb: &mut [f32]) {
    assert_eq!(srgb.len(), linear.len());
    for (l, s) in linear.iter().zip(srgb.iter_mut()) {
        *s = linear_to_srgb(*l);
    }
}

/// Convert 8-bit sRGB to linear f32
pub fn srgb_u8_to_linear_f32(srgb: u8) -> f32 {
    srgb_to_linear(srgb as f32 / 255.0)
}

/// Convert linear f32 to 8-bit sRGB
pub fn linear_f32_to_srgb_u8(linear: f32) -> u8 {
    (linear_to_srgb(linear) * 255.0).round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_linear_roundtrip() {
        let srgb = 0.5;
        let linear = srgb_to_linear(srgb);
        let srgb2 = linear_to_srgb(linear);
        assert!((srgb - srgb2).abs() < 0.0001);
    }

    #[test]
    fn test_u8_conversion() {
        let srgb_u8 = 128u8;
        let linear = srgb_u8_to_linear_f32(srgb_u8);
        let srgb_u8_2 = linear_f32_to_srgb_u8(linear);
        assert_eq!(srgb_u8, srgb_u8_2);
    }
}
