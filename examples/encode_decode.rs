//! # JPEG XL Reference Implementation - Encoding and Decoding Example
//!
//! This example demonstrates the basic API for encoding and decoding images using
//! the JPEG XL Rust reference implementation.
//!
//! **IMPORTANT:** This is an educational reference implementation. See LIMITATIONS.md
//! for details on what is and isn't implemented.
//!
//! ## What This Example Shows
//!
//! - Creating an Image with specified dimensions and format
//! - Filling an image buffer with pixel data
//! - Configuring encoder options (quality, effort)
//! - Encoding an image to a file
//! - Decoding an image from a file
//! - Accessing decoded image metadata
//!
//! ## Running This Example
//!
//! ```bash
//! cargo run --example encode_decode
//! ```
//!
//! ## Expected Output
//!
//! The example creates a 256×256 RGB gradient test image, encodes it to `/tmp/test_output.jxl`,
//! and then decodes it back, printing information about the decoded image.
//!
//! ## Note on Implementation Status
//!
//! This reference implementation uses simplified encoding/decoding that writes/reads
//! raw pixel data. It demonstrates the API structure but does not produce compliant
//! JPEG XL files. For production use, see:
//! - [libjxl](https://github.com/libjxl/libjxl) - Official C++ implementation
//! - [jxl-oxide](https://github.com/tirr-c/jxl-oxide) - Production Rust decoder

use jxl::{
    ColorChannels, ColorEncoding, Dimensions, EncoderOptions, Image, JxlDecoder, JxlEncoder,
    PixelType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("JPEG XL Rust Reference Implementation - Example");
    println!("================================================");
    println!("⚠️  This is an EDUCATIONAL reference implementation");
    println!("📖 See LIMITATIONS.md for important scope information\n");

    // ==================== IMAGE CREATION ====================

    // Define image dimensions
    // JPEG XL supports images up to 2^28 pixels in each dimension
    let width = 256;
    let height = 256;
    let dimensions = Dimensions::new(width, height);

    println!("Creating a {}x{} test image...", width, height);

    // Create an Image with:
    // - RGB color (3 channels)
    // - 8-bit unsigned integer pixels (0-255)
    // - sRGB color space (standard for displays)
    let mut image = Image::new(
        dimensions,
        ColorChannels::RGB,    // 3 color channels
        PixelType::U8,         // 8-bit per channel
        ColorEncoding::SRGB,   // Standard RGB
    )?;

    println!("✓ Image structure created");

    // ==================== FILL WITH TEST PATTERN ====================

    // Fill the image buffer with a gradient pattern
    // The buffer is a flat Vec<u8> with pixel data in row-major order: RGBRGBRGB...
    if let jxl::ImageBuffer::U8(ref mut buffer) = image.buffer {
        println!("Filling image with gradient pattern...");

        for y in 0..height {
            for x in 0..width {
                // Calculate the starting index for this pixel
                // Each pixel has 3 components (R, G, B)
                let idx = ((y * width + x) * 3) as usize;

                // Create a gradient pattern:
                // - Red channel varies horizontally (increases left to right)
                // - Green channel varies vertically (increases top to bottom)
                // - Blue channel varies diagonally
                buffer[idx] = (x % 256) as u8;           // Red
                buffer[idx + 1] = (y % 256) as u8;       // Green
                buffer[idx + 2] = ((x + y) / 2 % 256) as u8;  // Blue
            }
        }
    }

    println!("✓ Image filled with test data");
    println!("  Total pixels: {}", width * height);
    println!("  Buffer size: {} bytes", width * height * 3);

    // ==================== ENCODING ====================

    println!("\n📝 Encoding to JPEG XL format...");

    // Configure encoder options
    // Quality: 0-100 (higher = better quality, larger file)
    // Effort: 1-9 (higher = slower encoding, better compression)
    let encoder_options = EncoderOptions::default()
        .quality(90.0)   // High quality
        .effort(5);      // Medium effort

    println!("  Quality: 90.0 (high quality)");
    println!("  Effort: 5 (medium speed/compression tradeoff)");

    // Create encoder with options
    let encoder = JxlEncoder::new(encoder_options);

    // Encode to file
    let output_path = "/tmp/test_output.jxl";
    encoder.encode_file(&image, output_path)?;

    println!("✓ Image encoded to: {}", output_path);

    // Show file size
    if let Ok(metadata) = std::fs::metadata(output_path) {
        println!("  File size: {} bytes", metadata.len());
    }

    // ==================== DECODING ====================

    println!("\n📖 Decoding from JPEG XL format...");

    // Create a decoder
    let mut decoder = JxlDecoder::new();

    // Decode the file we just created
    let decoded_image = decoder.decode_file(output_path)?;

    println!("✓ Image decoded successfully");

    // ==================== DISPLAY DECODED IMAGE INFO ====================

    println!("\n📊 Decoded Image Information:");
    println!("  Dimensions: {}x{} pixels",
        decoded_image.width(),
        decoded_image.height()
    );
    println!("  Channels: {:?} ({} channels)",
        decoded_image.channels,
        decoded_image.channel_count()
    );
    println!("  Pixel type: {:?}", decoded_image.pixel_type);
    println!("  Color encoding: {:?}", decoded_image.color_encoding);
    println!("  Total buffer size: {} samples", decoded_image.buffer.len());

    // Show header information if available
    if let Some(header) = decoder.header() {
        println!("\n📋 Header Information:");
        println!("  Bit depth: {} bits per channel", header.bit_depth);
        println!("  Orientation: {:?}", header.orientation);
        println!("  Animation: {}", header.is_animation);
        println!("  Number of channels: {}", header.num_channels);
    }

    // ==================== VERIFICATION ====================

    println!("\n✅ Verification:");
    println!("  Dimensions match: {}",
        decoded_image.width() == width && decoded_image.height() == height
    );
    println!("  Pixel type matches: {}",
        decoded_image.pixel_type == PixelType::U8
    );
    println!("  Color encoding matches: {}",
        decoded_image.color_encoding == ColorEncoding::SRGB
    );

    // ==================== COMPLETION ====================

    println!("\n✓ Example completed successfully!");
    println!("\n💡 Next Steps:");
    println!("  - Read LIMITATIONS.md to understand implementation scope");
    println!("  - See IMPLEMENTATION.md for technical architecture details");
    println!("  - Check BUILD-AND-TEST.md for development workflow");
    println!("  - Visit https://github.com/lamco-admin/jxl-rust-reference for more info");

    Ok(())
}
