// Test AC coefficient encoding/decoding directly

use jxl_encoder::JxlEncoder;
use jxl_decoder::JxlDecoder;
use jxl_core::{Image, Dimensions, ColorChannels, PixelType, ColorEncoding, ImageBuffer};
use jxl_transform::{quantize_channel, dct_channel, generate_xyb_quant_tables, zigzag_scan_channel, separate_dc_ac};
use jxl_color::rgb_to_xyb;
use std::io::Cursor;

fn main() {
    println!("Testing AC coefficient preservation through encode/decode\n");

    // Create a simple 8x8 gradient (1 block)
    let width = 8u32;
    let height = 8u32;
    
    let mut original = Image::new(
        Dimensions::new(width, height),
        ColorChannels::RGB,
        PixelType::U8,
        ColorEncoding::SRGB,
    ).unwrap();

    if let ImageBuffer::U8(ref mut buffer) = original.buffer {
        for i in 0..64 {
            let val = (i * 4) as u8;
            buffer[i * 3] = val;
            buffer[i * 3 + 1] = val;
            buffer[i * 3 + 2] = val;
        }
    }

    // Manually extract and process to see AC coefficients
    let width_usize = width as usize;
    let height_usize = height as usize;

    // RGB to XYB
    let mut xyb = vec![0.0f32; (width_usize * height_usize * 3)];
    if let ImageBuffer::U8(ref buffer) = original.buffer {
        for i in 0..(width_usize * height_usize) {
            let r = buffer[i * 3] as f32 / 255.0;
            let g = buffer[i * 3 + 1] as f32 / 255.0;
            let b = buffer[i * 3 + 2] as f32 / 255.0;
            let (x, y, b_minus_y) = rgb_to_xyb(r, g, b);
            xyb[i * 3] = x;
            xyb[i * 3 + 1] = y;
            xyb[i * 3 + 2] = b_minus_y;
        }
    }

    // Extract Y channel, scale, DCT, quantize
    let mut y_channel = vec![0.0f32; width_usize * height_usize];
    for i in 0..(width_usize * height_usize) {
        y_channel[i] = xyb[i * 3 + 1] * 255.0;
    }

    let mut dct = vec![0.0f32; width_usize * height_usize];
    dct_channel(&y_channel, width_usize, height_usize, &mut dct);

    let tables = generate_xyb_quant_tables(85.0);
    let mut quantized = Vec::new();
    quantize_channel(&dct, width_usize, height_usize, &tables.y_table, &mut quantized);

    // Zigzag and separate DC/AC
    let mut zigzag = Vec::new();
    zigzag_scan_channel(&quantized, width_usize, height_usize, &mut zigzag);

    let (dc, ac) = separate_dc_ac(&zigzag);

    println!("Before encode:");
    println!("  DC: {:?}", dc);
    println!("  AC coeffs (first 20): {:?}", &ac[0..20.min(ac.len())]);
    println!("  AC non-zero count: {}", ac.iter().filter(|&&x| x != 0).count());

    // Now do full encode/decode
    let encoder = jxl_encoder::JxlEncoder::new(jxl_encoder::EncoderOptions::default().quality(85.0));
    let mut encoded = Vec::new();
    encoder.encode(&original, Cursor::new(&mut encoded)).unwrap();

    println!("\nEncoded size: {} bytes", encoded.len());

    let mut decoder = JxlDecoder::new();
    let decoded = decoder.decode(Cursor::new(&encoded)).unwrap();

    // Calculate PSNR
    let orig_buf = match &original.buffer {
        ImageBuffer::U8(b) => b,
        _ => panic!(),
    };
    let dec_buf = match &decoded.buffer {
        ImageBuffer::U8(b) => b,
        _ => panic!(),
    };

    let mut mse = 0.0;
    for i in 0..orig_buf.len() {
        let diff = orig_buf[i] as f64 - dec_buf[i] as f64;
        mse += diff * diff;
    }
    mse /= orig_buf.len() as f64;
    let psnr = 10.0 * ((255.0 * 255.0) / mse).log10();

    println!("PSNR: {:.2} dB", psnr);

    // Show pixel differences
    println!("\nPixel comparison (first 10):");
    for i in 0..10 {
        println!("  Pixel {}: {} -> {} (diff {})",
                 i, orig_buf[i * 3], dec_buf[i * 3], 
                 (orig_buf[i * 3] as i16 - dec_buf[i * 3] as i16).abs());
    }
}
