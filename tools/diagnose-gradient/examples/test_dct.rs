// Test if DCT/IDCT are perfectly invertible
use jxl_transform::{dct_channel, idct_channel};

fn main() {
    // Create a simple gradient
    let width = 8;
    let height = 8;
    let mut input = vec![0.0f32; width * height];
    for i in 0..(width * height) {
        input[i] = i as f32;
    }

    println!("Original input (first 8 values): {:?}", &input[0..8]);

    // Forward DCT
    let mut dct = vec![0.0f32; width * height];
    dct_channel(&input, width, height, &mut dct);

    println!("After DCT (first 8 values): {:?}", &dct[0..8]);

    // Inverse DCT
    let mut reconstructed = vec![0.0f32; width * height];
    idct_channel(&dct, width, height, &mut reconstructed);

    println!("After IDCT (first 8 values): {:?}", &reconstructed[0..8]);

    // Calculate error
    let mut max_error = 0.0f32;
    for i in 0..(width * height) {
        let err = (input[i] - reconstructed[i]).abs();
        max_error = max_error.max(err);
    }

    println!("\nMax reconstruction error (without quantization): {}", max_error);

    if max_error < 0.001 {
        println!("✓ DCT/IDCT are correctly invertible!");
    } else {
        println!("✗ DCT/IDCT have reconstruction error > 0.001!");
        println!("\nDetailed comparison:");
        for i in 0..8 {
            println!("  Position {}: {:.6} -> {:.6} (error: {:.6})",
                     i, input[i], reconstructed[i], input[i] - reconstructed[i]);
        }
    }
}
