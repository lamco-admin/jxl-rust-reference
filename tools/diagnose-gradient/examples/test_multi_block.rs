// Test if PSNR degrades with more blocks

use jxl_encoder::{JxlEncoder, EncoderOptions};
use jxl_decoder::JxlDecoder;
use jxl_core::{Image, Dimensions, ColorChannels, PixelType, ColorEncoding, ImageBuffer};
use std::io::Cursor;

fn create_gradient(width: u32, height: u32) -> Image {
    let mut image = Image::new(
        Dimensions::new(width, height),
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    ).unwrap();

    if let ImageBuffer::U8(ref mut buffer) = image.buffer {
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 3) as usize;
                let val = ((x + y) * 255 / (width + height - 2)).min(255) as u8;
                buffer[idx] = val;
                buffer[idx + 1] = val;
                buffer[idx + 2] = val;
            }
        }
    }

    image
}

fn calculate_psnr(img1: &Image, img2: &Image) -> f64 {
    let data1 = match &img1.buffer {
        ImageBuffer::U8(buf) => buf,
        _ => panic!("Expected U8"),
    };
    let data2 = match &img2.buffer {
        ImageBuffer::U8(buf) => buf,
        _ => panic!("Expected U8"),
    };

    let mut mse = 0.0;
    for i in 0..data1.len() {
        let diff = data1[i] as f64 - data2[i] as f64;
        mse += diff * diff;
    }
    mse /= data1.len() as f64;

    10.0 * ((255.0 * 255.0) / mse).log10()
}

fn test_size(width: u32, height: u32) {
    let original = create_gradient(width, height);

    let mut encoder = JxlEncoder::new(EncoderOptions::default().quality(85.0));
    let mut encoded = Vec::new();
    encoder.encode(&original, Cursor::new(&mut encoded)).expect("encode failed");

    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(Cursor::new(&encoded)).expect("decode failed");

    let psnr = calculate_psnr(&original, &decoded);

    let num_blocks = ((width + 7) / 8) * ((height + 7) / 8);
    println!("{}x{} ({:2} blocks): PSNR = {:.2} dB, size = {} bytes",
             width, height, num_blocks, psnr, encoded.len());
}

fn main() {
    println!("Testing PSNR vs image size (all gradients):\n");

    test_size(8, 8);      // 1 block
    test_size(16, 16);    // 4 blocks
    test_size(24, 24);    // 9 blocks
    test_size(32, 32);    // 16 blocks
    test_size(64, 64);    // 64 blocks
}
