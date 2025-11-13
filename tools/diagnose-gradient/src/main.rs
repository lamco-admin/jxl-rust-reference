// Deep diagnostic: Trace coefficients through encode-decode pipeline

use jxl_color::{rgb_to_xyb, xyb_to_rgb, linear_to_srgb};
use jxl_transform::{
    dct_channel, idct_channel, generate_xyb_quant_tables, quantize_channel, dequantize,
};
use jxl_core::consts::DEFAULT_QUALITY;

fn create_gradient_image(width: usize, height: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(width * height * 3);
    for y in 0..height {
        for x in 0..width {
            let val = ((x + y) * 255 / (width + height - 2)).min(255) as u8;
            data.push(val);
            data.push(val);
            data.push(val);
        }
    }
    data
}

fn main() {
    let width = 8;
    let height = 8;

    println!("=== Testing 8x8 gradient round-trip ===\n");

    let gradient = create_gradient_image(width, height);

    // Step 1: RGB to XYB
    let mut xyb = vec![0.0f32; width * height * 3];
    for i in 0..(width * height) {
        let r = gradient[i * 3] as f32 / 255.0;
        let g = gradient[i * 3 + 1] as f32 / 255.0;
        let b = gradient[i * 3 + 2] as f32 / 255.0;
        let (x, y, b_minus_y) = rgb_to_xyb(r, g, b);
        xyb[i * 3] = x;
        xyb[i * 3 + 1] = y;
        xyb[i * 3 + 2] = b_minus_y;
    }

    // Step 2: Scale XYB and apply DCT (Y channel only for simplicity)
    let mut y_channel = vec![0.0f32; width * height];
    for i in 0..(width * height) {
        y_channel[i] = xyb[i * 3 + 1] * 255.0;  // XYB scaling
    }

    println!("Y channel (scaled): min={:.2}, max={:.2}",
             y_channel.iter().copied().fold(f32::MAX, f32::min),
             y_channel.iter().copied().fold(f32::MIN, f32::max));

    let mut dct_coeffs = vec![0.0f32; width * height];
    dct_channel(&y_channel, width, height, &mut dct_coeffs);

    println!("DCT coeffs: min={:.2}, max={:.2}",
             dct_coeffs.iter().copied().fold(f32::MAX, f32::min),
             dct_coeffs.iter().copied().fold(f32::MIN, f32::max));
    println!("  First 8x8 block DCT coeffs:");
    for i in 0..8 {
        print!("    ");
        for j in 0..8 {
            print!("{:7.2} ", dct_coeffs[i * 8 + j]);
        }
        println!();
    }

    // Step 3: Quantize
    let xyb_tables = generate_xyb_quant_tables(DEFAULT_QUALITY);
    let mut quantized = Vec::new();
    quantize_channel(&dct_coeffs, width, height, &xyb_tables.y_table, &mut quantized);

    let nonzero = quantized.iter().filter(|&&x| x != 0).count();
    println!("\nQuantized: {} non-zero out of {}", nonzero, quantized.len());
    println!("  First 8x8 block quantized coeffs:");
    for i in 0..8 {
        print!("    ");
        for j in 0..8 {
            print!("{:4} ", quantized[i * 8 + j]);
        }
        println!();
    }

    // Step 4: Dequantize (simulating decoder)
    let mut dequant_block = [0i16; 64];
    let mut dequant_f32_block = [0.0f32; 64];
    for i in 0..64 {
        dequant_block[i] = quantized[i];
    }
    dequantize(&dequant_block, &xyb_tables.y_table, &mut dequant_f32_block);

    println!("\nDequantized coeffs:");
    for i in 0..8 {
        print!("    ");
        for j in 0..8 {
            print!("{:7.2} ", dequant_f32_block[i * 8 + j]);
        }
        println!();
    }

    // Step 5: IDCT
    let mut idct_result = vec![0.0f32; width * height];
    idct_channel(&dequant_f32_block.to_vec(), width, height, &mut idct_result);

    println!("\nIDCT result: min={:.2}, max={:.2}",
             idct_result.iter().copied().fold(f32::MAX, f32::min),
             idct_result.iter().copied().fold(f32::MIN, f32::max));

    // Step 6: Unscale and convert back to RGB
    for i in 0..(width * height) {
        idct_result[i] /= 255.0;  // Unscale
    }

    println!("After unscaling: min={:.6}, max={:.6}",
             idct_result.iter().copied().fold(f32::MAX, f32::min),
             idct_result.iter().copied().fold(f32::MIN, f32::max));

    // Step 7: XYB to RGB (using Y channel only, assuming X=0, B-Y=Y for grayscale)
    let mut reconstructed_rgb = vec![0u8; width * height * 3];
    for i in 0..(width * height) {
        let y_val = idct_result[i];
        // For grayscale: X=0, Y=luma, B-Y=Y
        let (r, g, b) = xyb_to_rgb(0.0, y_val, y_val);

        // Convert to sRGB and clamp
        let r_srgb = linear_to_srgb(r.clamp(0.0, 1.0));
        let g_srgb = linear_to_srgb(g.clamp(0.0, 1.0));
        let b_srgb = linear_to_srgb(b.clamp(0.0, 1.0));

        reconstructed_rgb[i * 3] = (r_srgb * 255.0).round().clamp(0.0, 255.0) as u8;
        reconstructed_rgb[i * 3 + 1] = (g_srgb * 255.0).round().clamp(0.0, 255.0) as u8;
        reconstructed_rgb[i * 3 + 2] = (b_srgb * 255.0).round().clamp(0.0, 255.0) as u8;
    }

    // Step 8: Compare
    println!("\nComparison (first 64 pixels):");
    println!("  Original -> Reconstructed (diff):");
    for i in 0..8 {
        let orig = gradient[i * 3];
        let recon = reconstructed_rgb[i * 3];
        let diff = (orig as i16 - recon as i16).abs();
        println!("    Pixel {}: {} -> {} (diff={})", i, orig, recon, diff);
    }

    // Calculate MSE
    let mut mse = 0.0;
    for i in 0..(width * height * 3) {
        let diff = gradient[i] as f32 - reconstructed_rgb[i] as f32;
        mse += diff * diff;
    }
    mse /= (width * height * 3) as f32;
    let psnr = 10.0 * (255.0 * 255.0 / mse).log10();

    println!("\nFinal PSNR: {:.2} dB", psnr);
}
