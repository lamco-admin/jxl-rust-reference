//! Lossless encoding tests
//!
//! Tests for modular mode lossless encoding and decoding

use jxl::{EncoderOptions, Image, JxlDecoder, JxlEncoder};
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

#[test]
fn test_lossless_roundtrip_solid_color() {
    // Test perfect lossless roundtrip with solid color
    let dimensions = Dimensions::new(32, 32);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with solid color
    if let ImageBuffer::U8(ref mut data) = original.buffer {
        for i in 0..32 * 32 * 3 {
            data[i] = 200; // Solid color
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("Solid color encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U8(orig_data), ImageBuffer::U8(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..32 * 32 * 3 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    }

    println!("✓ Lossless roundtrip: solid color perfect reconstruction");
}

#[test]
fn test_lossless_roundtrip_gradient() {
    // Test lossless roundtrip with gradient pattern
    let dimensions = Dimensions::new(64, 64);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with gradient pattern
    if let ImageBuffer::U8(ref mut data) = original.buffer {
        for y in 0..64 {
            for x in 0..64 {
                let idx = (y * 64 + x) * 3;
                data[idx] = ((x * 4) % 256) as u8;       // R gradient
                data[idx + 1] = ((y * 4) % 256) as u8;   // G gradient
                data[idx + 2] = 128;                     // B constant
            }
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("Gradient encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U8(orig_data), ImageBuffer::U8(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..64 * 64 * 3 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    }

    println!("✓ Lossless roundtrip: gradient perfect reconstruction");
}

#[test]
fn test_lossless_roundtrip_random_pattern() {
    // Test lossless roundtrip with pseudo-random pattern
    let dimensions = Dimensions::new(48, 48);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with pseudo-random pattern
    if let ImageBuffer::U8(ref mut data) = original.buffer {
        for i in 0..48 * 48 * 3 {
            // Use different seeds for different channels
            let channel = i % 3;
            let pixel = i / 3;
            data[i] = ((pixel * 73 + channel * 101) % 256) as u8;
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("Random pattern encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U8(orig_data), ImageBuffer::U8(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..48 * 48 * 3 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    }

    println!("✓ Lossless roundtrip: random pattern perfect reconstruction");
}

#[test]
fn test_lossless_roundtrip_edges() {
    // Test lossless with extreme values (0, 255)
    let dimensions = Dimensions::new(32, 32);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with checkerboard of extreme values
    if let ImageBuffer::U8(ref mut data) = original.buffer {
        for y in 0..32 {
            for x in 0..32 {
                let idx = (y * 32 + x) * 3;
                let is_black = (x + y) % 2 == 0;
                data[idx] = if is_black { 0 } else { 255 };     // R
                data[idx + 1] = if is_black { 255 } else { 0 }; // G
                data[idx + 2] = if is_black { 0 } else { 255 }; // B
            }
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("Edge values encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U8(orig_data), ImageBuffer::U8(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..32 * 32 * 3 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    }

    println!("✓ Lossless roundtrip: edge values perfect reconstruction");
}

#[test]
fn test_lossless_roundtrip_all_values() {
    // Test all possible 8-bit values
    let dimensions = Dimensions::new(16, 16);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with sequential values across all channels
    if let ImageBuffer::U8(ref mut data) = original.buffer {
        for i in 0..16 * 16 * 3 {
            data[i] = (i % 256) as u8;
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("All values encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U8(orig_data), ImageBuffer::U8(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..16 * 16 * 3 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    }

    println!("✓ Lossless roundtrip: all values perfect reconstruction");
}

#[test]
fn test_lossless_roundtrip_16bit_gradient() {
    // Test 16-bit lossless roundtrip with gradient pattern
    let dimensions = Dimensions::new(32, 32);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U16,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with 16-bit gradient pattern
    if let ImageBuffer::U16(ref mut data) = original.buffer {
        for y in 0..32 {
            for x in 0..32 {
                let idx = (y * 32 + x) * 3;
                data[idx] = (x * 2048) as u16;          // R: 0-63488
                data[idx + 1] = (y * 2048) as u16;      // G: 0-63488
                data[idx + 2] = 32768;                  // B: constant mid-value
            }
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("16-bit gradient encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U16(orig_data), ImageBuffer::U16(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..32 * 32 * 3 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    } else {
        panic!("Expected U16 buffers");
    }

    println!("✓ Lossless roundtrip: 16-bit gradient perfect reconstruction");
}

#[test]
fn test_lossless_roundtrip_16bit_extremes() {
    // Test 16-bit lossless with extreme values (0, 65535)
    let dimensions = Dimensions::new(24, 24);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U16,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with checkerboard of extreme 16-bit values
    if let ImageBuffer::U16(ref mut data) = original.buffer {
        for y in 0..24 {
            for x in 0..24 {
                let idx = (y * 24 + x) * 3;
                let is_black = (x + y) % 2 == 0;
                data[idx] = if is_black { 0 } else { 65535 };     // R
                data[idx + 1] = if is_black { 65535 } else { 0 }; // G
                data[idx + 2] = if is_black { 0 } else { 65535 }; // B
            }
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("16-bit extremes encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U16(orig_data), ImageBuffer::U16(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..24 * 24 * 3 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    } else {
        panic!("Expected U16 buffers");
    }

    println!("✓ Lossless roundtrip: 16-bit extremes perfect reconstruction");
}

#[test]
fn test_lossless_roundtrip_16bit_high_frequency() {
    // Test 16-bit with high-frequency pattern (challenging for predictive coding)
    let dimensions = Dimensions::new(32, 32);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U16,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with pseudo-random 16-bit pattern
    if let ImageBuffer::U16(ref mut data) = original.buffer {
        for i in 0..32 * 32 * 3 {
            // Use different seeds for different channels
            let channel = i % 3;
            let pixel = i / 3;
            // Generate values across full 16-bit range
            data[i] = ((pixel * 2731 + channel * 4909) % 65536) as u16;
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("16-bit high frequency encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U16(orig_data), ImageBuffer::U16(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..32 * 32 * 3 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    } else {
        panic!("Expected U16 buffers");
    }

    println!("✓ Lossless roundtrip: 16-bit high frequency perfect reconstruction");
}

#[test]
fn test_lossless_roundtrip_rgba() {
    // Test RGBA (4-channel) lossless encoding with alpha channel
    let dimensions = Dimensions::new(32, 32);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGBA,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with gradient pattern including varying alpha
    if let ImageBuffer::U8(ref mut data) = original.buffer {
        for y in 0..32 {
            for x in 0..32 {
                let idx = (y * 32 + x) * 4;
                data[idx] = ((x * 8) % 256) as u8;           // R
                data[idx + 1] = ((y * 8) % 256) as u8;       // G
                data[idx + 2] = ((x + y) * 4 % 256) as u8;   // B
                data[idx + 3] = 128;  // Solid alpha for simplicity
            }
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("RGBA encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U8(orig_data), ImageBuffer::U8(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..32 * 32 * 4 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    } else {
        panic!("Expected U8 buffers");
    }

    println!("✓ Lossless roundtrip: RGBA (with alpha) perfect reconstruction");
}

#[test]
fn test_lossless_roundtrip_rgba_16bit() {
    // Test RGBA 16-bit lossless encoding with alpha channel
    let dimensions = Dimensions::new(24, 24);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGBA,
        PixelType::U16,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with pattern including full 16-bit alpha range
    if let ImageBuffer::U16(ref mut data) = original.buffer {
        for y in 0..24 {
            for x in 0..24 {
                let idx = (y * 24 + x) * 4;
                data[idx] = ((x * 2731) % 65536) as u16;        // R
                data[idx + 1] = ((y * 4909) % 65536) as u16;    // G
                data[idx + 2] = ((x + y) * 1823 % 65536) as u16; // B
                data[idx + 3] = ((x * y + x) % 65536) as u16;   // A (full 16-bit range)
            }
        }
    }

    // Encode lossless
    let options = EncoderOptions::default().lossless(true);
    let mut encoder = JxlEncoder::new(options);
    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    println!("RGBA 16-bit encoded to {} bytes", encoded.len());

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(encoded.as_slice()).unwrap();

    // Verify perfect reconstruction
    if let (ImageBuffer::U16(orig_data), ImageBuffer::U16(dec_data)) =
        (&original.buffer, &decoded.buffer) {
        for i in 0..24 * 24 * 4 {
            assert_eq!(
                orig_data[i], dec_data[i],
                "Pixel mismatch at index {} (expected {}, got {})",
                i, orig_data[i], dec_data[i]
            );
        }
    } else {
        panic!("Expected U16 buffers");
    }

    println!("✓ Lossless roundtrip: RGBA 16-bit (with alpha) perfect reconstruction");
}
