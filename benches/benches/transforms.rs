//! Benchmarks for JPEG XL transform operations
//!
//! Run with: cargo bench

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use jxl_transform::{dct_8x8, idct_8x8, predict_left, predict_average};

fn bench_dct(c: &mut Criterion) {
    let mut group = c.benchmark_group("DCT Transform");

    // Create test data
    let input: Vec<f32> = (0..64).map(|i| (i as f32) / 64.0).collect();

    group.bench_function("dct_8x8_forward", |b| {
        b.iter(|| {
            let mut output = vec![0.0f32; 64];
            dct_8x8(black_box(&input), black_box(&mut output));
        });
    });

    group.bench_function("dct_8x8_inverse", |b| {
        let mut dct_output = vec![0.0f32; 64];
        dct_8x8(&input, &mut dct_output);

        b.iter(|| {
            let mut output = vec![0.0f32; 64];
            idct_8x8(black_box(&dct_output), black_box(&mut output));
        });
    });

    group.bench_function("dct_8x8_roundtrip", |b| {
        b.iter(|| {
            let mut dct_output = vec![0.0f32; 64];
            let mut final_output = vec![0.0f32; 64];
            dct_8x8(black_box(&input), &mut dct_output);
            idct_8x8(&dct_output, black_box(&mut final_output));
        });
    });

    group.finish();
}

fn bench_prediction(c: &mut Criterion) {
    let mut group = c.benchmark_group("Prediction Modes");

    let width = 256;
    let height = 256;
    let image: Vec<u8> = (0..(width * height)).map(|i| (i % 256) as u8).collect();

    group.bench_with_input(BenchmarkId::new("predict_left", width), &width, |b, &w| {
        b.iter(|| {
            for y in 0..height {
                for x in 1..w {
                    let idx = y * w + x;
                    let _pred = predict_left(black_box(&image), black_box(x), black_box(y), black_box(w));
                }
            }
        });
    });

    group.bench_with_input(BenchmarkId::new("predict_average", width), &width, |b, &w| {
        b.iter(|| {
            for y in 1..height {
                for x in 1..w {
                    let idx = y * w + x;
                    let _pred = predict_average(black_box(&image), black_box(x), black_box(y), black_box(w));
                }
            }
        });
    });

    group.finish();
}

fn bench_color_transforms(c: &mut Criterion) {
    let mut group = c.benchmark_group("Color Transforms");

    group.bench_function("rgb_to_xyb", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let r = (i % 256) as f32 / 255.0;
                let g = ((i * 2) % 256) as f32 / 255.0;
                let b = ((i * 3) % 256) as f32 / 255.0;
                let _xyb = jxl_color::rgb_to_xyb(black_box(r), black_box(g), black_box(b));
            }
        });
    });

    group.bench_function("xyb_to_rgb", |b| {
        b.iter(|| {
            for i in 0..1000 {
                let x = (i % 256) as f32 / 255.0;
                let y = ((i * 2) % 256) as f32 / 255.0;
                let b = ((i * 3) % 256) as f32 / 255.0;
                let _rgb = jxl_color::xyb_to_rgb(black_box(x), black_box(y), black_box(b));
            }
        });
    });

    group.finish();
}

criterion_group!(benches, bench_dct, bench_prediction, bench_color_transforms);
criterion_main!(benches);
