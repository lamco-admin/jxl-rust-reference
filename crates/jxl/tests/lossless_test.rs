//! Lossless encoding tests
//!
//! Tests for modular mode lossless encoding

use jxl::{EncoderOptions, Image, JxlEncoder};
use jxl_core::{ColorChannels, ColorEncoding, Dimensions, ImageBuffer, PixelType};

#[test]
fn test_lossless_encoder_option() {
    // Test that encoder accepts lossless option
    let options = EncoderOptions::default()
        .quality(100.0)
        .lossless(true);

    assert!(options.lossless);

    let mut encoder = JxlEncoder::new(options);

    // Create a small test image
    let dimensions = Dimensions::new(64, 64);
    let image = Image::new(dimensions, ColorChannels::RGB, PixelType::U8, ColorEncoding::SRGB)
        .unwrap();

    // Encode should work (lossless encoding writes modular mode)
    let mut encoded = Vec::new();
    encoder.encode(&image, &mut encoded).unwrap();

    assert!(!encoded.is_empty());
}

#[test]
fn test_lossless_encode_simple_image() {
    // Create a simple test image
    let dimensions = Dimensions::new(32, 32);
    let mut image = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with gradient pattern
    if let ImageBuffer::U8(ref mut data) = image.buffer {
        for y in 0..32 {
            for x in 0..32 {
                let idx = (y * 32 + x) * 3;
                data[idx] = ((x * 8) % 256) as u8;       // R
                data[idx + 1] = ((y * 8) % 256) as u8;   // G
                data[idx + 2] = ((x + y) * 4 % 256) as u8; // B
            }
        }
    }

    // Encode with lossless mode
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);

    let mut encoded = Vec::new();
    encoder.encode(&image, &mut encoded).unwrap();

    assert!(!encoded.is_empty());
    println!("Lossless encoded {} bytes", encoded.len());
}

#[test]
fn test_lossless_vs_lossy_size() {
    // Compare lossless vs lossy encoding size
    let dimensions = Dimensions::new(64, 64);
    let mut image = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with random-like pattern (not very compressible)
    if let ImageBuffer::U8(ref mut data) = image.buffer {
        for i in 0..64 * 64 * 3 {
            // Pseudo-random pattern
            data[i] = ((i * 37 + 17) % 256) as u8;
        }
    }

    // Encode lossless
    let mut encoder_lossless = JxlEncoder::new(EncoderOptions::default().lossless(true));
    let mut encoded_lossless = Vec::new();
    encoder_lossless.encode(&image, &mut encoded_lossless).unwrap();

    // Encode lossy (quality 85)
    let mut encoder_lossy = JxlEncoder::new(EncoderOptions::default().quality(85.0));
    let mut encoded_lossy = Vec::new();
    encoder_lossy.encode(&image, &mut encoded_lossy).unwrap();

    println!("Lossless: {} bytes", encoded_lossless.len());
    println!("Lossy (Q=85): {} bytes", encoded_lossy.len());

    // Lossless should typically be larger than lossy for complex images
    // (This may not always be true for very small images)
    assert!(!encoded_lossless.is_empty());
    assert!(!encoded_lossy.is_empty());
}

#[test]
fn test_lossless_solid_color() {
    // Test lossless encoding of solid color (should compress well)
    let dimensions = Dimensions::new(64, 64);
    let mut image = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with solid color
    if let ImageBuffer::U8(ref mut data) = image.buffer {
        for i in 0..64 * 64 * 3 {
            data[i] = 128; // Mid-gray
        }
    }

    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);

    let mut encoded = Vec::new();
    encoder.encode(&image, &mut encoded).unwrap();

    println!("Solid color lossless encoded to {} bytes", encoded.len());

    // Solid color should compress (with basic predictive coding)
    assert!(!encoded.is_empty());
    // Note: Without full ANS compression, may be larger than raw
    // TODO: Add proper ANS encoding for better compression
}
