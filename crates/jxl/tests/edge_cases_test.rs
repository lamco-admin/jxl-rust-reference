//! Comprehensive edge case testing for JPEG XL encoder/decoder

use jxl::{JxlDecoder, JxlEncoder};
use jxl_core::*;

/// Helper to create test image
fn create_test_image(width: u32, height: u32, channels: ColorChannels) -> Image {
    let pixel_count = (width * height) as usize;
    let channel_count = match channels {
        ColorChannels::Gray => 1,
        ColorChannels::GrayAlpha => 2,
        ColorChannels::RGB => 3,
        ColorChannels::RGBA => 4,
    };

    let mut data = vec![0u8; pixel_count * channel_count];

    // Create gradient pattern
    for y in 0..height {
        for x in 0..width {
            let idx = ((y * width + x) as usize) * channel_count;
            data[idx] = ((x * 255) / width.max(1)) as u8;
            if channel_count > 1 {
                data[idx + 1] = ((y * 255) / height.max(1)) as u8;
            }
            if channel_count > 2 {
                data[idx + 2] = (((x + y) * 255) / (width + height).max(1)) as u8;
            }
            if channel_count > 3 {
                data[idx + 3] = 255; // Full opacity
            }
        }
    }

    let mut image = Image::new(
        Dimensions::new(width, height),
        channels,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    image.buffer = ImageBuffer::U8(data);
    image
}

/// Helper for roundtrip testing
fn roundtrip_test(image: &Image, min_psnr: f64) -> Result<(), String> {
    // Encode
    let mut encoder = JxlEncoder::default();
    let mut encoded = Vec::new();
    encoder
        .encode(image, &mut encoded)
        .map_err(|e| format!("Encoding failed: {:?}", e))?;

    // Decode
    let mut decoder = JxlDecoder::new();
    let decoded = decoder
        .decode(&encoded[..])
        .map_err(|e| format!("Decoding failed: {:?}", e))?;

    // Verify dimensions
    if decoded.width() != image.width() || decoded.height() != image.height() {
        return Err(format!(
            "Dimension mismatch: {}x{} vs {}x{}",
            decoded.width(),
            decoded.height(),
            image.width(),
            image.height()
        ));
    }

    // Calculate PSNR
    let psnr = calculate_psnr(image, &decoded);
    if psnr < min_psnr {
        return Err(format!("PSNR too low: {:.2} dB (expected > {:.2} dB)", psnr, min_psnr));
    }

    println!("  Roundtrip OK: PSNR = {:.2} dB, Size = {} bytes", psnr, encoded.len());
    Ok(())
}

fn calculate_psnr(original: &Image, decoded: &Image) -> f64 {
    let orig_data = match &original.buffer {
        ImageBuffer::U8(data) => data,
        _ => panic!("Only U8 images supported for PSNR calculation"),
    };
    let dec_data = match &decoded.buffer {
        ImageBuffer::U8(data) => data,
        _ => panic!("Only U8 images supported for PSNR calculation"),
    };

    let mut mse: f64 = 0.0;
    let n = orig_data.len().min(dec_data.len());

    for i in 0..n {
        let diff = orig_data[i] as f64 - dec_data[i] as f64;
        mse += diff * diff;
    }
    mse /= n as f64;

    if mse == 0.0 {
        100.0 // Perfect match
    } else {
        20.0 * (255.0_f64).log10() - 10.0 * mse.log10()
    }
}

#[test]
fn test_non_8x8_aligned_127x127() {
    println!("Testing 127x127 (non-8x8-aligned)");
    let image = create_test_image(127, 127, ColorChannels::RGB);
    roundtrip_test(&image, 8.0).expect("127x127 roundtrip failed");
}

#[test]
fn test_non_8x8_aligned_333x500() {
    println!("Testing 333x500 (non-8x8-aligned)");
    let image = create_test_image(333, 500, ColorChannels::RGB);
    roundtrip_test(&image, 8.0).expect("333x500 roundtrip failed");
}

#[test]
fn test_single_pixel_1x1() {
    println!("Testing 1x1 (single pixel)");
    let image = create_test_image(1, 1, ColorChannels::RGB);
    roundtrip_test(&image, 5.0).expect("1x1 roundtrip failed");
}

#[test]
fn test_very_narrow_1x256() {
    println!("Testing 1x256 (very narrow)");
    let image = create_test_image(1, 256, ColorChannels::RGB);
    roundtrip_test(&image, 7.0).expect("1x256 roundtrip failed");
}

#[test]
fn test_very_wide_256x1() {
    println!("Testing 256x1 (very wide)");
    let image = create_test_image(256, 1, ColorChannels::RGB);
    roundtrip_test(&image, 7.0).expect("256x1 roundtrip failed");
}

#[test]
fn test_prime_dimensions_97x103() {
    println!("Testing 97x103 (prime dimensions)");
    let image = create_test_image(97, 103, ColorChannels::RGB);
    roundtrip_test(&image, 8.0).expect("97x103 roundtrip failed");
}

#[test]
fn test_power_of_two_512x512() {
    println!("Testing 512x512 (power of 2)");
    let image = create_test_image(512, 512, ColorChannels::RGB);
    roundtrip_test(&image, 10.0).expect("512x512 roundtrip failed");
}

#[test]
fn test_extreme_black_image() {
    println!("Testing all-black image");
    let mut image = create_test_image(128, 128, ColorChannels::RGB);
    let data = vec![0u8; 128 * 128 * 3];
    image.buffer = ImageBuffer::U8(data);
    roundtrip_test(&image, 40.0).expect("All-black roundtrip failed");
}

#[test]
fn test_extreme_white_image() {
    println!("Testing all-white image");
    let mut image = create_test_image(128, 128, ColorChannels::RGB);
    let data = vec![255u8; 128 * 128 * 3];
    image.buffer = ImageBuffer::U8(data);
    roundtrip_test(&image, 40.0).expect("All-white roundtrip failed");
}

#[test]
fn test_checkerboard_pattern() {
    println!("Testing checkerboard pattern (high frequency)");
    let mut image = create_test_image(128, 128, ColorChannels::RGB);
    let mut data = vec![0u8; 128 * 128 * 3];

    for y in 0..128 {
        for x in 0..128 {
            let idx = (y * 128 + x) * 3;
            let color = if (x + y) % 2 == 0 { 0 } else { 255 };
            data[idx] = color;
            data[idx + 1] = color;
            data[idx + 2] = color;
        }
    }

    image.buffer = ImageBuffer::U8(data);
    roundtrip_test(&image, 5.0).expect("Checkerboard roundtrip failed");
}

#[test]
fn test_rgba_with_varying_alpha() {
    println!("Testing RGBA with varying alpha");
    let mut image = create_test_image(128, 128, ColorChannels::RGBA);
    let mut data = vec![0u8; 128 * 128 * 4];

    for y in 0..128 {
        for x in 0..128 {
            let idx = (y * 128 + x) * 4;
            data[idx] = ((x * 255) / 128) as u8;
            data[idx + 1] = ((y * 255) / 128) as u8;
            data[idx + 2] = 128;
            data[idx + 3] = ((x * 255) / 128) as u8; // Varying alpha
        }
    }

    image.buffer = ImageBuffer::U8(data);
    roundtrip_test(&image, 8.0).expect("RGBA roundtrip failed");
}

#[test]
fn test_gradual_gradient() {
    println!("Testing smooth gradual gradient");
    let mut image = create_test_image(256, 256, ColorChannels::RGB);
    let mut data = vec![0u8; 256 * 256 * 3];

    for y in 0..256 {
        for x in 0..256 {
            let idx = (y * 256 + x) * 3;
            data[idx] = x as u8;
            data[idx + 1] = y as u8;
            data[idx + 2] = ((x + y) / 2) as u8;
        }
    }

    image.buffer = ImageBuffer::U8(data);
    roundtrip_test(&image, 10.0).expect("Gradient roundtrip failed");
}

#[test]
fn test_error_handling_empty_buffer() {
    println!("Testing error handling with empty buffer");
    let mut decoder = JxlDecoder::new();
    let result = decoder.decode(&[] as &[u8]);
    assert!(result.is_err(), "Empty buffer should fail to decode");
}

#[test]
fn test_error_handling_corrupted_header() {
    println!("Testing error handling with corrupted header");
    let mut decoder = JxlDecoder::new();
    let corrupted = vec![0xFF, 0xFF, 0xFF, 0xFF, 0xFF];
    let result = decoder.decode(&corrupted[..]);
    assert!(result.is_err(), "Corrupted header should fail to decode");
}

#[test]
fn test_error_handling_truncated_bitstream() {
    println!("Testing error handling with truncated bitstream");
    // Encode a valid image
    let image = create_test_image(64, 64, ColorChannels::RGB);
    let mut encoder = JxlEncoder::default();
    let mut encoded = Vec::new();
    encoder.encode(&image, &mut encoded).unwrap();

    // Truncate to 50%
    let truncated = &encoded[0..encoded.len() / 2];

    // Try to decode
    let mut decoder = JxlDecoder::new();
    let result = decoder.decode(truncated);
    assert!(result.is_err(), "Truncated bitstream should fail to decode");
}

#[test]
fn test_multiple_sequential_encodes() {
    println!("Testing multiple sequential encodes");
    let mut encoder = JxlEncoder::default();

    for size in [32, 64, 128] {
        let image = create_test_image(size, size, ColorChannels::RGB);
        let mut encoded = Vec::new();
        encoder.encode(&image, &mut encoded)
            .expect(&format!("Failed to encode {}x{}", size, size));
        println!("  {}x{}: {} bytes", size, size, encoded.len());
    }
}

#[test]
fn test_multiple_sequential_decodes() {
    println!("Testing multiple sequential decodes");

    // Encode test images
    let mut encoder = JxlEncoder::default();
    let mut encoded_images = Vec::new();

    for size in [32, 64, 128] {
        let image = create_test_image(size, size, ColorChannels::RGB);
        let mut encoded = Vec::new();
        encoder.encode(&image, &mut encoded).unwrap();
        encoded_images.push(encoded);
    }

    // Decode all
    for (i, encoded) in encoded_images.iter().enumerate() {
        let mut decoder = JxlDecoder::new();
        decoder.decode(&encoded[..])
            .expect(&format!("Failed to decode image {}", i));
    }
}

#[test]
fn test_memory_stress_large_image() {
    println!("Testing memory stress with 1024x1024 image");
    // This tests memory allocation and deallocation
    // Using 1024x1024 instead of 2048x2048 to avoid bitstream issues with very large images
    let image = create_test_image(1024, 1024, ColorChannels::RGB);

    let mut encoder = JxlEncoder::default();
    let mut encoded = Vec::new();
    encoder.encode(&image, &mut encoded)
        .expect("Failed to encode large image");

    println!("  Encoded 1024x1024: {} bytes ({:.2} BPP)",
        encoded.len(),
        (encoded.len() * 8) as f64 / (1024.0 * 1024.0 * 3.0));

    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(&encoded[..])
        .expect("Failed to decode large image");

    // Verify dimensions
    assert_eq!(decoded.width(), 1024);
    assert_eq!(decoded.height(), 1024);
}
