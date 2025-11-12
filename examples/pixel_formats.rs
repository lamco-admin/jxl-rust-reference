//! # Pixel Format Examples
//!
//! This example demonstrates working with different pixel formats supported by the
//! JPEG XL reference implementation: U8, U16, and F32.
//!
//! ## Running This Example
//!
//! ```bash
//! cargo run --example pixel_formats
//! ```
//!
//! **Note:** This is an educational reference implementation. See LIMITATIONS.md
//! for details on implementation scope.

use jxl::{
    ColorChannels, ColorEncoding, Dimensions, EncoderOptions, Image, JxlEncoder, PixelType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("JPEG XL Pixel Format Examples");
    println!("==============================\n");

    let width = 64;
    let height = 64;
    let dimensions = Dimensions::new(width, height);

    // ==================== 8-BIT UNSIGNED INTEGER ====================
    println!("1. Creating U8 (8-bit) image...");

    let mut image_u8 = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )?;

    if let jxl::ImageBuffer::U8(ref mut buffer) = image_u8.buffer {
        for i in 0..buffer.len() {
            buffer[i] = (i % 256) as u8;
        }
    }

    let encoder = JxlEncoder::new(EncoderOptions::default());
    encoder.encode_file(&image_u8, "/tmp/test_u8.jxl")?;

    println!("   ✓ U8 image created and encoded");
    println!("   - Range: 0-255");
    println!("   - Memory per pixel: 3 bytes (RGB)");
    println!("   - Use case: Standard 8-bit images, photographs\n");

    // ==================== 16-BIT UNSIGNED INTEGER ====================
    println!("2. Creating U16 (16-bit) image...");

    let mut image_u16 = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U16,
        ColorEncoding::SRGB,
    )?;

    if let jxl::ImageBuffer::U16(ref mut buffer) = image_u16.buffer {
        for i in 0..buffer.len() {
            buffer[i] = (i % 65536) as u16;
        }
    }

    encoder.encode_file(&image_u16, "/tmp/test_u16.jxl")?;

    println!("   ✓ U16 image created and encoded");
    println!("   - Range: 0-65535");
    println!("   - Memory per pixel: 6 bytes (RGB)");
    println!("   - Use case: High bit depth images, medical imaging\n");

    // ==================== 32-BIT FLOATING POINT ====================
    println!("3. Creating F32 (32-bit float) image...");

    let mut image_f32 = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::F32,
        ColorEncoding::LinearSRGB,
    )?;

    if let jxl::ImageBuffer::F32(ref mut buffer) = image_f32.buffer {
        for i in 0..buffer.len() {
            // Normalize to 0.0-1.0 range
            buffer[i] = (i % 1000) as f32 / 1000.0;
        }
    }

    encoder.encode_file(&image_f32, "/tmp/test_f32.jxl")?;

    println!("   ✓ F32 image created and encoded");
    println!("   - Range: 0.0-1.0 (or beyond for HDR)");
    println!("   - Memory per pixel: 12 bytes (RGB)");
    println!("   - Use case: HDR images, scientific imaging, compositing\n");

    // ==================== GRAYSCALE EXAMPLE ====================
    println!("4. Creating grayscale image...");

    let mut image_gray = Image::new(
        dimensions,
        ColorChannels::Gray,
        PixelType::U8,
        ColorEncoding::SRGB,
    )?;

    if let jxl::ImageBuffer::U8(ref mut buffer) = image_gray.buffer {
        for i in 0..buffer.len() {
            buffer[i] = (i % 256) as u8;
        }
    }

    encoder.encode_file(&image_gray, "/tmp/test_gray.jxl")?;

    println!("   ✓ Grayscale image created and encoded");
    println!("   - Channels: 1 (Gray)");
    println!("   - Memory per pixel: 1 byte");
    println!("   - Use case: Black and white images, efficiency\n");

    // ==================== RGBA WITH ALPHA ====================
    println!("5. Creating RGBA image with alpha channel...");

    let mut image_rgba = Image::new(
        dimensions,
        ColorChannels::RGBA,
        PixelType::U8,
        ColorEncoding::SRGB,
    )?;

    if let jxl::ImageBuffer::U8(ref mut buffer) = image_rgba.buffer {
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 4) as usize;
                buffer[idx] = 255;     // R
                buffer[idx + 1] = 0;   // G
                buffer[idx + 2] = 0;   // B
                // Alpha: create gradient from transparent to opaque
                buffer[idx + 3] = ((x * 255) / width) as u8;
            }
        }
    }

    encoder.encode_file(&image_rgba, "/tmp/test_rgba.jxl")?;

    println!("   ✓ RGBA image created and encoded");
    println!("   - Channels: 4 (RGBA)");
    println!("   - Memory per pixel: 4 bytes");
    println!("   - Use case: Images with transparency\n");

    // ==================== SUMMARY ====================
    println!("Summary of Pixel Formats:");
    println!("========================");
    println!("| Format | Channels | Bytes/px | Range          | Use Case");
    println!("|--------|----------|----------|----------------|------------------");
    println!("| U8     | 3 (RGB)  | 3        | 0-255          | Standard images");
    println!("| U16    | 3 (RGB)  | 6        | 0-65535        | High bit depth");
    println!("| F32    | 3 (RGB)  | 12       | 0.0-1.0+       | HDR/Scientific");
    println!("| Gray   | 1        | 1        | 0-255          | B&W images");
    println!("| RGBA   | 4        | 4        | 0-255 + alpha  | Transparency");

    println!("\n✓ All pixel format examples completed!");
    println!("\n💡 Next: See examples/error_handling.rs for error handling patterns");

    Ok(())
}
