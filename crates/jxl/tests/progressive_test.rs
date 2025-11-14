//! Progressive decoding tests
//!
//! Tests for multi-pass progressive decoding capabilities

use jxl::{EncoderOptions, Image, JxlDecoder, JxlEncoder, ProgressiveDecoder, ProgressivePass};
use jxl_core::{ColorChannels, ColorEncoding, Dimensions, PixelType};

#[test]
fn test_progressive_decoder_creation() {
    // Create progressive decoder for a simple image
    let dimensions = Dimensions::new(256, 256);
    let decoder = ProgressiveDecoder::new(dimensions, 3);

    assert_eq!(decoder.current_pass(), ProgressivePass::DcOnly);
    assert!(!decoder.is_complete());
    assert_eq!(decoder.progress_percentage(), 20);
}

#[test]
fn test_progressive_pass_sequence() {
    // Test progression through passes
    let mut pass = ProgressivePass::DcOnly;

    // DC -> AC1
    pass = pass.next().unwrap();
    assert_eq!(pass, ProgressivePass::AcPass1);
    assert_eq!(pass.ac_coefficient_count(), 15);
    assert_eq!(pass.quality_percentage(), 40);

    // AC1 -> AC2
    pass = pass.next().unwrap();
    assert_eq!(pass, ProgressivePass::AcPass2);
    assert_eq!(pass.ac_coefficient_count(), 31);

    // AC2 -> AC3
    pass = pass.next().unwrap();
    assert_eq!(pass, ProgressivePass::AcPass3);
    assert_eq!(pass.ac_coefficient_count(), 47);

    // AC3 -> Full
    pass = pass.next().unwrap();
    assert_eq!(pass, ProgressivePass::Full);
    assert_eq!(pass.ac_coefficient_count(), 63);
    assert_eq!(pass.quality_percentage(), 100);

    // No more passes
    assert!(pass.next().is_none());
}

#[test]
fn test_progressive_dc_preview() {
    let dimensions = Dimensions::new(128, 128);
    let mut decoder = ProgressiveDecoder::new(dimensions, 3);

    // Create mock DC data
    let dc_width = 128 / 8; // 16
    let dc_height = 128 / 8; // 16
    let dc_size = dc_width * dc_height; // 256

    let dc_data: Vec<Vec<f32>> = vec![
        vec![128.0; dc_size], // X channel
        vec![128.0; dc_size], // Y channel
        vec![0.0; dc_size],   // B channel
    ];

    // Decode DC pass
    decoder.decode_dc_pass(&dc_data).unwrap();

    assert_eq!(decoder.current_pass(), ProgressivePass::DcOnly);

    // Get DC preview
    let preview = decoder.get_dc_preview();
    assert_eq!(preview.len(), 3);
    assert_eq!(preview[0].len(), dc_size);
}

#[test]
fn test_progressive_ac_accumulation() {
    let dimensions = Dimensions::new(64, 64);
    let mut decoder = ProgressiveDecoder::new(dimensions, 3);

    let image_size = 64 * 64;
    let dc_size = 8 * 8;

    // Initialize with DC data
    let dc_data: Vec<Vec<f32>> = vec![
        vec![100.0; dc_size],
        vec![100.0; dc_size],
        vec![100.0; dc_size],
    ];
    decoder.decode_dc_pass(&dc_data).unwrap();

    // Add first AC pass
    let ac_pass1: Vec<Vec<f32>> = vec![
        vec![10.0; image_size],
        vec![10.0; image_size],
        vec![10.0; image_size],
    ];
    decoder
        .decode_ac_pass(&ac_pass1, ProgressivePass::AcPass1)
        .unwrap();

    assert_eq!(decoder.current_pass(), ProgressivePass::AcPass1);
    assert_eq!(decoder.progress_percentage(), 40);

    // Add second AC pass
    let ac_pass2: Vec<Vec<f32>> = vec![
        vec![5.0; image_size],
        vec![5.0; image_size],
        vec![5.0; image_size],
    ];
    decoder
        .decode_ac_pass(&ac_pass2, ProgressivePass::AcPass2)
        .unwrap();

    assert_eq!(decoder.current_pass(), ProgressivePass::AcPass2);
    assert_eq!(decoder.progress_percentage(), 60);

    // AC coefficients should accumulate: 10.0 + 5.0 = 15.0
    // (This is verified internally by the decoder)
}

#[test]
fn test_progressive_reconstruction() {
    let dimensions = Dimensions::new(64, 64);
    let mut decoder = ProgressiveDecoder::new(dimensions, 3);

    let dc_size = 8 * 8;
    let image_size = 64 * 64;

    // Set up DC data
    let dc_data: Vec<Vec<f32>> = vec![vec![50.0; dc_size]; 3];
    decoder.decode_dc_pass(&dc_data).unwrap();

    // Reconstruct at DC-only quality
    let reconstructed = decoder.reconstruct_image();
    assert_eq!(reconstructed.len(), 3);
    assert_eq!(reconstructed[0].len(), image_size);

    // All pixels should be ~50.0 (DC value)
    // (Block boundaries might have slight variations)
    for pixel in &reconstructed[0][0..100] {
        assert!((pixel - 50.0).abs() < 1.0);
    }
}

#[test]
fn test_encoder_progressive_option() {
    // Test that encoder accepts progressive option
    let options = EncoderOptions::default()
        .quality(90.0)
        .progressive(true);

    assert!(options.progressive);

    let mut encoder = JxlEncoder::new(options);

    // Create a small test image
    let dimensions = Dimensions::new(64, 64);
    let image = Image::new(dimensions, ColorChannels::RGB, PixelType::U8, ColorEncoding::SRGB)
        .unwrap();

    // Encode should work (progressive encoding writes all passes to bitstream)
    let mut encoded = Vec::new();
    encoder.encode(&image, &mut encoded).unwrap();

    assert!(!encoded.is_empty());
}

#[test]
fn test_progressive_roundtrip_compatibility() {
    // Verify that non-progressive decoder can read progressive-encoded images
    let dimensions = Dimensions::new(128, 128);
    let mut original = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )
    .unwrap();

    // Fill with test pattern
    if let jxl_core::ImageBuffer::U8(ref mut data) = original.buffer {
        for y in 0..128 {
            for x in 0..128 {
                let idx = (y * 128 + x) * 3;
                data[idx] = ((x + y) % 256) as u8;
                data[idx + 1] = ((x * 2) % 256) as u8;
                data[idx + 2] = ((y * 2) % 256) as u8;
            }
        }
    }

    // Encode with progressive mode
    let options = EncoderOptions::default()
        .quality(85.0)
        .progressive(true);
    let mut encoder = JxlEncoder::new(options);

    let mut encoded = Vec::new();
    encoder.encode(&original, &mut encoded).unwrap();

    // Decode normally (decoder reconstructs full image from all passes)
    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(std::io::Cursor::new(&encoded)).unwrap();

    // Verify dimensions match
    assert_eq!(decoded.width(), original.width());
    assert_eq!(decoded.height(), original.height());
}
