//! DCT (Discrete Cosine Transform) implementation

use std::f32::consts::PI;

/// 8x8 DCT-II (forward transform)
pub fn dct8x8_forward(input: &[f32; 64], output: &mut [f32; 64]) {
    const N: usize = 8;

    for u in 0..N {
        for v in 0..N {
            let mut sum = 0.0;
            for x in 0..N {
                for y in 0..N {
                    let px = input[y * N + x];
                    let cu = if u == 0 { 1.0 / 2.0f32.sqrt() } else { 1.0 };
                    let cv = if v == 0 { 1.0 / 2.0f32.sqrt() } else { 1.0 };

                    sum += px
                        * (((2 * x + 1) as f32 * u as f32 * PI) / (2.0 * N as f32)).cos()
                        * (((2 * y + 1) as f32 * v as f32 * PI) / (2.0 * N as f32)).cos()
                        * cu
                        * cv;
                }
            }
            output[v * N + u] = sum * 2.0 / N as f32;
        }
    }
}

/// 8x8 DCT-III (inverse transform)
pub fn dct8x8_inverse(input: &[f32; 64], output: &mut [f32; 64]) {
    const N: usize = 8;

    for x in 0..N {
        for y in 0..N {
            let mut sum = 0.0;
            for u in 0..N {
                for v in 0..N {
                    let coeff = input[v * N + u];
                    let cu = if u == 0 { 1.0 / 2.0f32.sqrt() } else { 1.0 };
                    let cv = if v == 0 { 1.0 / 2.0f32.sqrt() } else { 1.0 };

                    sum += coeff
                        * cu
                        * cv
                        * (((2 * x + 1) as f32 * u as f32 * PI) / (2.0 * N as f32)).cos()
                        * (((2 * y + 1) as f32 * v as f32 * PI) / (2.0 * N as f32)).cos();
                }
            }
            output[y * N + x] = sum * 2.0 / N as f32;
        }
    }
}

/// Apply DCT to a channel
pub fn dct_channel(channel: &[f32], width: usize, height: usize, output: &mut [f32]) {
    assert_eq!(channel.len(), width * height);
    assert_eq!(output.len(), width * height);

    let mut block = [0.0f32; 64];
    let mut transformed = [0.0f32; 64];

    for block_y in (0..height).step_by(8) {
        for block_x in (0..width).step_by(8) {
            // Extract 8x8 block
            for y in 0..8.min(height - block_y) {
                for x in 0..8.min(width - block_x) {
                    block[y * 8 + x] = channel[(block_y + y) * width + (block_x + x)];
                }
            }

            // Apply forward DCT
            dct8x8_forward(&block, &mut transformed);

            // Store result
            for y in 0..8.min(height - block_y) {
                for x in 0..8.min(width - block_x) {
                    output[(block_y + y) * width + (block_x + x)] = transformed[y * 8 + x];
                }
            }
        }
    }
}

/// Apply inverse DCT to a channel
pub fn idct_channel(channel: &[f32], width: usize, height: usize, output: &mut [f32]) {
    assert_eq!(channel.len(), width * height);
    assert_eq!(output.len(), width * height);

    let mut block = [0.0f32; 64];
    let mut transformed = [0.0f32; 64];

    for block_y in (0..height).step_by(8) {
        for block_x in (0..width).step_by(8) {
            // Extract 8x8 block
            for y in 0..8.min(height - block_y) {
                for x in 0..8.min(width - block_x) {
                    block[y * 8 + x] = channel[(block_y + y) * width + (block_x + x)];
                }
            }

            // Apply inverse DCT
            dct8x8_inverse(&block, &mut transformed);

            // Store result
            for y in 0..8.min(height - block_y) {
                for x in 0..8.min(width - block_x) {
                    output[(block_y + y) * width + (block_x + x)] = transformed[y * 8 + x];
                }
            }
        }
    }
}
