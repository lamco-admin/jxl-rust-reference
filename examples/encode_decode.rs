//! Example demonstrating basic JPEG XL encoding and decoding

use jxl::{
    ColorChannels, ColorEncoding, Dimensions, EncoderOptions, Image, JxlDecoder, JxlEncoder,
    PixelType,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("JPEG XL Rust Reference Implementation - Example");
    println!("================================================\n");

    // Create a sample image
    let width = 256;
    let height = 256;
    let dimensions = Dimensions::new(width, height);

    println!("Creating a {}x{} test image...", width, height);

    let mut image = Image::new(
        dimensions,
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    )?;

    // Fill with a gradient pattern
    if let jxl::ImageBuffer::U8(ref mut buffer) = image.buffer {
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 3) as usize;
                buffer[idx] = (x % 256) as u8; // Red channel
                buffer[idx + 1] = (y % 256) as u8; // Green channel
                buffer[idx + 2] = ((x + y) / 2 % 256) as u8; // Blue channel
            }
        }
    }

    println!("Image created successfully!");

    // Encode the image
    println!("\nEncoding to JPEG XL...");
    let encoder_options = EncoderOptions::default().quality(90.0).effort(5);

    let encoder = JxlEncoder::new(encoder_options);
    let output_path = "/tmp/test_output.jxl";
    encoder.encode_file(&image, output_path)?;

    println!("Image encoded to: {}", output_path);

    // Decode the image
    println!("\nDecoding from JPEG XL...");
    let mut decoder = JxlDecoder::new();
    let decoded_image = decoder.decode_file(output_path)?;

    println!(
        "Image decoded successfully: {}x{}",
        decoded_image.width(),
        decoded_image.height()
    );
    println!("Channels: {:?}", decoded_image.channels);
    println!("Pixel type: {:?}", decoded_image.pixel_type);
    println!("Color encoding: {:?}", decoded_image.color_encoding);

    if let Some(header) = decoder.header() {
        println!("\nDecoded header information:");
        println!("  Bit depth: {}", header.bit_depth);
        println!("  Orientation: {:?}", header.orientation);
        println!("  Is animation: {}", header.is_animation);
    }

    println!("\nExample completed successfully!");

    Ok(())
}
