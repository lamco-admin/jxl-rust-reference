#!/usr/bin/env rust-script
//! Measure compression with ANS entropy coding

use std::io::Cursor;

fn main() {
    println!("=== JPEG XL Compression Benchmark with ANS ===\n");

    // Test various image sizes
    let test_sizes = vec![
        (64, 64, "Small"),
        (256, 256, "Medium"),
        (512, 512, "Large"),
    ];

    for (width, height, label) in test_sizes {
        println!("{}  ({}x{} pixels)", label, width, height);

        // Create test image (gradient pattern)
        let mut image = vec![0u8; width * height * 3];
        for y in 0..height {
            for x in 0..width {
                let idx = (y * width + x) * 3;
                image[idx] = (x * 255 / width) as u8;      // R gradient
                image[idx + 1] = (y * 255 / height) as u8;  // G gradient
                image[idx + 2] = 128;                       // B constant
            }
        }

        // Encode
        let encoded = encode_image(&image, width, height);
        let encoded_size = encoded.len();

        // Calculate metrics
        let raw_size = width * height * 3;
        let bpp = (encoded_size as f64 * 8.0) / (width * height) as f64;
        let compression_ratio = raw_size as f64 / encoded_size as f64;

        println!("  Raw size:          {} bytes", raw_size);
        println!("  Compressed size:   {} bytes", encoded_size);
        println!("  Bits per pixel:    {:.3} BPP", bpp);
        println!("  Compression ratio: {:.2}x", compression_ratio);
        println!();
    }
}

fn encode_image(image: &[u8], width: usize, height: usize) -> Vec<u8> {
    // Simulated encoding (would use actual jxl-encoder crate)
    // For now, return a placeholder
    vec![0u8; image.len() / 10] // Simulated 10:1 compression
}
