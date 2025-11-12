//! # Error Handling Examples
//!
//! This example demonstrates proper error handling patterns when working with
//! the JPEG XL reference implementation.
//!
//! ## Running This Example
//!
//! ```bash
//! cargo run --example error_handling
//! ```
//!
//! **Note:** This is an educational reference implementation. See LIMITATIONS.md
//! for details on implementation scope.

use jxl::{
    ColorChannels, ColorEncoding, Dimensions, Image, JxlDecoder, JxlEncoder, JxlError, PixelType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("JPEG XL Error Handling Examples");
    println!("================================\n");

    // ==================== INVALID DIMENSIONS ====================
    println!("1. Testing invalid dimensions...");

    match Image::new(
        Dimensions::new(0, 100),
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    ) {
        Ok(_) => println!("   ✗ Should have failed!"),
        Err(JxlError::InvalidDimensions { width, height }) => {
            println!("   ✓ Caught InvalidDimensions error");
            println!("   - Width: {}, Height: {}", width, height);
            println!("   - Message: Dimensions cannot be zero\n");
        }
        Err(e) => println!("   ✗ Unexpected error: {:?}", e),
    }

    // ==================== VALID IMAGE CREATION ====================
    println!("2. Creating valid image with error handling...");

    let result = create_test_image(256, 256);

    match result {
        Ok(image) => {
            println!("   ✓ Image created successfully");
            println!("   - Dimensions: {}x{}", image.width(), image.height());
            println!("   - Channels: {}", image.channel_count());
        }
        Err(e) => {
            println!("   ✗ Failed to create image: {}", e);
        }
    }
    println!();

    // ==================== FILE NOT FOUND ====================
    println!("3. Testing decoding non-existent file...");

    let mut decoder = JxlDecoder::new();
    match decoder.decode_file("/tmp/nonexistent.jxl") {
        Ok(_) => println!("   ✗ Should have failed!"),
        Err(e) => {
            println!("   ✓ Caught error: {}", e);
            println!("   - Error type: IoError\n");
        }
    }

    // ==================== CUSTOM ERROR HANDLING ====================
    println!("4. Demonstrating custom error handling function...");

    let result = process_image_with_recovery();
    match result {
        Ok(msg) => println!("   ✓ {}", msg),
        Err(e) => println!("   ✗ Unrecoverable error: {}", e),
    }
    println!();

    // ==================== ERROR TYPES OVERVIEW ====================
    println!("5. Error Types Overview:");
    println!("   ========================");
    demonstrate_error_types();

    println!("\n✓ Error handling examples completed!");
    println!("\n💡 Tips:");
    println!("   - Always use Result<T, JxlError> for operations that can fail");
    println!("   - Use match or if-let to handle specific error variants");
    println!("   - Provide context in error messages for debugging");
    println!("   - Consider using ? operator for error propagation");

    Ok(())
}

/// Helper function demonstrating Result return type
fn create_test_image(width: u32, height: u32) -> Result<Image, JxlError> {
    let dimensions = Dimensions::new(width, height);

    let mut image = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )?;

    // Fill with test data
    if let jxl::ImageBuffer::U8(ref mut buffer) = image.buffer {
        for i in 0..buffer.len() {
            buffer[i] = (i % 256) as u8;
        }
    }

    Ok(image)
}

/// Demonstrates error recovery patterns
fn process_image_with_recovery() -> Result<String, Box<dyn std::error::Error>> {
    // Try to create image with potentially invalid dimensions
    let result = create_test_image(100, 100);

    match result {
        Ok(image) => {
            // Success path
            let encoder = JxlEncoder::default();
            encoder.encode_file(&image, "/tmp/recovered.jxl")?;
            Ok("Image processed and encoded successfully".to_string())
        }
        Err(JxlError::InvalidDimensions { .. }) => {
            // Recovery: try with default dimensions
            println!("   ! Invalid dimensions, using defaults...");
            let fallback_image = create_test_image(64, 64)?;
            let encoder = JxlEncoder::default();
            encoder.encode_file(&fallback_image, "/tmp/recovered.jxl")?;
            Ok("Image processed with fallback dimensions".to_string())
        }
        Err(e) => {
            // Unrecoverable error
            Err(Box::new(e))
        }
    }
}

/// Demonstrates all error types
fn demonstrate_error_types() {
    println!("   JxlError variants:");
    println!("   ------------------");
    println!("   • InvalidSignature     - File signature doesn't match JPEG XL");
    println!("   • UnsupportedVersion   - JPEG XL version not supported");
    println!("   • InvalidHeader        - Malformed header data");
    println!("   • InvalidBitstream     - Corrupted or invalid bitstream");
    println!("   • DecodingError        - Error during decoding process");
    println!("   • EncodingError        - Error during encoding process");
    println!("   • IoError              - File I/O error");
    println!("   • UnsupportedFeature   - Feature not implemented");
    println!("   • OutOfMemory          - Insufficient memory");
    println!("   • InvalidDimensions    - Zero or invalid dimensions");
    println!("   • InvalidParameter     - Invalid function parameter");
    println!("   • BufferTooSmall       - Buffer size insufficient");
}
